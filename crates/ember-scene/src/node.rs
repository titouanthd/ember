//! The `Node` trait. Every scene entity implements this.

use crate::{DrawContext, NodeContext};
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

/// The half of a node that `#[derive(Node)]` writes: the instance name
/// and the `#[export]` properties. Split from [`Node`] so the user writes
/// the lifecycle in their own `impl Node` block (ADR-030, ADR-037).
///
/// Hand-written nodes implement `name` / `set_name` and, if they load
/// from or save to `.escene` files, the three property methods.
pub trait NodeName {
    /// The instance's name, used by `NodeTree::find` and `.escene`
    /// serialization.
    fn name(&self) -> &str;

    /// Set the instance's name. Called by `NodeTree::attach` and by the
    /// scene loader.
    fn set_name(&mut self, name: String);

    /// Exported property names, in declaration order. The scene writer
    /// saves exactly these; an editor lists them.
    fn property_list(&self) -> &'static [&'static str] {
        &[]
    }

    /// The current value of an exported property; `None` for an unknown
    /// key.
    fn get_property(&self, _key: &str) -> Option<Variant> {
        None
    }

    /// Set an exported property, e.g. from a scene file. Returns `false`
    /// for an unknown key or a wrongly typed value (ADR-025).
    fn set_property(&mut self, _key: &str, _value: &Variant) -> bool {
        false
    }
}

/// Lifecycle of a scene entity (properties live on [`NodeName`]). Every
/// method has a default,
/// so a node without behaviour is `impl Node for T {}`.
///
/// ```ignore
/// #[derive(Node)]                  // Object + NodeName (+ properties)
/// #[class_name = "Player"]
/// struct Player { name: String, #[export] speed: f32 }
///
/// impl Node for Player {           // only what the node does
///     fn process(&mut self, dt: f32, ctx: &mut NodeContext) { /* … */ }
/// }
/// ```
pub trait Node: Object + NodeName {
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
    /// Runs after this node's children have drawn (and only when `draw`
    /// ran, i.e. the node is visible). The server's z is reset to `z()`
    /// first. Use it to close what `draw` opened for the subtree, such as
    /// a clip rect, or to draw on top of the children (ADR-035).
    fn draw_after_children(&self, _ctx: &mut DrawContext) {}
    fn finalize(&mut self) {}
}
