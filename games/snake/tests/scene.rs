//! Headless run of the node-based Snake on mock backends.

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use snake::GameState;
use snake::components::Direction;
use snake::config::load_config;
use snake::scene::{LEVEL_CLEARED_SECS, MAX_LEVELS, SnakeGame, build_tree};

type Game = SceneLoop<MockRenderer, MockAudio>;

const DT: f32 = 1.0 / 60.0;

fn game() -> Game {
    let ctx = load_config();
    let (w, h) = (ctx.screen_w(), ctx.screen_h());
    let mut g = SceneLoop::new(
        build_tree(ctx),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    assert!(g.init());
    g
}

fn step(g: &mut Game, input: Input) -> bool {
    g.set_input(input);
    let running = g.process(DT);
    g.draw();
    running
}

fn press(g: &mut Game, key: Key) -> bool {
    step(
        g,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    )
}

fn snake(g: &mut Game) -> &mut SnakeGame {
    let root = g.tree.root().unwrap();
    g.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<SnakeGame>()
        .unwrap()
}

fn texts(g: &Game) -> Vec<String> {
    g.rendering()
        .calls()
        .iter()
        .filter_map(|c| match c {
            RecordedCall::DrawText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn opens_on_the_title_screen() {
    let mut g = game();
    step(&mut g, Input::default());
    assert_eq!(snake(&mut g).world.state, GameState::Start);
    let t = texts(&g);
    assert!(t.iter().any(|s| s == "SNAKE"));
    assert!(t.iter().any(|s| s == "Press SPACE to start"));
}

#[test]
fn space_starts_and_the_snake_moves_on_its_own() {
    let mut g = game();
    press(&mut g, Key::Space);
    assert_eq!(snake(&mut g).world.state, GameState::Playing);
    let head = snake(&mut g).world.snake.head();
    for _ in 0..60 {
        step(&mut g, Input::default());
    }
    assert_ne!(snake(&mut g).world.snake.head(), head);
}

#[test]
fn arrows_turn_the_snake() {
    let mut g = game();
    press(&mut g, Key::Space);
    press(&mut g, Key::Down);
    for _ in 0..30 {
        step(&mut g, Input::default());
    }
    assert_eq!(snake(&mut g).world.snake.direction, Direction::Down);
}

#[test]
fn game_over_then_r_returns_to_the_title_and_keeps_the_best() {
    let mut g = game();
    press(&mut g, Key::Space);
    snake(&mut g).world.score = 4;
    snake(&mut g).world.state = GameState::GameOver;
    step(&mut g, Input::default());
    assert!(texts(&g).iter().any(|s| s == "GAME OVER"));
    press(&mut g, Key::R);
    let s = snake(&mut g);
    assert_eq!(
        (s.world.state, s.world.score, s.best),
        (GameState::Start, 0, 4)
    );
}

#[test]
fn a_cleared_level_moves_on_after_the_banner() {
    let mut g = game();
    press(&mut g, Key::Space);
    snake(&mut g).world.state = GameState::LevelCleared;
    let frames = (LEVEL_CLEARED_SECS / DT) as usize + 2;
    for _ in 0..frames {
        step(&mut g, Input::default());
    }
    let s = snake(&mut g);
    assert_eq!((s.world.state, s.current_level), (GameState::Playing, 1));
}

#[test]
fn clearing_the_last_level_wins() {
    let mut g = game();
    press(&mut g, Key::Space);
    snake(&mut g).current_level = MAX_LEVELS - 1;
    snake(&mut g).world.state = GameState::LevelCleared;
    for _ in 0..((LEVEL_CLEARED_SECS / DT) as usize + 2) {
        step(&mut g, Input::default());
    }
    assert_eq!(snake(&mut g).world.state, GameState::Win);
    assert!(texts(&g).iter().any(|s| s == "YOU WIN!"));
}

#[test]
fn escape_quits() {
    let mut g = game();
    assert!(!press(&mut g, Key::Escape));
}
