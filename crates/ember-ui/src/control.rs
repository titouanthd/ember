//! The `Control` trait and the state every control embeds.

use crate::{Anchors, Theme, UiDrawContext};
use ember_core::math::Rect;
use ember_input::{Key, MouseButton};
use ember_scene::{DrawContext, Node, NodeId};
use glam::Vec2;
use std::sync::Arc;

/// State shared by every control. Embed one and return it from
/// [`Control::base`] / [`Control::base_mut`]; the trait's defaults read it.
///
/// `rect`, `theme`, `focused` and `hovered` are written by the UI root
/// each frame; the rest is the control's own configuration.
#[derive(Debug, Clone)]
pub struct ControlBase {
    /// Final rect in screen pixels, set by the parent's layout.
    pub rect: Rect,
    /// Where the control sits in its parent when the parent does not
    /// arrange its children (spec §4.2).
    pub anchors: Anchors,
    /// Pixel offset added to the anchored position.
    pub offset: Vec2,
    /// Smallest size the control accepts; also the size used when the
    /// anchors give zero width or height (e.g. `Anchors::TOP_LEFT`).
    pub min_size: Vec2,
    pub visible: bool,
    pub focused: bool,
    pub hovered: bool,
    /// The UI root's theme, handed down during layout.
    pub theme: Arc<Theme>,
}

impl Default for ControlBase {
    fn default() -> Self {
        Self {
            rect: Rect::default(),
            anchors: Anchors::TOP_LEFT,
            offset: Vec2::ZERO,
            min_size: Vec2::ZERO,
            visible: true,
            focused: false,
            hovered: false,
            theme: Arc::new(Theme::dark()),
        }
    }
}

impl ControlBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_anchors(mut self, anchors: Anchors) -> Self {
        self.anchors = anchors;
        self
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }
}

/// What a parent sees of one child during layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChildLayout {
    pub id: NodeId,
    pub anchors: Anchors,
    pub offset: Vec2,
    pub min_size: Vec2,
    /// `measure(available)` of the child.
    pub desired: Vec2,
}

impl ChildLayout {
    /// The rect anchors give inside `parent`, plus the offset; a zero
    /// width or height falls back to the desired size (at least
    /// `min_size`).
    pub fn anchored_rect(&self, parent: Rect) -> Rect {
        let r = self.anchors.compute_rect(parent);
        let size = self.desired.max(self.min_size);
        let w = if r.w > 0.0 { r.w } else { size.x };
        let h = if r.h > 0.0 { r.h } else { size.y };
        Rect::new(r.x + self.offset.x, r.y + self.offset.y, w, h)
    }
}

/// A UI element: a scene node with a rect, laid out by its parent.
///
/// Required: [`base`](Self::base), [`base_mut`](Self::base_mut),
/// [`draw_control`](Self::draw_control). Everything else has a default
/// built on [`ControlBase`]. Pair the impl with [`crate::control_node!`].
pub trait Control: Node {
    fn base(&self) -> &ControlBase;
    fn base_mut(&mut self) -> &mut ControlBase;

    /// Draw the control. Children are drawn afterwards by the tree.
    fn draw_control(&self, ui: &mut UiDrawContext<'_, '_>);

    // --- Layout ---

    /// Desired size given the space the parent offers. Default:
    /// `min_size()`.
    fn measure(&self, _available: Vec2) -> Vec2 {
        self.min_size()
    }

    fn rect(&self) -> Rect {
        self.base().rect
    }

    fn set_rect(&mut self, rect: Rect) {
        self.base_mut().rect = rect;
    }

    /// The area children are laid out in. Default: the whole rect.
    /// Containers with padding or a scroll offset override it.
    fn content_rect(&self) -> Rect {
        self.rect()
    }

    fn anchors(&self) -> Anchors {
        self.base().anchors
    }

    fn min_size(&self) -> Vec2 {
        self.base().min_size
    }

    /// Containers (`VBox`, `HBox`, …) return one rect per child, in
    /// child order. `None` (default) places each child by its anchors.
    fn arrange_children(&self, _content: Rect, _children: &[ChildLayout]) -> Option<Vec<Rect>> {
        None
    }

    // --- Input (routed by the UI root, Phase 5.3) ---

    fn focusable(&self) -> bool {
        false
    }

    fn wants_mouse(&self) -> bool {
        true
    }

    fn on_mouse_enter(&mut self) {}
    fn on_mouse_leave(&mut self) {}
    fn on_mouse_down(&mut self, _button: MouseButton, _pos: Vec2) {}
    fn on_mouse_up(&mut self, _button: MouseButton, _pos: Vec2) {}
    fn on_mouse_wheel(&mut self, _delta: f32) {}
    fn on_key_press(&mut self, _key: Key) {}
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}
}

/// `Node::draw` body for a control: wraps the context with the control's
/// theme and focus state and calls [`Control::draw_control`]. Used by
/// [`crate::control_node!`].
pub fn draw_control_node<C: Control>(control: &C, ctx: &mut DrawContext<'_>) {
    let base = control.base();
    let theme = Arc::clone(&base.theme);
    let mut ui = UiDrawContext::new(ctx, &theme, base.focused, base.hovered);
    control.draw_control(&mut ui);
}
