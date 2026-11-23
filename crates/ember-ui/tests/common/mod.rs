//! Test controls shared by the ember-ui integration tests.
#![allow(dead_code)]

use ember_core::math::Rect;
use ember_core::{Color, Object};
use ember_input::{Input, Key};
use ember_scene::{Node, NodeId, NodeName, NodeTree, Servers};
use ember_servers::{LinearPhysics, MockAudio, MockRenderer, RenderingServer};
use ember_ui::prelude::*;
use glam::Vec2;
use std::any::Any;
use std::sync::{Arc, Mutex};

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

// --- UI harness --------------------------------------------------------

pub const VIEWPORT: Rect = Rect::new(0.0, 0.0, 640.0, 480.0);

/// A tree with a `UiRoot`, a listener node and handlers that record the
/// built-in signals.
pub struct Ui {
    pub tree: NodeTree,
    pub root: NodeId,
    pub render: MockRenderer,
    pub audio: MockAudio,
    pub physics: LinearPhysics,
    pub clicks: Arc<Mutex<Vec<NodeId>>>,
    pub focus_changes: Arc<Mutex<Vec<FocusChanged>>>,
    pub text_changes: Arc<Mutex<Vec<TextChanged>>>,
    pub submits: Arc<Mutex<Vec<TextSubmitted>>>,
}

impl Ui {
    /// A `UiRoot` with `Swatch` registered.
    pub fn new() -> Self {
        Self::with_root(UiRoot::default().register::<Swatch>())
    }

    pub fn with_root(ui_root: UiRoot) -> Self {
        let mut tree = NodeTree::with_root(ui_root);
        let root = tree.root().unwrap();
        let listener = tree.insert(Plain {
            name: "Listener".into(),
        });
        tree.attach(listener, root).unwrap();
        let clicks = Arc::new(Mutex::new(Vec::new()));
        let focus_changes = Arc::new(Mutex::new(Vec::new()));
        let c = Arc::clone(&clicks);
        tree.connect::<ButtonClicked>(listener, move |_ctx, ev| {
            c.lock().unwrap().push(ev.button);
        });
        let f = Arc::clone(&focus_changes);
        tree.connect::<FocusChanged>(listener, move |_ctx, ev| {
            f.lock().unwrap().push(*ev);
        });
        let text_changes = Arc::new(Mutex::new(Vec::new()));
        let t = Arc::clone(&text_changes);
        tree.connect::<TextChanged>(listener, move |_ctx, ev| {
            t.lock().unwrap().push(ev.clone());
        });
        let submits = Arc::new(Mutex::new(Vec::new()));
        let s = Arc::clone(&submits);
        tree.connect::<TextSubmitted>(listener, move |_ctx, ev| {
            s.lock().unwrap().push(ev.clone());
        });
        Self {
            tree,
            root,
            render: MockRenderer::new(),
            audio: MockAudio::new(),
            physics: LinearPhysics::new(),
            clicks,
            focus_changes,
            text_changes,
            submits,
        }
    }

    /// A button at `(x, y)`, attached to the root.
    pub fn button(&mut self, text: &str, x: f32, y: f32) -> NodeId {
        self.add(Button::new(text).with_offset(Vec2::new(x, y)))
    }

    pub fn add(&mut self, node: impl Node) -> NodeId {
        let id = self.tree.insert(node);
        self.tree.attach(id, self.root).unwrap();
        id
    }

    pub fn frame(&mut self, input: Input) {
        self.render.begin_frame(VIEWPORT);
        let mut servers = Servers {
            rendering: &mut self.render,
            audio: &mut self.audio,
            physics: &mut self.physics,
            input: &input,
        };
        self.tree.process(&mut servers, 0.016);
        self.render.end_frame();
    }

    pub fn keys(&mut self, keys: &[Key]) {
        self.frame(Input {
            keys_pressed: keys.to_vec(),
            ..Input::default()
        });
    }

    pub fn mouse(&mut self, pos: Vec2, pressed: bool, released: bool) {
        self.frame(Input {
            mouse_pos: pos,
            mouse_left_pressed: pressed,
            mouse_left_down: pressed,
            mouse_left_released: released,
            ..Input::default()
        });
    }

    pub fn get<T: 'static>(&self, id: NodeId) -> &T {
        self.tree
            .get(id)
            .unwrap()
            .as_any()
            .downcast_ref::<T>()
            .unwrap()
    }

    pub fn get_mut<T: 'static>(&mut self, id: NodeId) -> &mut T {
        self.tree
            .get_mut(id)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<T>()
            .unwrap()
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.get::<UiRoot>(self.root).focused()
    }

    pub fn clicks(&self) -> Vec<NodeId> {
        self.clicks.lock().unwrap().clone()
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}
