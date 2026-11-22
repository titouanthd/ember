//! Retained-mode UI on top of the scene tree: `Control` nodes, anchors,
//! themes and a `UiRoot` that lays its subtree out once per frame.
//!
//! See `ember-vault/10 Engine Spec v0.1/24-ember-ui.md` (frozen) and
//! ADR-034 for how the spec maps onto `ember_scene`:
//!
//! - every control embeds a [`ControlBase`] and implements [`Control`];
//! - [`control_node!`] writes its `impl Node` (z, visibility, draw);
//! - [`UiRoot`] owns the [`Theme`] and a [`ControlRegistry`] that turns
//!   tree nodes back into controls for layout and input;
//! - [`UiRoot`] also routes input, moves focus and emits control signals
//!   such as [`ButtonClicked`] (ADR-036).
//!
//! ```ignore
//! #[derive(Node)]
//! #[class_name = "Badge"]
//! struct Badge { name: String, base: ControlBase }
//!
//! impl Control for Badge {
//!     fn base(&self) -> &ControlBase { &self.base }
//!     fn base_mut(&mut self) -> &mut ControlBase { &mut self.base }
//!     fn draw_control(&self, ui: &mut UiDrawContext) {
//!         let bg = ui.theme().panel_bg;
//!         ui.draw_background(self.rect(), bg);
//!     }
//! }
//! ember_ui::control_node!(Badge);
//! ```

mod control;
mod draw;
mod focus;
pub mod layout;
mod measure;
mod registry;
mod root;
mod route;
mod signals;
mod theme;
pub mod widgets;

pub use control::{ChildLayout, Control, ControlBase, ControlExt, draw_control_node};
pub use draw::UiDrawContext;
pub use focus::FocusManager;
pub use layout::{Anchors, HBox, Margins, Stretch, StretchMode, VBox};
pub use measure::Measure;
pub use registry::ControlRegistry;
pub use root::UiRoot;
pub use signals::{ButtonClicked, FocusChanged};
pub use theme::Theme;
pub use widgets::{Alignment, Button, ButtonState, Image, ImageMode, Label, Panel};

/// Draw layer of every UI node: above the game world (spec §8).
pub const UI_Z: f32 = 100.0;

/// Paths used by [`control_node!`]; not part of the public API.
#[doc(hidden)]
pub mod __private {
    pub use ember_scene::{DrawContext, Node};
}

/// Implement `ember_scene::Node` for a control: UI z-layer, visibility from
/// [`ControlBase::visible`], and `draw` forwarding to
/// [`Control::draw_control`]. Use it instead of writing `impl Node` by hand
/// (spec §3.3: games extend `Control`, not `Node`).
#[macro_export]
macro_rules! control_node {
    ($t:ty) => {
        impl $crate::__private::Node for $t {
            fn z(&self) -> f32 {
                $crate::UI_Z
            }
            fn visible(&self) -> bool {
                $crate::Control::base(self).visible
            }
            fn draw(&self, ctx: &mut $crate::__private::DrawContext) {
                $crate::draw_control_node(self, ctx);
            }
        }
    };
}

pub mod prelude {
    pub use crate::{
        Alignment, Anchors, Button, ButtonClicked, ButtonState, ChildLayout, Control, ControlBase,
        ControlExt, ControlRegistry, FocusChanged, FocusManager, HBox, Image, ImageMode, Label,
        Margins, Measure, Panel, Stretch, StretchMode, Theme, UI_Z, UiDrawContext, UiRoot, VBox,
        control_node,
    };
}
