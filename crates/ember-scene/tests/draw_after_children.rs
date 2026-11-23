//! `Node::draw_after_children` (ADR-035): runs after the subtree, with
//! the node's own z restored, and not for hidden nodes.

use ember_core::math::Rect;
use ember_macros::Node;
use ember_scene::prelude::*;
use ember_servers::{MockRenderer, RecordedCall, RenderingServer};
use std::sync::{Arc, Mutex};

type Log = Arc<Mutex<Vec<String>>>;

#[derive(Node)]
#[class_name = "Wrap"]
struct Wrap {
    name: String,
    z: f32,
    visible: bool,
    log: Log,
}

impl Wrap {
    fn new(name: &str, z: f32, log: &Log) -> Self {
        Self {
            name: name.into(),
            z,
            visible: true,
            log: Arc::clone(log),
        }
    }
}

impl Node for Wrap {
    fn z(&self) -> f32 {
        self.z
    }
    fn visible(&self) -> bool {
        self.visible
    }
    fn draw(&self, _ctx: &mut DrawContext) {
        self.log.lock().unwrap().push(format!("{}:draw", self.name));
    }
    fn draw_after_children(&self, _ctx: &mut DrawContext) {
        self.log
            .lock()
            .unwrap()
            .push(format!("{}:after", self.name));
    }
}

fn tree(log: &Log) -> NodeTree {
    let mut tree = NodeTree::with_root(Wrap::new("root", 0.0, log));
    let root = tree.root().unwrap();
    let a = tree.insert(Wrap::new("a", 1.0, log));
    let a1 = tree.insert(Wrap::new("a1", 2.0, log));
    let b = tree.insert(Wrap::new("b", 3.0, log));
    tree.attach(a, root).unwrap();
    tree.attach(a1, a).unwrap();
    tree.attach(b, root).unwrap();
    tree
}

#[test]
fn after_children_runs_once_the_subtree_has_drawn() {
    let log = Log::default();
    let tree = tree(&log);
    let mut render = MockRenderer::new();
    render.begin_frame(Rect::new(0.0, 0.0, 100.0, 100.0));
    tree.draw(&mut DrawContext::new(&mut render));
    assert_eq!(
        *log.lock().unwrap(),
        [
            "root:draw",
            "a:draw",
            "a1:draw",
            "a1:after",
            "a:after",
            "b:draw",
            "b:after",
            "root:after"
        ]
    );
}

#[test]
fn after_children_restores_the_nodes_z() {
    let log = Log::default();
    let tree = tree(&log);
    let mut render = MockRenderer::new();
    render.begin_frame(Rect::new(0.0, 0.0, 100.0, 100.0));
    tree.draw(&mut DrawContext::new(&mut render));
    let zs: Vec<f32> = render
        .calls()
        .iter()
        .filter_map(|c| match c {
            RecordedCall::SetZ(z) => Some(*z),
            _ => None,
        })
        .collect();
    // root, a, a1, a again (after a1), b, root again (after b).
    assert_eq!(zs, [0.0, 1.0, 2.0, 1.0, 3.0, 0.0]);
}

#[test]
fn hidden_nodes_skip_after_children() {
    let log = Log::default();
    let mut tree = NodeTree::with_root(Wrap::new("root", 0.0, &log));
    let root = tree.root().unwrap();
    let mut hidden = Wrap::new("hidden", 0.0, &log);
    hidden.visible = false;
    let h = tree.insert(hidden);
    tree.attach(h, root).unwrap();
    let mut render = MockRenderer::new();
    render.begin_frame(Rect::new(0.0, 0.0, 100.0, 100.0));
    tree.draw(&mut DrawContext::new(&mut render));
    assert_eq!(*log.lock().unwrap(), ["root:draw", "root:after"]);
}
