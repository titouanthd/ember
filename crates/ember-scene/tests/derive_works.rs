//! Tests for `#[derive(Node)]` (ADR-019, ADR-030).
//!
//! The derive generates `Object` + `NodeName`; the user writes the
//! `impl Node` block with only the lifecycle the node needs.
//!
//! Failure-path tests (missing `class_name`, missing `name: String`,
//! applied to a non-struct) are deferred to a `trybuild` PR.

mod common;
use common::Harness;
use ember_core::{ClassDB, Object};
use ember_macros::Node;
use ember_scene::Node as NodeTrait;
use ember_scene::{ClassDbNodes, NodeContext, NodeName, NodeTree};

#[derive(Node)]
#[class_name = "Widget"]
pub struct Widget {
    pub name: String,
    pub count: u32,
}

impl NodeTrait for Widget {}

#[derive(Node)]
#[class_name = "Simple"]
pub struct Simple {
    pub name: String,
}

impl NodeTrait for Simple {}

/// The point of ADR-030: the derive and real behaviour on one struct.
#[derive(Node, Default)]
#[class_name = "Ticker"]
pub struct Ticker {
    pub name: String,
    pub ticks: u32,
    pub readied: bool,
}

impl NodeTrait for Ticker {
    fn ready(&mut self, _ctx: &mut NodeContext) {
        self.readied = true;
    }

    fn process(&mut self, _dt: f32, _ctx: &mut NodeContext) {
        self.ticks += 1;
    }

    fn z(&self) -> f32 {
        3.0
    }
}

#[test]
fn class_name_is_the_attribute_value() {
    assert_eq!(<Widget as Object>::class_name(), "Widget");
    assert_eq!(<Simple as Object>::class_name(), "Simple");
    assert_eq!(<Ticker as Object>::class_name(), "Ticker");
}

#[test]
fn as_any_downcasts_to_the_concrete_type() {
    let w = Widget {
        name: "w1".into(),
        count: 7,
    };
    let dyn_ref: &dyn Object = &w;
    let concrete = dyn_ref.as_any().downcast_ref::<Widget>().unwrap();
    assert_eq!(concrete.count, 7);
}

#[test]
fn as_any_mut_downcasts_and_mutates() {
    let mut w = Widget {
        name: "w1".into(),
        count: 7,
    };
    let dyn_mut: &mut dyn Object = &mut w;
    dyn_mut.as_any_mut().downcast_mut::<Widget>().unwrap().count = 99;
    assert_eq!(w.count, 99);
}

#[test]
fn name_getter_returns_the_field() {
    let w = Widget {
        name: "hello".into(),
        count: 0,
    };
    assert_eq!(NodeName::name(&w), "hello");
}

#[test]
fn name_setter_writes_the_field() {
    let mut w = Widget {
        name: "old".into(),
        count: 0,
    };
    NodeName::set_name(&mut w, "new".into());
    assert_eq!(w.name, "new");
}

#[test]
fn derived_node_can_be_used_as_dyn_node() {
    let w = Widget {
        name: "w".into(),
        count: 3,
    };
    let dyn_node: &dyn NodeTrait = &w;
    assert_eq!(dyn_node.name(), "w");
    assert_eq!(dyn_node.z(), 0.0);
}

#[test]
fn derived_node_is_send_sync_static() {
    fn assert_bounds<T: Send + Sync + 'static>() {}
    assert_bounds::<Widget>();
    assert_bounds::<Simple>();
    assert_bounds::<Ticker>();
}

#[test]
fn derived_node_with_behaviour_runs_in_a_tree() {
    let mut tree = NodeTree::with_root(Ticker {
        name: "t".into(),
        ..Default::default()
    });
    let root = tree.root().unwrap();
    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016); // flush runs ready, then process
    tree.process(&mut h.servers(), 0.016);

    let node = tree.get(root).unwrap();
    assert_eq!(node.name(), "t");
    assert_eq!(node.z(), 3.0);
    let t = node.as_any().downcast_ref::<Ticker>().unwrap();
    assert!(t.readied);
    assert_eq!(t.ticks, 2);
}

#[test]
fn derived_node_with_behaviour_registers_in_class_db() {
    let mut db = ClassDB::new();
    db.register_node::<Ticker>();
    let node = db.instantiate_node("Ticker").unwrap();
    assert_eq!(node.z(), 3.0);
}
