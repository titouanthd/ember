mod common;
use common::*;
use ember_scene::prelude::*;

#[test]
fn removed_id_is_stale_even_when_the_slot_is_reused() {
    let log = new_log();
    let mut tree = NodeTree::new();
    let old = tree.insert(Recorder::new("old", &log));
    assert!(tree.get(old).is_some());

    tree.remove(old);
    assert!(tree.get(old).is_none());
    assert!(!tree.contains(old));

    let new = tree.insert(Recorder::new("new", &log));
    assert_ne!(old, new);
    assert!(tree.get(old).is_none());
    assert_eq!(tree.get(new).unwrap().name(), "new");
}

#[test]
fn removing_a_subtree_invalidates_every_descendant_id() {
    let log = new_log();
    let (mut tree, [_, a, a1, a2, _]) = sample_tree(&log);
    tree.remove(a);
    for id in [a, a1, a2] {
        assert!(tree.get(id).is_none());
    }
}

#[test]
fn remove_finalizes_bottom_up() {
    let log = new_log();
    let (mut tree, [_, a, ..]) = sample_tree(&log);
    tree.remove(a);
    assert_eq!(entries(&log), ["a1:finalize", "a2:finalize", "a:finalize"]);
}

#[test]
fn null_id_is_never_live() {
    let log = new_log();
    let tree = NodeTree::with_root(Recorder::new("r", &log));
    assert!(NodeId::default().is_null());
    assert!(tree.get(NodeId::default()).is_none());
}
