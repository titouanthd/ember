//! Headless run of the node-based Asteroids on mock backends.

use asteroids::GameState;
use asteroids::config::load_config;
use asteroids::scene::{AsteroidsGame, build_tree};
use asteroids::systems::GameWorld;
use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use ember_stdlib::persistence::Persistence;

type Scene = SceneLoop<MockRenderer, MockAudio>;

fn scene(name: &str) -> Scene {
    let ctx = load_config();
    let mut world = GameWorld::new(&ctx);
    // Never touch the project's highscore.ron.
    let path = std::env::temp_dir().join(format!("asteroids_scene_best_{name}.ron"));
    let _ = std::fs::remove_file(&path);
    world.high_score_handle = Persistence::new(&path);
    world.high_score = 0;
    let (w, h) = (ctx.screen_w, ctx.screen_h);
    let mut s = SceneLoop::new(
        build_tree(ctx, world),
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

fn app(s: &mut Scene) -> &mut AsteroidsGame {
    let root = s.tree.root().unwrap();
    s.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<AsteroidsGame>()
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

fn count(s: &Scene, f: impl Fn(&RecordedCall) -> bool) -> usize {
    s.rendering().calls().iter().filter(|c| f(c)).count()
}

#[test]
fn the_start_screen_shows_title_and_stars() {
    let s = scene("start");
    let t = texts(&s);
    assert!(t.iter().any(|x| x == "ASTEROIDS"));
    assert!(t.iter().any(|x| x == "Press SPACE to start"));
    assert!(count(&s, |c| matches!(c, RecordedCall::DrawCircle { .. })) >= 60);
}

#[test]
fn space_starts_a_wave_and_draws_ship_and_asteroids() {
    let mut s = scene("wave");
    press(&mut s, Key::Space);
    let a = app(&mut s);
    assert_eq!(a.world.state, GameState::Playing);
    assert_eq!(a.world.wave, 1);
    let rocks = a.world.asteroids.len();
    assert!(rocks > 0);
    let before = count(&s, |c| matches!(c, RecordedCall::DrawLine { .. }));
    step(&mut s, Input::default());
    let lines = count(&s, |c| matches!(c, RecordedCall::DrawLine { .. })) - before;
    assert_eq!(lines, rocks * 8, "8 outline segments per asteroid");
}

#[test]
fn rotate_and_thrust_move_the_ship() {
    let mut s = scene("fly");
    press(&mut s, Key::Space);
    let r0 = app(&mut s).world.ship.transform.rotation;
    hold(&mut s, Key::Right, 10);
    assert!(app(&mut s).world.ship.transform.rotation > r0);
    hold(&mut s, Key::Up, 10);
    assert!(app(&mut s).world.ship.velocity.length() > 0.0);
}

#[test]
fn holding_space_fires_at_the_cooldown_rate() {
    let mut s = scene("fire");
    // This press starts the game; firing needs Space held while playing.
    press(&mut s, Key::Space);
    for rock in &mut app(&mut s).world.asteroids {
        rock.transform.position = Vec2::new(5.0, 5.0);
        rock.velocity = Vec2::ZERO;
    }
    hold(&mut s, Key::Space, 1);
    assert_eq!(app(&mut s).world.bullets.len(), 1);
    hold(&mut s, Key::Space, 1);
    assert_eq!(app(&mut s).world.bullets.len(), 1, "cooldown holds fire");
}

#[test]
fn clearing_a_wave_shows_the_banner_then_starts_the_next() {
    let mut s = scene("cleared");
    press(&mut s, Key::Space);
    app(&mut s).world.asteroids.clear();
    step(&mut s, Input::default());
    assert_eq!(app(&mut s).world.state, GameState::LevelCleared);
    assert!(texts(&s).iter().any(|x| x == "WAVE 1 CLEARED!"));
    let secs = app(&mut s).ctx.wave_transition_secs;
    for _ in 0..((secs * 60.0) as usize + 2) {
        step(&mut s, Input::default());
    }
    let a = app(&mut s);
    assert_eq!((a.world.state, a.world.wave), (GameState::Playing, 2));
}

#[test]
fn game_over_shows_the_banner_and_r_resets() {
    let mut s = scene("over");
    press(&mut s, Key::Space);
    app(&mut s).world.state = GameState::GameOver;
    step(&mut s, Input::default());
    assert!(texts(&s).iter().any(|x| x == "GAME OVER"));
    press(&mut s, Key::R);
    assert_eq!(app(&mut s).world.state, GameState::Start);
}

#[test]
fn escape_quits() {
    let mut s = scene("quit");
    press(&mut s, Key::Escape);
    assert!(s.tree.quit_requested());
}
