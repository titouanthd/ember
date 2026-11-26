//! `Document`: one opened scene, editable without a window.

use crate::value;
use ember_core::{ClassDB, Variant};
use ember_scene::{NodeId, NodeTree, PackedScene, SceneError, Servers};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum EditorError {
    Scene(SceneError),
    Io(String),
    /// The node id is not in the document.
    UnknownNode,
    /// The node has no exported property of that name.
    UnknownProperty(String),
    /// The text does not parse as the property's kind.
    BadValue {
        key: String,
        expected: &'static str,
    },
    /// The node refused a value of the right kind (e.g. out of range).
    Rejected(String),
    /// `save` on a document that has no path yet.
    NoPath,
}

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scene(e) => write!(f, "{e}"),
            Self::Io(m) => write!(f, "{m}"),
            Self::UnknownNode => write!(f, "no such node"),
            Self::UnknownProperty(k) => write!(f, "no property `{k}`"),
            Self::BadValue { key, expected } => write!(f, "`{key}` expects {expected}"),
            Self::Rejected(k) => write!(f, "`{k}` rejected that value"),
            Self::NoPath => write!(f, "the scene has no file yet"),
        }
    }
}

impl std::error::Error for EditorError {}

impl From<SceneError> for EditorError {
    fn from(e: SceneError) -> Self {
        Self::Scene(e)
    }
}

/// One line of the outline (node tree panel).
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineRow {
    pub id: NodeId,
    pub name: String,
    pub class: &'static str,
    /// 0 for the root.
    pub depth: usize,
}

/// An opened scene: the nodes live in their own [`NodeTree`], built by
/// `PackedScene::instantiate` and saved with `PackedScene::from_tree`.
pub struct Document {
    classes: ClassDB,
    tree: NodeTree,
    root: NodeId,
    uid: String,
    path: Option<PathBuf>,
    dirty: bool,
}

impl Document {
    /// Run `ready` on the scene's nodes, as a game's first frame does,
    /// so nodes that set themselves up there (children, fonts) preview
    /// as they play. Does not mark the document dirty; a node that
    /// rewrites its own exported fields in `ready` changes what Save
    /// writes.
    pub fn ready(&mut self, servers: &mut Servers) {
        self.tree.flush(servers);
    }

    /// Instantiate `scene` with `classes` (no file attached).
    pub fn from_scene(scene: &PackedScene, classes: ClassDB) -> Result<Self, EditorError> {
        let mut tree = NodeTree::new();
        let root = scene.instantiate(&classes, &mut tree)?;
        tree.set_root(root);
        Ok(Self {
            classes,
            tree,
            root,
            uid: scene.uid().to_string(),
            path: None,
            dirty: false,
        })
    }

    pub fn open(path: impl AsRef<Path>, classes: ClassDB) -> Result<Self, EditorError> {
        let path = path.as_ref();
        let scene = PackedScene::from_file(path)?;
        let mut doc = Self::from_scene(&scene, classes)?;
        doc.path = Some(path.to_path_buf());
        Ok(doc)
    }

    pub fn tree(&self) -> &NodeTree {
        &self.tree
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Edited since the last open or save.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Every node, depth first, as the outline panel lists them.
    pub fn outline(&self) -> Vec<OutlineRow> {
        let mut rows = Vec::new();
        self.outline_from(self.root, 0, &mut rows);
        rows
    }

    fn outline_from(&self, id: NodeId, depth: usize, rows: &mut Vec<OutlineRow>) {
        let Some(node) = self.tree.get(id) else {
            return;
        };
        rows.push(OutlineRow {
            id,
            name: node.name().to_string(),
            class: self.class_of(id).unwrap_or("?"),
            depth,
        });
        for &child in self.tree.children(id) {
            self.outline_from(child, depth + 1, rows);
        }
    }

    /// The node's class, as registered in the `ClassDB`.
    pub fn class_of(&self, id: NodeId) -> Option<&'static str> {
        let node = self.tree.get(id)?;
        self.classes.class_of(std::any::Any::type_id(node.as_any()))
    }

    /// `"Sprite/Gun"`-style path below the root (`""` for the root).
    pub fn find(&self, path: &str) -> Option<NodeId> {
        if path.is_empty() {
            return Some(self.root);
        }
        self.tree.find(self.root, path)
    }

    /// The node's exported properties and their values, in declaration
    /// order.
    pub fn properties(&self, id: NodeId) -> Result<Vec<(&'static str, Variant)>, EditorError> {
        let node = self.tree.get(id).ok_or(EditorError::UnknownNode)?;
        Ok(node
            .property_list()
            .iter()
            .filter_map(|&k| Some((k, node.get_property(k)?)))
            .collect())
    }

    /// The property as the inspector shows it.
    pub fn property_text(&self, id: NodeId, key: &str) -> Result<String, EditorError> {
        let node = self.tree.get(id).ok_or(EditorError::UnknownNode)?;
        let v = node
            .get_property(key)
            .ok_or_else(|| EditorError::UnknownProperty(key.to_string()))?;
        Ok(value::format(&v))
    }

    pub fn set_property(&mut self, id: NodeId, key: &str, v: &Variant) -> Result<(), EditorError> {
        let node = self.tree.get_mut(id).ok_or(EditorError::UnknownNode)?;
        if node.get_property(key).is_none() {
            return Err(EditorError::UnknownProperty(key.to_string()));
        }
        if !node.set_property(key, v) {
            return Err(EditorError::Rejected(key.to_string()));
        }
        self.dirty = true;
        Ok(())
    }

    /// Parse `text` as the property's current kind and set it.
    pub fn set_property_text(
        &mut self,
        id: NodeId,
        key: &str,
        text: &str,
    ) -> Result<(), EditorError> {
        let node = self.tree.get(id).ok_or(EditorError::UnknownNode)?;
        let current = node
            .get_property(key)
            .ok_or_else(|| EditorError::UnknownProperty(key.to_string()))?;
        let v = value::parse_like(&current, text).map_err(|expected| EditorError::BadValue {
            key: key.to_string(),
            expected,
        })?;
        self.set_property(id, key, &v)
    }

    /// The document as a scene (what `save` writes).
    pub fn to_scene(&self) -> Result<PackedScene, EditorError> {
        Ok(PackedScene::from_tree(
            &self.tree,
            self.root,
            &self.classes,
            self.uid.clone(),
        )?)
    }

    /// Write to the file it was opened from.
    pub fn save(&mut self) -> Result<(), EditorError> {
        let path = self.path.clone().ok_or(EditorError::NoPath)?;
        self.save_as(path)
    }

    /// Write to `path` and make it the document's file.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<(), EditorError> {
        let path = path.as_ref();
        let text = self.to_scene()?.to_ron();
        std::fs::write(path, text)
            .map_err(|e| EditorError::Io(format!("{}: {e}", path.display())))?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }
}
