//! `Stretch`: fit or fill its children into its rect (spec §4.4).

use crate::{ChildLayout, Control, ControlBase, Margins, UiDrawContext};
use ember_core::math::Rect;
use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StretchMode {
    /// Every child gets the whole content rect.
    #[default]
    Fill,
    /// Every child keeps its desired size (clamped to the content) and is
    /// centred.
    Fit,
}

/// Usually wraps a single child; with several, each is placed the same
/// way (they overlap).
pub struct Stretch {
    name: String,
    base: ControlBase,
    pub mode: StretchMode,
    pub padding: Margins,
}

impl Stretch {
    pub fn new(mode: StretchMode) -> Self {
        Self {
            name: "Stretch".into(),
            base: ControlBase::new(),
            mode,
            padding: Margins::ZERO,
        }
    }

    pub fn with_padding(mut self, padding: Margins) -> Self {
        self.padding = padding;
        self
    }
}

impl Default for Stretch {
    fn default() -> Self {
        Self::new(StretchMode::Fill)
    }
}

crate::widgets::ui_object!(Stretch, "Stretch");

impl Control for Stretch {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }
    fn draw_control(&self, _ui: &mut UiDrawContext) {}

    fn content_rect(&self) -> Rect {
        self.padding.shrink(self.rect())
    }

    fn measure_children(&self, own: Vec2, children: &[Vec2]) -> Vec2 {
        let largest = children.iter().fold(Vec2::ZERO, |acc, size| acc.max(*size));
        own.max(self.padding.grow(largest))
    }

    fn arrange_children(&self, content: Rect, children: &[ChildLayout]) -> Option<Vec<Rect>> {
        Some(
            children
                .iter()
                .map(|c| match self.mode {
                    StretchMode::Fill => content,
                    StretchMode::Fit => {
                        let w = c.desired.x.min(content.w);
                        let h = c.desired.y.min(content.h);
                        Rect::new(
                            content.x + (content.w - w) / 2.0,
                            content.y + (content.h - h) / 2.0,
                            w,
                            h,
                        )
                    }
                })
                .collect(),
        )
    }
}

crate::control_node!(Stretch);
