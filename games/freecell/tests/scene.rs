//! Headless run of the node-based FreeCell on mock backends.

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use ember_stdlib::persistence::Persistence;
use freecell::GameState;
use freecell::config::load_config;
use freecell::layout;
use freecell::scene::{FreeCellGame, build_tree, button_x, format_time};
use freecell::systems::Game;

type Scene = SceneLoop<MockRenderer, MockAudio>;

fn scene(name: &str) -> Scene {
    let mut game = Game::new();
    // Never touch the project's best_times.ron.
    let path = std::env::temp_dir().join(format!("freecell_scene_best_{name}.ron"));
    let _ = std::fs::remove_file(&path);
    game.best_handle = Persistence::new(&path);
    game.best_times.clear();
    let ctx = load_config();
    let (w, h) = (ctx.window_w, ctx.window_h);
    let mut s = SceneLoop::new(
        build_tree(ctx, game),
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

fn app(s: &mut Scene) -> &mut FreeCellGame {
    let root = s.tree.root().unwrap();
    s.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<FreeCellGame>()
        .unwrap()
}

fn button(s: &Scene, id: NodeId) -> &Button {
    s.tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<Button>()
        .unwrap()
}

fn centre(r: Rect) -> Vec2 {
    Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5)
}

fn held(pos: Vec2) -> Input {
    Input {
        mouse_pos: pos,
        mouse_left_down: true,
        ..Input::default()
    }
}

/// Press at `from`, hold while moving to `to`, release there, idle.
fn drag(s: &mut Scene, from: Vec2, to: Vec2) {
    step(
        s,
        Input {
            mouse_left_pressed: true,
            ..held(from)
        },
    );
    step(s, held((from + to) * 0.5));
    step(s, held(to));
    step(
        s,
        Input {
            mouse_pos: to,
            mouse_left_released: true,
            ..Input::default()
        },
    );
    step(s, Input::default());
}

fn click_button(s: &mut Scene, id: NodeId) {
    let p = centre(button(s, id).rect());
    drag(s, p, p);
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
    press(s, Key::Enter);
    assert_eq!(app(s).game.state, GameState::Playing);
}

#[test]
fn the_menu_loads_the_font_and_hides_the_buttons() {
    let mut s = scene("menu");
    assert!(
        app(&mut s).font_loaded,
        "DejaVu Sans Mono is the default font"
    );
    assert!(
        s.rendering()
            .calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::SetDefaultFont(Some(_))))
    );
    let t = texts(&s);
    assert!(t.iter().any(|x| x == "FREECELL"));
    assert!(t.iter().any(|x| x == "Seed: 1"));
    let b = app(&mut s).buttons.unwrap();
    for id in [b.hint, b.restart, b.new_game, b.menu] {
        assert!(!s.tree.get(id).unwrap().visible());
    }
}

#[test]
fn enter_deals_the_seed_and_shows_the_board() {
    let mut s = scene("deal");
    start(&mut s);
    let a = app(&mut s);
    assert_eq!(a.game.seed, 1);
    let dealt: usize = a.game.columns.iter().map(Vec::len).sum();
    assert_eq!(dealt, 52);
    let b = a.buttons.unwrap();
    assert!(s.tree.get(b.hint).unwrap().visible());
    assert!(!button(&s, b.hint).is_disabled());
    let ctx = load_config();
    assert_eq!(button(&s, b.hint).rect().x, button_x(&ctx, 0));
    assert!(texts(&s).iter().any(|x| x == "Moves: 0"));
}

#[test]
fn dragging_a_card_to_a_free_cell_moves_it_and_z_undoes() {
    let mut s = scene("drag");
    start(&mut s);
    let ctx = load_config();
    let last = app(&mut s).game.columns[0].len() - 1;
    let card = app(&mut s).game.columns[0][last];
    let from = centre(layout::card_rect_in_column(0, last, &ctx));
    let to = centre(layout::free_cell_rect(0, &ctx));
    drag(&mut s, from, to);
    let a = app(&mut s);
    assert_eq!(a.game.free_cells[0], Some(card));
    assert_eq!(a.game.moves, 1);
    assert!(a.game.drag.is_idle());
    press(&mut s, Key::Z);
    let a = app(&mut s);
    assert_eq!(a.game.free_cells[0], None);
    assert_eq!(a.game.columns[0][last], card);
}

#[test]
fn the_buttons_hint_restart_and_go_to_the_menu() {
    let mut s = scene("buttons");
    start(&mut s);
    let b = app(&mut s).buttons.unwrap();
    click_button(&mut s, b.hint);
    assert!(app(&mut s).game.hint.is_some(), "a dealt board has a move");
    assert!(texts(&s).iter().any(|x| x.starts_with("Hint: ")));
    click_button(&mut s, b.restart);
    let a = app(&mut s);
    assert_eq!((a.game.seed, a.game.moves), (1, 0));
    assert!(a.game.hint.is_none());
    click_button(&mut s, b.menu);
    assert_eq!(app(&mut s).game.state, GameState::Start);
}

#[test]
fn the_win_banner_freezes_the_buttons_and_m_returns_to_the_menu() {
    let mut s = scene("win");
    start(&mut s);
    app(&mut s).game.state = GameState::Win;
    step(&mut s, Input::default());
    let t = texts(&s);
    assert!(t.iter().any(|x| x.starts_with("SOLVED in ")));
    let b = app(&mut s).buttons.unwrap();
    assert!(s.tree.get(b.menu).unwrap().visible());
    assert!(button(&s, b.menu).is_disabled());
    press(&mut s, Key::Semicolon);
    assert_eq!(app(&mut s).game.state, GameState::Start);
}

#[test]
fn escape_quits() {
    let mut s = scene("quit");
    press(&mut s, Key::Escape);
    assert!(s.tree.quit_requested());
}

#[test]
fn time_is_shown_like_the_old_timer() {
    assert_eq!(format_time(59.9), "00:59");
    assert_eq!(format_time(3600.0), "01:00:00");
}
