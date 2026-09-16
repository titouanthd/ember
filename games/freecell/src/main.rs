//! FreeCell — terminal-themed solitaire.

use freecell::GameState;
use freecell::components::{Card, Color as CardColor, Suit, Zone};
use freecell::config::{GameContext, load_config};
use freecell::drag::DragState;
use freecell::font;
use freecell::layout;
use freecell::systems::Game;

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
            KeyCode::Tab,
            KeyCode::Z,
            KeyCode::W,
            KeyCode::Y,
            KeyCode::M,
            KeyCode::Semicolon,
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

fn handle_start_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    if input.is_key_pressed(KeyCode::N) {
        let seed = random_seed();
        game.start_game(seed, ctx);
    }
    if input.is_key_pressed(KeyCode::Enter) || input.is_key_pressed(KeyCode::Space) {
        game.start_game(game.seed, ctx);
    }
}

fn render_drop_target(ctx: &GameContext, game: &Game, input: &Input) {
    if !game.drag.is_dragging() {
        return;
    }
    let Some(target) = game.zone_at(input.mouse_pos, ctx) else {
        return;
    };
    let rect = match target {
        Zone::FreeCell(i) => layout::free_cell_rect(i, ctx),
        Zone::Foundation(i) => layout::foundation_rect(i, ctx),
        Zone::Column(i) => {
            // Highlight the whole column, not just the top card.
            layout::column_bounds(i, ctx)
        }
    };
    let valid = game.can_drop_at(target);
    let color = if valid {
        Color::new(0.30, 0.90, 0.45, 0.65)
    } else {
        Color::new(0.95, 0.30, 0.35, 0.55)
    };
    draw_rectangle_lines(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0, 3.0, color);
}

fn handle_playing_input(game: &mut Game, ctx: &GameContext, input: &Input) {
    if input.is_key_pressed(KeyCode::Z) || input.is_key_pressed(KeyCode::W) {
        game.undo();
        return;
    }
    if input.is_key_pressed(KeyCode::Y) {
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

    if input.is_key_pressed(KeyCode::Tab) {
        game.state = GameState::Start;
        game.hint = None;
        return;
    }

    if game.hint_btn.update(input) == ButtonEvent::Clicked {
        game.hint = game.find_hint();
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
        game.hint = None;
        return;
    }

    if input.mouse_left_pressed {
        game.start_drag(input.mouse_pos, ctx);
    }
    if input.mouse_left_down {
        game.update_drag(input.mouse_pos, ctx);
    }
    if input.mouse_left_released {
        game.end_drag(input.mouse_pos, ctx);
    }
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
    if input.is_key_pressed(KeyCode::Tab)
        || input.is_key_pressed(KeyCode::M)
        || input.is_key_pressed(KeyCode::Semicolon)
    {
        game.state = GameState::Start;
        game.hint = None;
    }
}

fn random_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(1)
}

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
        "Drag cards to move them. Z / Y for undo / redo.",
        cx,
        520.0,
        18,
        ctx.color_text_dim,
    )
    .centered()
    .draw();
}

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

    render_drop_target(ctx, game, input);
    render_hint_overlay(ctx, game);
    render_drag_overlay(ctx, game, input);
    render_footer(ctx);
}

fn render_header(ctx: &GameContext, game: &Game, input: &Input) {
    Panel::new(0.0, 0.0, ctx.window_w, ctx.hud_h, ctx.color_header_bg).draw();

    let cy = ctx.hud_h * 0.5 + 7.0;

    // --- Colonne de gauche : seed + moves ---
    let seed_str = format!("Seed: {}", game.seed);
    Label::new(&seed_str, 12.0, cy, 20, ctx.color_text).draw();

    let seed_dims = measure_text(&seed_str, None, 20, 1.0);
    Label::new(
        format!("Moves: {}", game.moves),
        12.0 + seed_dims.width + 30.0,
        cy,
        20,
        ctx.color_text,
    )
    .draw();

    // --- Timer : juste à gauche du bouton Hint ---
    let time_str = format!("Time: {}", TimerDisplay::format(game.elapsed));
    let time_color = if game.timer_running {
        ctx.color_text
    } else {
        ctx.color_text_dim
    };
    let time_dims = measure_text(&time_str, None, 20, 1.0);

    // `Button::rect` est un tuple (x, y, w, h) dans ember-stdlib.
    let (hint_x, _, _, _) = game.hint_btn.rect;
    let time_x = (hint_x - time_dims.width - 20.0).max(0.0);
    Label::new(&time_str, time_x, cy, 20, time_color).draw();

    // --- Boutons (dessinés en dernier pour rester au-dessus) ---
    game.hint_btn.draw(
        input,
        ctx.color_btn_idle,
        ctx.color_btn_hover,
        ctx.color_btn_pressed,
        ctx.color_text,
    );
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
        "Drag cards · Hint shows a move · N new · R restart · Tab menu · Z undo · Y redo · Esc quit",
        ctx.window_w * 0.5,
        ctx.window_h - ctx.footer_h * 0.5 + 5.0,
        14,
        ctx.color_text_dim,
    )
    .centered()
    .draw();
}

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
        None => {
            draw_empty_slot(ctx, r);

            let suit = Suit::ALL[i];
            let symbol = suit.symbol().to_string();

            // Fond sombre → un noir à alpha 0.20 est invisible.
            // On utilise une couleur claire pour les deux, et on monte
            // l'alpha pour que le placeholder soit lisible.
            let base = match suit.color() {
                CardColor::Red => ctx.color_card_red,
                CardColor::Black => Color::new(0.55, 0.60, 0.68, 1.0),
            };
            let color = Color::new(base.r, base.g, base.b, 0.45);

            let font_size = (r.w * 0.55) as u16;
            let dims = measure_text(&symbol, None, font_size, 1.0);

            // `draw_text` prend une baseline, pas un top. Pour centrer
            // verticalement, on part du centre du rect et on rajoute
            // ~35 % de la hauteur du glyphe (descente + centrage optique).
            let x = r.x + (r.w - dims.width) * 0.5;
            let y = r.y + r.h * 0.5 + dims.height * 0.35;
            draw_text(&symbol, x, y, font_size as f32, color);
        }
    }
}

fn draw_empty_slot(ctx: &GameContext, r: Rect) {
    draw_rectangle(r.x, r.y, r.w, r.h, ctx.color_slot_empty);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, ctx.color_slot_border);
}

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
// Hint overlay
// ---------------------------------------------------------------------------

fn render_hint_overlay(ctx: &GameContext, game: &Game) {
    let Some(hint) = game.hint else {
        return;
    };

    draw_hint_zone(ctx, game, hint.from);
    draw_hint_zone(ctx, game, hint.to);

    let from = card_display(&hint.card);
    let destination = zone_display(hint.to);
    let text = format!("Hint: {} → {}", from, destination);
    Label::new(
        &text,
        ctx.window_w * 0.5,
        ctx.hud_h + 28.0,
        18,
        ctx.color_accent,
    )
    .centered()
    .draw();
}

fn draw_hint_zone(ctx: &GameContext, game: &Game, zone: Zone) {
    let r = match zone {
        Zone::FreeCell(i) => layout::free_cell_rect(i, ctx),
        Zone::Foundation(i) => layout::foundation_rect(i, ctx),
        Zone::Column(i) => {
            if let Some(card_index) = game.columns[i].len().checked_sub(1) {
                layout::card_rect_in_column(i, card_index, ctx)
            } else {
                let top = layout::column_top(i, ctx);
                Rect::new(top.x, top.y, ctx.card_w, ctx.card_h)
            }
        }
    };

    draw_rectangle_lines(r.x, r.y, r.w, r.h, 4.0, ctx.color_accent);
}

fn card_display(card: &Card) -> String {
    format!("{}{}", card.rank_label(), card.symbol())
}

fn zone_display(zone: Zone) -> String {
    match zone {
        Zone::Column(i) => format!("column {}", i + 1),
        Zone::FreeCell(i) => format!("free cell {}", i + 1),
        Zone::Foundation(i) => format!("foundation {}", i + 1),
    }
}

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
        "R / Space: play again · N: new seed · Tab: menu",
        cx,
        box_y + box_h + 30.0,
        16,
        ctx.color_text_dim,
    )
    .centered()
    .draw();
}
