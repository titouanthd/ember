//! Headless run of the node-based Air Hockey on mock backends.

use air_hockey::scene::{AirHockeyGame, WINDOW_H, WINDOW_W, build_tree, countdown_label};
use air_hockey::systems::GameMode;
use air_hockey::{Game, GameContext, GameEvent, Phase};
use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};

type Scene = SceneLoop<MockRenderer, MockAudio>;

fn scene() -> Scene {
    let ctx = GameContext::default_hermetic();
    let game = Game::new(&ctx);
    let mut s = SceneLoop::new(
        build_tree(ctx, game),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
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

fn app(s: &mut Scene) -> &mut AirHockeyGame {
    let root = s.tree.root().unwrap();
    s.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<AirHockeyGame>()
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

/// Space, then idle frames until the countdown is over.
fn start_pvp(s: &mut Scene) {
    press(s, Key::Space);
    for _ in 0..600 {
        if app(s).game.phase == Phase::Playing {
            return;
        }
        step(s, Input::default());
    }
    panic!("still {:?}", app(s).game.phase);
}

#[test]
fn the_menu_loads_font_and_sounds_and_shows_the_title() {
    let mut s = scene();
    let a = app(&mut s);
    assert!(a.font_loaded, "DejaVu Sans Mono is the default font");
    assert!(a.sounds.is_some());
    assert_eq!(a.game.phase, Phase::Menu);
    let t = texts(&s);
    assert!(t.iter().any(|x| x == "AIR HOCKEY"));
    assert!(t.iter().any(|x| x == "ENTRÉE : Joueur vs IA"));
    assert!(t.iter().any(|x| x == "J1   0  —  0   J2"));
}

#[test]
fn space_starts_a_pvp_countdown_that_beeps_then_goes() {
    let mut s = scene();
    press(&mut s, Key::Space);
    let a = app(&mut s);
    assert!(matches!(a.game.phase, Phase::Countdown { .. }));
    assert_eq!(a.game.mode, GameMode::Pvp);
    assert_eq!(a.last_events, vec![GameEvent::CountdownBeep]);
    assert!(texts(&s).iter().any(|x| x == "3"));
    start_pvp(&mut s);
    assert_eq!(app(&mut s).last_events, vec![GameEvent::CountdownGo]);
}

#[test]
fn enter_starts_against_the_ai() {
    let mut s = scene();
    press(&mut s, Key::Enter);
    assert_eq!(app(&mut s).game.mode, GameMode::PvAi);
}

#[test]
fn player_one_moves_with_d_and_p_pauses() {
    let mut s = scene();
    start_pvp(&mut s);
    let x0 = app(&mut s).game.paddles[0].pos.x;
    hold(&mut s, Key::D, 10);
    assert!(app(&mut s).game.paddles[0].pos.x > x0);
    press(&mut s, Key::P);
    assert!(app(&mut s).game.paused);
    assert!(texts(&s).iter().any(|x| x == "PAUSE"));
    let x1 = app(&mut s).game.paddles[0].pos.x;
    hold(&mut s, Key::D, 10);
    assert_eq!(app(&mut s).game.paddles[0].pos.x, x1, "frozen while paused");
}

#[test]
fn the_countdown_label_punches_and_ends_on_go() {
    assert_eq!(countdown_label(3.0, 3.0), ("3".to_string(), 1.4));
    assert_eq!(countdown_label(3.0, 0.1).0, "GO!");
}

#[test]
fn escape_quits() {
    let mut s = scene();
    press(&mut s, Key::Escape);
    assert!(s.tree.quit_requested());
}
