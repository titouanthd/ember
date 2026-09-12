//! Minesweeper — Session A window.
//!
//! Renders an inert 9x9 grid with hover feedback, a header with a restart
//! button, a live timer, and a footer showing the current "difficulty".
//! Clicking cells does nothing yet — this session validates the UI.

use minesweeper::components::{Board, CellState};
use minesweeper::config::load_config;

use ember_stdlib::input::Input;
use ember_stdlib::ui::button::{Button, ButtonEvent};
use ember_stdlib::ui::label::Label;
use ember_stdlib::ui::panel::Panel;
use ember_stdlib::ui::timer::TimerDisplay;

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

// Session A placeholder constants. Session C will load these from .ron.
const CELL_SIZE: f32 = 40.0;
const GRID_W: usize = 9;
const GRID_H: usize = 9;
const MINE_COUNT: usize = 10;

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = load_config();
    let mut board = Board::new(GRID_W, GRID_H, MINE_COUNT);
    let mut elapsed: f32 = 0.0;
    let best: f32 = 0.0;

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);

        // Build the frame input from macroquad. Session A only needs mouse
        // and the R key for restart.
        let mut input = Input::from_macroquad();
        if is_key_pressed(KeyCode::R) {
            input.keys_pressed.push(KeyCode::R);
        }

        elapsed += dt;

        // --- Layout ---
        let grid_px_w = board.width as f32 * CELL_SIZE;
        let grid_px_h = board.height as f32 * CELL_SIZE;
        let grid_origin = ctx.grid_origin(grid_px_w, grid_px_h);

        let restart_btn = Button::new(ctx.window_w - 120.0, 10.0, 110.0, 30.0, "Restart");

        // --- Update ---
        if restart_btn.update(&input) == ButtonEvent::Clicked {
            board = Board::new(GRID_W, GRID_H, MINE_COUNT);
            elapsed = 0.0;
            println!("[minesweeper] restart");
        }
        if input.is_key_pressed(KeyCode::R) {
            board = Board::new(GRID_W, GRID_H, MINE_COUNT);
            elapsed = 0.0;
            println!("[minesweeper] restart via R");
        }

        // --- Render ---
        clear_background(ctx.color_bg);

        // Header
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
            format!("Mines: {}", board.mines_remaining()),
            340.0,
            ctx.hud_h * 0.5 + 6.0,
            20,
            ctx.color_text,
        )
        .draw();
        TimerDisplay::new(560.0, ctx.hud_h * 0.5 + 6.0, 20, ctx.color_text).draw(elapsed);
        restart_btn.draw(
            &input,
            ctx.color_btn_idle,
            ctx.color_btn_hover,
            ctx.color_btn_pressed,
            ctx.color_text,
        );

        // Grid
        let mouse = input.mouse_pos;
        for row in 0..board.height {
            for col in 0..board.width {
                let cell = *board.cells.get(col, row).unwrap();
                let x = grid_origin.x + col as f32 * CELL_SIZE;
                let y = grid_origin.y + row as f32 * CELL_SIZE;

                let hovered = mouse.x >= x
                    && mouse.x <= x + CELL_SIZE
                    && mouse.y >= y
                    && mouse.y <= y + CELL_SIZE;

                let bg = match cell.state {
                    CellState::Hidden => {
                        if hovered {
                            ctx.color_cell_hover
                        } else {
                            ctx.color_cell_hidden
                        }
                    }
                    CellState::Revealed => ctx.color_cell_revealed,
                    CellState::Flagged => ctx.color_flag,
                };

                draw_rectangle(x, y, CELL_SIZE, CELL_SIZE, bg);
                draw_rectangle_lines(x, y, CELL_SIZE, CELL_SIZE, 1.0, ctx.color_grid_line);
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
        let footer_text = if best > 0.0 {
            format!("Best: {}", TimerDisplay::format(best))
        } else {
            "Best: --:--".to_owned()
        };
        Label::new(
            footer_text,
            ctx.window_w * 0.5,
            ctx.window_h - ctx.footer_h * 0.5 + 5.0,
            16,
            ctx.color_text,
        )
        .centered()
        .draw();

        next_frame().await;
    }
}