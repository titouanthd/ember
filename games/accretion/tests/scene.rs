//! Headless run of the node-based Accretion on mock backends.

use std::sync::atomic::{AtomicUsize, Ordering};

use accretion::app::{AppScreen, title_begin_rect};
use accretion::scene::{Accretion, WINDOW_H, WINDOW_W, build_tree};
use accretion::systems::Game;
use accretion::{disk, frame};
use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};

type Loop = SceneLoop<MockRenderer, MockAudio>;

/// Counts saves instead of writing `save.ron` in the working directory.
static SAVES: AtomicUsize = AtomicUsize::new(0);

fn count_save(_: &Game) -> bool {
    SAVES.fetch_add(1, Ordering::SeqCst);
    true
}

fn never_save(_: &Game) -> bool {
    true
}

fn game_with(saver: fn(&Game) -> bool) -> Loop {
    let mut l = SceneLoop::new(
        build_tree(Game::default(), saver),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    )
    .with_viewport_following_window();
    assert!(l.init());
    l.resize(WINDOW_W, WINDOW_H);
    l
}

fn game() -> Loop {
    game_with(never_save)
}

/// One frame: draw (so the viewport is the window), then process.
fn frame_with(l: &mut Loop, input: Input, dt: f32) -> bool {
    l.draw();
    l.set_input(input);
    let running = l.process(dt);
    l.set_input(Input::default());
    running
}

fn step(l: &mut Loop, input: Input) -> bool {
    frame_with(l, input, 1.0 / 60.0)
}

fn press(l: &mut Loop, key: Key) {
    assert!(step(
        l,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        }
    ));
}

fn click_at(l: &mut Loop, pos: Vec2) {
    assert!(step(
        l,
        Input {
            mouse_pos: pos,
            mouse_left_pressed: true,
            mouse_left_down: true,
            ..Input::default()
        }
    ));
}

fn accretion(l: &mut Loop) -> &mut Accretion {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Accretion>()
        .unwrap()
}

/// From a fresh run, `Begin` leads into the game.
fn begin(l: &mut Loop) {
    step(l, Input::default());
    click_at(l, title_begin_rect().center());
    assert_eq!(accretion(l).app.screen, AppScreen::Game);
}

#[test]
fn ready_installs_the_default_font() {
    let l = game();
    let calls = l.rendering().calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::CreateFont(_)))
    );
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::SetDefaultFont(Some(_))))
    );
}

#[test]
fn the_frame_follows_the_window() {
    let mut l = game();
    l.resize(1000.0, 700.0);
    step(&mut l, Input::default());
    assert_eq!(
        (frame::screen_width(), frame::screen_height()),
        (1000.0, 700.0)
    );
}

#[test]
fn begin_then_clicking_the_disk_gathers_matter() {
    let mut l = game();
    begin(&mut l);
    let before: f64 = accretion(&mut l).app.game.resources.values().sum();
    click_at(&mut l, disk::center());
    let after: f64 = accretion(&mut l).app.game.resources.values().sum();
    assert!(after > before, "{before} → {after}");
}

#[test]
fn number_keys_set_the_speed() {
    let mut l = game();
    begin(&mut l);
    assert_eq!(accretion(&mut l).app.speed(), 1.0);
    press(&mut l, Key::Num3);
    assert_eq!(accretion(&mut l).app.speed(), 3.0);
    press(&mut l, Key::Num1);
    assert_eq!(accretion(&mut l).app.speed(), 1.0);
}

#[test]
fn the_buildings_tab_clips_its_cards() {
    let mut l = game();
    begin(&mut l);
    press(&mut l, Key::B);
    l.draw();
    let calls = l.rendering().calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::PushClipRect { .. }))
    );
    assert!(calls.iter().any(|c| matches!(c, RecordedCall::PopClipRect)));
}

#[test]
fn playing_autosaves_every_thirty_seconds() {
    let mut l = game_with(count_save);
    begin(&mut l);
    let start = SAVES.load(Ordering::SeqCst);
    for _ in 0..31 {
        assert!(frame_with(&mut l, Input::default(), 1.0));
    }
    assert_eq!(SAVES.load(Ordering::SeqCst) - start, 1);
}
