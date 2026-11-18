//! Headless run of the node-based Pong on mock backends.

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer};
use pong::scene::{Pong, Scoreboard, build_tree};
use pong::{GameContext, GameState};

fn game() -> SceneLoop<MockRenderer, MockAudio> {
    let cfg = GameContext::default();
    let (w, h) = (cfg.screen_w, cfg.screen_h);
    let mut l = SceneLoop::new(
        build_tree(cfg, 42),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    assert!(l.init());
    l
}

fn press(l: &mut SceneLoop<MockRenderer, MockAudio>, key: Key) {
    let mut input = Input::default();
    input.keys_pressed.push(key);
    input.keys_down.push(key);
    l.set_input(input);
    l.process(1.0 / 60.0);
    l.set_input(Input::default());
}

fn pong(l: &mut SceneLoop<MockRenderer, MockAudio>) -> &mut Pong {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Pong>()
        .unwrap()
}

fn board(l: &SceneLoop<MockRenderer, MockAudio>) -> (u32, u32) {
    let root = l.tree.root().unwrap();
    let id = l.tree.find(root, "Scoreboard").unwrap();
    let b = l
        .tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<Scoreboard>()
        .unwrap();
    (b.player, b.ai)
}

#[test]
fn space_starts_and_ball_moves() {
    let mut l = game();
    assert_eq!(pong(&mut l).state.state, GameState::Start);
    press(&mut l, Key::Space);
    assert_eq!(pong(&mut l).state.state, GameState::Playing);
    let x0 = pong(&mut l).ball().transform.position.x;
    l.process(1.0 / 60.0);
    assert_ne!(pong(&mut l).ball().transform.position.x, x0);
}

#[test]
fn scoreboard_follows_signals_and_resets() {
    let mut l = game();
    press(&mut l, Key::Space);
    pong(&mut l).state.ball.transform.position.x = -100.0;
    l.process(1.0 / 60.0);
    assert_eq!(board(&l), (0, 1));
    pong(&mut l).state.ball.transform.position.x = 10_000.0;
    l.process(1.0 / 60.0);
    assert_eq!(board(&l), (1, 1));

    pong(&mut l).state.state = GameState::GameOver;
    press(&mut l, Key::R);
    assert_eq!(board(&l), (0, 0));
    assert_eq!(pong(&mut l).state.state, GameState::Start);
}

#[test]
fn draws_a_closed_frame_and_escape_quits() {
    let mut l = game();
    l.draw();
    l.rendering().assert_frame_closed();
    let mut input = Input::default();
    input.keys_pressed.push(Key::Escape);
    l.set_input(input);
    assert!(!l.process(1.0 / 60.0));
}
