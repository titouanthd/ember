//! `Document`: one opened scene, editable without a window.

use crate::value;
use ember_core::{ClassDB, Variant};
use ember_scene::{
    ClassDbNodes, NodeId, NodeOrigins, NodeTree, PackedScene, SceneConnection, SceneError, Servers,
};
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
    /// `add_node` with a class the `ClassDB` cannot build as a node.
    UnknownClass(String),
    /// Empty, contains `/`, or is `.` / `..`.
    InvalidName(String),
    /// A sibling already has that name (scene paths must be unique).
    NameTaken(String),
    /// The root cannot be removed or moved.
    IsRoot,
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
            Self::UnknownClass(c) => write!(f, "no node class `{c}`"),
            Self::InvalidName(n) => write!(f, "`{n}` is not a valid node name"),
            Self::NameTaken(n) => write!(f, "a sibling is already called `{n}`"),
            Self::IsRoot => write!(f, "the root cannot be removed or moved"),
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

/// One undoable edit (ADR-045). Each holds both states, so redo and undo
/// run the same change either way. Removed and un-added nodes stay in
/// the document's tree, detached, so every id stays valid.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    SetProperty {
        node: NodeId,
        key: &'static str,
        before: Variant,
        after: Variant,
    },
    /// `node` was inserted once; it is attached at `index` under `parent`.
    AddNode {
        node: NodeId,
        parent: NodeId,
        index: usize,
    },
    /// `node` was at `index` under `parent`.
    RemoveNode {
        node: NodeId,
        parent: NodeId,
        index: usize,
    },
    RenameNode {
        node: NodeId,
        before: String,
        after: String,
    },
    /// From `(parent, index)` to `(parent, index)`.
    Reparent {
        node: NodeId,
        from: (NodeId, usize),
        to: (NodeId, usize),
    },
}

impl Command {
    fn run(&self, tree: &mut NodeTree, forward: bool) -> Result<(), EditorError> {
        match self {
            Self::SetProperty {
                node,
                key,
                before,
                after,
            } => {
                let n = tree.get_mut(*node).ok_or(EditorError::UnknownNode)?;
                if !n.set_property(key, if forward { after } else { before }) {
                    return Err(EditorError::Rejected(key.to_string()));
                }
            }
            Self::AddNode {
                node,
                parent,
                index,
            }
            | Self::RemoveNode {
                node,
                parent,
                index,
            } => {
                let attach = matches!(self, Self::AddNode { .. }) == forward;
                if attach {
                    tree.attach_at(*node, *parent, *index)?;
                } else {
                    tree.detach(*node)?;
                }
            }
            Self::RenameNode {
                node,
                before,
                after,
            } => {
                let n = tree.get_mut(*node).ok_or(EditorError::UnknownNode)?;
                n.set_name(if forward { after } else { before }.clone());
            }
            Self::Reparent { node, from, to } => {
                let (parent, index) = if forward { to } else { from };
                tree.detach(*node)?;
                tree.attach_at(*node, *parent, *index)?;
            }
        }
        Ok(())
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && name != "." && name != ".."
}

/// An opened scene: the nodes live in their own [`NodeTree`], built by
/// `PackedScene::instantiate_tracked` and saved with
/// `PackedScene::resave`, so ext resources, connections and properties
/// the nodes do not know come back on save (M4.1).
pub struct Document {
    classes: ClassDB,
    tree: NodeTree,
    root: NodeId,
    /// The scene as opened: its header (uid, ext resources,
    /// connections) is written back on save.
    base: PackedScene,
    origins: NodeOrigins,
    connections: Vec<SceneConnection>,
    path: Option<PathBuf>,
    /// Applied commands; `history[..cursor]` are done, the rest redoable.
    history: Vec<Command>,
    cursor: usize,
    /// `cursor` at the last open or save; `None` once that state was
    /// dropped from the stack.
    saved_at: Option<usize>,
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
        let t = scene.instantiate_tracked(&classes, &mut tree)?;
        tree.set_root(t.root);
        Ok(Self {
            classes,
            tree,
            root: t.root,
            base: scene.clone(),
            origins: t.origins,
            connections: t.connections,
            path: None,
            history: Vec::new(),
            cursor: 0,
            saved_at: Some(0),
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

    /// Edited since the last open or save (undoing back to it clears it).
    pub fn is_dirty(&self) -> bool {
        self.saved_at != Some(self.cursor)
    }

    /// The node classes `add_node` can build, sorted.
    pub fn addable_classes(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self
            .classes
            .names()
            .into_iter()
            .filter(|c| self.classes.instantiate_node(c).is_some())
            .collect();
        names.sort_unstable();
        names
    }

    /// The node is the root or under it (not removed).
    pub fn is_live(&self, id: NodeId) -> bool {
        let mut at = id;
        loop {
            if at == self.root {
                return self.tree.contains(id);
            }
            match self.tree.parent(at) {
                Some(p) => at = p,
                None => return false,
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor < self.history.len()
    }

    /// Done commands, oldest first.
    pub fn undo_stack(&self) -> &[Command] {
        &self.history[..self.cursor]
    }

    /// Run `cmd` and push it, dropping the redo tail.
    pub fn apply(&mut self, cmd: Command) -> Result<(), EditorError> {
        cmd.run(&mut self.tree, true)?;
        self.record(cmd);
        Ok(())
    }

    fn record(&mut self, cmd: Command) {
        self.history.truncate(self.cursor);
        if self.saved_at.is_some_and(|s| s > self.cursor) {
            self.saved_at = None;
        }
        self.history.push(cmd);
        self.cursor += 1;
    }

    /// Undo the last command; `false` if there is none.
    pub fn undo(&mut self) -> Result<bool, EditorError> {
        if !self.can_undo() {
            return Ok(false);
        }
        self.history[self.cursor - 1].run(&mut self.tree, false)?;
        self.cursor -= 1;
        Ok(true)
    }

    /// Redo the next command; `false` if there is none.
    pub fn redo(&mut self) -> Result<bool, EditorError> {
        if !self.can_redo() {
            return Ok(false);
        }
        self.history[self.cursor].run(&mut self.tree, true)?;
        self.cursor += 1;
        Ok(true)
    }

    fn live(&self, id: NodeId) -> Result<(), EditorError> {
        if self.is_live(id) {
            Ok(())
        } else {
            Err(EditorError::UnknownNode)
        }
    }

    fn sibling_named(&self, parent: NodeId, name: &str, except: NodeId) -> bool {
        self.tree
            .children(parent)
            .iter()
            .any(|&c| c != except && self.tree.get(c).is_some_and(|n| n.name() == name))
    }

    fn place(&self, id: NodeId) -> Result<(NodeId, usize), EditorError> {
        if id == self.root {
            return Err(EditorError::IsRoot);
        }
        self.live(id)?;
        let parent = self.tree.parent(id).ok_or(EditorError::UnknownNode)?;
        let index = self.tree.children(parent).iter().position(|&c| c == id);
        Ok((parent, index.unwrap_or(0)))
    }

    /// Add a new `class` node as the last child of `parent`, named after
    /// its class (`Sprite`, `Sprite2`…).
    pub fn add_node(&mut self, parent: NodeId, class: &str) -> Result<NodeId, EditorError> {
        self.live(parent)?;
        let mut node = self
            .classes
            .instantiate_node(class)
            .ok_or_else(|| EditorError::UnknownClass(class.to_string()))?;
        let mut name = class.to_string();
        let mut n = 2;
        while self.sibling_named(parent, &name, NodeId::default()) {
            name = format!("{class}{n}");
            n += 1;
        }
        node.set_name(name);
        let id = self.tree.insert_boxed(node);
        let index = self.tree.children(parent).len();
        self.apply(Command::AddNode {
            node: id,
            parent,
            index,
        })?;
        Ok(id)
    }

    /// Remove `id` and its subtree (undo puts them back).
    pub fn remove_node(&mut self, id: NodeId) -> Result<(), EditorError> {
        let (parent, index) = self.place(id)?;
        self.apply(Command::RemoveNode {
            node: id,
            parent,
            index,
        })
    }

    pub fn rename_node(&mut self, id: NodeId, name: &str) -> Result<(), EditorError> {
        self.live(id)?;
        if !valid_name(name) {
            return Err(EditorError::InvalidName(name.to_string()));
        }
        let before = self
            .tree
            .get(id)
            .ok_or(EditorError::UnknownNode)?
            .name()
            .to_string();
        if before == name {
            return Ok(());
        }
        if let Some(parent) = self.tree.parent(id)
            && self.sibling_named(parent, name, id)
        {
            return Err(EditorError::NameTaken(name.to_string()));
        }
        self.apply(Command::RenameNode {
            node: id,
            before,
            after: name.to_string(),
        })
    }

    /// Move `id` under `parent` at `index` (`None`: last).
    pub fn reparent(
        &mut self,
        id: NodeId,
        parent: NodeId,
        index: Option<usize>,
    ) -> Result<(), EditorError> {
        let from = self.place(id)?;
        self.live(parent)?;
        let mut at = Some(parent);
        while let Some(p) = at {
            if p == id {
                return Err(SceneError::Cycle { child: id, parent }.into());
            }
            at = self.tree.parent(p);
        }
        let name = self
            .tree
            .get(id)
            .ok_or(EditorError::UnknownNode)?
            .name()
            .to_string();
        if self.sibling_named(parent, &name, id) {
            return Err(EditorError::NameTaken(name));
        }
        let others = self
            .tree
            .children(parent)
            .iter()
            .filter(|&&c| c != id)
            .count();
        let to = (parent, index.unwrap_or(others).min(others));
        if to == from {
            return Ok(());
        }
        self.apply(Command::Reparent { node: id, from, to })
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

    /// Set an exported property (one undoable `SetProperty`). Setting
    /// the value it already has pushes nothing.
    pub fn set_property(&mut self, id: NodeId, key: &str, v: &Variant) -> Result<(), EditorError> {
        let node = self.tree.get_mut(id).ok_or(EditorError::UnknownNode)?;
        let key = node
            .property_list()
            .iter()
            .copied()
            .find(|&k| k == key)
            .ok_or_else(|| EditorError::UnknownProperty(key.to_string()))?;
        let before = node
            .get_property(key)
            .ok_or_else(|| EditorError::UnknownProperty(key.to_string()))?;
        if !node.set_property(key, v) {
            return Err(EditorError::Rejected(key.to_string()));
        }
        let after = node.get_property(key).unwrap_or_else(|| v.clone());
        if after != before {
            self.record(Command::SetProperty {
                node: id,
                key,
                before,
                after,
            });
        }
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
        Ok(self.base.resave_with(
            &self.tree,
            self.root,
            &self.classes,
            &self.origins,
            &self.connections,
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
        self.saved_at = Some(self.cursor);
        Ok(())
    }
}
