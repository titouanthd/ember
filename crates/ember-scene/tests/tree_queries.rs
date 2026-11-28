//! Typed tree queries (ADR-051): `get_as`, `find_by_class`,
//! `find_all_by_class`, `children_of`, and their `NodeContext` forms.

mod common;
use common::*;
use ember_macros::Node;
use ember_scene::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Node, Default)]
#[class_name = "Unit"]
struct Unit {
    name: String,
    hp: u32,
}

impl ember_scene::Node for Unit {}

#[derive(Node, Default)]
#[class_name = "Marker"]
struct Marker {
    name: String,
}

impl ember_scene::Node for Marker {}

fn unit(name: &str, hp: u32) -> Unit {
    Unit {
        name: name.into(),
        hp,
    }
}

fn marker(name: &str) -> Marker {
    Marker { name: name.into() }
}

/// root(Marker) -> [m(Marker) -> [u1(Unit) -> [u2(Unit)]], u3(Unit), x(Marker)]
fn mixed_tree() -> (NodeTree, [NodeId; 6]) {
    let mut t = NodeTree::with_root(marker("root"));
    let root = t.root().unwrap();
    let m = t.insert(marker("m"));
    let u1 = t.insert(unit("u1", 1));
    let u2 = t.insert(unit("u2", 2));
    let u3 = t.insert(unit("u3", 3));
    let x = t.insert(marker("x"));
    t.attach(m, root).unwrap();
    t.attach(u1, m).unwrap();
    t.attach(u2, u1).unwrap();
    t.attach(u3, root).unwrap();
    t.attach(x, root).unwrap();
    (t, [root, m, u1, u2, u3, x])
}

#[test]
fn get_as_downcasts_or_returns_none() {
    let (mut t, [root, _, u1, ..]) = mixed_tree();
    assert_eq!(t.get_as::<Unit>(u1).unwrap().hp, 1);
    assert!(t.get_as::<Marker>(u1).is_none(), "wrong type");
    assert!(t.get_as::<Unit>(root).is_none());
    t.get_as_mut::<Unit>(u1).unwrap().hp = 10;
    assert_eq!(t.get_as::<Unit>(u1).unwrap().hp, 10);
    t.remove(u1);
    assert!(t.get_as::<Unit>(u1).is_none(), "stale id");
}

#[test]
fn find_by_class_is_the_first_descendant_in_tree_order() {
    let (t, [root, m, u1, u2, u3, x]) = mixed_tree();
    assert_eq!(t.find_by_class::<Unit>(root), Some(u1));
    assert_eq!(t.find_by_class::<Unit>(u1), Some(u2), "excludes `from`");
    assert_eq!(t.find_by_class::<Marker>(root), Some(m), "excludes `from`");
    assert_eq!(t.find_by_class::<Marker>(u3), None, "leaf");
    assert_eq!(t.find_by_class::<Marker>(m), None);
    let _ = x;
}

#[test]
fn find_all_by_class_follows_descendants_order() {
    let (t, [root, m, u1, u2, u3, x]) = mixed_tree();
    assert_eq!(t.find_all_by_class::<Unit>(root), vec![u1, u2, u3]);
    assert_eq!(t.find_all_by_class::<Marker>(root), vec![m, x]);
    assert_eq!(t.find_all_by_class::<Unit>(m), vec![u1, u2]);
    assert!(t.find_all_by_class::<Unit>(NodeId::default()).is_empty());
}

#[test]
fn children_of_yields_direct_typed_children_in_child_order() {
    let (mut t, [root, _, u1, ..]) = mixed_tree();
    let late = t.insert(unit("late", 9));
    t.attach_at(late, root, 0).unwrap();
    let got: Vec<(NodeId, u32)> = t
        .children_of::<Unit>(root)
        .map(|(id, u)| (id, u.hp))
        .collect();
    assert_eq!(got.len(), 2, "u2 is a grandchild, markers are skipped");
    assert_eq!(got[0], (late, 9));
    assert_eq!(got[1].1, 3);
    assert_eq!(t.children_of::<Unit>(u1).count(), 1);
    assert_eq!(t.children_of::<Marker>(u1).count(), 0);
}

#[test]
fn context_queries_search_the_whole_tree_and_skip_the_current_node() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let out = seen.clone();
    // The root is a `Script`: checked out while it runs, so never found.
    let mut t = NodeTree::with_root(Script::new("script", move |ctx| {
        let units = ctx.find_all_by_class::<Unit>();
        let alive = units
            .iter()
            .filter(|id| ctx.tree().get_as::<Unit>(**id).is_some_and(|u| u.hp > 0))
            .count();
        let first = ctx
            .find_by_class::<Unit>()
            .map(|id| ctx.tree().get(id).unwrap().name().to_string());
        let scripts = ctx.find_all_by_class::<Script>().len();
        let direct: Vec<String> = ctx
            .children_of::<Unit>()
            .map(|(_, u)| u.name.clone())
            .collect();
        out.lock().unwrap().push(format!(
            "units={} alive={alive} first={first:?} scripts={scripts} direct={direct:?}",
            units.len()
        ));
    }));
    let root = t.root().unwrap();
    let wave = t.insert(marker("wave"));
    t.attach(wave, root).unwrap();
    for (name, hp) in [("a", 3), ("b", 0), ("c", 1)] {
        let id = t.insert(unit(name, hp));
        t.attach(id, wave).unwrap();
    }
    let top = t.insert(unit("top", 5));
    t.attach(top, root).unwrap();

    let mut h = Harness::new();
    t.process(&mut h.servers(), 0.016);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [r#"units=4 alive=3 first=Some("a") scripts=0 direct=["top"]"#]
    );
}

#[test]
fn context_queries_include_the_root_and_handle_no_root() {
    let found: Arc<Mutex<Vec<usize>>> = Arc::default();
    let out = found.clone();
    let mut t = NodeTree::with_root(marker("root"));
    let root = t.root().unwrap();
    let probe = t.insert(Script::new("probe", move |ctx| {
        out.lock()
            .unwrap()
            .push(ctx.find_all_by_class::<Marker>().len());
        assert_eq!(ctx.find_by_class::<Marker>(), ctx.tree().root());
    }));
    t.attach(probe, root).unwrap();
    let mut h = Harness::new();
    t.process(&mut h.servers(), 0.016);
    assert_eq!(found.lock().unwrap().as_slice(), [1]);
}
