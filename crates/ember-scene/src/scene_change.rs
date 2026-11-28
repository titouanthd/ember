//! Runtime scene change (ADR-054): a node asks for the current scene to
//! be replaced; whoever owns the frame boundary (`SceneLoop`) applies it
//! with `NodeTree::apply_scene_change`, never mid-pass.

use crate::{NodeId, NodeTree, PackedScene, SceneError};
use ember_assets::AssetServer;
use ember_core::ClassDB;
use std::fmt;
use std::sync::Arc;

/// What a scene change may build with: the `ClassDB` and `AssetServer`
/// the loop was given (`SceneLoop::with_class_db` / `with_assets`).
/// Nodes have no `AssetServer`; this is lent to the builder only while
/// the change is applied.
#[derive(Clone, Copy, Default)]
pub struct SceneLibrary<'a> {
    pub classes: Option<&'a ClassDB>,
    pub assets: Option<&'a AssetServer>,
}

impl<'a> SceneLibrary<'a> {
    pub fn new(classes: Option<&'a ClassDB>, assets: Option<&'a AssetServer>) -> Self {
        Self { classes, assets }
    }

    /// The `ClassDB`, or `SceneError::SceneChange` if none was given.
    pub fn classes(&self) -> Result<&'a ClassDB, SceneError> {
        self.classes.ok_or_else(|| {
            SceneError::SceneChange("no ClassDB to build with (SceneLoop::with_class_db)".into())
        })
    }

    /// The `AssetServer`, or `SceneError::SceneChange` if none was given.
    pub fn assets(&self) -> Result<&'a AssetServer, SceneError> {
        self.assets.ok_or_else(|| {
            SceneError::SceneChange("no AssetServer to load with (SceneLoop::with_assets)".into())
        })
    }

    /// Build `scene` in `tree`, detached: with the `AssetServer` if there
    /// is one (`instance:` entries resolve), else without.
    pub fn instantiate(
        &self,
        scene: &PackedScene,
        tree: &mut NodeTree,
    ) -> Result<NodeId, SceneError> {
        let classes = self.classes()?;
        match self.assets {
            Some(assets) => scene.instantiate_with(classes, assets, tree),
            None => scene.instantiate(classes, tree),
        }
    }

    /// Load the `.escene` at `path` through the `AssetServer`, then
    /// [`instantiate`](Self::instantiate) it.
    pub fn instantiate_path(&self, path: &str, tree: &mut NodeTree) -> Result<NodeId, SceneError> {
        let handle = self
            .assets()?
            .load::<PackedScene>(path)
            .map_err(|e| SceneError::Asset(format!("{path}: {e}")))?;
        let scene = handle.get();
        self.instantiate(&scene, tree)
    }
}

/// Builds the next scene's root as a floating subtree of the tree it is
/// given, and returns it. It must not touch the old scene's nodes.
pub type SceneBuilder = Box<
    dyn for<'a> FnOnce(&mut NodeTree, &SceneLibrary<'a>) -> Result<NodeId, SceneError>
        + Send
        + Sync,
>;

/// A requested scene change (`NodeContext::change_scene`).
pub enum SceneChange {
    /// Instanced with the loop's `ClassDB` (and `AssetServer`, if any).
    Packed(Arc<PackedScene>),
    /// An `.escene` path, loaded through the loop's `AssetServer`.
    Path(String),
    /// Anything else: instance a scene and set it up, or build nodes by
    /// hand.
    Build(SceneBuilder),
}

impl SceneChange {
    pub fn build(
        f: impl for<'a> FnOnce(&mut NodeTree, &SceneLibrary<'a>) -> Result<NodeId, SceneError>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self::Build(Box::new(f))
    }

    pub(crate) fn run(
        self,
        tree: &mut NodeTree,
        library: &SceneLibrary,
    ) -> Result<NodeId, SceneError> {
        match self {
            Self::Packed(scene) => library.instantiate(&scene, tree),
            Self::Path(path) => library.instantiate_path(&path, tree),
            Self::Build(f) => f(tree, library),
        }
    }
}

impl fmt::Debug for SceneChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Packed(s) => f.debug_tuple("Packed").field(&s.uid()).finish(),
            Self::Path(p) => f.debug_tuple("Path").field(p).finish(),
            Self::Build(_) => f.write_str("Build(..)"),
        }
    }
}

impl From<PackedScene> for SceneChange {
    fn from(scene: PackedScene) -> Self {
        Self::Packed(Arc::new(scene))
    }
}

impl From<Arc<PackedScene>> for SceneChange {
    fn from(scene: Arc<PackedScene>) -> Self {
        Self::Packed(scene)
    }
}

impl From<&str> for SceneChange {
    fn from(path: &str) -> Self {
        Self::Path(path.to_string())
    }
}

impl From<String> for SceneChange {
    fn from(path: String) -> Self {
        Self::Path(path)
    }
}
