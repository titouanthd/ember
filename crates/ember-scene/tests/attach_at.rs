//! `NodeTree::attach_at` (ADR-045): a child placed among its siblings.

mod common;
use common::{Recorder, new_log};
use ember_scene::{NodeTree, SceneError};

#[test]
fn attach_at_places_the_child_and_clamps_the_index() {
    let log = new_log();
    let mut tree = NodeTree::with_root(Recorder::new("root", &log));
    let root = tree.root().unwrap();
    let a = tree.insert(Recorder::new("a", &log));
    let b = tree.insert(Recorder::new("b", &log));
    let c = tree.insert(Recorder::new("c", &log));
    tree.attach(a, root).unwrap();
    tree.attach_at(b, root, 0).unwrap();
    tree.attach_at(c, root, 99).unwrap();
    assert_eq!(tree.children(root), [b, a, c]);
    tree.detach(a).unwrap();
    tree.attach_at(a, root, 1).unwrap();
    assert_eq!(tree.children(root), [b, a, c]);
    assert_eq!(
        tree.attach_at(a, root, 0),
        Err(SceneError::AlreadyAttached(a))
    );
    assert_eq!(
        tree.children(root),
        [b, a, c],
        "a refused attach changes nothing"
    );
}
