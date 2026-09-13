//! Minesweeper — Session C: difficulties, menu, persistence.

use std::collections::HashMap;

use minesweeper::components::{Board, BoardState, Cell, CellState};
use minesweeper::config::{load_config, GameContext};
use minesweeper::difficulties::{load_difficulties, DifficultyData};
use minesweeper::persistence;
use minesweeper::systems;
use minesweeper::GameState;

use ember_core::rng::Rng;
use ember_stdlib::input::Input;
use ember_stdlib::ui::button::{Button, ButtonEvent};
use ember_stdlib::ui::label::Label;
use ember_stdlib::ui::panel::Panel;
use ember_stdlib::ui::timer::TimerDisplay;

use glam::Vec2;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    let ctx = load_config();
    Conf {
        window_title: "Minesweeper".to_owned(),
        window_width: ctx.window_w as i32,
        window_height: ctx.window_h as i32,
        window_resizable: false,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

struct Game {
    state: GameState,
    difficulties: Vec<DifficultyData>,
    selected_difficulty: usize,

    // Active board (valid when state == Playing / Win / GameOver)
    board: Board,
    cell_size: f32,
    grid_origin: Vec2,
    elapsed: f32,
    timer_running: bool,
    rng: Rng,

    // Persistence
    best_times: HashMap<String, f32>,
    best_times_path: std::path::PathBuf,
    was_new_best: bool,

    // Header buttons — persisted so update() and draw() share the same
    // instance and hover/pressed states stay consistent.
    restart_btn: Button,
    menu_btn: Button,
}

impl Game {
    fn new() -> Self {
        let ctx = load_config();
        let difficulties = load_difficulties();
        let path = persistence::default_path();
        let best_times = persistence::load_best_times(&path);
        let mut g = Self {
            state: GameState::Start,
            difficulties,
            selected_difficulty: 0,
            board: Board::new(9, 9, 10),
            cell_size: 32.0,
            grid_origin: Vec2::ZERO,
            elapsed: 0.0,
            timer_running: false,
            rng: Rng::new(0xDEAD_BEEF),
            best_times,
            best_times_path: path,
            was_new_best: false,
            restart_btn: Button::new(
                ctx.window_w - 220.0,
                10.0,
                100.0,
                30.0,
                "Restart",
            ),
            menu_btn: Button::new(ctx.window_w - 110.0, 10.0, 100.0, 30.0, "Menu"),
        };
        g.update_layout();
        g
    }

    fn current_difficulty(&self) -> &DifficultyData {
        &self.difficulties[self.selected_difficulty]
    }

    fn start_game(&mut self, _ctx: &GameContext) {
        let d = self.current_difficulty().clone();
        self.board = Board::new(d.width, d.height, d.mines);
        self.elapsed = 0.0;
        self.timer_running = false;
        self.was_new_best = false;
        // Advance the RNG state so each game gets a different mine layout.
        self.rng.set_state(self.rng.state().wrapping_add(0x9E37_79B9));
        self.update_layout();
        self.state = GameState::Playing;
    }

    fn update_layout(&mut self) {
        let ctx = load_config();
        let d = self.current_difficulty().clone();
        self.cell_size = d.cell_size(&ctx);
        let w = self.board.width as f32 * self.cell_size;
        let h = self.board.height as f32 * self.cell_size;
        self.grid_origin = ctx.grid_origin(w, h);
    }

    fn cell_at(&self, mouse: Vec2) -> Option<(usize, usize)> {
        let gx = mouse.x - self.grid_origin.x;
        let gy = mouse.y - self.grid_origin.y;
        if gx < 0.0 || gy < 0.0 {
            return None;
        }
        let col = (gx / self.cell_size) as usize;
        let row = (gy / self.cell_size) as usize;
        if col >= self.board.width || row >= self.board.height {
            return None;
        }
        Some((col, row))
    }

    fn is_playable(&self) -> bool {
        self.state == GameState::Playing
            && matches!(
                self.board.state,
                BoardState::Ready | BoardState::Playing
            )
    }

    fn record_best_if_needed(&mut self) {
        let name = self.current_difficulty().name.clone();
        if persistence::update_if_better(&mut self.best_times, &name, self.elapsed) {
            persistence::save_best_times(&self.best_times_path, &self.best_times);
            self.was_new_best = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = load_config();
    let mut game = Game::new();

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);

        let mut input = Input::from_macroquad();
        if is_key_pressed(KeyCode::R) {
            input.keys_pressed.push(KeyCode::R);
        }
        if is_key_pressed(KeyCode::Escape) {
            input.keys_pressed.push(KeyCode::Escape);
        }
        if is_key_pressed(KeyCode::Enter) {
            input.keys_pressed.push(KeyCode::Enter);
        }
        if is_key_pressed(KeyCode::Key1) {
            input.keys_pressed.push(KeyCode::Key1);
        }
        if is_key_pressed(KeyCode::Key2) {
            input.keys_pressed.push(KeyCode::Key2);
        }
        if is_key_pressed(KeyCode::Key3) {
            input.keys_pressed.push(KeyCode::Key3);
        }

        match game.state {
            GameState::Start => handle_menu_input(&mut game, &ctx, &input),
            GameState::Playing => handle_playing_input(&mut game, &ctx, &input, dt),
            GameState::Win | GameState::GameOver => {
                handle_end_input(&mut game, &ctx, &input)
            }
            _ => {}
        }

        // --- Render ---
        clear_background(ctx.color_bg);

        match game.state {
            GameState::Start => render_menu(&ctx, &game, &input),
            GameState::Playing | GameState::Win | GameState::GameOver => {
                render_game(&ctx, &game, &input);
            }
            _ => {}
        }

        next_frame().await;
    }
}

// ---------------------------------------------------------------------------
// Input handlers
// ---------------------------------------------------------------------------

fn handle_menu_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    // 1/2/3 quick-select.
    if input.is_key_pressed(KeyCode::Key1) && !game.difficulties.is_empty() {
        game.selected_difficulty = 0;
    }
    if input.is_key_pressed(KeyCode::Key2) && game.difficulties.len() > 1 {
        game.selected_difficulty = 1;
    }
    if input.is_key_pressed(KeyCode::Key3) && game.difficulties.len() > 2 {
        game.selected_difficulty = 2;
    }

    let (rects, start_btn) = menu_layout(ctx, game);

    // Click on a difficulty entry.
    for (i, rect) in rects.iter().enumerate() {
        if input.mouse_left_pressed && rect_contains(*rect, input.mouse_pos) {
            game.selected_difficulty = i;
        }
    }

    // Click start button.
    if start_btn.update(input) == ButtonEvent::Clicked
        || input.is_key_pressed(KeyCode::Enter)
    {
        game.start_game(ctx);
    }
}

fn handle_playing_input(game: &mut Game, ctx: &GameContext, input: &Input, dt: f32) {
    if game.timer_running && game.board.state == BoardState::Playing {
        game.elapsed += dt;
    }

    // Header buttons — update the SAME instances that render_game draws.
    if game.restart_btn.update(input) == ButtonEvent::Clicked {
        game.start_game(ctx);
        return;
    }
    if game.menu_btn.update(input) == ButtonEvent::Clicked {
        game.state = GameState::Start;
        return;
    }

    // Escape -> back to menu.
    if input.is_key_pressed(KeyCode::Escape) {
        game.state = GameState::Start;
        return;
    }

    // R -> restart current difficulty.
    if input.is_key_pressed(KeyCode::R) {
        game.start_game(ctx);
        return;
    }

    if game.is_playable() {
        let cell_hit = game.cell_at(input.mouse_pos);
        if let Some((col, row)) = cell_hit {
            if input.mouse_left_pressed {
                if !game.board.mines_placed {
                    systems::place_mines(&mut game.board, col, row, &mut game.rng);
                    game.timer_running = true;
                    systems::reveal(&mut game.board, col, row);
                } else {
                    let cell = *game.board.cells.get(col, row).unwrap();
                    if cell.is_revealed() {
                        systems::chord(&mut game.board, col, row);
                    } else {
                        systems::reveal(&mut game.board, col, row);
                    }
                }
            }
            if input.mouse_middle_pressed {
                systems::chord(&mut game.board, col, row);
            }
            if input.mouse_right_pressed {
                systems::toggle_flag(&mut game.board, col, row);
            }
        }
    }

    // Transition to end states.
    match game.board.state {
        BoardState::Won => {
            game.timer_running = false;
            game.record_best_if_needed();
            game.state = GameState::Win;
        }
        BoardState::Lost => {
            game.timer_running = false;
            game.state = GameState::GameOver;
        }
        _ => {}
    }
}

fn handle_end_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    if input.is_key_pressed(KeyCode::Escape) || input.is_key_pressed(KeyCode::R) {
        game.state = GameState::Start;
        return;
    }
    if input.is_key_pressed(KeyCode::Enter) {
        game.start_game(ctx);
    }
}

// ---------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------

type Rect = (f32, f32, f32, f32);

fn rect_contains(r: Rect, p: Vec2) -> bool {
    p.x >= r.0 && p.x <= r.0 + r.2 && p.y >= r.1 && p.y <= r.1 + r.3
}

fn menu_layout(ctx: &GameContext, game: &Game) -> (Vec<Rect>, Button) {
    let cx = ctx.window_w * 0.5;
    let mut rects = Vec::new();

    let btn_w = 360.0;
    let btn_h = 60.0;
    let gap = 16.0;
    let n = game.difficulties.len() as f32;
    let total_h = n * btn_h + (n - 1.0).max(0.0) * gap;
    let first_y = ctx.window_h * 0.5 - total_h * 0.5 + 40.0;

    for i in 0..game.difficulties.len() {
        let y = first_y + i as f32 * (btn_h + gap);
        rects.push((cx - btn_w * 0.5, y, btn_w, btn_h));
    }

    let start_y = first_y + total_h + 30.0;
    let start_btn = Button::new(cx - 100.0, start_y, 200.0, 50.0, "START");
    (rects, start_btn)
}

fn render_menu(ctx: &GameContext, game: &Game, input: &Input) {
    Label::new(
        "MINESWEEPER",
        ctx.window_w * 0.5,
        100.0,
        56,
        ctx.color_text,
    )
    .centered()
    .draw();

    let (rects, start_btn) = menu_layout(ctx, game);

    for (i, rect) in rects.iter().enumerate() {
        let d = &game.difficulties[i];
        let selected = i == game.selected_difficulty;
        let hovered = rect_contains(*rect, input.mouse_pos);

        let bg = if selected {
            ctx.color_menu_selected
        } else if hovered {
            ctx.color_menu_hover
        } else {
            ctx.color_btn_idle
        };
        draw_rectangle(rect.0, rect.1, rect.2, rect.3, bg);
        draw_rectangle_lines(rect.0, rect.1, rect.2, rect.3, 1.5, ctx.color_grid_line);

        let line1 = &d.name;
        let line2 = format!("{}×{} · {} mines", d.width, d.height, d.mines);
        let best = game.best_times.get(&d.name);
        let line3 = match best {
            Some(&t) => format!("Best: {}", TimerDisplay::format(t)),
            None => "Best: --:--".to_owned(),
        };

        draw_text(line1, rect.0 + 20.0, rect.1 + 22.0, 22.0, ctx.color_text);
        draw_text(line2, rect.0 + 20.0, rect.1 + 42.0, 16.0, ctx.color_text);
        let dims = measure_text(&line3, None, 16, 1.0);
        draw_text(
            &line3,
            rect.0 + rect.2 - 20.0 - dims.width,
            rect.1 + 42.0,
            16.0,
            ctx.color_text,
        );
    }

    start_btn.draw(
        input,
        ctx.color_btn_idle,
        ctx.color_btn_hover,
        ctx.color_btn_pressed,
        ctx.color_text,
    );
}

// ---------------------------------------------------------------------------
// Game rendering
// ---------------------------------------------------------------------------

fn render_game(ctx: &GameContext, game: &Game, input: &Input) {
    // Header
    Panel::new(0.0, 0.0, ctx.window_w, ctx.hud_h, ctx.color_header_bg).draw();
    let d = game.current_difficulty();
    Label::new(
        format!("{} · {}×{} · {} mines", d.name, d.width, d.height, d.mines),
        12.0,
        ctx.hud_h * 0.5 + 6.0,
        20,
        ctx.color_text,
    )
    .draw();
    Label::new(
        format!("Mines: {}", game.board.mines_remaining()),
        380.0,
        ctx.hud_h * 0.5 + 6.0,
        20,
        ctx.color_text,
    )
    .draw();
    TimerDisplay::new(600.0, ctx.hud_h * 0.5 + 6.0, 20, ctx.color_text).draw(game.elapsed);

    // Header buttons — same instances as in handle_playing_input.
    game.restart_btn.draw(
        input,
        ctx.color_btn_idle,
        ctx.color_btn_hover,
        ctx.color_btn_pressed,
        ctx.color_text,
    );
    game.menu_btn.draw(
        input,
        ctx.color_btn_idle,
        ctx.color_btn_hover,
        ctx.color_btn_pressed,
        ctx.color_text,
    );

    // Grid
    let cell_hit = game.cell_at(input.mouse_pos);
    let cell_size = game.cell_size;
    for row in 0..game.board.height {
        for col in 0..game.board.width {
            let cell = *game.board.cells.get(col, row).unwrap();
            let x = game.grid_origin.x + col as f32 * cell_size;
            let y = game.grid_origin.y + row as f32 * cell_size;

            let hovered = matches!(cell_hit, Some((c, r)) if c == col && r == row);

            let bg = cell_background(ctx, &cell, hovered && game.is_playable(), game.board.state);
            draw_rectangle(x, y, cell_size, cell_size, bg);
            draw_rectangle_lines(x, y, cell_size, cell_size, 1.0, ctx.color_grid_line);

            if cell.is_revealed() && !cell.is_mine && cell.adjacent > 0 {
                let txt = cell.adjacent.to_string();
                let color = number_color(cell.adjacent);
                let font_size = (cell_size * 0.6).clamp(12.0, 26.0);
                let dims = measure_text(&txt, None, font_size as u16, 1.0);
                let tx = x + (cell_size - dims.width) * 0.5;
                let ty = y + (cell_size + font_size) * 0.5 - 4.0;
                draw_text(&txt, tx, ty, font_size, color);
            }

            if cell.is_mine && game.board.state == BoardState::Lost && !cell.is_flagged() {
                draw_circle(
                    x + cell_size * 0.5,
                    y + cell_size * 0.5,
                    cell_size * 0.2,
                    BLACK,
                );
            }
        }
    }

    // Footer
    Panel::new(
        0.0,
        ctx.window_h - ctx.footer_h,
        ctx.window_w,
        ctx.footer_h,
        ctx.color_header_bg,
    )
    .draw();
    Label::new(
        "R restart · Esc menu · left-click reveal · right-click flag · middle-click chord",
        ctx.window_w * 0.5,
        ctx.window_h - ctx.footer_h * 0.5 + 5.0,
        14,
        ctx.color_text,
    )
    .centered()
    .draw();

    // End banners
    match game.state {
        GameState::Win => {
            let line1 = format!("YOU WIN — {}", TimerDisplay::format(game.elapsed));
            let line2 = if game.was_new_best {
                "NEW BEST!".to_string()
            } else {
                match game.best_times.get(&game.current_difficulty().name) {
                    Some(&t) => format!("Best: {}", TimerDisplay::format(t)),
                    None => String::new(),
                }
            };
            draw_end_banner(ctx, &line1, &line2, Color::new(0.4, 1.0, 0.5, 1.0));
        }
        GameState::GameOver => {
            let line1 = "GAME OVER".to_string();
            let line2 = "Press R for menu, Enter to retry".to_string();
            draw_end_banner(ctx, &line1, &line2, Color::new(1.0, 0.35, 0.4, 1.0));
        }
        _ => {}
    }
}

fn draw_end_banner(ctx: &GameContext, line1: &str, line2: &str, color: Color) {
    let d1 = measure_text(line1, None, 48, 1.0);
    let d2 = measure_text(line2, None, 20, 1.0);
    let cx = ctx.window_w * 0.5;
    let cy = ctx.hud_h + ctx.playfield_h() * 0.5;

    let pad_x = 30.0;
    let pad_y = 25.0;
    let box_w = d1.width.max(d2.width) + pad_x * 2.0;
    let box_h = d1.height + d2.height + pad_y * 2.0 + 10.0;
    let box_x = cx - box_w * 0.5;
    let box_y = cy - box_h * 0.5;

    draw_rectangle(box_x, box_y, box_w, box_h, Color::new(0.0, 0.0, 0.0, 0.82));

    draw_text(
        line1,
        cx - d1.width * 0.5,
        box_y + pad_y + d1.height,
        48.0,
        color,
    );
    draw_text(
        line2,
        cx - d2.width * 0.5,
        box_y + pad_y + d1.height + 10.0 + d2.height,
        20.0,
        ctx.color_text,
    );
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn cell_background(
    ctx: &GameContext,
    cell: &Cell,
    hovered: bool,
    board_state: BoardState,
) -> Color {
    match cell.state {
        CellState::Hidden => {
            if hovered {
                ctx.color_cell_hover
            } else {
                ctx.color_cell_hidden
            }
        }
        CellState::Revealed => {
            if cell.is_mine && board_state == BoardState::Lost {
                ctx.color_cell_mine
            } else {
                ctx.color_cell_revealed
            }
        }
        CellState::Flagged => ctx.color_flag,
    }
}

fn number_color(n: u8) -> Color {
    match n {
        1 => Color::new(0.35, 0.65, 1.00, 1.0),
        2 => Color::new(0.35, 0.85, 0.45, 1.0),
        3 => Color::new(1.00, 0.40, 0.40, 1.0),
        4 => Color::new(0.60, 0.40, 1.00, 1.0),
        5 => Color::new(1.00, 0.60, 0.20, 1.0),
        6 => Color::new(0.30, 0.85, 0.85, 1.0),
        7 => Color::new(0.95, 0.95, 0.95, 1.0),
        8 => Color::new(0.60, 0.60, 0.60, 1.0),
        _ => Color::new(0.5, 0.5, 0.5, 1.0),
    }
}