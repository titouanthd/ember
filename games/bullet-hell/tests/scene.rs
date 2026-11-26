//! Headless run of the node-based Bullet Hell on mock backends.

use bullet_hell::GameState;
use bullet_hell::config::load_config;
use bullet_hell::scene::{BulletHellGame, build_tree, ship_input};
use bullet_hell::systems::World;
use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};

type Scene = SceneLoop<MockRenderer, MockAudio>;

fn scene() -> Scene {
    let ctx = load_config();
    let world = World::new(&ctx);
    let (w, h) = (ctx.window_w, ctx.window_h);
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

fn app(s: &mut Scene) -> &mut BulletHellGame {
    let root = s.tree.root().unwrap();
    s.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<BulletHellGame>()
        .unwrap()
}

fn keys(pressed: &[Key], down: &[Key]) -> Input {
    Input {
        keys_pressed: pressed.to_vec(),
        keys_down: down.to_vec(),
        ..Input::default()
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
fn the_title_screen_shows_and_hides_the_ship() {
    let s = scene();
    let t = texts(&s);
    assert!(t.iter().any(|x| x == "BULLET HELL"));
    assert!(t.iter().any(|x| x == "PRESS SPACE TO START"));
    assert!(t.iter().any(|x| x.starts_with("SCORE 000000")));
    let triangles = s
        .rendering()
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedCall::DrawTriangle { .. }))
        .count();
    assert_eq!(triangles, 0, "no ship before the run starts");
}

#[test]
fn space_or_enter_starts_the_first_wave() {
    for key in [Key::Space, Key::Enter] {
        let mut s = scene();
        step(&mut s, keys(&[key], &[key]));
        let a = app(&mut s);
        assert_eq!(a.world.state, GameState::Playing);
        assert_eq!(a.world.wave, 0);
        assert!(!a.world.enemies.is_empty());
    }
}

#[test]
fn keys_map_to_the_ship_input_including_shift_focus() {
    let i = ship_input(&keys(&[], &[Key::A, Key::Up, Key::LeftShift, Key::Space]));
    assert_eq!((i.dx, i.dy, i.focus, i.fire), (-1.0, -1.0, true, true));
    let i = ship_input(&keys(&[], &[Key::Right, Key::S, Key::RightShift]));
    assert_eq!((i.dx, i.dy, i.focus, i.fire), (1.0, 1.0, true, false));
}

#[test]
fn moving_and_firing_while_playing() {
    let mut s = scene();
    step(&mut s, keys(&[Key::Space], &[Key::Space]));
    let x0 = app(&mut s).world.player.transform.position.x;
    for _ in 0..10 {
        step(&mut s, keys(&[], &[Key::Left, Key::Space]));
    }
    let a = app(&mut s);
    assert!(a.world.player.transform.position.x < x0);
    assert!(!a.world.player_bullets.is_empty());
}

#[test]
fn focus_draws_the_hitbox_rings() {
    let mut s = scene();
    step(&mut s, keys(&[Key::Space], &[Key::Space]));
    let rings = |s: &Scene| {
        s.rendering()
            .calls()
            .iter()
            .filter(|c| matches!(c, RecordedCall::DrawCircleOutline { .. }))
            .count()
    };
    let before = rings(&s);
    step(&mut s, keys(&[], &[Key::LeftShift]));
    assert!(app(&mut s).world.player.focus);
    let bullets = {
        let w = &app(&mut s).world;
        w.player_bullets.len() + w.enemy_bullets.len()
    };
    assert_eq!(rings(&s) - before, bullets + 2, "one ring per bullet + 2");
}

#[test]
fn game_over_shows_the_banner_and_r_resets() {
    let mut s = scene();
    step(&mut s, keys(&[Key::Space], &[Key::Space]));
    app(&mut s).world.state = GameState::GameOver;
    step(&mut s, Input::default());
    assert!(texts(&s).iter().any(|x| x == "GAME OVER"));
    step(&mut s, keys(&[Key::R], &[Key::R]));
    let a = app(&mut s);
    assert_eq!((a.world.score, a.world.lives), (0, 3));
}
