//! Headless run of the node-based Breakout on mock backends.

use breakout::GameState;
use breakout::config::load_config;
use breakout::scene::{BreakoutGame, MAX_LEVELS, build_tree};
use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};

type Scene = SceneLoop<MockRenderer, MockAudio>;

fn scene() -> Scene {
    let ctx = load_config();
    let (w, h) = (ctx.screen_w, ctx.screen_h);
    let mut s = SceneLoop::new(
        build_tree(ctx),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    assert!(s.init());
    s.draw();
    step(&mut s, Input::default());
    s
}

fn step(s: &mut Scene, input: Input) {
    s.set_input(input);
    s.process(1.0 / 60.0);
    s.draw();
}

fn app(s: &mut Scene) -> &mut BreakoutGame {
    let root = s.tree.root().unwrap();
    s.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<BreakoutGame>()
        .unwrap()
}

fn press(s: &mut Scene, key: Key) {
    step(
        s,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    );
}

fn hold(s: &mut Scene, key: Key, frames: usize) {
    for _ in 0..frames {
        step(
            s,
            Input {
                keys_down: vec![key],
                ..Input::default()
            },
        );
    }
}

fn texts(s: &Scene) -> Vec<String> {
    s.rendering()
        .calls()
        .iter()
        .filter_map(|c| match c {
            RecordedCall::DrawText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_start_screen_shows_level_one() {
    let mut s = scene();
    let a = app(&mut s);
    assert_eq!(a.world.state, GameState::Start);
    assert_eq!(a.world.max_levels, MAX_LEVELS);
    let alive = a.world.bricks.iter().filter(|b| b.is_alive()).count();
    assert!(alive > 0);
    let t = texts(&s);
    assert!(t.iter().any(|x| x == "BREAKOUT"));
    assert!(
        t.iter()
            .any(|x| x == &format!("Level 1 - {alive} bricks to destroy!"))
    );
}

#[test]
fn space_serves_and_the_ball_moves() {
    let mut s = scene();
    press(&mut s, Key::Space);
    assert_eq!(app(&mut s).world.state, GameState::Playing);
    let before = app(&mut s).world.ball.transform.position;
    step(&mut s, Input::default());
    assert_ne!(app(&mut s).world.ball.transform.position, before);
}

#[test]
fn arrows_and_a_d_move_the_paddle_only_while_playing() {
    let mut s = scene();
    let x0 = app(&mut s).world.paddle.transform.position.x;
    hold(&mut s, Key::Left, 5);
    assert_eq!(app(&mut s).world.paddle.transform.position.x, x0);
    press(&mut s, Key::Space);
    let x1 = app(&mut s).world.paddle.transform.position.x;
    hold(&mut s, Key::A, 5);
    let x2 = app(&mut s).world.paddle.transform.position.x;
    assert!(x2 < x1);
    hold(&mut s, Key::Right, 10);
    assert!(app(&mut s).world.paddle.transform.position.x > x2);
}

#[test]
fn clearing_a_level_loads_the_next_one_faster() {
    let mut s = scene();
    press(&mut s, Key::Space);
    let speed = app(&mut s).ctx.ball_speed;
    for b in &mut app(&mut s).world.bricks {
        b.health = 0;
    }
    step(&mut s, Input::default());
    let a = app(&mut s);
    assert_eq!(a.world.current_level, 1);
    assert!((a.ctx.ball_speed - speed * 1.1).abs() < 1e-3);
}

#[test]
fn game_over_shows_the_banner_and_r_restarts() {
    let mut s = scene();
    press(&mut s, Key::Space);
    app(&mut s).world.state = GameState::GameOver;
    step(&mut s, Input::default());
    assert!(texts(&s).iter().any(|x| x == "GAME OVER"));
    press(&mut s, Key::R);
    let a = app(&mut s);
    assert_eq!(a.world.state, GameState::Start);
    assert_eq!((a.world.score, a.world.current_level), (0, 0));
}

#[test]
fn escape_quits() {
    let mut s = scene();
    press(&mut s, Key::Escape);
    assert!(s.tree.quit_requested());
}
