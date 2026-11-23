//! The `Control` trait and the state every control embeds.

use crate::{Anchors, Measure, Theme, UiDrawContext};
use ember_core::math::Rect;
use ember_input::{Key, MouseButton};
use ember_scene::{DrawContext, Node, NodeContext, NodeId, Signal};
use glam::Vec2;
use std::sync::Arc;

/// A signal a control asked to emit; `UiRoot` runs it with the control's
/// id after routing input (ADR-036).
pub(crate) type PendingSignal = Box<dyn FnOnce(&mut NodeContext<'_>, NodeId) + Send + Sync>;

/// State shared by every control. Embed one and return it from
/// [`Control::base`] / [`Control::base_mut`]; the trait's defaults read it.
///
/// `rect`, `theme`, `focused`, `hovered` and `pressed` are written by the
/// UI root each frame; the rest is the control's own configuration.
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
    /// The left mouse button went down on this control and has not been
    /// released yet; it still gets `on_mouse_up` if the pointer left.
    pub pressed: bool,
    /// The UI root's theme, handed down during layout.
    pub theme: Arc<Theme>,
    outbox: Vec<PendingSignal>,
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
            pressed: false,
            theme: Arc::new(Theme::dark()),
            outbox: Vec::new(),
        }
    }
}

/// Clones the configuration and state; queued signals are not cloned.
impl Clone for ControlBase {
    fn clone(&self) -> Self {
        Self {
            rect: self.rect,
            anchors: self.anchors,
            offset: self.offset,
            min_size: self.min_size,
            visible: self.visible,
            focused: self.focused,
            hovered: self.hovered,
            pressed: self.pressed,
            theme: Arc::clone(&self.theme),
            outbox: Vec::new(),
        }
    }
}

impl std::fmt::Debug for ControlBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlBase")
            .field("rect", &self.rect)
            .field("anchors", &self.anchors)
            .field("offset", &self.offset)
            .field("min_size", &self.min_size)
            .field("visible", &self.visible)
            .field("focused", &self.focused)
            .field("hovered", &self.hovered)
            .field("pressed", &self.pressed)
            .field("queued_signals", &self.outbox.len())
            .finish()
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

    /// Queue a signal for `UiRoot` to emit after this frame's input
    /// routing. `make` receives the control's `NodeId` (ADR-036):
    /// `self.base.emit(|id| ButtonClicked { button: id })`.
    pub fn emit<T: Signal>(&mut self, make: impl FnOnce(NodeId) -> T + Send + Sync + 'static) {
        self.outbox
            .push(Box::new(move |ctx, id| ctx.emit(make(id))));
    }

    /// Number of signals waiting to be emitted.
    pub fn queued_signals(&self) -> usize {
        self.outbox.len()
    }

    pub(crate) fn take_signals(&mut self) -> Vec<PendingSignal> {
        std::mem::take(&mut self.outbox)
    }
}

/// What a parent sees of one child during layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChildLayout {
    pub id: NodeId,
    pub anchors: Anchors,
    pub offset: Vec2,
    pub min_size: Vec2,
    /// The child's desired size: its own `measure`, combined with its
    /// children's through `measure_children`, at least `min_size`.
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

    /// Draw on top of the children (after the whole subtree drew).
    /// Runs outside [`clip_children`](Self::clip_children), so a
    /// scrollbar drawn here is not clipped (ADR-035).
    fn draw_overlay(&self, _ui: &mut UiDrawContext<'_, '_>) {}

    /// Clip the children's drawing to this rect (screen pixels). `None`
    /// (default) = no clip. Must not change between `draw` and
    /// `draw_overlay` of one frame (it is read in both).
    fn clip_children(&self) -> Option<Rect> {
        None
    }

    // --- Layout ---

    /// Desired size of the control itself (text, image…) given the space
    /// the parent offers. Default: `min_size()`.
    fn measure(&self, _available: Vec2, _m: &Measure) -> Vec2 {
        self.min_size()
    }

    /// Combine the control's own desired size with its children's
    /// (measured first, bottom-up). Containers override this; the default
    /// ignores children.
    fn measure_children(&self, own: Vec2, _children: &[Vec2]) -> Vec2 {
        own
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

    /// Called after layout placed this control's children, with their
    /// final rects in child order. Lets a container remember what it
    /// holds (e.g. `ScrollContainer`'s content height).
    fn children_arranged(&mut self, _rects: &[Rect]) {}

    // --- Input (routed by `UiRoot`, spec §9, ADR-036) ---

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
    /// A typed character (from `Input::chars_typed`, control characters
    /// removed) while focused.
    fn on_text_input(&mut self, _ch: char) {}
    /// `true` for text entry: Space types a space instead of activating
    /// the control (Enter still activates).
    fn captures_text(&self) -> bool {
        false
    }
    /// Enter / Space / gamepad South while focused (spec §6.2).
    fn on_activate(&mut self) {}
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}
}

/// Builder helpers available on every control:
/// `Label::new("Play").with_anchors(Anchors::CENTER)`.
pub trait ControlExt: Control + Sized {
    fn with_anchors(mut self, anchors: Anchors) -> Self {
        self.base_mut().anchors = anchors;
        self
    }

    fn with_offset(mut self, offset: Vec2) -> Self {
        self.base_mut().offset = offset;
        self
    }

    fn with_min_size(mut self, min_size: Vec2) -> Self {
        self.base_mut().min_size = min_size;
        self
    }

    fn with_name(mut self, name: impl Into<String>) -> Self {
        self.set_name(name.into());
        self
    }

    fn hidden(mut self) -> Self {
        self.base_mut().visible = false;
        self
    }
}

impl<T: Control + Sized> ControlExt for T {}

/// `Node::draw` body for a control: wraps the context with the control's
/// theme and focus state, calls [`Control::draw_control`], then opens the
/// [`Control::clip_children`] clip for the subtree. Used by
/// [`crate::control_node!`].
pub fn draw_control_node<C: Control>(control: &C, ctx: &mut DrawContext<'_>) {
    let base = control.base();
    let theme = Arc::clone(&base.theme);
    {
        let mut ui = UiDrawContext::new(ctx, &theme, base.focused, base.hovered);
        control.draw_control(&mut ui);
    }
    // Pushed on the raw server: `UiDrawContext` pops its own clips on drop.
    if let Some(clip) = control.clip_children() {
        ctx.render().push_clip_rect(clip);
    }
}

/// `Node::draw_after_children` body for a control: closes the
/// [`Control::clip_children`] clip, then calls
/// [`Control::draw_overlay`] (ADR-035).
pub fn draw_control_after_children<C: Control>(control: &C, ctx: &mut DrawContext<'_>) {
    if control.clip_children().is_some() {
        ctx.render().pop_clip_rect();
    }
    let base = control.base();
    let theme = Arc::clone(&base.theme);
    let mut ui = UiDrawContext::new(ctx, &theme, base.focused, base.hovered);
    control.draw_overlay(&mut ui);
}
