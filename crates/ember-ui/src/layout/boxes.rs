//! `VBox` / `HBox`: stack children along one axis (spec §4.3).

use crate::{ChildLayout, Control, ControlBase, Margins, UiDrawContext};
use ember_core::math::Rect;
use glam::Vec2;

#[derive(Clone, Copy)]
enum Axis {
    Vertical,
    Horizontal,
}

/// Rects for `children` stacked along `axis` inside `content`: each child
/// gets its desired size on the main axis and the full content size on
/// the cross axis; `spacing` between consecutive children.
fn stack(axis: Axis, content: Rect, children: &[ChildLayout], spacing: f32) -> Vec<Rect> {
    let mut cursor = 0.0;
    children
        .iter()
        .map(|c| {
            let r = match axis {
                Axis::Vertical => Rect::new(content.x, content.y + cursor, content.w, c.desired.y),
                Axis::Horizontal => {
                    Rect::new(content.x + cursor, content.y, c.desired.x, content.h)
                }
            };
            cursor += match axis {
                Axis::Vertical => r.h,
                Axis::Horizontal => r.w,
            } + spacing;
            r
        })
        .collect()
}

/// Size of a stack: the sum on the main axis (plus spacing), the largest
/// child on the cross axis.
fn stack_size(axis: Axis, children: &[Vec2], spacing: f32) -> Vec2 {
    let gaps = spacing * children.len().saturating_sub(1) as f32;
    let sum_x: f32 = children.iter().map(|s| s.x).sum();
    let sum_y: f32 = children.iter().map(|s| s.y).sum();
    let max_x = children.iter().fold(0.0_f32, |a, s| a.max(s.x));
    let max_y = children.iter().fold(0.0_f32, |a, s| a.max(s.y));
    match axis {
        Axis::Vertical => Vec2::new(max_x, sum_y + gaps),
        Axis::Horizontal => Vec2::new(sum_x + gaps, max_y),
    }
}

macro_rules! stack_container {
    ($(#[$doc:meta])* $name:ident, $class:literal, $axis:expr) => {
        $(#[$doc])*
        pub struct $name {
            name: String,
            base: ControlBase,
            /// Pixels between consecutive children.
            pub spacing: f32,
            /// Space kept free inside the box.
            pub padding: Margins,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    name: $class.into(),
                    base: ControlBase::new(),
                    spacing: 0.0,
                    padding: Margins::ZERO,
                }
            }

            pub fn with_spacing(mut self, spacing: f32) -> Self {
                self.spacing = spacing;
                self
            }

            pub fn with_padding(mut self, padding: Margins) -> Self {
                self.padding = padding;
                self
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        crate::widgets::ui_object!($name, $class);

        impl Control for $name {
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
                own.max(self.padding.grow(stack_size($axis, children, self.spacing)))
            }

            fn arrange_children(&self, content: Rect, children: &[ChildLayout]) -> Option<Vec<Rect>> {
                Some(stack($axis, content, children, self.spacing))
            }
        }

        crate::control_node!($name);
    };
}

stack_container!(
    /// Stacks its children top to bottom. Each child gets its desired
    /// height and the full content width.
    VBox,
    "VBox",
    Axis::Vertical
);

stack_container!(
    /// Stacks its children left to right. Each child gets its desired
    /// width and the full content height.
    HBox,
    "HBox",
    Axis::Horizontal
);
