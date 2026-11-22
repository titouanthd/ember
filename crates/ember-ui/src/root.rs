//! `UiRoot`: the node every UI subtree hangs from (spec §8).

use crate::layout::layout_children;
use crate::{Control, ControlRegistry, Theme, UI_Z};
use ember_core::Object;
use ember_scene::{Node, NodeContext, NodeName};
use std::any::Any;
use std::sync::Arc;

/// Owns the theme and the control registry. Each `process` it lays out
/// its whole subtree inside the viewport (one pass, spec §4.1) and hands
/// the theme to every control. Draws nothing itself; sits at
/// [`UI_Z`] so the UI is above the world.
pub struct UiRoot {
    name: String,
    theme: Arc<Theme>,
    registry: ControlRegistry,
}

impl UiRoot {
    pub fn new(theme: Theme) -> Self {
        Self {
            name: "UiRoot".into(),
            theme: Arc::new(theme),
            registry: ControlRegistry::with_builtins(),
        }
    }

    /// Make a custom control type visible to layout and input. Built-in
    /// widgets are already registered.
    pub fn register<T: Control>(mut self) -> Self {
        self.registry.register::<T>();
        self
    }

    pub fn registry(&self) -> &ControlRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ControlRegistry {
        &mut self.registry
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Replace the theme; controls pick it up at the next layout.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = Arc::new(theme);
    }
}

impl Default for UiRoot {
    fn default() -> Self {
        Self::new(Theme::dark())
    }
}

impl Object for UiRoot {
    fn class_name() -> &'static str {
        "UiRoot"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for UiRoot {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for UiRoot {
    fn z(&self) -> f32 {
        UI_Z
    }

    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        let viewport = ctx.render().viewport();
        let me = ctx.current_id();
        let (tree, render) = ctx.tree_and_render();
        layout_children(tree, &self.registry, render, me, viewport, &self.theme);
    }
}
