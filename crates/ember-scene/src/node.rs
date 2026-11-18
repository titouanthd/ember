//! The `Node` trait. Every scene entity implements this.

use crate::{DrawContext, NodeContext, SceneWriter};
use ember_core::Object;

pub trait Node: Object {
    /// The instance's name, used by `NodeTree::find` and `.escene`
    /// serialization.
    fn name(&self) -> &str;

    /// Set the instance's name. Called by `NodeTree::attach` and by the
    /// scene loader.
    fn set_name(&mut self, name: String);

    // --- Lifecycle: all defaulted ---

    fn ready(&mut self, _ctx: &mut NodeContext) {}
    fn process(&mut self, _dt: f32, _ctx: &mut NodeContext) {}
    fn physics_process(&mut self, _fixed_dt: f32, _ctx: &mut NodeContext) {}
    fn draw(&self, _ctx: &mut DrawContext) {}
    fn finalize(&mut self) {}

    /// Serialize `#[export]` fields. Stub in v0.1 — Phase 11 fills in
    /// the `.escene` writer.
    fn serialize(&self, _out: &mut SceneWriter) {}
}
