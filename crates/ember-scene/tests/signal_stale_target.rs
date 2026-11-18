mod common;
use common::*;
use ember_scene::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

struct Hit;

#[test]
fn a_despawned_target_is_skipped_and_its_handlers_dropped() {
    let log = new_log();
    let calls = Arc::new(AtomicU32::new(0));
    let (mut tree, [a, b, _]) = script_tree(&log, |ctx| ctx.emit(Hit));
    for id in [a, b] {
        let calls = calls.clone();
        tree.connect(id, move |_: &mut NodeContext, _: &Hit| {
            calls.fetch_add(1, Ordering::SeqCst);
        });
    }
    tree.remove(a);
    assert_eq!(tree.signals().len(), 1);

    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn a_target_despawned_via_ctx_is_skipped_once_the_flush_applied_it() {
    let log = new_log();
    let calls = Arc::new(AtomicU32::new(0));
    let mut frame = 0;
    let (mut tree, [a, _, _]) = script_tree(&log, move |ctx| {
        frame += 1;
        if frame == 1 {
            let target = ctx.find("a").expect("child a");
            ctx.despawn(target);
        } else {
            ctx.emit(Hit);
        }
    });
    let counter = calls.clone();
    tree.connect(a, move |_: &mut NodeContext, _: &Hit| {
        counter.fetch_add(1, Ordering::SeqCst);
    });

    let mut h = Harness::new();
    tree.process(&mut h.servers(), 0.016);
    tree.process(&mut h.servers(), 0.016);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(tree.signals().is_empty());
}
