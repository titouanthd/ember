//! Memory — terminal-themed pair matching game.

use memory::GameState;
use memory::components::{Card, CardState};
use memory::config::{GameContext, load_config};
use memory::systems::Game;

use ember_stdlib::input::Input;
use ember_stdlib::ui::button::ButtonEvent;
use ember_stdlib::ui::label::Label;
use ember_stdlib::ui::panel::Panel;
use ember_stdlib::ui::timer::TimerDisplay;

use glam::Vec2;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    let ctx = load_config();
    Conf {
        window_title: "Memory".to_owned(),
        window_width: ctx.window_w as i32,
        window_height: ctx.window_h as i32,
        window_resizable: false,
        ..Default::default()
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

        let input = Input::from_macroquad_with_keys(&[
            KeyCode::R,
            KeyCode::M,
            KeyCode::Escape,
            KeyCode::Enter,
            KeyCode::Space,
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
        ]);

        if input.is_key_pressed(KeyCode::Escape) {
            break;
        }

        // Tick game time + resolve no-match (only when playing).
        if game.state == GameState::Playing {
            game.tick(dt);
        }

        match game.state {
            GameState::Start => handle_menu_input(&mut game, &ctx, &input),
            GameState::Playing => handle_playing_input(&mut game, &ctx, &input),
            GameState::Win => handle_win_input(&mut game, &ctx, &input),
            _ => {}
        }

        // --- Render ---
        clear_background(ctx.color_bg);

        match game.state {
            GameState::Start => render_menu(&ctx, &game, &input),
            GameState::Playing | GameState::Win => {
                render_game(&ctx, &game, &input);
                if game.state == GameState::Win {
                    render_win_banner(&ctx, &game);
                }
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

    let rects = difficulty_rects(ctx, game);

    // Click on a difficulty entry.
    for (i, rect) in rects.iter().enumerate() {
        if input.mouse_left_pressed && btn_rect_contains(*rect, input.mouse_pos) {
            game.selected_difficulty = i;
        }
    }

    // Click start button — same instance as the one drawn.
    if game.start_btn.update(input) == ButtonEvent::Clicked
        || input.is_key_pressed(KeyCode::Enter)
        || input.is_key_pressed(KeyCode::Space)
    {
        game.start_game(ctx);
    }
}

fn handle_playing_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    // Header buttons — same instances as the ones drawn.
    if game.restart_btn.update(input) == ButtonEvent::Clicked {
        game.start_game(ctx);
        return;
    }
    if game.menu_btn.update(input) == ButtonEvent::Clicked {
        game.state = GameState::Start;
        return;
    }

    if input.is_key_pressed(KeyCode::R) {
        game.start_game(ctx);
        return;
    }
    if input.is_key_pressed(KeyCode::M) {
        game.state = GameState::Start;
        return;
    }

    // Card click.
    if game.is_playable()
        && input.mouse_left_pressed
        && let Some((col, row)) = game.cell_at(input.mouse_pos, ctx)
    {
        game.reveal(col, row, ctx);
    }
}

fn handle_win_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    if input.is_key_pressed(KeyCode::R) || input.is_key_pressed(KeyCode::Space) {
        game.start_game(ctx);
        return;
    }
    if input.is_key_pressed(KeyCode::M) {
        game.state = GameState::Start;
    }
}

// ---------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------

/// Local tuple rect for menu items (not to be confused with `macroquad::Rect`).
type BtnRect = (f32, f32, f32, f32);

fn btn_rect_contains(r: BtnRect, p: Vec2) -> bool {
    p.x >= r.0 && p.x <= r.0 + r.2 && p.y >= r.1 && p.y <= r.1 + r.3
}

fn difficulty_rects(ctx: &GameContext, game: &Game) -> Vec<BtnRect> {
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

    rects
}

fn render_menu(ctx: &GameContext, game: &Game, input: &Input) {
    Label::new("MEMORY", ctx.window_w * 0.5, 100.0, 56, ctx.color_accent)
        .centered()
        .draw();
    Label::new(
        "// find the pairs",
        ctx.window_w * 0.5,
        135.0,
        18,
        ctx.color_text_dim,
    )
    .centered()
    .draw();

    let rects = difficulty_rects(ctx, game);

    for (i, rect) in rects.iter().enumerate() {
        let d = &game.difficulties[i];
        let selected = i == game.selected_difficulty;
        let hovered = btn_rect_contains(*rect, input.mouse_pos);

        let bg = if selected {
            ctx.color_menu_selected
        } else if hovered {
            ctx.color_menu_hover
        } else {
            ctx.color_btn_idle
        };
        draw_rectangle(rect.0, rect.1, rect.2, rect.3, bg);
        draw_rectangle_lines(rect.0, rect.1, rect.2, rect.3, 1.5, ctx.color_hidden_border);

        let line1 = format!("[{}] {}", i + 1, d.name);
        let line2 = format!("{}x{} · {} pairs", d.cols, d.rows, d.pairs);
        let best = game.best_times.get(&d.name);
        let line3 = match best {
            Some(&t) => format!("best: {}", TimerDisplay::format(t)),
            None => "best: --:--".to_owned(),
        };

        draw_text(&line1, rect.0 + 20.0, rect.1 + 26.0, 24.0, ctx.color_text);
        draw_text(
            &line2,
            rect.0 + 20.0,
            rect.1 + 46.0,
            16.0,
            ctx.color_text_dim,
        );
        let dims = measure_text(&line3, None, 16, 1.0);
        draw_text(
            &line3,
            rect.0 + rect.2 - 20.0 - dims.width,
            rect.1 + 46.0,
            16.0,
            ctx.color_accent,
        );
    }

    game.start_btn.draw(
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
        format!("{}  ·  {}x{}", d.name, d.cols, d.rows),
        12.0,
        ctx.hud_h * 0.5 + 6.0,
        20,
        ctx.color_text,
    )
    .draw();
    Label::new(
        format!("pairs: {}/{}", game.pairs_found, game.total_pairs),
        ctx.window_w * 0.5,
        ctx.hud_h * 0.5 + 6.0,
        20,
        ctx.color_accent,
    )
    .centered()
    .draw();

    // Timer — right-aligned, ending 20 px before the Restart button.
    let time_str = TimerDisplay::format(game.elapsed);
    let label_str = "time: ";
    let time_size = 20u16;
    let time_dims = measure_text(&time_str, None, time_size, 1.0);
    let label_dims = measure_text(label_str, None, time_size, 1.0);
    let time_right = ctx.window_w - 260.0;
    let x_label = time_right - time_dims.width - label_dims.width;
    let y = ctx.hud_h * 0.5 + 7.0;
    draw_text(label_str, x_label, y, time_size as f32, ctx.color_text_dim);
    let time_color = if game.timer_running {
        ctx.color_text
    } else {
        ctx.color_text_dim
    };
    draw_text(
        &time_str,
        x_label + label_dims.width,
        y,
        time_size as f32,
        time_color,
    );

    // Header buttons
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

    // Board
    let hovered_cell = if game.is_playable() {
        game.cell_at(input.mouse_pos, ctx)
    } else {
        None
    };

    for row in 0..game.board.height() {
        for col in 0..game.board.width() {
            let card = *game.board.get(col, row).unwrap();
            let r = game.card_rect(col, row, ctx);
            let hovered = matches!(hovered_cell, Some((c, rr)) if c == col && rr == row);
            let error = game.is_resolving_at(col, row);

            draw_card(ctx, &card, r, hovered, error);
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
        "R restart · M menu · Esc quit · click cards to flip",
        ctx.window_w * 0.5,
        ctx.window_h - ctx.footer_h * 0.5 + 5.0,
        14,
        ctx.color_text_dim,
    )
    .centered()
    .draw();
}

fn draw_card(ctx: &GameContext, card: &Card, r: Rect, hovered: bool, error: bool) {
    let (bg, border) = match card.state {
        CardState::Hidden => {
            let bg = if hovered {
                ctx.color_hidden_hover
            } else {
                ctx.color_hidden
            };
            (bg, ctx.color_hidden_border)
        }
        CardState::Flipped => {
            if error {
                (ctx.color_flipped, Color::new(1.0, 0.30, 0.35, 1.0))
            } else {
                (ctx.color_flipped, ctx.color_flipped_border)
            }
        }
        CardState::Matched => (ctx.color_matched, ctx.color_matched_border),
    };

    // Body
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, border);

    // Symbol (or placeholder for hidden)
    let (text, color) = match card.state {
        CardState::Hidden => ("?".to_string(), ctx.color_hidden_symbol),
        CardState::Flipped => (card.symbol.as_char().to_string(), ctx.color_symbol),
        CardState::Matched => (card.symbol.as_char().to_string(), ctx.color_matched_border),
    };

    let font_size = (r.w * 0.55).clamp(16.0, 48.0) as u16;
    let dims = measure_text(&text, None, font_size, 1.0);
    let x = r.x + (r.w - dims.width) * 0.5;
    let y = r.y + (r.h + dims.height) * 0.5 - font_size as f32 * 0.1;
    draw_text(&text, x, y, font_size as f32, color);
}

fn render_win_banner(ctx: &GameContext, game: &Game) {
    let line1 = format!("SOLVED in {}", TimerDisplay::format(game.elapsed));
    let line2 = if game.was_new_best {
        "NEW BEST!".to_string()
    } else {
        match game.best_times.get(&game.current_difficulty().name) {
            Some(&t) => format!("best: {}", TimerDisplay::format(t)),
            None => String::new(),
        }
    };

    let d1 = measure_text(&line1, None, 40, 1.0);
    let d2 = measure_text(&line2, None, 20, 1.0);
    let cx = ctx.window_w * 0.5;
    let cy = ctx.hud_h + ctx.playfield_h() * 0.5;

    let pad_x = 30.0;
    let pad_y = 25.0;
    let box_w = d1.width.max(d2.width) + pad_x * 2.0;
    let box_h = d1.height + d2.height + pad_y * 2.0 + 10.0;
    let box_x = cx - box_w * 0.5;
    let box_y = cy - box_h * 0.5;

    draw_rectangle(box_x, box_y, box_w, box_h, Color::new(0.0, 0.0, 0.0, 0.85));
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, ctx.color_matched_border);

    draw_text(
        &line1,
        cx - d1.width * 0.5,
        box_y + pad_y + d1.height,
        40.0,
        ctx.color_matched_border,
    );
    draw_text(
        &line2,
        cx - d2.width * 0.5,
        box_y + pad_y + d1.height + 10.0 + d2.height,
        20.0,
        ctx.color_text,
    );
    Label::new(
        "R or Space to play again",
        cx,
        box_y + box_h + 30.0,
        16,
        ctx.color_text_dim,
    )
    .centered()
    .draw();
}
