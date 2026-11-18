mod common;
use common::*;
use ember_scene::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

struct Ping;
struct Pong;

#[test]
fn deferred_a_b_cycle_runs_to_completion_without_recursing() {
    const LIMIT: u32 = 200;
    let log = new_log();
    let (mut tree, [a, b, _]) = script_tree(&log, |ctx| ctx.emit(Ping));
    let count = Arc::new(AtomicU32::new(0));

    let n = count.clone();
    tree.connect(a, move |ctx: &mut NodeContext, _: &Ping| {
        if n.fetch_add(1, Ordering::SeqCst) < LIMIT {
            ctx.emit_deferred(Pong);
        }
    });
    let n = count.clone();
    tree.connect(b, move |ctx: &mut NodeContext, _: &Pong| {
        if n.fetch_add(1, Ordering::SeqCst) < LIMIT {
            ctx.emit_deferred(Ping);
        }
    });

    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    assert!(count.load(Ordering::SeqCst) > LIMIT);
}

#[test]
fn deferred_emits_are_fifo_and_wait_for_the_end_of_the_pass() {
    let log = new_log();
    let (mut tree, [a, _, _]) = script_tree(&log, |ctx| {
        ctx.emit_deferred(1u32);
        ctx.emit_deferred(2u32);
        ctx.emit(0u32);
    });
    let l = log.clone();
    tree.connect(a, move |_: &mut NodeContext, v: &u32| {
        l.lock().unwrap().push(format!("sig{v}"));
    });
    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    let sigs: Vec<_> = entries(&log)
        .into_iter()
        .filter(|e| e.starts_with("sig"))
        .collect();
    assert_eq!(sigs, ["sig0", "sig1", "sig2"]);
}
