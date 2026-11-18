mod common;
use common::*;
use ember_scene::prelude::*;

struct Hit(u32);

#[test]
fn handlers_run_in_connection_order_with_target_as_current() {
    let log = new_log();
    let sink = log.clone();
    let (mut tree, [a, b, c]) = script_tree(&log, |ctx| ctx.emit(Hit(7)));
    for (name, id) in [("c", c), ("a", a), ("b", b)] {
        let sink = sink.clone();
        tree.connect(id, move |ctx: &mut NodeContext, hit: &Hit| {
            assert_eq!(ctx.current_id(), id);
            sink.lock().unwrap().push(format!("{name}:{}", hit.0));
        });
    }
    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    let hits: Vec<_> = entries(&log)
        .into_iter()
        .filter(|e| e.ends_with(":7"))
        .collect();
    assert_eq!(hits, ["c:7", "a:7", "b:7"]);
}

#[test]
fn nested_emit_finishes_before_the_outer_dispatch_continues() {
    struct Outer;
    struct Inner;
    let log = new_log();
    let (mut tree, [a, b, _]) = script_tree(&log, |ctx| ctx.emit(Outer));
    let l = log.clone();
    tree.connect(a, move |ctx: &mut NodeContext, _: &Outer| {
        l.lock().unwrap().push("outer-1".into());
        ctx.emit(Inner);
        l.lock().unwrap().push("outer-1-done".into());
    });
    let l = log.clone();
    tree.connect(b, move |_: &mut NodeContext, _: &Outer| {
        l.lock().unwrap().push("outer-2".into());
    });
    let l = log.clone();
    tree.connect(b, move |_: &mut NodeContext, _: &Inner| {
        l.lock().unwrap().push("inner".into());
    });
    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    let order: Vec<_> = entries(&log)
        .into_iter()
        .filter(|e| e.starts_with("outer") || e == "inner")
        .collect();
    assert_eq!(order, ["outer-1", "inner", "outer-1-done", "outer-2"]);
}

#[test]
fn disconnect_removes_only_that_type_and_target() {
    struct Ping;
    struct Pong;
    let log = new_log();
    let (mut tree, [a, b, _]) = script_tree(&log, |_| {});
    tree.connect(a, |_: &mut NodeContext, _: &Ping| {});
    tree.connect(a, |_: &mut NodeContext, _: &Pong| {});
    tree.connect(b, |_: &mut NodeContext, _: &Ping| {});
    tree.disconnect::<Ping>(a);
    assert_eq!(tree.signals().len(), 2);
}
