#![allow(dead_code)]

use ember_core::Object;
use ember_core::math::Rect;
use ember_input::Input;
use ember_scene::prelude::*;
use ember_servers::{LinearPhysics, MockAudio, MockRenderer, RenderingServer};
use std::any::Any;
use std::sync::{Arc, Mutex};

pub type Log = Arc<Mutex<Vec<String>>>;

pub fn new_log() -> Log {
    Arc::default()
}

pub fn entries(log: &Log) -> Vec<String> {
    log.lock().unwrap().clone()
}

/// Logs every lifecycle call as `"<name>:<event>"`.
pub struct Recorder {
    name: String,
    log: Log,
    pub z: f32,
    pub visible: bool,
}

impl Recorder {
    pub fn new(name: &str, log: &Log) -> Self {
        Self {
            name: name.into(),
            log: log.clone(),
            z: 0.0,
            visible: true,
        }
    }

    fn push(&self, event: &str) {
        self.log
            .lock()
            .unwrap()
            .push(format!("{}:{event}", self.name));
    }
}

impl Object for Recorder {
    fn class_name() -> &'static str {
        "Recorder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Node for Recorder {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
    fn z(&self) -> f32 {
        self.z
    }
    fn visible(&self) -> bool {
        self.visible
    }
    fn ready(&mut self, _ctx: &mut NodeContext) {
        self.push("ready");
    }
    fn process(&mut self, _dt: f32, _ctx: &mut NodeContext) {
        self.push("process");
    }
    fn physics_process(&mut self, _dt: f32, _ctx: &mut NodeContext) {
        self.push("physics");
    }
    fn draw(&self, _ctx: &mut DrawContext) {
        self.push("draw");
    }
    fn finalize(&mut self) {
        self.push("finalize");
    }
}

/// Owns the backends so tests can build a `Servers` per call.
pub struct Harness {
    pub render: MockRenderer,
    pub audio: MockAudio,
    pub physics: LinearPhysics,
    pub input: Input,
}

impl Harness {
    pub fn new() -> Self {
        let mut render = MockRenderer::new();
        render.begin_frame(Rect::new(0.0, 0.0, 800.0, 600.0));
        Self {
            render,
            audio: MockAudio::new(),
            physics: LinearPhysics::new(),
            input: Input::default(),
        }
    }

    pub fn servers(&mut self) -> Servers<'_> {
        Servers {
            rendering: &mut self.render,
            audio: &mut self.audio,
            physics: &mut self.physics,
            input: &self.input,
        }
    }
}

/// root -> [a -> [a1, a2], b]
pub fn sample_tree(log: &Log) -> (NodeTree, [NodeId; 5]) {
    let mut t = NodeTree::new();
    let root = t.insert(Recorder::new("root", log));
    let a = t.insert(Recorder::new("a", log));
    let a1 = t.insert(Recorder::new("a1", log));
    let a2 = t.insert(Recorder::new("a2", log));
    let b = t.insert(Recorder::new("b", log));
    t.attach(a, root).unwrap();
    t.attach(a1, a).unwrap();
    t.attach(a2, a).unwrap();
    t.attach(b, root).unwrap();
    t.set_root(root);
    (t, [root, a, a1, a2, b])
}

/// Runs `f` on every `process`; lets a test act through a `NodeContext`.
pub struct Script {
    name: String,
    f: Box<dyn FnMut(&mut NodeContext) + Send + Sync>,
}

impl Script {
    pub fn new(name: &str, f: impl FnMut(&mut NodeContext) + Send + Sync + 'static) -> Self {
        Self {
            name: name.into(),
            f: Box::new(f),
        }
    }
}

impl Object for Script {
    fn class_name() -> &'static str {
        "Script"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Node for Script {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        (self.f)(ctx);
    }
}

/// A tree whose root is a `Script`, plus three Recorder children
/// `a`, `b`, `c` to use as signal targets.
pub fn script_tree(
    log: &Log,
    f: impl FnMut(&mut NodeContext) + Send + Sync + 'static,
) -> (NodeTree, [NodeId; 3]) {
    let mut t = NodeTree::with_root(Script::new("script", f));
    let root = t.root().unwrap();
    let ids = ["a", "b", "c"].map(|n| {
        let id = t.insert(Recorder::new(n, log));
        t.attach(id, root).unwrap();
        id
    });
    (t, ids)
}
