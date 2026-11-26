//! Headless run of the node-based Memory on mock backends.

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use ember_stdlib::persistence::Persistence;
use memory::GameState;
use memory::components::CardState;
use memory::config::load_config;
use memory::scene::{MemoryGame, build_tree, format_time};
use memory::systems::Game;
use std::collections::HashMap;

type Scene = SceneLoop<MockRenderer, MockAudio>;

fn scene(name: &str) -> Scene {
    let path = std::env::temp_dir().join(format!("memory_scene_best_{name}.ron"));
    let _ = std::fs::remove_file(&path);
    let handle: Persistence<HashMap<String, f32>> = Persistence::new(&path);
    let ctx = load_config();
    let (w, h) = (ctx.window_w, ctx.window_h);
    let mut s = SceneLoop::new(
        build_tree(ctx, Game::with_best_handle(handle)),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    assert!(s.init());
    s.draw();
    step(&mut s, Input::default());
    s
}

fn step(s: &mut Scene, input: Input) -> bool {
    s.set_input(input);
    let running = s.process(1.0 / 60.0);
    s.draw();
    running
}

fn app(s: &mut Scene) -> &mut MemoryGame {
    let root = s.tree.root().unwrap();
    s.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<MemoryGame>()
        .unwrap()
}

fn visible(s: &Scene, id: NodeId) -> bool {
    s.tree.get(id).unwrap().visible()
}

/// Press then release the left button at `pos`, then one idle frame.
fn click(s: &mut Scene, pos: Vec2) {
    let down = Input {
        mouse_pos: pos,
        mouse_left_pressed: true,
        mouse_left_down: true,
        ..Input::default()
    };
    let up = Input {
        mouse_pos: pos,
        mouse_left_released: true,
        ..Input::default()
    };
    step(s, down);
    step(s, up);
    step(s, Input::default());
}

fn centre(r: Rect) -> Vec2 {
    Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5)
}

fn click_button(s: &mut Scene, id: NodeId) {
    let r = s
        .tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<Button>()
        .unwrap()
        .rect();
    click(s, centre(r));
}

fn press(s: &mut Scene, key: Key) -> bool {
    step(
        s,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    )
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

fn start(s: &mut Scene) {
    let b = app(s).buttons.unwrap();
    click_button(s, b.start);
    assert_eq!(app(s).game.state, GameState::Playing);
}

#[test]
fn the_menu_shows_the_title_and_only_the_start_button() {
    let mut s = scene("menu");
    let b = app(&mut s).buttons.unwrap();
    assert!(texts(&s).iter().any(|t| t == "MEMORY"));
    assert!(visible(&s, b.start));
    assert!(!visible(&s, b.restart) && !visible(&s, b.menu));
}

#[test]
fn clicking_a_difficulty_selects_it() {
    let mut s = scene("select");
    let rects = app(&mut s).difficulty_rects();
    assert!(rects.len() > 1);
    click(&mut s, centre(rects[1]));
    assert_eq!(app(&mut s).game.selected_difficulty, 1);
    press(&mut s, Key::Num1);
    assert_eq!(app(&mut s).game.selected_difficulty, 0);
}

#[test]
fn start_button_begins_a_game_and_swaps_the_buttons() {
    let mut s = scene("start");
    start(&mut s);
    let b = app(&mut s).buttons.unwrap();
    assert!(!visible(&s, b.start));
    assert!(visible(&s, b.restart) && visible(&s, b.menu));
    assert!(texts(&s).iter().any(|t| t.starts_with("pairs: 0/")));
}

#[test]
fn space_after_clicking_start_does_not_restart() {
    let mut s = scene("space");
    start(&mut s);
    let a = app(&mut s);
    let (c, r) = (0, 0);
    let rect = a.game.card_rect(c, r, &a.ctx);
    click(&mut s, centre(rect));
    assert_eq!(
        app(&mut s).game.board.get(c, r).unwrap().state,
        CardState::Flipped
    );
    press(&mut s, Key::Space);
    step(&mut s, Input::default());
    assert_eq!(
        app(&mut s).game.board.get(c, r).unwrap().state,
        CardState::Flipped,
        "the START button lost focus, Space does nothing in play"
    );
}

#[test]
fn clicking_two_matching_cards_scores_a_pair() {
    let mut s = scene("pair");
    start(&mut s);
    let a = app(&mut s);
    let board = &a.game.board;
    let mut pair = None;
    'outer: for (c1, r1, x) in board.iter() {
        for (c2, r2, y) in board.iter() {
            if (c1, r1) != (c2, r2) && x.symbol == y.symbol {
                pair = Some(((c1, r1), (c2, r2)));
                break 'outer;
            }
        }
    }
    let ((c1, r1), (c2, r2)) = pair.unwrap();
    let (p1, p2) = (
        centre(a.game.card_rect(c1, r1, &a.ctx)),
        centre(a.game.card_rect(c2, r2, &a.ctx)),
    );
    click(&mut s, p1);
    click(&mut s, p2);
    assert_eq!(app(&mut s).game.pairs_found, 1);
}

#[test]
fn menu_and_restart_buttons_work_in_play() {
    let mut s = scene("buttons");
    start(&mut s);
    let b = app(&mut s).buttons.unwrap();
    click_button(&mut s, b.menu);
    assert_eq!(app(&mut s).game.state, GameState::Start);
    start(&mut s);
    click_button(&mut s, b.restart);
    let a = app(&mut s);
    assert_eq!((a.game.state, a.game.pairs_found), (GameState::Playing, 0));
}

#[test]
fn escape_quits() {
    let mut s = scene("quit");
    assert!(!press(&mut s, Key::Escape));
}

#[test]
fn time_is_shown_like_the_old_timer() {
    assert_eq!(format_time(0.0), "00:00");
    assert_eq!(format_time(75.9), "01:15");
    assert_eq!(format_time(3725.0), "01:02:05");
}
