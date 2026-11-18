//! `PackedScene`: a parsed `.escene` file that instantiates into a
//! `NodeTree` (ADR-025).

use crate::{ClassDbNodes, NodeId, NodeTree, SceneError};
use ember_assets::{Asset, AssetError, AssetLoadContext, AssetServer, Handle};
use ember_core::{ClassDB, Variant};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

const SUPPORTED_FORMAT: u32 = 1;

fn yes() -> bool {
    true
}
fn is_yes(v: &bool) -> bool {
    *v
}
fn is_zero(v: &f32) -> bool {
    *v == 0.0
}
fn is_empty<T: AsRef<str>>(v: &T) -> bool {
    v.as_ref().is_empty()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ExtResource {
    #[serde(rename = "type")]
    ty: String,
    uid: String,
    path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SubResource {
    #[serde(rename = "type")]
    ty: String,
    id: String,
    #[serde(default)]
    props: BTreeMap<String, Variant>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct NodeEntry {
    name: String,
    #[serde(default, skip_serializing_if = "is_empty")]
    class: String,
    /// Empty for the root; otherwise `"."`, or a path below the root.
    #[serde(default, skip_serializing_if = "is_empty")]
    parent: String,
    /// Nested scene path, mutually exclusive with `class`.
    #[serde(default, skip_serializing_if = "is_empty")]
    instance: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    properties: BTreeMap<String, Variant>,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    visible: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    z: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Connection {
    signal: String,
    from: String,
    to: String,
    method: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SceneFile {
    format: u32,
    uid: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ext_resources: Vec<ExtResource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sub_resources: Vec<SubResource>,
    nodes: Vec<NodeEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    connections: Vec<Connection>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PackedScene {
    file: SceneFile,
    /// Full path of each node (`Root/Child`), parallel to `file.nodes`.
    paths: Vec<String>,
}

impl Asset for PackedScene {
    fn extensions() -> &'static [&'static str] {
        &["escene"]
    }

    fn load(bytes: &[u8], _ctx: &mut AssetLoadContext) -> Result<Self, AssetError> {
        let text = std::str::from_utf8(bytes).map_err(|e| AssetError::Parse(e.to_string()))?;
        Self::from_ron(text).map_err(|e| AssetError::Parse(e.to_string()))
    }
}

/// `assets.scene("player.escene")`.
pub trait SceneAssets {
    fn scene(&self, path: &str) -> Result<Handle<PackedScene>, AssetError>;
}

impl SceneAssets for AssetServer {
    fn scene(&self, path: &str) -> Result<Handle<PackedScene>, AssetError> {
        self.load(path)
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && name != ".." && name != "."
}

impl PackedScene {
    pub fn from_ron(text: &str) -> Result<Self, SceneError> {
        let file: SceneFile = ron::from_str(text).map_err(|e| SceneError::Parse(e.to_string()))?;
        if file.format > SUPPORTED_FORMAT {
            return Err(SceneError::UnsupportedFormat(file.format));
        }
        let paths = Self::resolve_paths(&file)?;
        for c in &file.connections {
            for end in [&c.from, &c.to] {
                if Self::lookup(&file, &paths, end).is_none() {
                    return Err(SceneError::Parse(format!(
                        "connection `{}` references unknown node `{end}`",
                        c.signal
                    )));
                }
            }
        }
        Ok(Self { file, paths })
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, SceneError> {
        let text = std::fs::read_to_string(path.as_ref())
            .map_err(|e| SceneError::Io(format!("{}: {e}", path.as_ref().display())))?;
        Self::from_ron(&text)
    }

    pub fn to_ron(&self) -> String {
        let cfg = ron::ser::PrettyConfig::default();
        ron::ser::to_string_pretty(&self.file, cfg).expect("a scene always serializes")
    }

    pub fn uid(&self) -> &str {
        &self.file.uid
    }

    pub fn root_template_class(&self) -> &str {
        &self.file.nodes[0].class
    }

    fn resolve_paths(file: &SceneFile) -> Result<Vec<String>, SceneError> {
        let first = file
            .nodes
            .first()
            .ok_or_else(|| SceneError::Parse("scene has no nodes".into()))?;
        if !first.parent.is_empty() {
            return Err(SceneError::Parse("first node must be the root".into()));
        }
        let mut paths: Vec<String> = Vec::with_capacity(file.nodes.len());
        for (i, n) in file.nodes.iter().enumerate() {
            if !valid_name(&n.name) {
                return Err(SceneError::InvalidName(n.name.clone()));
            }
            if n.class.is_empty() == n.instance.is_empty() {
                return Err(SceneError::Parse(format!(
                    "node `{}` needs exactly one of `class` and `instance`",
                    n.name
                )));
            }
            if i == 0 {
                paths.push(n.name.clone());
                continue;
            }
            let root = &paths[0];
            let parent = match n.parent.as_str() {
                "" => {
                    return Err(SceneError::Parse(format!(
                        "node `{}` has no parent but the root already exists",
                        n.name
                    )));
                }
                "." => root.clone(),
                p => format!("{root}/{p}"),
            };
            if !paths[..i].contains(&parent) {
                return Err(SceneError::Parse(format!(
                    "parent `{}` of `{}` is not an earlier node",
                    n.parent, n.name
                )));
            }
            let full = format!("{parent}/{}", n.name);
            if paths.contains(&full) {
                return Err(SceneError::InvalidName(format!(
                    "{} (duplicate under `{}`)",
                    n.name, n.parent
                )));
            }
            paths.push(full);
        }
        Ok(paths)
    }

    /// Resolves a connection endpoint (`"."`, `"Name/Child"`) to a node index.
    fn lookup(file: &SceneFile, paths: &[String], rel: &str) -> Option<usize> {
        let _ = file;
        let full = if rel == "." {
            paths[0].clone()
        } else {
            format!("{}/{rel}", paths[0])
        };
        paths.iter().position(|p| *p == full)
    }

    /// Builds the scene's nodes in `tree`. The root is returned
    /// detached; the caller attaches it or calls `set_root`.
    ///
    /// Unknown or mistyped properties are skipped with a warning (they
    /// stay in the template). Connections are validated but not wired:
    /// see ADR-025. Nodes with `instance:` fail with `Asset`; use
    /// [`Self::instantiate_with`] for those.
    pub fn instantiate(
        &self,
        classes: &ClassDB,
        tree: &mut NodeTree,
    ) -> Result<NodeId, SceneError> {
        self.build(classes, None, tree, &mut Vec::new())
    }

    /// Like [`Self::instantiate`], resolving `instance:` entries through
    /// `assets`. Cycles are rejected with `InstanceCycle`.
    pub fn instantiate_with(
        &self,
        classes: &ClassDB,
        assets: &AssetServer,
        tree: &mut NodeTree,
    ) -> Result<NodeId, SceneError> {
        self.build(classes, Some(assets), tree, &mut Vec::new())
    }

    fn build(
        &self,
        classes: &ClassDB,
        assets: Option<&AssetServer>,
        tree: &mut NodeTree,
        chain: &mut Vec<String>,
    ) -> Result<NodeId, SceneError> {
        if chain.contains(&self.file.uid) {
            chain.push(self.file.uid.clone());
            return Err(SceneError::InstanceCycle(chain.join(" -> ")));
        }
        for n in &self.file.nodes {
            if n.instance.is_empty() && !classes.is_registered(&n.class) {
                return Err(SceneError::ClassNotRegistered(n.class.clone()));
            }
        }
        chain.push(self.file.uid.clone());
        let root_path = self.paths[0].as_str();
        let mut ids: HashMap<&str, NodeId> = HashMap::new();
        let mut result = Ok(());
        for (n, path) in self.file.nodes.iter().zip(&self.paths) {
            match self.build_node(n, path, classes, assets, tree, chain, &ids) {
                Ok(id) => {
                    ids.insert(path, id);
                }
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        chain.pop();
        match result {
            Ok(()) => Ok(ids[root_path]),
            Err(e) => {
                if let Some(&root) = ids.get(root_path) {
                    tree.remove(root);
                }
                Err(e)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_node(
        &self,
        n: &NodeEntry,
        path: &str,
        classes: &ClassDB,
        assets: Option<&AssetServer>,
        tree: &mut NodeTree,
        chain: &mut Vec<String>,
        ids: &HashMap<&str, NodeId>,
    ) -> Result<NodeId, SceneError> {
        let id = if n.instance.is_empty() {
            let node = classes
                .instantiate_node(&n.class)
                .ok_or_else(|| SceneError::ClassNotRegistered(n.class.clone()))?;
            tree.insert_boxed(node)
        } else {
            let assets = assets.ok_or_else(|| {
                SceneError::Asset(format!(
                    "node `{}` instances `{}` but no AssetServer was given",
                    n.name, n.instance
                ))
            })?;
            let handle = assets
                .load::<PackedScene>(&n.instance)
                .map_err(|e| SceneError::Asset(format!("{}: {e}", n.instance)))?;
            let nested = handle.get();
            nested.build(classes, Some(assets), tree, chain)?
        };
        let finish = |tree: &mut NodeTree| -> Result<(), SceneError> {
            let node = tree.get_mut(id).ok_or(SceneError::NotFound(id))?;
            node.set_name(n.name.clone());
            for (key, value) in &n.properties {
                if !node.set_property(key, value) {
                    eprintln!(
                        "warning: scene `{}`: node `{}` ignores property `{key}`",
                        self.file.uid, n.name
                    );
                }
            }
            if !n.visible {
                node.set_property("visible", &Variant::Bool(false));
            }
            if n.z != 0.0 {
                node.set_property("z", &Variant::Float(f64::from(n.z)));
            }
            if let Some((parent, _)) = path.rsplit_once('/') {
                tree.attach(id, ids[parent])?;
            }
            Ok(())
        };
        if let Err(e) = finish(tree) {
            tree.remove(id);
            return Err(e);
        }
        Ok(id)
    }

    pub fn instantiate_into(
        &self,
        classes: &ClassDB,
        tree: &mut NodeTree,
        parent: NodeId,
    ) -> Result<NodeId, SceneError> {
        let root = self.instantiate(classes, tree)?;
        if let Err(e) = tree.attach(root, parent) {
            tree.remove(root);
            return Err(e);
        }
        Ok(root)
    }
}
