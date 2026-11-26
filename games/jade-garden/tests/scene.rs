//! Headless run of the node-based Jade Garden on mock backends.

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedAudioCall, RecordedCall};
use jade_garden::Progress;
use jade_garden::app::AppScreen;
use jade_garden::config::GameContext;
use jade_garden::scene::{JadeGarden, WINDOW_H, WINDOW_W, build_tree};

type Loop = SceneLoop<MockRenderer, MockAudio>;

/// A fresh campaign that is never saved: these tests win no level and
/// never open the epilogue, the only paths that write progress.
fn game() -> Loop {
    let mut l = SceneLoop::new(
        build_tree(GameContext::defaults(), Progress::default()),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    )
    .with_viewport_following_window();
    assert!(l.init());
    l.resize(WINDOW_W, WINDOW_H);
    l
}

/// One frame: draw (so the viewport is the window), then process.
fn frame(l: &mut Loop, input: Input) -> bool {
    l.draw();
    l.set_input(input);
    let running = l.process(1.0 / 60.0);
    l.set_input(Input::default());
    running
}

fn idle(l: &mut Loop, frames: usize) {
    for _ in 0..frames {
        assert!(frame(l, Input::default()));
    }
}

fn press(l: &mut Loop, key: Key) -> bool {
    frame(
        l,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    )
}

fn click_at(l: &mut Loop, pos: Vec2) -> bool {
    frame(
        l,
        Input {
            mouse_pos: pos,
            mouse_left_pressed: true,
            mouse_left_down: true,
            ..Input::default()
        },
    )
}

fn jade(l: &mut Loop) -> &mut JadeGarden {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<JadeGarden>()
        .unwrap()
}

fn screen(l: &mut Loop) -> AppScreen {
    jade(l).app.screen()
}

#[test]
fn ready_installs_both_fonts_and_loads_the_three_tracks() {
    let mut l = game();
    let calls = l.rendering().calls();
    let fonts = calls
        .iter()
        .filter(|c| matches!(c, RecordedCall::CreateFont(_)))
        .count();
    assert_eq!(fonts, 2);
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::SetDefaultFont(Some(_))))
    );
    let clips = l
        .audio()
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedAudioCall::LoadClip(_)))
        .count();
    assert_eq!(clips, 3);
    assert_eq!(screen(&mut l), AppScreen::Menu);
}

#[test]
fn the_music_starts_on_the_first_frame() {
    let mut l = game();
    idle(&mut l, 1);
    assert!(l.audio().calls().iter().any(|c| matches!(
        c,
        RecordedAudioCall::PlayLooped { volume, .. } if *volume == 0.4
    )));
}

#[test]
fn the_viewport_follows_the_window() {
    let mut l = game();
    l.resize(1000.0, 700.0);
    idle(&mut l, 1);
    let ctx = &jade(&mut l).ctx;
    assert_eq!((ctx.viewport_w, ctx.viewport_h), (1000.0, 700.0));
}

#[test]
fn a_level_card_opens_the_chapter_intro_then_the_level() {
    let mut l = game();
    idle(&mut l, 1);
    let viewport = Vec2::new(WINDOW_W, WINDOW_H);
    let card = jade(&mut l)
        .app
        .menu()
        .level_card_rect(0, viewport)
        .expect("level 0 has a card");
    assert!(click_at(&mut l, card.center()));
    assert_eq!(screen(&mut l), AppScreen::Intro(0));

    // The intro ignores input for its first 0.8 s.
    idle(&mut l, 60);
    assert!(press(&mut l, Key::Enter));
    assert_eq!(screen(&mut l), AppScreen::Game(0));
    assert!(jade(&mut l).app.game().is_some());

    // The board draws for a while, then Escape returns to the menu.
    idle(&mut l, 120);
    assert!(press(&mut l, Key::Escape));
    assert_eq!(screen(&mut l), AppScreen::Menu);
    assert!(jade(&mut l).app.game().is_none());
}

#[test]
fn tab_opens_the_help_which_is_clipped_and_scrolls() {
    let mut l = game();
    idle(&mut l, 1);
    assert!(press(&mut l, Key::Tab));
    assert!(jade(&mut l).app.help_open());
    l.draw();
    assert!(
        l.rendering()
            .calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::PushClipRect { .. }))
    );
    assert!(press(&mut l, Key::Tab));
    assert!(!jade(&mut l).app.help_open());
    // Escape on the menu quits.
    assert!(!press(&mut l, Key::Escape));
}

/// Kept from the macroquad loop: on the menu, Escape closes the help and
/// the menu then sees the same Escape, so the game quits.
#[test]
fn escape_with_the_help_open_on_the_menu_still_quits() {
    let mut l = game();
    idle(&mut l, 1);
    assert!(press(&mut l, Key::Tab));
    assert!(!press(&mut l, Key::Escape));
}
