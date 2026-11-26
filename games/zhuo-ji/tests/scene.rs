//! Headless run of the node-based Zhuo Ji on mock backends.

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use zhuo_ji::help::{HelpState, HelpTab};
use zhuo_ji::layout::TableLayout;
use zhuo_ji::menu::{self, MenuGeom};
use zhuo_ji::scene::{AppScreen, WINDOW_H, WINDOW_W, ZhuoJi, build_tree};
use zhuo_ji::{GameContext, Phase};

type Loop = SceneLoop<MockRenderer, MockAudio>;

const SEED: u32 = 0x5EED_0001;

/// Rules already seen, so the overlay is closed and closing it never
/// writes the real prefs file.
fn seen_help() -> HelpState {
    HelpState {
        visible: false,
        has_seen: true,
        tab: HelpTab::Goal,
        suit_filter: None,
        rank_filter: None,
    }
}

fn game() -> Loop {
    let mut l = SceneLoop::new(
        build_tree(GameContext::default_hermetic(), seen_help(), Some(SEED)),
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

fn zhuo_ji(l: &mut Loop) -> &mut ZhuoJi {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<ZhuoJi>()
        .unwrap()
}

/// Centre of menu button `i` on the current layout.
fn menu_button(l: &mut Loop, i: usize) -> Vec2 {
    let layout = zhuo_ji(l).layout();
    let (x, y, w, h) = menu::button_rect(&MenuGeom::new(&layout), i);
    Vec2::new(x + w / 2.0, y + h / 2.0)
}

#[test]
fn ready_installs_the_tile_font() {
    let mut l = game();
    assert!(zhuo_ji(&mut l).ctx.font.is_available());
    let created = l
        .rendering()
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedCall::CreateFont(_)))
        .count();
    assert_eq!(created, 1);
}

#[test]
fn the_menu_draws_the_table_background_and_the_title() {
    let mut l = game();
    l.draw();
    let calls = l.rendering().calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::SetClearColor(_)))
    );
    // The Chinese title is drawn with the tile font.
    assert!(calls.iter().any(
        |c| matches!(c, RecordedCall::DrawTextWithFont { text, .. } if text == "贵阳捉鸡麻将")
    ));
}

#[test]
fn the_layout_follows_the_window() {
    let mut l = game();
    l.resize(1000.0, 700.0);
    frame(&mut l, Input::default());
    let layout = zhuo_ji(&mut l).layout();
    assert_eq!((layout.scr_w, layout.scr_h), (1000.0, 700.0));
    assert_eq!(layout.cx(), TableLayout::new(1000.0, 700.0).cx());
}

#[test]
fn clicking_new_game_starts_a_match_and_escape_returns_to_the_menu() {
    let mut l = game();
    let new_game = menu_button(&mut l, 0);
    assert!(click_at(&mut l, new_game));
    assert_eq!(zhuo_ji(&mut l).screen, AppScreen::Game);

    assert!(press(&mut l, Key::Escape));
    assert_eq!(zhuo_ji(&mut l).screen, AppScreen::Menu);
    // Escape on the menu quits.
    assert!(!press(&mut l, Key::Escape));
}

#[test]
fn the_quit_button_and_h_for_the_rules() {
    let mut l = game();
    assert!(press(&mut l, Key::H));
    assert!(zhuo_ji(&mut l).help_state.visible);
    assert!(press(&mut l, Key::Right));
    assert_eq!(zhuo_ji(&mut l).help_state.tab, HelpTab::Goal.next());
    assert!(press(&mut l, Key::H));
    assert!(!zhuo_ji(&mut l).help_state.visible);

    let quit = menu_button(&mut l, 2);
    assert!(!click_at(&mut l, quit));
}

#[test]
fn a_hand_plays_through_and_the_human_discards_by_click() {
    let mut l = game();
    assert!(press(&mut l, Key::Enter));
    assert_eq!(zhuo_ji(&mut l).screen, AppScreen::Game);

    // Deal until the human is to discard, drawing every frame.
    let mut frames = 0;
    while !matches!(
        zhuo_ji(&mut l).game.phase,
        Phase::AwaitingDiscard { player: 0 }
    ) {
        assert!(frame(&mut l, Input::default()));
        frames += 1;
        assert!(frames < 5_000, "never reached the human's discard");
    }

    // Click the first tile of the hand.
    let z = zhuo_ji(&mut l);
    let layout = z.layout();
    let n = z.game.players[0].concealed.len();
    let size = layout.sizes.hand;
    let pos = layout.hand_tile_pos(0, n) + Vec2::new(size.w / 2.0, size.h / 2.0);
    let discards = z.game.players[0].discards.len();
    assert!(click_at(&mut l, pos));
    assert_eq!(zhuo_ji(&mut l).game.players[0].discards.len(), discards + 1);

    // The table keeps drawing as the AIs play on.
    for _ in 0..600 {
        assert!(frame(&mut l, Input::default()));
    }
    let calls = l.rendering().calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawRect { .. }))
    );
}
