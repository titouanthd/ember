//! `#[export]` properties, loading them from `.escene` and writing live
//! trees back with `PackedScene::from_tree` / `SceneWriter` (ADR-037).

use ember_core::{ClassDB, Color, Variant};
use ember_macros::Node;
use ember_scene::prelude::*;
use glam::Vec2;

#[derive(Node, Default)]
#[class_name = "Player"]
struct Player {
    name: String,
    #[export]
    speed: f32,
    #[export]
    hp: u32,
    #[export]
    tint: Color,
    #[export]
    visible: bool,
    /// Not exported: comes from the constructor, never saved.
    frames: u32,
}

impl ember_scene::Node for Player {
    fn visible(&self) -> bool {
        self.visible
    }
}

#[derive(Node, Default)]
#[class_name = "Sprite"]
struct Sprite {
    name: String,
    #[export]
    size: Vec2,
    #[export]
    texture: String,
    #[export(skip)]
    cache: u8,
}

impl ember_scene::Node for Sprite {
    fn z(&self) -> f32 {
        2.0
    }
}

/// Not registered in the `ClassDB` below.
#[derive(Node, Default)]
#[class_name = "Stray"]
struct Stray {
    name: String,
}

impl ember_scene::Node for Stray {}

fn classes() -> ClassDB {
    let mut db = ClassDB::new();
    db.register_node_with(|| Player {
        visible: true,
        frames: 7,
        ..Default::default()
    });
    db.register_node::<Sprite>();
    db
}

const SCENE: &str = r#"(
    format: 1,
    uid: "uid://player",
    nodes: [
        (name: "Player", class: "Player",
         properties: { "speed": Float(120.0), "hp": Int(3), "tint": Color(1.0, 0.5, 0.0, 1.0) }),
        (name: "Body", class: "Sprite", parent: ".",
         properties: { "size": Vec2(32.0, 48.0), "texture": String("player.png") }),
        (name: "Gun", class: "Sprite", parent: "Body",
         properties: { "size": Vec2(8.0, 4.0) }),
    ],
)"#;

fn get<T: 'static>(tree: &NodeTree, id: NodeId) -> &T {
    tree.get(id).unwrap().as_any().downcast_ref::<T>().unwrap()
}

fn load(text: &str) -> (NodeTree, NodeId) {
    let scene = PackedScene::from_ron(text).unwrap();
    let mut tree = NodeTree::new();
    let root = scene.instantiate(&classes(), &mut tree).unwrap();
    tree.set_root(root);
    (tree, root)
}

#[test]
fn derive_generates_the_property_methods() {
    let mut p = Player::default();
    assert_eq!(p.property_list(), ["speed", "hp", "tint", "visible"]);
    assert!(p.set_property("speed", &Variant::Int(5)), "Int into f32");
    assert!(p.set_property("hp", &Variant::Int(9)));
    assert!(!p.set_property("hp", &Variant::Int(-1)), "out of range");
    assert!(!p.set_property("hp", &Variant::String("x".into())));
    assert!(!p.set_property("frames", &Variant::Int(1)), "not exported");
    assert_eq!(p.get_property("speed"), Some(Variant::Float(5.0)));
    assert_eq!(p.get_property("hp"), Some(Variant::Int(9)));
    assert_eq!(p.get_property("frames"), None);
    let sprite = Sprite::default();
    assert_eq!(sprite.property_list(), ["size", "texture"], "skip honoured");
    assert_eq!(sprite.cache, 0);
    assert!(Stray::default().property_list().is_empty());
}

#[test]
fn escene_properties_reach_exported_fields() {
    let (tree, root) = load(SCENE);
    let p = get::<Player>(&tree, root);
    assert_eq!((p.speed, p.hp), (120.0, 3));
    assert_eq!(p.tint, Color::new(1.0, 0.5, 0.0, 1.0));
    assert_eq!(p.frames, 7, "unexported field keeps the constructor value");
    let gun = tree.find(root, "Body/Gun").unwrap();
    assert_eq!(get::<Sprite>(&tree, gun).size, Vec2::new(8.0, 4.0));
}

#[test]
fn a_live_tree_saves_and_loads_back() {
    let (mut tree, root) = load(SCENE);
    let body = tree.find(root, "Body").unwrap();
    // Edit like an editor would: through the property API.
    let node = tree.get_mut(body).unwrap();
    assert!(node.set_property("texture", &Variant::String("hero.png".into())));
    tree.get_mut(root)
        .unwrap()
        .set_property("visible", &Variant::Bool(false));

    let saved = PackedScene::from_tree(&tree, root, &classes(), "uid://player").unwrap();
    let text = saved.to_ron();
    let (tree2, root2) = load(&text);

    let p = get::<Player>(&tree2, root2);
    assert_eq!((p.speed, p.hp, p.visible), (120.0, 3, false));
    let body2 = tree2.find(root2, "Body").unwrap();
    assert_eq!(get::<Sprite>(&tree2, body2).texture, "hero.png");
    assert_eq!(tree2.get(body2).unwrap().z(), 2.0);
    let gun2 = tree2.find(root2, "Body/Gun").unwrap();
    assert_eq!(get::<Sprite>(&tree2, gun2).size, Vec2::new(8.0, 4.0));

    // Saving the reloaded tree gives the same file.
    let again = PackedScene::from_tree(&tree2, root2, &classes(), "uid://player").unwrap();
    assert_eq!(again, saved);
}

#[test]
fn saving_an_unregistered_node_fails() {
    let (mut tree, root) = load(SCENE);
    let stray = tree.insert(Stray {
        name: "Stray".into(),
    });
    tree.attach(stray, root).unwrap();
    let err = PackedScene::from_tree(&tree, root, &classes(), "uid://x").unwrap_err();
    assert_eq!(err, SceneError::NotSaveable("Stray".into()));
}

#[test]
fn scene_writer_validates_like_the_loader() {
    let record = |name: &str, parent: Option<&str>| NodeRecord {
        name: name.into(),
        class: "Sprite".into(),
        parent: parent.map(str::to_string),
        properties: Default::default(),
        visible: true,
        z: 0.0,
        groups: Vec::new(),
    };
    let mut w = SceneWriter::new("uid://w");
    w.write_node(record("Root", None));
    w.write_node(record("A", Some(".")));
    w.write_node(record("A", Some(".")));
    assert!(matches!(w.finish(), Err(SceneError::InvalidName(_))));

    let mut w = SceneWriter::new("uid://w");
    w.write_node(record("Root", None));
    w.write_node(record("A", Some(".")));
    w.write_node(record("B", Some("A")));
    let scene = w.finish().unwrap();
    assert_eq!(scene.uid(), "uid://w");
    assert_eq!(scene.root_template_class(), "Sprite");
}
