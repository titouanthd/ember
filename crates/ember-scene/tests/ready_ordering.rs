mod common;
use common::*;

#[test]
fn ready_fires_bottom_up_once() {
    let log = new_log();
    let (mut tree, _) = sample_tree(&log);
    let mut h = Harness::new();

    tree.process(&mut h.servers(), 0.016);
    tree.process(&mut h.servers(), 0.016);

    let ready: Vec<_> = entries(&log)
        .into_iter()
        .filter(|e| e.ends_with(":ready"))
        .collect();
    assert_eq!(
        ready,
        ["a1:ready", "a2:ready", "a:ready", "b:ready", "root:ready"]
    );
}

#[test]
fn ready_precedes_the_first_process() {
    let log = new_log();
    let (mut tree, _) = sample_tree(&log);
    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    let all = entries(&log);
    let last_ready = all.iter().rposition(|e| e.ends_with(":ready")).unwrap();
    let first_process = all.iter().position(|e| e.ends_with(":process")).unwrap();
    assert!(last_ready < first_process);
}

#[test]
fn nodes_attached_later_get_ready_on_the_next_flush() {
    let log = new_log();
    let (mut tree, [root, ..]) = sample_tree(&log);
    let mut h = Harness::new();
    tree.flush(&mut h.servers());
    log.lock().unwrap().clear();

    let late = tree.insert(Recorder::new("late", &log));
    tree.attach(late, root).unwrap();
    tree.flush(&mut h.servers());
    assert_eq!(entries(&log), ["late:ready"]);
}
