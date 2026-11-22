//! `Label`: one line of text (spec §5.1).

use crate::{Control, ControlBase, Measure, UiDrawContext};
use ember_core::Color;
use ember_core::math::Rect;
use glam::Vec2;

/// Where content of a given size sits inside a rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    TopLeft,
    Top,
    TopRight,
    /// Left edge, vertically centred (the usual label alignment).
    #[default]
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Alignment {
    /// Top-left corner of a `size` box aligned inside `rect`.
    pub fn place(self, rect: Rect, size: Vec2) -> Vec2 {
        let (fx, fy) = match self {
            Self::TopLeft => (0.0, 0.0),
            Self::Top => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::Bottom => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
        };
        Vec2::new(
            rect.x + (rect.w - size.x) * fx,
            rect.y + (rect.h - size.y) * fy,
        )
    }
}

/// Text in the theme's font size and colour unless overridden.
pub struct Label {
    name: String,
    base: ControlBase,
    pub text: String,
    /// `None` = `Theme::font_size`.
    pub size: Option<f32>,
    /// `None` = `Theme::text`.
    pub color: Option<Color>,
    pub align: Alignment,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            name: "Label".into(),
            base: ControlBase::new(),
            text: text.into(),
            size: None,
            color: None,
            align: Alignment::Left,
        }
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl Default for Label {
    fn default() -> Self {
        Self::new("")
    }
}

crate::widgets::ui_object!(Label, "Label");

impl Control for Label {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }

    fn measure(&self, _available: Vec2, m: &Measure) -> Vec2 {
        let size = self.size.unwrap_or(m.theme().font_size);
        m.text(&self.text, size).max(self.min_size())
    }

    fn draw_control(&self, ui: &mut UiDrawContext) {
        if self.text.is_empty() {
            return;
        }
        let size = self.size.unwrap_or(ui.theme().font_size);
        let color = self.color.unwrap_or(ui.theme().text);
        let metrics = ui.render().measure_text(&self.text, size);
        let top_left = self
            .align
            .place(self.rect(), Vec2::new(metrics.width, metrics.height));
        // `draw_text` takes the baseline; `offset_y` is its distance from
        // the top of the text box.
        let baseline = Vec2::new(top_left.x, top_left.y + metrics.offset_y);
        ui.draw_text(&self.text, baseline, size, color);
    }
}

crate::control_node!(Label);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_places_inside_the_rect() {
        let r = Rect::new(10.0, 20.0, 100.0, 40.0);
        let s = Vec2::new(30.0, 10.0);
        assert_eq!(Alignment::TopLeft.place(r, s), Vec2::new(10.0, 20.0));
        assert_eq!(Alignment::Center.place(r, s), Vec2::new(45.0, 35.0));
        assert_eq!(Alignment::BottomRight.place(r, s), Vec2::new(80.0, 50.0));
        assert_eq!(Alignment::Left.place(r, s), Vec2::new(10.0, 35.0));
    }
}
