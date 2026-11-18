mod common;
use common::*;
use ember_scene::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

struct Hit;

fn emit_once(tree: &mut NodeTree, h: &mut Harness) {
    tree.process(&mut h.servers(), 0.016);
}

#[test]
fn dropping_a_scoped_connection_disconnects() {
    let log = new_log();
    let (mut tree, [a, _, _]) = script_tree(&log, |ctx| ctx.emit(Hit));
    let calls = Arc::new(AtomicU32::new(0));
    let n = calls.clone();
    let conn = tree.connect_scoped(a, move |_: &mut NodeContext, _: &Hit| {
        n.fetch_add(1, Ordering::SeqCst);
    });

    let mut h = Harness::new();
    emit_once(&mut tree, &mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    drop(conn);
    emit_once(&mut tree, &mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(tree.signals().is_empty());
}

#[test]
fn only_the_dropped_connection_goes_away() {
    let log = new_log();
    let (mut tree, [a, b, _]) = script_tree(&log, |ctx| ctx.emit(Hit));
    let calls = Arc::new(AtomicU32::new(0));
    let n = calls.clone();
    let keep = tree.connect_scoped(a, move |_: &mut NodeContext, _: &Hit| {
        n.fetch_add(1, Ordering::SeqCst);
    });
    let gone = tree.connect_scoped(b, |_: &mut NodeContext, _: &Hit| {});
    drop(gone);

    let mut h = Harness::new();
    emit_once(&mut tree, &mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(tree.signals().len(), 1);
    drop(keep);
}

#[test]
fn forget_keeps_the_handler_connected() {
    let log = new_log();
    let (mut tree, [a, _, _]) = script_tree(&log, |_| {});
    tree.connect_scoped(a, |_: &mut NodeContext, _: &Hit| {})
        .forget();
    let mut h = Harness::new();
    emit_once(&mut tree, &mut h);
    assert_eq!(tree.signals().len(), 1);
}
