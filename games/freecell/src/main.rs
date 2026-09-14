//! FreeCell — terminal-themed solitaire.

use freecell::components::{Card, Color as CardColor, Zone};
use freecell::config::{load_config, GameContext};
use freecell::drag::DragState;
use freecell::font;
use freecell::layout;
use freecell::systems::Game;
use freecell::GameState;

use ember_stdlib::input::Input;
use ember_stdlib::ui::button::ButtonEvent;
use ember_stdlib::ui::label::Label;
use ember_stdlib::ui::panel::Panel;
use ember_stdlib::ui::timer::TimerDisplay;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    let ctx = load_config();
    Conf {
        window_title: "FreeCell".to_owned(),
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

    let _font = font::load_default_font().await;

    let mut game = Game::new();

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);

        let input = Input::from_macroquad_with_keys(&[
            KeyCode::Escape,
            KeyCode::Space,
            KeyCode::Enter,
            KeyCode::R,
            KeyCode::N,
            KeyCode::M,
            KeyCode::Z,
            KeyCode::Y,
            KeyCode::LeftControl,
            KeyCode::RightControl,
        ]);

        if input.is_key_pressed(KeyCode::Escape) {
            break;
        }

        if game.state == GameState::Playing {
            game.tick(dt);
        }

        match game.state {
            GameState::Start => handle_start_input(&mut game, &ctx, &input),
            GameState::Playing => handle_playing_input(&mut game, &ctx, &input),
            GameState::Win => handle_win_input(&mut game, &ctx, &input),
            _ => {}
        }

        clear_background(ctx.color_bg);

        match game.state {
            GameState::Start => render_start(&ctx, &game),
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

fn handle_start_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    if input.is_key_pressed(KeyCode::N) {
        let seed = random_seed();
        game.start_game(seed, ctx);
    }
    if input.is_key_pressed(KeyCode::Enter) || input.is_key_pressed(KeyCode::Space) {
        game.start_game(game.seed, ctx);
    }
}

fn handle_playing_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    let ctrl = input.is_key_down(KeyCode::LeftControl)
        || input.is_key_down(KeyCode::RightControl);
    if ctrl && input.is_key_pressed(KeyCode::Z) {
        game.undo();
        return;
    }
    if ctrl && input.is_key_pressed(KeyCode::Y) {
        game.redo();
        return;
    }

    if input.is_key_pressed(KeyCode::N) {
        let seed = random_seed();
        game.start_game(seed, ctx);
        return;
    }
    if input.is_key_pressed(KeyCode::R) {
        game.start_game(game.seed, ctx);
        return;
    }
    if input.is_key_pressed(KeyCode::M) {
        game.state = GameState::Start;
        return;
    }

    if game.restart_btn.update(input) == ButtonEvent::Clicked {
        game.start_game(game.seed, ctx);
        return;
    }
    if game.new_game_btn.update(input) == ButtonEvent::Clicked {
        let seed = random_seed();
        game.start_game(seed, ctx);
        return;
    }
    if game.menu_btn.update(input) == ButtonEvent::Clicked {
        game.state = GameState::Start;
        return;
    }

    // Drag-and-drop.
    if input.mouse_left_pressed {
        game.start_drag(input.mouse_pos, ctx);
    }
    if input.mouse_left_down {
        game.update_drag(input.mouse_pos, ctx);
    }
    if input.mouse_left_released {
        game.end_drag(input.mouse_pos, ctx);
    }
    // Safety: if the mouse is not down but we're still dragging (mouse
    // left the window, focus lost, etc.), cancel.
    if !input.mouse_left_down && game.drag.is_dragging() {
        game.cancel_drag();
    }
}

fn handle_win_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    if input.is_key_pressed(KeyCode::R) || input.is_key_pressed(KeyCode::Space) {
        game.start_game(game.seed, ctx);
        return;
    }
    if input.is_key_pressed(KeyCode::N) {
        let seed = random_seed();
        game.start_game(seed, ctx);
        return;
    }
    if input.is_key_pressed(KeyCode::M) {
        game.state = GameState::Start;
    }
}

fn random_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(1)
}

// ---------------------------------------------------------------------------
// Start screen
// ---------------------------------------------------------------------------

fn render_start(ctx: &GameContext, game: &Game) {
    let cx = ctx.window_w * 0.5;

    Label::new("FREECELL", cx, 120.0, 72, ctx.color_accent)
        .centered()
        .draw();
    Label::new("// all cards visible", cx, 160.0, 20, ctx.color_text_dim)
        .centered()
        .draw();

    let seed_str = format!("Seed: {}", game.seed);
    Label::new(&seed_str, cx, 260.0, 32, ctx.color_text)
        .centered()
        .draw();

    let best_str = match game.best_times.get(&game.seed.to_string()) {
        Some(&t) => format!("Best: {}", TimerDisplay::format(t)),
        None => "Best: --:--".to_owned(),
    };
    Label::new(&best_str, cx, 310.0, 24, ctx.color_accent)
        .centered()
        .draw();

    Label::new("N: new game (random seed)", cx, 400.0, 24, ctx.color_text)
        .centered()
        .draw();
    Label::new("Enter / Space: start", cx, 440.0, 24, ctx.color_text)
        .centered()
        .draw();

    Label::new(
        "Drag cards to move them. Ctrl+Z / Ctrl+Y for undo / redo.",
        cx,
        520.0,
        18,
        ctx.color_text_dim,
    )
    .centered()
    .draw();
}

// ---------------------------------------------------------------------------
// Game rendering
// ---------------------------------------------------------------------------

fn render_game(ctx: &GameContext, game: &Game, input: &Input) {
    render_header(ctx, game, input);

    for i in 0..4 {
        render_free_cell(ctx, game, i);
    }
    for i in 0..4 {
        render_foundation(ctx, game, i);
    }

    for col in 0..8 {
        render_column(ctx, game, col);
    }

    render_drag_overlay(ctx, game, input);

    render_footer(ctx);
}

fn render_header(ctx: &GameContext, game: &Game, input: &Input) {
    Panel::new(0.0, 0.0, ctx.window_w, ctx.hud_h, ctx.color_header_bg).draw();

    let cy = ctx.hud_h * 0.5 + 7.0;

    // Seed.
    let seed_str = format!("Seed: {}", game.seed);
    Label::new(&seed_str, 12.0, cy, 20, ctx.color_text).draw();

    // Moves, right after seed.
    let seed_dims = measure_text(&seed_str, None, 20, 1.0);
    Label::new(
        format!("Moves: {}", game.moves),
        12.0 + seed_dims.width + 30.0,
        cy,
        20,
        ctx.color_text,
    )
    .draw();

    // Time, right-aligned before the buttons.
    let time_str = format!("Time: {}", TimerDisplay::format(game.elapsed));
    let time_color = if game.timer_running {
        ctx.color_text
    } else {
        ctx.color_text_dim
    };
    let time_dims = measure_text(&time_str, None, 20, 1.0);
    let time_x = ctx.window_w - 380.0 - time_dims.width;
    Label::new(&time_str, time_x, cy, 20, time_color).draw();

    // Buttons.
    game.restart_btn.draw(
        input,
        ctx.color_btn_idle,
        ctx.color_btn_hover,
        ctx.color_btn_pressed,
        ctx.color_text,
    );
    game.new_game_btn.draw(
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
        "Drag cards · N new · R restart · M menu · Ctrl+Z undo · Ctrl+Y redo · Esc quit",
        ctx.window_w * 0.5,
        ctx.window_h - ctx.footer_h * 0.5 + 5.0,
        14,
        ctx.color_text_dim,
    )
    .centered()
    .draw();
}

// ---------------------------------------------------------------------------
// Free cells and foundations
// ---------------------------------------------------------------------------

fn render_free_cell(ctx: &GameContext, game: &Game, i: usize) {
    let r = layout::free_cell_rect(i, ctx);
    match game.free_cells[i] {
        Some(card) => draw_card(ctx, card, r, false),
        None => draw_empty_slot(ctx, r),
    }
}

fn render_foundation(ctx: &GameContext, game: &Game, i: usize) {
    let r = layout::foundation_rect(i, ctx);
    match game.foundations[i].last() {
        Some(card) => draw_card(ctx, *card, r, false),
        None => draw_empty_slot(ctx, r),
    }
}

fn draw_empty_slot(ctx: &GameContext, r: Rect) {
    draw_rectangle(r.x, r.y, r.w, r.h, ctx.color_slot_empty);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, ctx.color_slot_border);
}

// ---------------------------------------------------------------------------
// Columns
// ---------------------------------------------------------------------------

fn render_column(ctx: &GameContext, game: &Game, col: usize) {
    let len = game.columns[col].len();
    if len == 0 {
        let top = layout::column_top(col, ctx);
        let r = Rect::new(top.x, top.y, ctx.card_w, ctx.card_h);
        draw_empty_slot(ctx, r);
        return;
    }
    for card_index in 0..len {
        if is_being_dragged(game, col, card_index) {
            continue;
        }
        let card = game.columns[col][card_index];
        let r = layout::card_rect_in_column(col, card_index, ctx);
        draw_card(ctx, card, r, false);
    }
}

/// True if the given card is part of the current drag stack.
///
/// Uses `start_index` and `cards.len()` to identify the exact range of
/// dragged indices in the origin column.
fn is_being_dragged(game: &Game, col: usize, card_index: usize) -> bool {
    if let DragState::Dragging {
        origin,
        start_index,
        cards,
        ..
    } = &game.drag
        && let Zone::Column(c) = origin
        && *c == col
    {
        return card_index >= *start_index && card_index < *start_index + cards.len();
    }
    false
}

// ---------------------------------------------------------------------------
// Card rendering
// ---------------------------------------------------------------------------

fn draw_card(ctx: &GameContext, card: Card, r: Rect, hovered: bool) {
    let bg = if hovered {
        ctx.color_slot_hover
    } else {
        ctx.color_card_bg
    };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, ctx.color_card_border);

    let fg = match card.color() {
        CardColor::Red => ctx.color_card_red,
        CardColor::Black => ctx.color_card_black,
    };

    let rank_str = card.rank_label().to_string();
    draw_text(&rank_str, r.x + 6.0, r.y + 28.0, 26.0, fg);

    let suit_str = card.symbol().to_string();
    let suit_dims = measure_text(&suit_str, None, 26, 1.0);
    draw_text(
        &suit_str,
        r.x + r.w - 6.0 - suit_dims.width,
        r.y + 28.0,
        26.0,
        fg,
    );
}

// ---------------------------------------------------------------------------
// Drag overlay
// ---------------------------------------------------------------------------

fn render_drag_overlay(ctx: &GameContext, game: &Game, input: &Input) {
    let DragState::Dragging { cards, offset, .. } = &game.drag else {
        return;
    };

    let base = input.mouse_pos - *offset;
    let drag_offset_y = 20.0;

    for (i, card) in cards.iter().enumerate() {
        let y = base.y + i as f32 * drag_offset_y;
        let r = Rect::new(base.x, y, ctx.card_w, ctx.card_h);

        draw_rectangle(r.x + 4.0, r.y + 4.0, r.w, r.h, ctx.color_drag_shadow);
        draw_card(ctx, *card, r, false);
    }
}

// ---------------------------------------------------------------------------
// Win banner
// ---------------------------------------------------------------------------

fn render_win_banner(ctx: &GameContext, game: &Game) {
    let line1 = format!("SOLVED in {}", TimerDisplay::format(game.elapsed));
    let line2 = format!("Moves: {}", game.moves);
    let line3 = if game.was_new_best {
        "NEW BEST!".to_string()
    } else {
        match game.best_times.get(&game.seed.to_string()) {
            Some(&t) => format!("Best: {}", TimerDisplay::format(t)),
            None => String::new(),
        }
    };

    let d1 = measure_text(&line1, None, 40, 1.0);
    let d2 = measure_text(&line2, None, 24, 1.0);
    let d3 = measure_text(&line3, None, 24, 1.0);
    let cx = ctx.window_w * 0.5;
    let cy = ctx.hud_h + ctx.playfield_h() * 0.5;

    let pad_x = 40.0;
    let pad_y = 30.0;
    let box_w = d1.width.max(d2.width).max(d3.width) + pad_x * 2.0;
    let box_h = d1.height + d2.height + d3.height + pad_y * 2.0 + 30.0;
    let box_x = cx - box_w * 0.5;
    let box_y = cy - box_h * 0.5;

    draw_rectangle(box_x, box_y, box_w, box_h, Color::new(0.0, 0.0, 0.0, 0.88));
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, ctx.color_accent);

    let mut y = box_y + pad_y + d1.height;
    draw_text(&line1, cx - d1.width * 0.5, y, 40.0, ctx.color_accent);
    y += d2.height + 15.0;
    draw_text(&line2, cx - d2.width * 0.5, y, 24.0, ctx.color_text);
    y += d3.height + 10.0;
    if !line3.is_empty() {
        draw_text(&line3, cx - d3.width * 0.5, y, 24.0, ctx.color_text);
    }

    Label::new(
        "R / Space: play again · N: new seed · M: menu",
        cx,
        box_y + box_h + 30.0,
        16,
        ctx.color_text_dim,
    )
    .centered()
    .draw();
}