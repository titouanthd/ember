//! `UiDrawContext`: a `DrawContext` plus the theme and focus state.

use crate::Theme;
use ember_core::Color;
use ember_core::math::Rect;
use ember_scene::DrawContext;
use ember_servers::RenderingServer;
use glam::Vec2;

/// Passed to [`crate::Control::draw_control`]. Clips pushed through it
/// are popped automatically when it is dropped, so an early return in a
/// control cannot leak a clip into the rest of the frame.
pub struct UiDrawContext<'a, 'b> {
    inner: &'a mut DrawContext<'b>,
    theme: &'a Theme,
    focused: bool,
    hovered: bool,
    clips: usize,
}

impl<'a, 'b> UiDrawContext<'a, 'b> {
    pub fn new(
        inner: &'a mut DrawContext<'b>,
        theme: &'a Theme,
        focused: bool,
        hovered: bool,
    ) -> Self {
        Self {
            inner,
            theme,
            focused,
            hovered,
            clips: 0,
        }
    }

    pub fn render(&mut self) -> &mut dyn RenderingServer {
        self.inner.render()
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }

    /// True if the control being drawn has keyboard focus.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// True if the mouse is over the control being drawn.
    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Clip further drawing to `rect` (screen pixels, ADR-031).
    pub fn push_clip(&mut self, rect: Rect) {
        self.inner.render().push_clip_rect(rect);
        self.clips += 1;
    }

    /// Undo the last [`push_clip`](Self::push_clip) of this context.
    /// Ignored when this context pushed nothing.
    pub fn pop_clip(&mut self) {
        if self.clips > 0 {
            self.inner.render().pop_clip_rect();
            self.clips -= 1;
        }
    }

    pub fn draw_background(&mut self, rect: Rect, color: Color) {
        self.inner.render().draw_rect(rect, color);
    }

    pub fn draw_border(&mut self, rect: Rect, thickness: f32, color: Color) {
        self.inner
            .render()
            .draw_rect_outline(rect, thickness, color);
    }

    pub fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color) {
        self.inner.render().draw_text(text, pos, size, color);
    }
}

impl Drop for UiDrawContext<'_, '_> {
    fn drop(&mut self) {
        while self.clips > 0 {
            self.pop_clip();
        }
    }
}
