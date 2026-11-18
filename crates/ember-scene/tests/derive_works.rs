//! Happy-path tests for `#[derive(Node)]`.
//!
//! Failure-path tests (missing `class_name`, missing `name: String`,
//! applied to a non-struct) are deferred to a `trybuild` PR after
//! Phase 5.

use ember_core::Object;
use ember_macros::Node;
use ember_scene::Node as NodeTrait;

#[derive(Node)]
#[class_name = "Widget"]
pub struct Widget {
    pub name: String,
    pub count: u32,
}

#[derive(Node)]
#[class_name = "Simple"]
pub struct Simple {
    pub name: String,
}

#[test]
fn class_name_is_the_attribute_value() {
    assert_eq!(<Widget as Object>::class_name(), "Widget");
    assert_eq!(<Simple as Object>::class_name(), "Simple");
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
    assert_eq!(NodeTrait::name(&w), "hello");
}

#[test]
fn name_setter_writes_the_field() {
    let mut w = Widget {
        name: "old".into(),
        count: 0,
    };
    NodeTrait::set_name(&mut w, "new".into());
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
}

#[test]
fn derived_node_is_send_sync_static() {
    fn assert_bounds<T: Send + Sync + 'static>() {}
    assert_bounds::<Widget>();
    assert_bounds::<Simple>();
}
