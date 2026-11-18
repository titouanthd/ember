//! Node, NodeId, NodeTree, NodeContext, PackedScene. The tree the game
//! lives in.
//!
//! Phase 4 status: only the trait skeletons and the two context stubs
//! exist. `NodeTree`, `NodeId`, `PackedScene`, signals machinery, and
//! the full contexts land in Phases 6–7.

pub mod context;
pub mod node;
pub mod scene;
pub mod signal;

// Re-export the crate under its own extern name so that
// `#[derive(Node)]`'s expansion (`::ember_scene::Node`) resolves even
// when applied to a struct inside `ember-scene` itself. This is the
// standard self-alias pattern (tokio, bytemuck, serde use it).
pub use crate as ember_scene;

pub use context::{DrawContext, NodeContext};
pub use node::Node;
pub use scene::SceneWriter;
pub use signal::Signal;

pub mod prelude {
    pub use crate::context::{DrawContext, NodeContext};
    pub use crate::node::Node;
    pub use crate::scene::SceneWriter;
    pub use crate::signal::Signal;
    pub use ember_core::Object;
}
