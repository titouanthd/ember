use ember_core::ClassDB;
use ember_scene::prelude::*;

#[derive(Default)]
struct Plain;

#[derive(Default, ember_macros::Node)]
#[class_name = "Derived"]
struct Derived {
    name: String,
}

impl Node for Derived {}

impl Object for Plain {
    fn class_name() -> &'static str {
        "Plain"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[test]
fn derived_node_instantiates_as_object_and_as_node() {
    let mut db = ClassDB::new();
    db.register_node::<Derived>();

    let obj = db.instantiate("Derived").expect("object");
    assert!(obj.as_any().is::<Derived>());

    let mut node = db.instantiate_node("Derived").expect("node");
    node.set_name("Hero".into());
    assert_eq!(node.name(), "Hero");
    assert!(node.as_any().is::<Derived>());
}

#[test]
fn custom_node_constructor_is_used() {
    let mut db = ClassDB::new();
    db.register_node_with::<Derived>(|| Derived {
        name: "preset".into(),
    });
    let node = db.instantiate_node("Derived").unwrap();
    assert_eq!(node.name(), "preset");
}

#[test]
fn plain_objects_and_unknown_names_are_not_nodes() {
    let mut db = ClassDB::new();
    db.register::<Plain>();
    assert!(db.instantiate("Plain").is_some());
    assert!(db.instantiate_node("Plain").is_none());
    assert!(db.instantiate_node("Missing").is_none());
}
