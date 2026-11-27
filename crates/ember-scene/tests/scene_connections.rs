//! Scene-declared connections are wired at instantiate time (spec 30 §8,
//! M4.1): once every node exists, in file order, through receivers
//! registered with `ClassDbNodes::register_receiver`.

mod common;
use common::Harness;
use ember_core::ClassDB;
use ember_macros::Node;
use ember_scene::Node as NodeTrait;
use ember_scene::{ClassDbNodes, NodeContext, NodeTree, PackedScene};

struct Ping(u32);
struct Pong;

/// Emits one `Ping(3)` on its first frame.
#[derive(Node, Default)]
#[class_name = "Emitter"]
pub struct Emitter {
    pub name: String,
    sent: bool,
}

impl NodeTrait for Emitter {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        if !self.sent {
            self.sent = true;
            ctx.emit(Ping(3));
            ctx.emit(Pong);
        }
    }
}

#[derive(Node, Default)]
#[class_name = "Counter"]
pub struct Counter {
    pub name: String,
    log: Vec<String>,
}

impl Counter {
    fn on_ping(&mut self, p: &Ping) {
        self.log.push(format!("ping {}", p.0));
    }
    fn on_ping_twice(&mut self, p: &Ping) {
        self.log.push(format!("twice {}", p.0 * 2));
    }
}

impl NodeTrait for Counter {}

fn classes() -> ClassDB {
    let mut db = ClassDB::new();
    db.register_node::<Emitter>();
    db.register_node::<Counter>();
    db.register_receiver::<Counter, Ping>("Ping", "on_ping", Counter::on_ping);
    db.register_receiver::<Counter, Ping>("Ping", "on_ping_twice", Counter::on_ping_twice);
    db
}

const SCENE: &str = r#"(
    format: 1,
    uid: "uid://wired",
    nodes: [
        (name: "Root", class: "Emitter"),
        (name: "A", class: "Counter", parent: "."),
        (name: "B", class: "Counter", parent: "."),
    ],
    connections: [
        (signal: "Ping", from: ".", to: "B", method: "on_ping_twice"),
        (signal: "Ping", from: ".", to: "A", method: "on_ping"),
        (signal: "Ping", from: ".", to: "B", method: "on_ping"),
        (signal: "Ping", from: ".", to: "A", method: "no_such_method"),
        (signal: "Pong", from: ".", to: "A", method: "on_ping"),
    ],
)"#;

fn run_one_frame() -> (NodeTree, Vec<String>, Vec<String>) {
    let mut tree = NodeTree::new();
    let root = PackedScene::from_ron(SCENE)
        .unwrap()
        .instantiate(&classes(), &mut tree)
        .unwrap();
    tree.set_root(root);
    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    let log = |name: &str| {
        let id = tree.find(root, name).unwrap();
        let c = tree.get(id).unwrap().as_any().downcast_ref::<Counter>();
        c.unwrap().log.clone()
    };
    let (a, b) = (log("A"), log("B"));
    (tree, a, b)
}

#[test]
fn declared_connections_call_the_receiver_on_its_target() {
    let (_, a, b) = run_one_frame();
    assert_eq!(a, ["ping 3"]);
    assert_eq!(b, ["twice 6", "ping 3"], "each target sees its own calls");
}

#[test]
fn connections_are_wired_in_file_order() {
    let (tree, _, _) = run_one_frame();
    // Three wired (unknown method and wrong signal skipped), in order:
    // B.twice, A.ping, B.ping, so ids 0..3.
    assert_eq!(tree.signals().len(), 3);
    for id in 0..3 {
        assert!(tree.signals().is_connected(id));
    }
}

#[test]
fn a_connection_without_receiver_is_skipped_not_fatal() {
    let mut tree = NodeTree::new();
    let mut db = ClassDB::new();
    db.register_node::<Emitter>();
    db.register_node::<Counter>();
    let root = PackedScene::from_ron(SCENE)
        .unwrap()
        .instantiate(&db, &mut tree);
    assert!(root.is_ok());
    assert!(tree.signals().is_empty());
}
