//! Headless run of the node-based Tàijí on mock backends.

use ember::prelude::*;
use ember_servers::{Camera2d, MockAudio, MockRenderer, RecordedCall};
use taiji::scene::{Taiji, build_tree};
use taiji::{AppState, GameContext};

type Game = SceneLoop<MockRenderer, MockAudio>;

fn game() -> Game {
    let mut l = SceneLoop::new(
        build_tree(),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    )
    .with_virtual_resolution(800.0, 600.0);
    assert!(l.init());
    l
}

fn press(l: &mut Game, key: Key) -> bool {
    l.set_input(Input {
        keys_pressed: vec![key],
        keys_down: vec![key],
        ..Input::default()
    });
    let running = l.process(1.0 / 60.0);
    l.set_input(Input::default());
    running
}

fn taiji(l: &mut Game) -> &mut Taiji {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Taiji>()
        .unwrap()
}

#[test]
fn space_starts_a_fresh_match_and_the_ball_moves() {
    let mut l = game();
    assert_eq!(taiji(&mut l).app.state, AppState::MainMenu);
    press(&mut l, Key::Space);
    assert_eq!(taiji(&mut l).app.state, AppState::Playing);
    let x0 = taiji(&mut l).match_state.ball.transform.position.x;
    l.process(1.0 / 60.0);
    assert!(taiji(&mut l).match_state.ball.transform.position.x > x0);
}

#[test]
fn escape_pauses_resumes_and_quits_from_the_menu() {
    let mut l = game();
    press(&mut l, Key::Space);
    press(&mut l, Key::Escape);
    assert_eq!(taiji(&mut l).app.state, AppState::Paused);
    let x = taiji(&mut l).match_state.ball.transform.position.x;
    l.process(1.0 / 60.0);
    assert_eq!(
        taiji(&mut l).match_state.ball.transform.position.x,
        x,
        "paused"
    );
    press(&mut l, Key::Escape);
    assert_eq!(taiji(&mut l).app.state, AppState::Playing);

    let mut menu = game();
    assert!(!press(&mut menu, Key::Escape), "Escape on the menu quits");
}

#[test]
fn points_reset_the_round_and_five_end_the_match() {
    let mut l = game();
    press(&mut l, Key::Space);
    let win = GameContext::default().win_score;
    for point in 1..=win {
        taiji(&mut l).match_state.ball.transform.position.x = 10_000.0;
        l.process(1.0 / 60.0);
        assert_eq!(taiji(&mut l).app.score_player, point);
    }
    assert_eq!(taiji(&mut l).app.state, AppState::GameOver);
    press(&mut l, Key::Space);
    assert_eq!(taiji(&mut l).app.state, AppState::MainMenu);
    press(&mut l, Key::Space);
    assert_eq!(taiji(&mut l).app.score_player, 0);
}

#[test]
fn draws_on_the_letterboxed_canvas() {
    let mut l = game();
    l.resize(1600.0, 600.0);
    press(&mut l, Key::Space);
    l.draw();
    l.rendering().assert_frame_closed();
    let calls = l.rendering().calls();
    let letterbox = Camera2d::letterbox(Vec2::new(1600.0, 600.0), Vec2::new(800.0, 600.0));
    assert!(calls.contains(&RecordedCall::SetCamera(letterbox)));
    assert!(calls.iter().any(|c| matches!(
        c,
        RecordedCall::DrawRect { rect, color, .. }
            if *rect == Rect::new(0.0, 0.0, 800.0, 600.0) && *color == Color::WHITE
    )));
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawCircle { .. }))
    );
}
