//! Headless run of the node-based Ember Wars on mock backends.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use ember_wars::catapult::ShotKind;
use ember_wars::config::GameContext;
use ember_wars::menu::{self, AppScreen, CONTENT_TOP};
use ember_wars::progress::Progress;
use ember_wars::scene::{EmberWars, WINDOW_H, WINDOW_W, build_tree};
use ember_wars::{Phase, frame};

type Loop = SceneLoop<MockRenderer, MockAudio>;

fn fixed_seed() -> u32 {
    0xC0FFEE
}

/// A fresh campaign whose saves are counted, never written.
fn game_counting(saves: Arc<AtomicUsize>) -> Loop {
    let save = Box::new(move |_: &Progress| {
        saves.fetch_add(1, Ordering::SeqCst);
    });
    let mut l = SceneLoop::new(
        build_tree(
            GameContext::from_env(),
            Progress::default(),
            save,
            fixed_seed,
        ),
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
    game_counting(Arc::new(AtomicUsize::new(0)))
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

fn press(l: &mut Loop, key: Key) -> bool {
    step(
        l,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    )
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

fn wars(l: &mut Loop) -> &mut EmberWars {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<EmberWars>()
        .unwrap()
}

/// The first level's card, clicked from the menu.
fn start_first_level(l: &mut Loop) {
    step(l, Input::default());
    let card = menu::level_rect(
        0,
        frame::screen_width(),
        menu::chapter_top_y(0, CONTENT_TOP),
    );
    click_at(l, card.center());
    assert_eq!(wars(l).app.screen, AppScreen::Game);
    assert!(wars(l).app.game.is_some());
}

#[test]
fn ready_installs_the_default_font() {
    let l = game();
    assert!(
        l.rendering()
            .calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::SetDefaultFont(Some(_))))
    );
}

#[test]
fn the_viewport_follows_the_window() {
    let mut l = game();
    l.resize(1200.0, 700.0);
    step(&mut l, Input::default());
    let ctx = &wars(&mut l).ctx;
    assert_eq!((ctx.viewport_w, ctx.viewport_h), (1200.0, 700.0));
    assert_eq!(frame::screen_width(), 1200.0);
}

#[test]
fn clicking_the_first_level_starts_it_and_the_battle_draws() {
    let mut l = game();
    start_first_level(&mut l);
    for _ in 0..120 {
        assert!(step(&mut l, Input::default()));
    }
    l.draw();
    let calls = l.rendering().calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawRect { .. }))
    );
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawText { .. }))
    );
}

#[test]
fn number_keys_pick_the_shot() {
    let mut l = game();
    start_first_level(&mut l);
    press(&mut l, Key::Num2);
    assert_eq!(
        wars(&mut l).app.game.as_ref().unwrap().catapult.shot_kind,
        ShotKind::Piercing
    );
    press(&mut l, Key::Num3);
    assert_eq!(
        wars(&mut l).app.game.as_ref().unwrap().catapult.shot_kind,
        ShotKind::Explosive
    );
}

#[test]
fn tab_toggles_the_upgrade_modal() {
    let mut l = game();
    step(&mut l, Input::default());
    assert!(press(&mut l, Key::Tab));
    assert!(wars(&mut l).app.menu_state.upgrade_modal_open);
    assert!(press(&mut l, Key::Tab));
    assert!(!wars(&mut l).app.menu_state.upgrade_modal_open);
    // Escape on the menu quits.
    assert!(!press(&mut l, Key::Escape));
}

/// Kept from the macroquad loop: with the modal open, one Escape closes
/// it in the menu branch, and the loop's later Escape check then sees the
/// menu with no modal and quits, in the same frame.
#[test]
fn escape_with_the_modal_open_still_quits() {
    let mut l = game();
    step(&mut l, Input::default());
    assert!(press(&mut l, Key::Tab));
    assert!(!press(&mut l, Key::Escape));
}

#[test]
fn a_finished_level_is_saved_once_and_enter_returns_to_the_menu() {
    let saves = Arc::new(AtomicUsize::new(0));
    let mut l = game_counting(saves.clone());
    start_first_level(&mut l);
    // End the battle: the player's tower falls.
    wars(&mut l).app.game.as_mut().unwrap().phase = Phase::Lost;
    assert!(step(&mut l, Input::default()));
    assert!(step(&mut l, Input::default()));
    assert_eq!(saves.load(Ordering::SeqCst), 1);
    assert!(press(&mut l, Key::Enter));
    assert_eq!(wars(&mut l).app.screen, AppScreen::Menu);
}
