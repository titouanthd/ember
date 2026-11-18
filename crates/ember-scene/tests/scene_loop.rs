mod common;
use common::*;
use ember_core::MainLoop;
use ember_core::math::Rect;
use ember_scene::prelude::*;
use ember_servers::{MockAudio, MockRenderer};

fn make() -> (SceneLoop<MockRenderer, MockAudio>, Log) {
    let log = new_log();
    let (tree, _) = sample_tree(&log);
    let l = SceneLoop::new(
        tree,
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    );
    (l, log)
}

#[test]
fn drives_init_physics_process_draw_finalize() {
    let (mut l, log) = make();
    assert!(l.init());
    l.physics_process(1.0 / 60.0);
    assert!(l.process(0.016));
    l.draw();
    l.rendering().assert_frame_closed();
    l.finalize();

    let e = entries(&log);
    let pos = |s: &str| e.iter().position(|x| x == s).unwrap();
    assert!(pos("root:ready") < pos("root:physics"));
    assert!(pos("root:physics") < pos("root:process"));
    assert!(pos("root:process") < pos("root:draw"));
    assert!(pos("root:draw") < pos("root:finalize"));
    assert!(l.tree.is_empty());
}

#[test]
fn request_quit_ends_the_loop() {
    let (mut l, _) = make();
    l.request_quit();
    assert!(!l.process(0.016));
}
