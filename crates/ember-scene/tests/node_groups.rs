//! Node groups (ADR-051): membership, join order, cleanup on removal,
//! and `groups:` in `.escene` files.

mod common;
use common::*;
use ember_core::ClassDB;
use ember_macros::Node;
use ember_scene::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Node, Default)]
#[class_name = "Tile"]
struct Tile {
    name: String,
}

impl ember_scene::Node for Tile {}

fn tile(name: &str) -> Tile {
    Tile { name: name.into() }
}

fn classes() -> ClassDB {
    let mut db = ClassDB::new();
    db.register_node::<Tile>();
    db
}

#[test]
fn membership_keeps_join_order() {
    let mut t = NodeTree::new();
    let [a, b, c] = ["a", "b", "c"].map(|n| t.insert(tile(n)));
    assert!(t.add_to_group(c, "lanterns"));
    assert!(t.add_to_group(a, "lanterns"));
    assert!(t.add_to_group(b, "lanterns"));
    assert!(!t.add_to_group(a, "lanterns"), "already in");
    assert!(!t.add_to_group(a, ""), "empty name");
    assert_eq!(t.nodes_in_group("lanterns"), vec![c, a, b]);

    assert!(t.add_to_group(a, "solid"));
    assert!(t.is_in_group(a, "lanterns") && t.is_in_group(a, "solid"));
    assert!(!t.is_in_group(b, "solid"));
    assert_eq!(t.groups_of(a), ["lanterns", "solid"]);

    assert!(t.remove_from_group(a, "lanterns"));
    assert!(!t.remove_from_group(a, "lanterns"), "not in it any more");
    assert_eq!(t.nodes_in_group("lanterns"), vec![c, b]);
    // Rejoining goes to the back.
    assert!(t.add_to_group(a, "lanterns"));
    assert_eq!(t.nodes_in_group("lanterns"), vec![c, b, a]);
    assert_eq!(t.groups_of(a), ["solid", "lanterns"]);
    assert!(t.nodes_in_group("nope").is_empty());
}

#[test]
fn group_names_are_sorted_and_empty_groups_vanish() {
    let mut t = NodeTree::new();
    let a = t.insert(tile("a"));
    t.add_to_group(a, "zeta");
    t.add_to_group(a, "alpha");
    assert_eq!(t.group_names().collect::<Vec<_>>(), ["alpha", "zeta"]);
    t.remove_from_group(a, "zeta");
    assert_eq!(t.group_names().collect::<Vec<_>>(), ["alpha"]);
}

#[test]
fn removing_a_subtree_leaves_its_groups() {
    let mut t = NodeTree::with_root(tile("root"));
    let root = t.root().unwrap();
    let parent = t.insert(tile("p"));
    let child = t.insert(tile("c"));
    let other = t.insert(tile("o"));
    t.attach(parent, root).unwrap();
    t.attach(child, parent).unwrap();
    t.attach(other, root).unwrap();
    for id in [parent, child, other] {
        t.add_to_group(id, "pieces");
    }
    t.add_to_group(child, "only_child");

    t.remove(parent);
    assert_eq!(t.nodes_in_group("pieces"), vec![other]);
    assert!(!t.is_in_group(child, "pieces"), "stale id");
    assert!(t.groups_of(child).is_empty());
    assert_eq!(t.group_names().collect::<Vec<_>>(), ["pieces"]);

    t.clear();
    assert_eq!(t.group_names().count(), 0);
}

#[test]
fn despawn_cleans_up_at_flush_and_spawned_nodes_join_once_flushed() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let out = seen.clone();
    let mut frame = 0;
    let mut spawned = None;
    let mut t = NodeTree::with_root(Script::new("script", move |ctx| {
        frame += 1;
        let me = ctx.current_id();
        match frame {
            1 => {
                assert!(ctx.add_to_group("director"));
                assert!(ctx.is_in_group("director"));
                let id = ctx.spawn_child(me, tile("new"));
                // A queued node may join before it is live.
                assert!(ctx.tree().add_to_group(id, "pieces"));
                spawned = Some(id);
            }
            2 => {
                ctx.despawn(spawned.unwrap());
            }
            _ => {}
        }
        out.lock().unwrap().push(format!(
            "frame {frame}: pieces={} director={}",
            ctx.nodes_in_group("pieces").len(),
            ctx.nodes_in_group("director").len()
        ));
    }));
    let mut h = Harness::new();
    t.process(&mut h.servers(), 0.016);
    t.process(&mut h.servers(), 0.016);
    t.process(&mut h.servers(), 0.016);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [
            "frame 1: pieces=0 director=1", // queued, not live yet
            "frame 2: pieces=1 director=1", // despawn is deferred
            "frame 3: pieces=0 director=1",
        ]
    );
    assert_eq!(t.group_names().collect::<Vec<_>>(), ["director"]);
}

const GROUPED: &str = r#"(
    format: 1,
    uid: "uid://board",
    nodes: [
        (name: "Board", class: "Tile", groups: ["boards"]),
        (name: "L1", class: "Tile", parent: ".", groups: ["lanterns", "solid"]),
        (name: "Rock", class: "Tile", parent: "."),
        (name: "L2", class: "Tile", parent: ".", groups: ["lanterns"]),
    ],
)"#;

fn load(text: &str) -> (NodeTree, NodeId, PackedScene) {
    let scene = PackedScene::from_ron(text).unwrap();
    let mut tree = NodeTree::new();
    let root = scene.instantiate(&classes(), &mut tree).unwrap();
    tree.set_root(root);
    (tree, root, scene)
}

#[test]
fn escene_groups_load_in_file_order() {
    let (t, root, _) = load(GROUPED);
    let l1 = t.find(root, "L1").unwrap();
    let l2 = t.find(root, "L2").unwrap();
    assert_eq!(t.nodes_in_group("lanterns"), vec![l1, l2]);
    assert_eq!(t.nodes_in_group("boards"), vec![root]);
    assert_eq!(t.groups_of(l1), ["lanterns", "solid"]);
    assert!(t.groups_of(t.find(root, "Rock").unwrap()).is_empty());
}

#[test]
fn files_without_groups_still_load_and_write_no_groups_key() {
    let plain = r#"(format: 1, uid: "uid://p", nodes: [(name: "A", class: "Tile")])"#;
    let (t, root, scene) = load(plain);
    assert_eq!(t.group_names().count(), 0);
    assert!(!scene.to_ron().contains("groups"));
    let saved = PackedScene::from_tree(&t, root, &classes(), "uid://p").unwrap();
    assert!(!saved.to_ron().contains("groups"));
}

#[test]
fn groups_round_trip_through_from_tree_and_resave() {
    let db = classes();
    let scene = PackedScene::from_ron(GROUPED).unwrap();
    let mut t = NodeTree::new();
    let tracked = scene.instantiate_tracked(&db, &mut t).unwrap();
    let root = tracked.root;

    // Unchanged resave gives the same text.
    let resaved = scene.resave(&t, root, &db, &tracked.origins).unwrap();
    assert_eq!(resaved.to_ron(), scene.to_ron());

    // Runtime changes are saved, in join order.
    let rock = t.find(root, "Rock").unwrap();
    let l1 = t.find(root, "L1").unwrap();
    t.add_to_group(rock, "solid");
    t.remove_from_group(l1, "solid");
    t.add_to_group(l1, "goal");
    let saved = PackedScene::from_tree(&t, root, &db, "uid://board").unwrap();
    let (back, broot, _) = load(&saved.to_ron());
    let id = |p: &str| back.find(broot, p).unwrap();
    assert_eq!(back.nodes_in_group("solid"), vec![id("Rock")]);
    assert_eq!(back.groups_of(id("L1")), ["lanterns", "goal"]);
    assert_eq!(back.nodes_in_group("lanterns"), vec![id("L1"), id("L2")]);
}

#[test]
fn scene_writer_writes_groups() {
    let mut w = SceneWriter::new("uid://w");
    w.write_node(NodeRecord {
        name: "Root".into(),
        class: "Tile".into(),
        parent: None,
        properties: Default::default(),
        visible: true,
        z: 0.0,
        groups: vec!["boards".into()],
    });
    let scene = w.finish().unwrap();
    assert!(scene.to_ron().contains("groups"));
    let (t, root, _) = load(&scene.to_ron());
    assert!(t.is_in_group(root, "boards"));
}

#[test]
fn empty_or_repeated_groups_are_rejected() {
    for groups in [r#"[""]"#, r#"["a", "a"]"#] {
        let text = format!(
            r#"(format: 1, uid: "u", nodes: [(name: "A", class: "Tile", groups: {groups})])"#
        );
        assert!(
            matches!(PackedScene::from_ron(&text), Err(SceneError::Parse(_))),
            "{groups}"
        );
    }
}

#[test]
fn an_instance_entry_adds_its_groups_after_the_nested_roots() {
    let piece = r#"(format: 1, uid: "uid://piece", nodes: [
        (name: "Piece", class: "Tile", groups: ["pieces"]),
    ])"#;
    let level = r#"(format: 1, uid: "uid://level", nodes: [
        (name: "Level", class: "Tile"),
        (name: "Box", instance: "piece.escene", parent: ".", groups: ["pushable", "pieces"]),
    ])"#;
    let dir = std::env::temp_dir().join(format!("ember-groups-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("piece.escene"), piece).unwrap();
    std::fs::write(dir.join("level.escene"), level).unwrap();
    let server = ember_assets::AssetServer::new(&dir).unwrap();
    let mut t = NodeTree::new();
    let scene = server.scene("level.escene").unwrap();
    let root = scene
        .get()
        .instantiate_with(&classes(), &server, &mut t)
        .unwrap();
    let bx = t.find(root, "Box").unwrap();
    assert_eq!(t.groups_of(bx), ["pieces", "pushable"]);
}
