//! Test controls shared by the ember-ui integration tests.
#![allow(dead_code)]

use ember_core::math::Rect;
use ember_core::{Color, Object};
use ember_scene::{Node, NodeName};
use ember_ui::prelude::*;
use std::any::Any;

// --- Test controls -----------------------------------------------------

/// A filled rectangle.
pub struct Swatch {
    pub name: String,
    pub base: ControlBase,
    pub color: Color,
}

impl Swatch {
    pub fn new(base: ControlBase) -> Self {
        Self {
            name: "Swatch".into(),
            base,
            color: Color::RED,
        }
    }
}

impl Object for Swatch {
    fn class_name() -> &'static str {
        "Swatch"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for Swatch {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Control for Swatch {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }
    fn draw_control(&self, ui: &mut UiDrawContext) {
        ui.draw_background(self.rect(), self.color);
        let border = ui.theme().panel_border;
        ui.draw_border(self.rect(), 1.0, border);
    }
}

control_node!(Swatch);

/// Stacks its children top to bottom at their desired height.
pub struct Column {
    pub name: String,
    pub base: ControlBase,
}

impl Object for Column {
    fn class_name() -> &'static str {
        "Column"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for Column {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Control for Column {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }
    fn draw_control(&self, _ui: &mut UiDrawContext) {}
    fn arrange_children(&self, content: Rect, children: &[ChildLayout]) -> Option<Vec<Rect>> {
        let mut y = content.y;
        Some(
            children
                .iter()
                .map(|c| {
                    let r = Rect::new(content.x, y, content.w, c.desired.y.max(c.min_size.y));
                    y += r.h;
                    r
                })
                .collect(),
        )
    }
}

control_node!(Column);

/// Pushes two clips and forgets to pop them.
pub struct Leaky {
    pub name: String,
    pub base: ControlBase,
}

impl Object for Leaky {
    fn class_name() -> &'static str {
        "Leaky"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for Leaky {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Control for Leaky {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }
    fn draw_control(&self, ui: &mut UiDrawContext) {
        ui.push_clip(Rect::new(0.0, 0.0, 50.0, 50.0));
        ui.push_clip(Rect::new(10.0, 10.0, 50.0, 50.0));
    }
}

control_node!(Leaky);

/// A plain node (not a control).
pub struct Plain {
    pub name: String,
}

impl Object for Plain {
    fn class_name() -> &'static str {
        "Plain"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for Plain {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Plain {}
