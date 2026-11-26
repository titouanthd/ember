//! Headless run of the node-based Minesweeper on mock backends.

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use ember_stdlib::persistence::Persistence;
use minesweeper::GameState;
use minesweeper::components::{BoardState, CellState};
use minesweeper::config::load_config;
use minesweeper::scene::{MinesweeperGame, build_tree, format_time};
use minesweeper::systems::Game;

type Scene = SceneLoop<MockRenderer, MockAudio>;

fn scene(name: &str) -> Scene {
    let mut game = Game::new();
    // Never touch the project's best_times.ron.
    let path = std::env::temp_dir().join(format!("minesweeper_scene_best_{name}.ron"));
    let _ = std::fs::remove_file(&path);
    game.best_handle = Persistence::new(&path);
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

fn app(s: &mut Scene) -> &mut MinesweeperGame {
    let root = s.tree.root().unwrap();
    s.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<MinesweeperGame>()
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

/// A press frame (with `which` button), a release frame, an idle frame.
fn mouse(s: &mut Scene, pos: Vec2, which: MouseButton) {
    let mut down = Input {
        mouse_pos: pos,
        ..Input::default()
    };
    let mut up = down.clone();
    match which {
        MouseButton::Left => {
            down.mouse_left_pressed = true;
            down.mouse_left_down = true;
            up.mouse_left_released = true;
        }
        MouseButton::Right => {
            down.mouse_right_pressed = true;
            down.mouse_right_down = true;
            up.mouse_right_released = true;
        }
        _ => {
            down.mouse_middle_pressed = true;
            down.mouse_middle_down = true;
            up.mouse_middle_released = true;
        }
    }
    step(s, down);
    step(s, up);
    step(s, Input::default());
}

fn click_button(s: &mut Scene, id: NodeId) {
    let r = button(s, id).rect();
    mouse(s, centre(r), MouseButton::Left);
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
    let b = app(s).buttons.unwrap();
    click_button(s, b.start);
    assert_eq!(app(s).game.state, GameState::Playing);
}

#[test]
fn the_menu_lists_the_difficulties_and_only_start_shows() {
    let mut s = scene("menu");
    let b = app(&mut s).buttons.unwrap();
    let t = texts(&s);
    assert!(t.iter().any(|x| x == "MINESWEEPER"));
    assert!(t.iter().any(|x| x == "Beginner"));
    assert!(s.tree.get(b.start).unwrap().visible());
    assert!(!s.tree.get(b.restart).unwrap().visible());
}

#[test]
fn keys_and_clicks_pick_the_difficulty() {
    let mut s = scene("pick");
    press(&mut s, Key::Num2);
    assert_eq!(app(&mut s).game.selected_difficulty, 1);
    let rects = app(&mut s).difficulty_rects();
    mouse(&mut s, centre(rects[0]), MouseButton::Left);
    assert_eq!(app(&mut s).game.selected_difficulty, 0);
}

#[test]
fn the_first_click_places_mines_and_starts_the_clock() {
    let mut s = scene("first");
    start(&mut s);
    assert!(!app(&mut s).game.board.mines_placed);
    let p = centre(app(&mut s).cell_rect(4, 4));
    mouse(&mut s, p, MouseButton::Left);
    let a = app(&mut s);
    assert!(a.game.board.mines_placed);
    assert!(a.game.timer_running);
    let c = a.game.board.cells.get(4, 4).unwrap();
    assert!(!c.is_mine, "the first click is always safe");
    assert_eq!(c.state, CellState::Revealed);
}

#[test]
fn right_click_toggles_a_flag() {
    let mut s = scene("flag");
    start(&mut s);
    let p = centre(app(&mut s).cell_rect(0, 0));
    mouse(&mut s, p, MouseButton::Right);
    assert_eq!(
        app(&mut s).game.board.cells.get(0, 0).unwrap().state,
        CellState::Flagged
    );
    mouse(&mut s, p, MouseButton::Right);
    assert_eq!(
        app(&mut s).game.board.cells.get(0, 0).unwrap().state,
        CellState::Hidden
    );
}

#[test]
fn losing_shows_the_banner_and_freezes_the_buttons() {
    let mut s = scene("lose");
    start(&mut s);
    app(&mut s).game.board.state = BoardState::Lost;
    step(&mut s, Input::default());
    step(&mut s, Input::default());
    assert_eq!(app(&mut s).game.state, GameState::GameOver);
    assert!(texts(&s).iter().any(|x| x == "GAME OVER"));
    let b = app(&mut s).buttons.unwrap();
    assert!(button(&s, b.restart).is_disabled() && button(&s, b.menu).is_disabled());
    press(&mut s, Key::Enter);
    assert_eq!(app(&mut s).game.state, GameState::Playing, "Enter retries");
    press(&mut s, Key::Escape);
    assert_eq!(
        app(&mut s).game.state,
        GameState::Start,
        "Escape goes to the menu"
    );
}

#[test]
fn restart_and_menu_buttons_work_while_playing() {
    let mut s = scene("buttons");
    start(&mut s);
    let p = centre(app(&mut s).cell_rect(4, 4));
    mouse(&mut s, p, MouseButton::Left);
    let b = app(&mut s).buttons.unwrap();
    click_button(&mut s, b.restart);
    assert!(!app(&mut s).game.board.mines_placed, "a fresh board");
    click_button(&mut s, b.menu);
    assert_eq!(app(&mut s).game.state, GameState::Start);
}

#[test]
fn time_is_shown_like_the_old_timer() {
    assert_eq!(format_time(59.9), "00:59");
    assert_eq!(format_time(3600.0), "01:00:00");
}
