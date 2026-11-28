//! Node, NodeId, NodeTree, NodeContext, PackedScene. The tree the game
//! lives in.
//!
//! Phase 6 status: `NodeTree`, `NodeId`, the contexts and the
//! `SceneLoop` adapter exist. `PackedScene`, signals and `ClassDB` exist.

pub mod class_db;
mod context;
pub mod error;
pub mod node;
pub mod packed;
pub mod scene;
pub mod scene_loop;
pub mod signal;
pub mod tiles;
pub mod tree;

#[cfg(test)]
mod test_support;

// Re-export the crate under its own extern name so that
// `#[derive(Node)]`'s expansion (`::ember_scene::NodeName`) resolves even
// when applied to a struct inside `ember-scene` itself. This is the
// standard self-alias pattern (tokio, bytemuck, serde use it).
pub use crate as ember_scene;

pub use class_db::ClassDbNodes;
pub use context::{DrawContext, NodeContext, Servers};
pub use error::SceneError;
pub use node::{Node, NodeId, NodeName, PropertyHint};
pub use packed::{
    NodeOrigin, NodeOrigins, PackedScene, SceneAssets, SceneConnection, TrackedScene,
};
pub use scene::{NodeRecord, SceneWriter};
pub use scene_loop::{SceneLoop, VirtualResolution};
pub use signal::{Handler, ScopedConnection, Signal, SignalTable};
pub use tiles::{Tile, TileMap, TileSet};
pub use tree::NodeTree;

pub mod prelude {
    pub use crate::class_db::ClassDbNodes;
    pub use crate::context::{DrawContext, NodeContext, Servers};
    pub use crate::error::SceneError;
    pub use crate::node::{Node, NodeId, NodeName};
    pub use crate::packed::{PackedScene, SceneAssets};
    pub use crate::scene::{NodeRecord, SceneWriter};
    pub use crate::scene_loop::SceneLoop;
    pub use crate::signal::{ScopedConnection, Signal};
    pub use crate::tiles::{Tile, TileMap, TileSet};
    pub use crate::tree::NodeTree;
    pub use ember_core::Object;
}
