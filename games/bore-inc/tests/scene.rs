//! Headless run of the node-based Bore Inc. on mock backends.

use std::sync::atomic::{AtomicUsize, Ordering};

use bore_inc::config::GameContext;
use bore_inc::progress::Progress;
use bore_inc::scene::{BoreInc, WINDOW_H, WINDOW_W, build_tree};
use bore_inc::systems::AppScreen;
use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall, TextureFilter};

type Loop = SceneLoop<MockRenderer, MockAudio>;

/// Counts saves instead of writing the real progress file.
static SAVES: AtomicUsize = AtomicUsize::new(0);

fn count_save(_: &Progress) -> Result<(), String> {
    SAVES.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn never_save(_: &Progress) -> Result<(), String> {
    Ok(())
}

fn game() -> Loop {
    game_with(never_save)
}

/// Only the autosave test counts saves, so parallel tests cannot race.
fn game_with(saver: fn(&Progress) -> Result<(), String>) -> Loop {
    let mut l = SceneLoop::new(
        build_tree(GameContext::defaults(), Progress::default(), saver),
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
fn frame(l: &mut Loop, input: Input) {
    l.draw();
    l.set_input(input);
    assert!(l.process(1.0 / 60.0));
    l.set_input(Input::default());
}

fn idle(l: &mut Loop, frames: usize) {
    for _ in 0..frames {
        frame(l, Input::default());
    }
}

fn press(l: &mut Loop, key: Key) {
    frame(
        l,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    );
}

fn hold(l: &mut Loop, key: Key, frames: usize) {
    for _ in 0..frames {
        frame(
            l,
            Input {
                keys_down: vec![key],
                ..Input::default()
            },
        );
    }
}

fn bore(l: &mut Loop) -> &mut BoreInc {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<BoreInc>()
        .unwrap()
}

fn screen(l: &mut Loop) -> AppScreen {
    bore(l).state.as_ref().expect("built in ready").screen
}

/// Title → intro → level, each on a movement key.
fn into_the_level(l: &mut Loop) {
    idle(l, 1);
    press(l, Key::Space);
    assert_eq!(screen(l), AppScreen::Intro);
    press(l, Key::Space);
    assert_eq!(screen(l), AppScreen::Game);
}

#[test]
fn ready_builds_the_app_with_nearest_filtered_textures() {
    let mut l = game();
    assert_eq!(screen(&mut l), AppScreen::Title);
    let calls = l.rendering().calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::SetDefaultFont(Some(_))))
    );
    let textures: Vec<_> = calls
        .iter()
        .filter_map(|c| match c {
            RecordedCall::CreateTexture(id) => Some(*id),
            _ => None,
        })
        .collect();
    // The atlas plus the background layers.
    assert!(textures.len() >= 2, "{textures:?}");
    for id in textures {
        assert!(
            calls.contains(&RecordedCall::SetTextureFilter(id, TextureFilter::Nearest)),
            "{id:?} is not nearest-filtered"
        );
    }
}

#[test]
fn the_viewport_follows_the_window() {
    let mut l = game();
    l.resize(1000.0, 600.0);
    idle(&mut l, 1);
    let ctx = &bore(&mut l).ctx;
    assert_eq!((ctx.viewport_w, ctx.viewport_h), (1000.0, 600.0));
}

#[test]
fn the_level_draws_sprites_from_the_atlas() {
    let mut l = game();
    into_the_level(&mut l);
    idle(&mut l, 2);
    l.draw();
    assert!(
        l.rendering()
            .calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawSprite { .. }))
    );
}

#[test]
fn holding_right_walks_archie_right() {
    let mut l = game();
    into_the_level(&mut l);
    // Let Archie land first.
    idle(&mut l, 60);
    let x0 = bore(&mut l)
        .state
        .as_ref()
        .unwrap()
        .game
        .as_ref()
        .unwrap()
        .archie
        .pos
        .x;
    hold(&mut l, Key::Right, 30);
    let x1 = bore(&mut l)
        .state
        .as_ref()
        .unwrap()
        .game
        .as_ref()
        .unwrap()
        .archie
        .pos
        .x;
    assert!(x1 > x0, "{x0} → {x1}");
}

#[test]
fn escape_leaves_the_level_for_the_title() {
    let mut l = game();
    into_the_level(&mut l);
    press(&mut l, Key::Escape);
    assert_eq!(screen(&mut l), AppScreen::Title);
}

#[test]
fn a_reroll_is_saved_after_the_autosave_delay() {
    let mut l = game_with(count_save);
    into_the_level(&mut l);
    let start = SAVES.load(Ordering::SeqCst);
    // R rerolls the level, which marks progress dirty.
    press(&mut l, Key::R);
    idle(&mut l, 30);
    assert_eq!(
        SAVES.load(Ordering::SeqCst),
        start,
        "saved before the delay"
    );
    idle(&mut l, 120);
    assert_eq!(SAVES.load(Ordering::SeqCst) - start, 1);
}
