//! Zhuo Ji — entry point.
//!
//! Session 1: renders the human hand at the bottom, three opponent
//! zones around the table, and a center panel with wall/turn/phase.
//! Click a tile in your hand during your discard window to discard it.

use ember_stdlib::input::Input;
use macroquad::prelude::*;

use zhuo_ji::{Game, GameContext, Phase};
use zhuo_ji::components::{ClaimKind, Tile};


fn window_conf() -> Conf {
    Conf {
        window_title: "Zhuo Ji — 贵阳捉鸡麻将".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = GameContext::load();
    // Fixed seed for now. Session 5 will let the human choose one.
    let mut game = Game::new(0x2E5B_1234);

    loop {
        let input = Input::from_macroquad();
        let dt = get_frame_time().min(1.0 / 30.0);

        // Human discard on click.
        if input.mouse_left_pressed {
            if let Some(idx) = hand_hit_test(&game, &ctx, input.mouse_pos) {
                game.human_discard(idx, &ctx);
            }
        }

        if let Some(kind) = draw_claim_prompt(&game, &ctx, input.mouse_pos) {
            game.human_claim(kind);
        }

        if let Some(action) = draw_own_turn_actions(&game, &ctx, input.mouse_pos) {
            match action {
                OwnAction::Zimo => { game.human_zimo(); }
                OwnAction::AnGang(t) => { game.human_an_gang(t); }
            }
        }

        let _ = game.update(&input, &ctx, dt);

        clear_background(ctx.colors.table_bg);
        draw_table_border(&ctx);
        draw_opponents(&game, &ctx);
        draw_center_panel(&game, &ctx);
        draw_human_hand(&game, &ctx, input.mouse_pos);

        next_frame().await;
    }
}

// ─── Layout helpers ────────────────────────────────────────────────────────

fn draw_claim_prompt(game: &Game, ctx: &GameContext, mouse: Vec2) -> Option<ClaimKind> {
    let Phase::AwaitingClaims { discard, from, .. } = game.phase else {
        return None;
    };
    let opts = game.human_claim_options(discard, from)?;

    let cx = ctx.layout.window_w * 0.5;
    let y = ctx.layout.window_h * 0.5 + 100.0;
    let btn_w = 130.0;
    let btn_h = 52.0;
    let gap = 20.0;
    let total_w = opts.len() as f32 * btn_w + (opts.len().saturating_sub(1)) as f32 * gap;
    let mut x = cx - total_w * 0.5;

    let clicked = is_mouse_button_pressed(MouseButton::Left);
    let mut hit: Option<ClaimKind> = None;

    for &kind in &opts {
        let hovered = mouse.x >= x && mouse.x <= x + btn_w
            && mouse.y >= y && mouse.y <= y + btn_h;
        let bg = if hovered { ctx.colors.highlight } else { ctx.colors.tile_edge };
        draw_rectangle(x, y, btn_w, btn_h, bg);
        draw_rectangle_lines(x, y, btn_w, btn_h, 2.0, ctx.colors.tile_text);

        let label = match kind {
            ClaimKind::Peng => "Peng",
            ClaimKind::Gang => "Gang",
            ClaimKind::Hu => "Hu",
        };
        let dim = measure_text(label, None, 28, 1.0);
        draw_text(
            label,
            x + (btn_w - dim.width) * 0.5,
            y + 34.0,
            28.0,
            ctx.colors.tile_text,
        );

        if hovered && clicked {
            hit = Some(kind);
        }
        x += btn_w + gap;
    }
    hit
}

/// Top-left of tile `i` in the human's hand row.
fn human_tile_origin(ctx: &GameContext, index: usize) -> Vec2 {
    let l = &ctx.layout;
    let n = 13.0_f32;
    let total_w = n * l.tile_w + (n - 1.0) * l.tile_gap;
    let start_x = (l.window_w - total_w) * 0.5;
    let y = l.window_h - l.tile_h - 30.0;
    Vec2::new(start_x + index as f32 * (l.tile_w + l.tile_gap), y)
}

/// Return the index of the human's tile under `mouse`, if any.
fn hand_hit_test(game: &Game, ctx: &GameContext, mouse: Vec2) -> Option<usize> {
    if !matches!(game.phase, Phase::AwaitingDiscard { player: 0 }) {
        return None;
    }
    let l = &ctx.layout;
    for i in 0..game.players[0].concealed.len() {
        let o = human_tile_origin(ctx, i);
        if mouse.x >= o.x && mouse.x <= o.x + l.tile_w
            && mouse.y >= o.y && mouse.y <= o.y + l.tile_h
        {
            return Some(i);
        }
    }
    None
}

// ─── Rendering ─────────────────────────────────────────────────────────────

fn draw_table_border(ctx: &GameContext) {
    let l = &ctx.layout;
    draw_rectangle_lines(
        0.0,
        0.0,
        l.window_w,
        l.window_h,
        4.0,
        ctx.colors.table_border,
    );
}

fn draw_center_panel(game: &Game, ctx: &GameContext) {
    let cx = ctx.layout.window_w * 0.5;
    let cy = ctx.layout.window_h * 0.5;

    let wall_text = format!("Wall: {}", game.wall.len());
    draw_centered(&wall_text, cx, cy - 20.0, 32, ctx.colors.text);

    let turn_text = format!("Turn: {}", player_name(game.turn));
    draw_centered(&turn_text, cx, cy + 20.0, 28, ctx.colors.text);

    let phase_text = match game.phase {
        Phase::Deal { .. } => "Dealing…",
        Phase::AwaitingDraw { .. } => "Drawing…",
        Phase::DrawAnim { .. } => "Drawing…",
        Phase::AiThinking { .. } => "Thinking…",
        Phase::AwaitingDiscard { player: 0 } => "Your discard",
        Phase::AwaitingDiscard { .. } => "Discarding…",
        Phase::AwaitingClaims { .. } => "Claims…",
        Phase::ClaimAnim { .. } => "Claim…",
        Phase::Hu { winner, .. } => {
            if winner == 0 { "You win!" } else { "AI wins" }
        }
        Phase::HuangZhuang => "Wall empty",
    };
    draw_centered(phase_text, cx, cy + 60.0, 22, ctx.colors.text_dim);
}

fn draw_opponents(game: &Game, ctx: &GameContext) {
    draw_opponent_zone(game, ctx, 2, OpponentSlot::Top);
    draw_opponent_zone(game, ctx, 1, OpponentSlot::Left);
    draw_opponent_zone(game, ctx, 3, OpponentSlot::Right);
}

enum OpponentSlot {
    Top,
    Left,
    Right,
}

fn draw_opponent_zone(game: &Game, ctx: &GameContext, player: usize, slot: OpponentSlot) {
    let l = &ctx.layout;
    let p = &game.players[player];
    let header = player_name(player).to_string();
    let hand_line = format!("{} tiles", p.concealed.len());
    let river_line = format!("river: {}", p.discards.len());

    match slot {
        OpponentSlot::Top => {
            let x = l.window_w * 0.5;
            let y = 40.0;
            draw_centered(&header, x, y, 22, ctx.colors.text);
            draw_centered(&hand_line, x, y + 28.0, 18, ctx.colors.text_dim);
            draw_centered(&river_line, x, y + 52.0, 18, ctx.colors.text_dim);
        }
        OpponentSlot::Left => {
            let x = 60.0;
            let y = l.window_h * 0.4;
            draw_text(&header, x, y, 22.0, ctx.colors.text);
            draw_text(&hand_line, x, y + 28.0, 18.0, ctx.colors.text_dim);
            draw_text(&river_line, x, y + 52.0, 18.0, ctx.colors.text_dim);
        }
        OpponentSlot::Right => {
            let x = l.window_w - 60.0;
            let y = l.window_h * 0.4;
            draw_right_aligned(&header, x, y, 22, ctx.colors.text);
            draw_right_aligned(&hand_line, x, y + 28.0, 18, ctx.colors.text_dim);
            draw_right_aligned(&river_line, x, y + 52.0, 18, ctx.colors.text_dim);
        }
    }
}

fn draw_human_hand(game: &Game, ctx: &GameContext, mouse: Vec2) {
    let l = &ctx.layout;
    let discardable = matches!(game.phase, Phase::AwaitingDiscard { player: 0 });

    for (i, tile) in game.players[0].concealed.iter().enumerate() {
        let o = human_tile_origin(ctx, i);
        let hovered = discardable
            && mouse.x >= o.x
            && mouse.x <= o.x + l.tile_w
            && mouse.y >= o.y
            && mouse.y <= o.y + l.tile_h;

        // Tile body
        draw_rectangle(o.x, o.y, l.tile_w, l.tile_h, ctx.colors.tile_face);
        let edge = if hovered {
            ctx.colors.highlight
        } else {
            ctx.colors.tile_edge
        };
        draw_rectangle_lines(o.x, o.y, l.tile_w, l.tile_h, 2.0, edge);

        // Label
        let label = tile.label();
        let dim = measure_text(&label, None, 28, 1.0);
        let tx = o.x + (l.tile_w - dim.width) * 0.5;
        let ty = o.y + l.tile_h * 0.5 + dim.height * 0.3;
        draw_text(&label, tx, ty, 28.0, ctx.colors.tile_text);
    }
}

// ─── Small text helpers (local, to avoid stdlib path guessing) ─────────────

fn player_name(idx: usize) -> &'static str {
    match idx {
        0 => "You",
        1 => "AI-1",
        2 => "AI-2",
        3 => "AI-3",
        _ => "?",
    }
}

fn draw_centered(text: &str, center_x: f32, y: f32, size: u16, color: Color) {
    let dim = measure_text(text, None, size, 1.0);
    draw_text(text, center_x - dim.width / 2.0, y, size as f32, color);
}

fn draw_right_aligned(text: &str, right_x: f32, y: f32, size: u16, color: Color) {
    let dim = measure_text(text, None, size, 1.0);
    draw_text(text, right_x - dim.width, y, size as f32, color);
}

enum OwnAction {
    Zimo,
    AnGang(Tile),
}

fn draw_own_turn_actions(game: &Game, ctx: &GameContext, mouse: Vec2) -> Option<OwnAction> {
    let Phase::AwaitingDiscard { player: 0 } = game.phase else {
        return None;
    };

    let mut actions: Vec<(OwnAction, &'static str)> = Vec::new();
    if game.can_zimo(0) {
        actions.push((OwnAction::Zimo, "Zimo"));
    }
    if let Some(tile) = game.an_gang_tile(0) {
        actions.push((OwnAction::AnGang(tile), "An Gang"));
    }
    if actions.is_empty() {
        return None;
    }

    let btn_w = 130.0;
    let btn_h = 52.0;
    let gap = 12.0;
    let x = ctx.layout.window_w - btn_w - 40.0;
    let mut y = ctx.layout.window_h * 0.4;

    let clicked = is_mouse_button_pressed(MouseButton::Left);
    let mut hit = None;
    for (action, label) in actions {
        let hovered = mouse.x >= x && mouse.x <= x + btn_w
            && mouse.y >= y && mouse.y <= y + btn_h;
        let bg = if hovered { ctx.colors.highlight } else { ctx.colors.tile_edge };
        draw_rectangle(x, y, btn_w, btn_h, bg);
        draw_rectangle_lines(x, y, btn_w, btn_h, 2.0, ctx.colors.tile_text);
        let dim = measure_text(label, None, 22, 1.0);
        draw_text(label, x + (btn_w - dim.width) * 0.5, y + 32.0, 22.0, ctx.colors.tile_text);
        if hovered && clicked {
            hit = Some(action);
        }
        y += btn_h + gap;
    }
    hit
}