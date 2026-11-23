//! `ScrollContainer`: a vertically scrolling viewport (spec §5.4).

use crate::{ChildLayout, Control, ControlBase, UiDrawContext};
use ember_core::math::Rect;
use glam::Vec2;

/// Width of the scrollbar, in pixels. Reserved on the right only while
/// the content is taller than the container.
pub const SCROLLBAR_WIDTH: f32 = 8.0;
/// Pixels scrolled per wheel notch.
pub const SCROLL_STEP: f32 = 40.0;
/// Smallest scrollbar thumb, in pixels.
const MIN_THUMB: f32 = 16.0;

/// Children are laid out at the container's width and their desired
/// height, from the top of the content, then shifted by the scroll
/// offset. Their drawing and the pointer are clipped to the container
/// (ADR-035). Usually holds a single `VBox`.
///
/// Vertical only in v0.1. The wheel scrolls while the pointer is over it.
pub struct ScrollContainer {
    name: String,
    base: ControlBase,
    scroll: f32,
    content_height: f32,
}

impl ScrollContainer {
    pub fn new() -> Self {
        Self {
            name: "ScrollContainer".into(),
            base: ControlBase::new(),
            scroll: 0.0,
            content_height: 0.0,
        }
    }

    /// Pixels scrolled from the top.
    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    /// Scroll to `y` pixels from the top, clamped to the content.
    pub fn set_scroll(&mut self, y: f32) {
        self.scroll = y.clamp(0.0, self.max_scroll());
    }

    /// Height of the content at the last layout.
    pub fn content_height(&self) -> f32 {
        self.content_height
    }

    pub fn max_scroll(&self) -> f32 {
        (self.content_height - self.rect().h).max(0.0)
    }

    fn overflows(&self) -> bool {
        self.content_height > self.rect().h
    }

    /// The visible area, without the scrollbar.
    fn viewport(&self) -> Rect {
        let r = self.rect();
        let bar = if self.overflows() {
            SCROLLBAR_WIDTH
        } else {
            0.0
        };
        Rect::new(r.x, r.y, (r.w - bar).max(0.0), r.h)
    }

    /// Scrollbar thumb, if the content overflows.
    pub fn thumb(&self) -> Option<Rect> {
        if !self.overflows() {
            return None;
        }
        let r = self.rect();
        let h = (r.h * r.h / self.content_height).max(MIN_THUMB).min(r.h);
        let travel = r.h - h;
        let t = if self.max_scroll() > 0.0 {
            self.scroll / self.max_scroll()
        } else {
            0.0
        };
        Some(Rect::new(
            r.x + r.w - SCROLLBAR_WIDTH,
            r.y + travel * t,
            SCROLLBAR_WIDTH,
            h,
        ))
    }
}

impl Default for ScrollContainer {
    fn default() -> Self {
        Self::new()
    }
}

crate::widgets::ui_object!(ScrollContainer, "ScrollContainer");

impl Control for ScrollContainer {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }

    /// The viewport moved up by the scroll offset.
    fn content_rect(&self) -> Rect {
        let v = self.viewport();
        Rect::new(v.x, v.y - self.scroll, v.w, v.h)
    }

    /// Wants only its `min_size`: a scroll area takes the room it is given
    /// instead of growing to its content.
    fn measure_children(&self, own: glam::Vec2, _children: &[Vec2]) -> Vec2 {
        own
    }

    fn arrange_children(&self, content: Rect, children: &[ChildLayout]) -> Option<Vec<Rect>> {
        Some(
            children
                .iter()
                .map(|c| {
                    let h = c.desired.y.max(c.min_size.y);
                    Rect::new(content.x + c.offset.x, content.y + c.offset.y, content.w, h)
                })
                .collect(),
        )
    }

    fn children_arranged(&mut self, rects: &[Rect]) {
        let top = self.content_rect().y;
        self.content_height = rects.iter().map(|r| r.y + r.h - top).fold(0.0, f32::max);
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
    }

    fn clip_children(&self) -> Option<Rect> {
        Some(self.viewport())
    }

    /// Wheel up (positive y) scrolls towards the top.
    fn on_mouse_wheel(&mut self, delta: f32) {
        self.set_scroll(self.scroll - delta * SCROLL_STEP);
    }

    fn draw_control(&self, _ui: &mut UiDrawContext) {}

    fn draw_overlay(&self, ui: &mut UiDrawContext) {
        if let Some(thumb) = self.thumb() {
            let color = ui.theme().scrollbar;
            ui.draw_background(thumb, color);
        }
    }
}

crate::control_node!(ScrollContainer);

#[cfg(test)]
mod tests {
    use super::*;

    fn sc(h: f32, content: f32) -> ScrollContainer {
        let mut s = ScrollContainer::new();
        s.base.rect = Rect::new(0.0, 0.0, 100.0, h);
        s.content_height = content;
        s
    }

    #[test]
    fn scroll_is_clamped_to_the_content() {
        let mut s = sc(100.0, 250.0);
        s.set_scroll(1000.0);
        assert_eq!(s.scroll(), 150.0);
        s.set_scroll(-5.0);
        assert_eq!(s.scroll(), 0.0);
        assert_eq!(sc(100.0, 50.0).max_scroll(), 0.0);
    }

    #[test]
    fn wheel_up_scrolls_towards_the_top() {
        let mut s = sc(100.0, 400.0);
        s.on_mouse_wheel(-1.0);
        assert_eq!(s.scroll(), SCROLL_STEP);
        s.on_mouse_wheel(1.0);
        assert_eq!(s.scroll(), 0.0);
    }

    #[test]
    fn thumb_only_when_overflowing_and_tracks_the_scroll() {
        assert_eq!(sc(100.0, 100.0).thumb(), None);
        let mut s = sc(100.0, 400.0);
        assert_eq!(s.thumb(), Some(Rect::new(92.0, 0.0, 8.0, 25.0)));
        s.set_scroll(300.0);
        assert_eq!(s.thumb(), Some(Rect::new(92.0, 75.0, 8.0, 25.0)));
    }
}
