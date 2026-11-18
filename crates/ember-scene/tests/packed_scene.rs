use ember_core::{ClassDB, Variant};
use ember_scene::prelude::*;

macro_rules! thing {
    ($t:ident) => {
        #[derive(Default)]
        struct $t {
            name: String,
            hp: f64,
            size: glam::Vec2,
            visible: bool,
            z: f32,
        }

        impl Object for $t {
            fn class_name() -> &'static str {
                stringify!($t)
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        impl Node for $t {
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
            fn set_property(&mut self, key: &str, v: &Variant) -> bool {
                match key {
                    "max_hp" => v.as_f64().map(|x| self.hp = x).is_some(),
                    "speed" => v.as_f64().is_some(),
                    "size" => v.as_vec2().map(|x| self.size = x).is_some(),
                    "visible" => v.as_bool().map(|x| self.visible = x).is_some(),
                    "z" => v.as_f64().map(|x| self.z = x as f32).is_some(),
                    _ => false,
                }
            }
        }
    };
}

thing!(Player);
thing!(Sprite);

fn classes() -> ClassDB {
    let mut db = ClassDB::new();
    db.register_node_with(|| Player {
        visible: true,
        ..Default::default()
    });
    db.register_node_with(|| Sprite {
        visible: true,
        ..Default::default()
    });
    db
}

const FIXTURE: &str = include_str!("../../../assets/test_fixtures/player.escene");

#[test]
fn fixture_round_trips() {
    let a = PackedScene::from_ron(FIXTURE).unwrap();
    let b = PackedScene::from_ron(&a.to_ron()).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.root_template_class(), "Player");
}

#[test]
fn fixture_instantiates() {
    let db = classes();
    let scene = PackedScene::from_ron(FIXTURE).unwrap();
    let mut tree = NodeTree::new();
    let root = scene.instantiate(&db, &mut tree).unwrap();
    tree.set_root(root);

    assert_eq!(tree.children(root).len(), 2);
    let sprite = tree.find(root, "Sprite").unwrap();
    assert_eq!(tree.get(sprite).unwrap().z(), 1.0);
    assert!(tree.find(root, "Sprite/Gun").is_some());
    let hitbox = tree.find(root, "Hitbox").unwrap();
    assert!(!tree.get(hitbox).unwrap().visible());
}

#[test]
fn rejects_bad_input() {
    let bad = |s: &str| PackedScene::from_ron(s).unwrap_err();
    assert_eq!(
        bad(r#"(format: 2, uid: "u", nodes: [(name: "A", class: "C")])"#),
        SceneError::UnsupportedFormat(2)
    );
    assert!(matches!(
        bad(r#"(format: 1, uid: "u", nodes: [(name: "A/B", class: "C")])"#),
        SceneError::InvalidName(_)
    ));
    assert!(matches!(
        bad(r#"(format: 1, uid: "u", nodes: [(name: "A", class: "C"),
            (name: "B", class: "C", parent: "."), (name: "B", class: "C", parent: ".")])"#),
        SceneError::InvalidName(_)
    ));
    assert!(matches!(
        bad(
            r#"(format: 1, uid: "u", nodes: [(name: "A", class: "C"), (name: "B", class: "C", parent: "X")])"#
        ),
        SceneError::Parse(_)
    ));
    assert!(matches!(bad("(nonsense"), SceneError::Parse(_)));
}

#[test]
fn unregistered_class_leaves_tree_untouched() {
    let scene = PackedScene::from_ron(FIXTURE).unwrap();
    let mut tree = NodeTree::new();
    let err = scene.instantiate(&ClassDB::new(), &mut tree).unwrap_err();
    assert_eq!(err, SceneError::ClassNotRegistered("Player".into()));
    assert_eq!(tree.len(), 0);
}

#[test]
fn instantiate_into_attaches() {
    let db = classes();
    let scene = PackedScene::from_ron(FIXTURE).unwrap();
    let mut tree = NodeTree::new();
    let host = tree.insert(Player::default());
    tree.set_root(host);
    let sub = scene.instantiate_into(&db, &mut tree, host).unwrap();
    assert_eq!(tree.parent(sub), Some(host));
}
