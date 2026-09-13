//! Minesweeper — Session B: playable.

use minesweeper::components::{Board, BoardState, CellState};
use minesweeper::config::{load_config, GameContext};
use minesweeper::systems;

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

const CELL_SIZE: f32 = 40.0;
const GRID_W: usize = 9;
const GRID_H: usize = 9;
const MINE_COUNT: usize = 10;

/// All mutable game state for one session.
struct Game {
    board: Board,
    elapsed: f32,
    timer_running: bool,
    rng: u32,
}

impl Game {
    fn new() -> Self {
        Self {
            board: Board::new(GRID_W, GRID_H, MINE_COUNT),
            elapsed: 0.0,
            timer_running: false,
            rng: 0xDEAD_BEEF,
        }
    }

    fn restart(&mut self) {
        // Vary the seed so the next game isn't identical.
        let next_seed = self.rng.wrapping_add(0x9E37_79B9);
        *self = Self::new();
        self.rng = next_seed;
    }

    fn grid_origin(&self, ctx: &GameContext) -> Vec2 {
        let w = self.board.width as f32 * CELL_SIZE;
        let h = self.board.height as f32 * CELL_SIZE;
        ctx.grid_origin(w, h)
    }

    /// Convert a mouse position to a cell coord, or None if outside the grid.
    fn cell_at(&self, ctx: &GameContext, mouse: Vec2) -> Option<(usize, usize)> {
        let origin = self.grid_origin(ctx);
        let gx = mouse.x - origin.x;
        let gy = mouse.y - origin.y;
        if gx < 0.0 || gy < 0.0 {
            return None;
        }
        let col = (gx / CELL_SIZE) as usize;
        let row = (gy / CELL_SIZE) as usize;
        if col >= self.board.width || row >= self.board.height {
            return None;
        }
        Some((col, row))
    }

    fn is_playable(&self) -> bool {
        matches!(
            self.board.state,
            BoardState::Ready | BoardState::Playing
        )
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = load_config();
    let mut game = Game::new();

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);

        // --- Input snapshot ---
        let mut input = Input::from_macroquad();
        if is_key_pressed(KeyCode::R) {
            input.keys_pressed.push(KeyCode::R);
        }

        // Timer advances while the board is being played.
        if game.timer_running && game.board.state == BoardState::Playing {
            game.elapsed += dt;
        }

        // --- Restart button ---
        let restart_btn = Button::new(ctx.window_w - 120.0, 10.0, 110.0, 30.0, "Restart");
        if restart_btn.update(&input) == ButtonEvent::Clicked
            || input.is_key_pressed(KeyCode::R)
        {
            game.restart();
        }

        // --- Grid input ---
        let cell_hit = game.cell_at(&ctx, input.mouse_pos);
        if game.is_playable()
            && let Some((col, row)) = cell_hit
        {
            // Left click: reveal a hidden cell, or chord a revealed one.
            if input.mouse_left_pressed {
                if !game.board.mines_placed {
                    let mut rng = game.rng;
                    systems::place_mines(&mut game.board, col, row, &mut rng);
                    game.rng = rng;
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

        // Stop the timer once the game ends.
        if matches!(game.board.state, BoardState::Won | BoardState::Lost) {
            game.timer_running = false;
        }

        // --- Render ---
        clear_background(ctx.color_bg);

        render_header(&ctx, &game, &input, &restart_btn);
        render_grid(&ctx, &game, cell_hit);
        render_overlay(&ctx, &game);
        render_footer(&ctx);

        next_frame().await;
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

fn render_header(ctx: &GameContext, game: &Game, input: &Input, restart_btn: &Button) {
    Panel::new(0.0, 0.0, ctx.window_w, ctx.hud_h, ctx.color_header_bg).draw();

    Label::new(
        "Beginner · 9×9 · 10 mines",
        12.0,
        ctx.hud_h * 0.5 + 6.0,
        20,
        ctx.color_text,
    )
    .draw();

    Label::new(
        format!("Mines: {}", game.board.mines_remaining()),
        340.0,
        ctx.hud_h * 0.5 + 6.0,
        20,
        ctx.color_text,
    )
    .draw();

    TimerDisplay::new(560.0, ctx.hud_h * 0.5 + 6.0, 20, ctx.color_text).draw(game.elapsed);

    restart_btn.draw(
        input,
        ctx.color_btn_idle,
        ctx.color_btn_hover,
        ctx.color_btn_pressed,
        ctx.color_text,
    );
}

fn render_grid(ctx: &GameContext, game: &Game, cell_hit: Option<(usize, usize)>) {
    let origin = game.grid_origin(ctx);
    let playable = game.is_playable();

    for row in 0..game.board.height {
        for col in 0..game.board.width {
            let cell = *game.board.cells.get(col, row).unwrap();
            let x = origin.x + col as f32 * CELL_SIZE;
            let y = origin.y + row as f32 * CELL_SIZE;

            let hovered = matches!(cell_hit, Some((c, r)) if c == col && r == row);

            let bg = cell_background(ctx, &cell, hovered && playable, game.board.state);
            draw_rectangle(x, y, CELL_SIZE, CELL_SIZE, bg);
            draw_rectangle_lines(x, y, CELL_SIZE, CELL_SIZE, 1.0, ctx.color_grid_line);

            // Revealed non-mine cells with adjacent > 0 show a colored number.
            if cell.is_revealed() && !cell.is_mine && cell.adjacent > 0 {
                let txt = cell.adjacent.to_string();
                let color = number_color(cell.adjacent);
                let dims = measure_text(&txt, None, 24, 1.0);
                let tx = x + (CELL_SIZE - dims.width) * 0.5;
                let ty = y + (CELL_SIZE + 24.0) * 0.5 - 4.0;
                draw_text(&txt, tx, ty, 24.0, color);
            }

            // Reveal mines visually on loss.
            if cell.is_mine && game.board.state == BoardState::Lost && !cell.is_flagged() {
                draw_circle(
                    x + CELL_SIZE * 0.5,
                    y + CELL_SIZE * 0.5,
                    8.0,
                    BLACK,
                );
            }
        }
    }
}

fn cell_background(
    ctx: &GameContext,
    cell: &minesweeper::components::Cell,
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

fn render_overlay(ctx: &GameContext, game: &Game) {
    match game.board.state {
        BoardState::Won => {
            let msg = format!("YOU WIN — {}", TimerDisplay::format(game.elapsed));
            draw_banner(ctx, &msg, Color::new(0.4, 1.0, 0.5, 1.0));
        }
        BoardState::Lost => {
            draw_banner(ctx, "GAME OVER", Color::new(1.0, 0.35, 0.4, 1.0));
        }
        _ => {}
    }
}

fn draw_banner(ctx: &GameContext, msg: &str, color: Color) {
    let dims = measure_text(msg, None, 48, 1.0);
    let cx = ctx.window_w * 0.5;
    let cy = ctx.hud_h + ctx.playfield_h() * 0.5;

    let pad_x = 20.0;
    let pad_y = 20.0;
    draw_rectangle(
        cx - dims.width * 0.5 - pad_x,
        cy - dims.height - pad_y,
        dims.width + pad_x * 2.0,
        dims.height + pad_y * 2.0,
        Color::new(0.0, 0.0, 0.0, 0.75),
    );
    draw_text(msg, cx - dims.width * 0.5, cy, 48.0, color);
}

fn render_footer(ctx: &GameContext) {
    Panel::new(
        0.0,
        ctx.window_h - ctx.footer_h,
        ctx.window_w,
        ctx.footer_h,
        ctx.color_header_bg,
    )
    .draw();

    Label::new(
        "R to restart · left-click reveal · right-click flag · middle-click chord",
        ctx.window_w * 0.5,
        ctx.window_h - ctx.footer_h * 0.5 + 5.0,
        14,
        ctx.color_text,
    )
    .centered()
    .draw();
}

/// Classic Minesweeper number colors.
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