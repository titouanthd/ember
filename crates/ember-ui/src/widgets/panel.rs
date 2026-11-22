//! `Panel`: a background with an optional border, laid out like a
//! container with padding (spec §5.3).

use crate::{Control, ControlBase, Margins, UiDrawContext};
use ember_core::Color;
use ember_core::math::Rect;
use glam::Vec2;

/// Children are placed by their own anchors inside the padded rect.
pub struct Panel {
    name: String,
    base: ControlBase,
    /// `None` = `Theme::panel_bg`.
    pub background: Option<Color>,
    /// Border thickness in pixels; `0.0` = no border.
    pub border_width: f32,
    /// `None` = `Theme::panel_border`.
    pub border_color: Option<Color>,
    pub padding: Margins,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            name: "Panel".into(),
            base: ControlBase::new(),
            background: None,
            border_width: 1.0,
            border_color: None,
            padding: Margins::ZERO,
        }
    }

    pub fn with_background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn with_border(mut self, width: f32, color: Option<Color>) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    pub fn with_padding(mut self, padding: Margins) -> Self {
        self.padding = padding;
        self
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

crate::widgets::ui_object!(Panel, "Panel");

impl Control for Panel {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }

    fn content_rect(&self) -> Rect {
        self.padding.shrink(self.rect())
    }

    fn measure_children(&self, own: Vec2, children: &[Vec2]) -> Vec2 {
        let largest = children.iter().fold(Vec2::ZERO, |acc, size| acc.max(*size));
        own.max(self.padding.grow(largest))
    }

    fn draw_control(&self, ui: &mut UiDrawContext) {
        let rect = self.rect();
        let bg = self.background.unwrap_or(ui.theme().panel_bg);
        ui.draw_background(rect, bg);
        if self.border_width > 0.0 {
            let color = self.border_color.unwrap_or(ui.theme().panel_border);
            ui.draw_border(rect, self.border_width, color);
        }
    }
}

crate::control_node!(Panel);
