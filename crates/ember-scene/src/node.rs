//! The `Node` trait. Every scene entity implements this.

use crate::{DrawContext, NodeContext, SceneWriter};
use ember_core::Object;
use ember_core::Variant;

slotmap::new_key_type! {
    /// Generational handle to a node in a `NodeTree`. `Copy`, hashable,
    /// and stale-safe: after the node is removed, `tree.get(id)` is
    /// `None`. `NodeId::default()` is the null id; it is never issued.
    pub struct NodeId;
}

impl NodeId {
    pub fn is_null(self) -> bool {
        self == Self::default()
    }
}

pub trait Node: Object {
    /// The instance's name, used by `NodeTree::find` and `.escene`
    /// serialization.
    fn name(&self) -> &str;

    /// Set the instance's name. Called by `NodeTree::attach` and by the
    /// scene loader.
    fn set_name(&mut self, name: String);

    /// Draw layer, forwarded to `RenderingServer::set_z` before `draw`.
    fn z(&self) -> f32 {
        0.0
    }

    /// `false` hides this node and its whole subtree from `draw`;
    /// `process` still runs.
    fn visible(&self) -> bool {
        true
    }

    // --- Lifecycle: all defaulted ---

    fn ready(&mut self, _ctx: &mut NodeContext) {}
    fn process(&mut self, _dt: f32, _ctx: &mut NodeContext) {}
    fn physics_process(&mut self, _fixed_dt: f32, _ctx: &mut NodeContext) {}
    fn draw(&self, _ctx: &mut DrawContext) {}
    fn finalize(&mut self) {}

    /// Sets an exported property from a scene file. Returns `false`
    /// for an unknown key or a wrongly typed value (see ADR-025).
    fn set_property(&mut self, _key: &str, _value: &Variant) -> bool {
        false
    }

    /// Serialize `#[export]` fields. Stub in v0.1 — Phase 11 fills in
    /// the `.escene` writer.
    fn serialize(&self, _out: &mut SceneWriter) {}
}
