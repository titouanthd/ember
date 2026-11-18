mod common;
use common::*;
use ember_scene::prelude::*;

#[test]
fn process_is_depth_first_top_down_in_insertion_order() {
    let log = new_log();
    let (mut tree, _) = sample_tree(&log);
    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016); // first call also runs ready
    log.lock().unwrap().clear();

    tree.process(&mut h.servers(), 0.016);
    assert_eq!(
        entries(&log),
        [
            "root:process",
            "a:process",
            "a1:process",
            "a2:process",
            "b:process"
        ]
    );
}

#[test]
fn physics_process_uses_the_same_order() {
    let log = new_log();
    let (mut tree, _) = sample_tree(&log);
    let mut h = Harness::new();
    tree.physics_process(&mut h.servers(), 1.0 / 60.0);
    let physics: Vec<_> = entries(&log)
        .into_iter()
        .filter(|e| e.ends_with(":physics"))
        .collect();
    assert_eq!(
        physics,
        [
            "root:physics",
            "a:physics",
            "a1:physics",
            "a2:physics",
            "b:physics"
        ]
    );
}

#[test]
fn draw_runs_parents_before_children_and_skips_hidden_subtrees() {
    let log = new_log();
    let (mut tree, _) = sample_tree(&log);
    let mut h = Harness::new();
    tree.flush(&mut h.servers());
    log.lock().unwrap().clear();

    tree.draw(&mut DrawContext::new(&mut h.render));
    assert_eq!(
        entries(&log),
        ["root:draw", "a:draw", "a1:draw", "a2:draw", "b:draw"]
    );
}

#[test]
fn hidden_node_hides_its_subtree_but_not_its_siblings() {
    let log = new_log();
    let mut hidden = Recorder::new("hidden", &log);
    hidden.visible = false;
    let mut tree = NodeTree::with_root(Recorder::new("root", &log));
    let root = tree.root().unwrap();
    let h_id = tree.insert(hidden);
    let under = tree.insert(Recorder::new("under", &log));
    let sibling = tree.insert(Recorder::new("sibling", &log));
    tree.attach(h_id, root).unwrap();
    tree.attach(under, h_id).unwrap();
    tree.attach(sibling, root).unwrap();

    let mut h = Harness::new();
    tree.draw(&mut DrawContext::new(&mut h.render));
    assert_eq!(entries(&log), ["root:draw", "sibling:draw"]);
}

#[test]
fn draw_sets_each_nodes_z_on_the_server() {
    use ember_servers::RecordedCall;
    let log = new_log();
    let mut top = Recorder::new("top", &log);
    top.z = 5.0;
    let tree = NodeTree::with_root(top);
    let mut h = Harness::new();
    h.render.clear_calls();
    tree.draw(&mut DrawContext::new(&mut h.render));
    assert!(h.render.calls().contains(&RecordedCall::SetZ(5.0)));
}
