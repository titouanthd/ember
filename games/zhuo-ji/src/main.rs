//! Zhuo Ji — entry point.
//!
//! Layout:
//! - Top-left: shanten, hand counter, turn, phase (the "info stack").
//! - Top-right: wall pile.
//! - Avatars (50x50) with name, tile count, score below each player.
//! - Clock at the EXACT screen center (runtime h + v).
//! - Rivers centered "in front of" each player.
//! - Drawn tile on the RIGHT of the concealed hand.
//! - Action buttons right-aligned.

use std::time::{SystemTime, UNIX_EPOCH};

use ember_stdlib::input::Input;
use macroquad::prelude::*;

use zhuo_ji::components::{ClaimKind, Meld, Suit, Tile};
use zhuo_ji::hand;
use zhuo_ji::tiles::{self, TileSize};
use zhuo_ji::{Game, GameContext, Phase};

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

fn fresh_seed() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_nanos() ^ (d.as_nanos() >> 32)) as u32)
        .unwrap_or(0x2E5B_1234)
}

const TRACKED_KEYS: &[KeyCode] = &[KeyCode::Space, KeyCode::R, KeyCode::Escape];

// ─── Layout constants ──────────────────────────────────────────────────────

const MARGIN: f32 = 20.0;

const TILE_W: f32 = 40.0;
const TILE_H: f32 = 54.0;
const TILE_GAP: f32 = 4.0;
const DRAWN_GAP: f32 = 24.0;

const HAND_Y: f32 = 646.0;
const MELDS_Y: f32 = 578.0;
const HUMAN_RIVER_Y: f32 = 490.0;
const HUMAN_RIVER_COLS: usize = 8;
const MELD_GROUP_GAP: f32 = 12.0;

const BTN_W: f32 = 120.0;
const BTN_H: f32 = 50.0;
const BTN_GAP: f32 = 10.0;
const BTN_Y: f32 = 420.0;

const CLOCK_R: f32 = 42.0;

const WALL_W: f32 = 70.0;
const WALL_H: f32 = 44.0;

const AVATAR_SIZE: f32 = 50.0;

const TOP_RIVER_COLS: usize = 6;
const SIDE_RIVER_COLS: usize = 4;
const RIVER_ROWS_MAX: usize = 3;

const AVATAR_LEFT_X: f32 = MARGIN;
const AVATAR_LEFT_Y: f32 = 240.0;
const AVATAR_HUMAN_X: f32 = MARGIN;
const AVATAR_HUMAN_Y: f32 = 570.0;

const AVATAR_NAME_OFFSET: f32 = 18.0;
const AVATAR_TILES_OFFSET: f32 = 18.0;
const AVATAR_SCORE_OFFSET: f32 = 22.0;

const INFO_X: f32 = MARGIN;
const INFO_Y: f32 = 30.0;

// ─── Runtime screen-dependent helpers ──────────────────────────────────────

fn scr_w() -> f32 { screen_width() }
fn scr_h() -> f32 { screen_height() }
fn scr_cx() -> f32 { scr_w() * 0.5 }
fn scr_cy() -> f32 { scr_h() * 0.5 }

fn clock_cx() -> f32 { scr_cx() }
fn clock_cy() -> f32 { scr_cy() }

fn wall_x() -> f32 { scr_w() - MARGIN - WALL_W }
fn wall_y() -> f32 { MARGIN }

fn avatar_top_x() -> f32 { scr_cx() - AVATAR_SIZE * 0.5 }
fn avatar_right_x() -> f32 { scr_w() - MARGIN - AVATAR_SIZE }
fn btn_right() -> f32 { scr_w() - MARGIN }

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = GameContext::load();
    let mut game = Game::new(fresh_seed());

    loop {
        let mut input = Input::from_macroquad_with_keys(TRACKED_KEYS);
        input.keys_down = TRACKED_KEYS
            .iter()
            .copied()
            .filter(|k| is_key_down(*k))
            .collect();
        input.keys_released = TRACKED_KEYS
            .iter()
            .copied()
            .filter(|k| is_key_released(*k))
            .collect();

        let dt = get_frame_time().min(1.0 / 30.0);

        if input.is_key_pressed(KeyCode::Escape) {
            break;
        }

        if input.mouse_left_pressed {
            if let Some(action) = hit_claim_button(&game, input.mouse_pos) {
                match action {
                    ClaimAction::Claim(kind) => { game.human_claim(kind); }
                    ClaimAction::Pass => { game.human_pass(&ctx); }
                }
            } else if let Some(action) = hit_own_turn_button(&game, input.mouse_pos) {
                match action {
                    OwnAction::Zimo => { game.human_zimo(); }
                    OwnAction::AnGang(t) => { game.human_an_gang(t); }
                }
            } else if let Some(idx) = hit_hand_tile(&game, input.mouse_pos) {
                game.human_discard(idx, &ctx);
            }
        }

        let _ = game.update(&input, &ctx, dt);

        clear_background(ctx.colors.table_bg);
        draw_table_border(&ctx);

        draw_wall(&game, &ctx);
        draw_clock(&game, &ctx);
        draw_under_clock(&game, &ctx);

        draw_avatars(&game, &ctx);

        draw_opponent_zone(&game, &ctx, 2, Zone::Top);
        draw_opponent_zone(&game, &ctx, 1, Zone::Left);
        draw_opponent_zone(&game, &ctx, 3, Zone::Right);

        draw_human_melds(&game, &ctx);
        draw_human_river(&game, &ctx);
        draw_human_hand(&game, &ctx, input.mouse_pos);

        draw_info_stack(&game, &ctx);
        draw_claim_buttons(&game, &ctx, input.mouse_pos);
        draw_own_turn_buttons(&game, &ctx, input.mouse_pos);

        next_frame().await;
    }
}

// ─── Hit testing ───────────────────────────────────────────────────────────

fn rect_contains(x: f32, y: f32, w: f32, h: f32, p: Vec2) -> bool {
    p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
}

fn human_hand_layout(game: &Game) -> (Option<Vec2>, f32) {
    let concealed_len = game.players[0].concealed.len();
    let has_drawn = game.players[0].drawn.is_some();

    let concealed_width = if concealed_len == 0 {
        0.0
    } else {
        concealed_len as f32 * TILE_W + (concealed_len - 1) as f32 * TILE_GAP
    };
    let gap = if concealed_len > 0 { DRAWN_GAP } else { 0.0 };

    let mut total = concealed_width;
    if has_drawn {
        total += gap + TILE_W;
    }
    let start_x = (scr_w() - total) * 0.5;

    if has_drawn {
        let drawn_x = start_x + concealed_width + gap;
        (Some(Vec2::new(drawn_x, HAND_Y)), start_x)
    } else {
        (None, start_x)
    }
}

fn hit_hand_tile(game: &Game, mouse: Vec2) -> Option<usize> {
    if !matches!(game.phase, Phase::AwaitingDiscard { player: 0 }) {
        return None;
    }
    let (drawn_origin, concealed_start) = human_hand_layout(game);

    for i in 0..game.players[0].concealed.len() {
        let x = concealed_start + i as f32 * (TILE_W + TILE_GAP);
        if rect_contains(x, HAND_Y, TILE_W, TILE_H, mouse) {
            return Some(i);
        }
    }

    if let Some(o) = drawn_origin
        && rect_contains(o.x, o.y, TILE_W, TILE_H, mouse)
    {
        return Some(game.players[0].concealed.len());
    }
    None
}

enum OwnAction {
    Zimo,
    AnGang(Tile),
}

fn own_turn_actions(game: &Game) -> Vec<(OwnAction, &'static str)> {
    let Phase::AwaitingDiscard { player: 0 } = game.phase else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if game.can_zimo(0) {
        out.push((OwnAction::Zimo, "ZIMO!"));
    }
    if let Some(tile) = game.an_gang_tile(0) {
        out.push((OwnAction::AnGang(tile), "An Gang"));
    }
    out
}

fn own_turn_button_rects(game: &Game) -> Vec<(OwnAction, (f32, f32, f32, f32))> {
    let actions = own_turn_actions(game);
    if actions.is_empty() { return Vec::new(); }
    let n = actions.len();
    let total = n as f32 * BTN_W + (n - 1) as f32 * BTN_GAP;
    let mut x = btn_right() - total;
    let mut out = Vec::new();
    for (action, _) in actions {
        out.push((action, (x, BTN_Y, BTN_W, BTN_H)));
        x += BTN_W + BTN_GAP;
    }
    out
}

fn hit_own_turn_button(game: &Game, mouse: Vec2) -> Option<OwnAction> {
    for (action, (x, y, w, h)) in own_turn_button_rects(game) {
        if rect_contains(x, y, w, h, mouse) {
            return Some(action);
        }
    }
    None
}

enum ClaimAction {
    Claim(ClaimKind),
    Pass,
}

fn claim_button_rects(game: &Game) -> Vec<(ClaimAction, (f32, f32, f32, f32))> {
    let Phase::AwaitingClaims { discard, from, .. } = game.phase else {
        return Vec::new();
    };
    let Some(opts) = game.human_claim_options(discard, from) else {
        return Vec::new();
    };
    let n = opts.len() + 1;
    let total = n as f32 * BTN_W + (n - 1) as f32 * BTN_GAP;
    let mut x = btn_right() - total;
    let mut out = Vec::new();
    for kind in opts {
        out.push((ClaimAction::Claim(kind), (x, BTN_Y, BTN_W, BTN_H)));
        x += BTN_W + BTN_GAP;
    }
    out.push((ClaimAction::Pass, (x, BTN_Y, BTN_W, BTN_H)));
    out
}

fn hit_claim_button(game: &Game, mouse: Vec2) -> Option<ClaimAction> {
    for (action, (x, y, w, h)) in claim_button_rects(game) {
        if rect_contains(x, y, w, h, mouse) {
            return Some(action);
        }
    }
    None
}

// ─── Rendering ─────────────────────────────────────────────────────────────

fn draw_table_border(ctx: &GameContext) {
    draw_rectangle_lines(0.0, 0.0, scr_w(), scr_h(), 4.0, ctx.colors.table_border);
}

fn draw_wall(game: &Game, ctx: &GameContext) {
    tiles::draw_wall(
        wall_x(),
        wall_y(),
        WALL_W,
        WALL_H,
        game.wall.len(),
        &ctx.colors,
    );
}

fn clock_value(game: &Game, ctx: &GameContext) -> Option<(f32, f32)> {
    match game.phase {
        Phase::AwaitingDiscard { player: 0 } => {
            Some((game.turn_timer, ctx.layout.turn_window))
        }
        Phase::AwaitingClaims { t, .. } => Some((t, ctx.layout.claim_window)),
        Phase::AiThinking { t, .. } => Some((t, ctx.layout.ai_think_duration)),
        _ => None,
    }
}

fn clock_arc_color(frac: f32, ctx: &GameContext) -> Color {
    if frac > 0.5 {
        Color::new(0.35, 0.80, 0.45, 1.0)
    } else if frac > 0.2 {
        Color::new(0.95, 0.75, 0.30, 1.0)
    } else {
        ctx.colors.danger
    }
}

fn draw_clock(game: &Game, ctx: &GameContext) {
    let cx = clock_cx();
    let cy = clock_cy();

    draw_circle(cx, cy, CLOCK_R + 3.0, Color::new(0.05, 0.08, 0.06, 1.0));
    draw_circle(cx, cy, CLOCK_R, Color::new(0.14, 0.18, 0.14, 1.0));
    draw_circle_lines(cx, cy, CLOCK_R, 2.0, ctx.colors.table_border);

    if let Some((remaining, limit)) = clock_value(game, ctx) {
        let frac = (remaining / limit.max(1e-3)).clamp(0.0, 1.0);
        let color = clock_arc_color(frac, ctx);
        draw_arc_ring(cx, cy, CLOCK_R - 3.0, frac, color, 5.0);

        let secs = remaining.max(0.0).ceil() as i32;
        let text = format!("{secs}");
        let dim = measure_text(&text, None, 30, 1.0);
        draw_text(
            &text,
            cx - dim.width * 0.5,
            cy + dim.height * 0.30,
            30.0,
            ctx.colors.text,
        );
    } else {
        let text = "—";
        let dim = measure_text(text, None, 30, 1.0);
        draw_text(
            text,
            cx - dim.width * 0.5,
            cy + dim.height * 0.30,
            30.0,
            ctx.colors.text_dim,
        );
    }

    let angle = seat_angle(game.turn);
    draw_pointer(cx, cy, angle, CLOCK_R + 8.0, CLOCK_R + 20.0, ctx.colors.highlight);
}

fn seat_angle(player: usize) -> f32 {
    match player {
        0 => std::f32::consts::FRAC_PI_2,
        1 => std::f32::consts::PI,
        2 => -std::f32::consts::FRAC_PI_2,
        3 => 0.0,
        _ => 0.0,
    }
}

fn draw_pointer(cx: f32, cy: f32, angle: f32, r_inner: f32, r_outer: f32, color: Color) {
    let half = 0.22;
    let tip_x = cx + r_outer * angle.cos();
    let tip_y = cy + r_outer * angle.sin();
    let lx = cx + r_inner * (angle + half).cos();
    let ly = cy + r_inner * (angle + half).sin();
    let rx = cx + r_inner * (angle - half).cos();
    let ry = cy + r_inner * (angle - half).sin();
    draw_triangle(
        Vec2::new(tip_x, tip_y),
        Vec2::new(lx, ly),
        Vec2::new(rx, ry),
        color,
    );
}

fn draw_arc_ring(cx: f32, cy: f32, r: f32, frac: f32, color: Color, thickness: f32) {
    if frac <= 0.0 { return; }
    let start = -std::f32::consts::FRAC_PI_2;
    let span = frac * std::f32::consts::TAU;
    let segments = ((span / (std::f32::consts::TAU / 64.0)).ceil() as i32).max(2);
    for i in 0..segments {
        let t0 = start + span * (i as f32 / segments as f32);
        let t1 = start + span * ((i + 1) as f32 / segments as f32);
        let p0 = Vec2::new(cx + r * t0.cos(), cy + r * t0.sin());
        let p1 = Vec2::new(cx + r * t1.cos(), cy + r * t1.sin());
        draw_line(p0.x, p0.y, p1.x, p1.y, thickness, color);
    }
}

fn draw_under_clock(game: &Game, ctx: &GameContext) {
    let cx = clock_cx();
    let base_y = clock_cy() + CLOCK_R + 34.0;

    if let Some(ji) = game.last_ji {
        draw_centered(
            &format!("Ji: {}", ji.primary.label()),
            cx,
            base_y,
            16,
            ctx.colors.text_dim,
        );
    }

    if matches!(game.phase, Phase::MatchOver { .. }) {
        draw_centered(
            "Space: 16 more    R: new match    Échap: quit",
            scr_cx(),
            base_y + 30.0,
            16,
            ctx.colors.highlight,
        );
    }
}

fn phase_label(game: &Game) -> String {
    match game.phase {
        Phase::Deal { .. } => "Dealing…".into(),
        Phase::AwaitingDraw { .. } => "Drawing…".into(),
        Phase::DrawAnim { .. } => "Drawing…".into(),
        Phase::AiThinking { .. } => "Thinking…".into(),
        Phase::AwaitingDiscard { player: 0 } => "Your discard".into(),
        Phase::AwaitingDiscard { .. } => "Discarding…".into(),
        Phase::AwaitingClaims { t, .. } => format!("Claims… ({:.1}s)", t.max(0.0)),
        Phase::ClaimAnim { .. } => "Claim…".into(),
        Phase::Hu { winner, .. } => {
            if winner == 0 { "You win! (Space)".into() }
            else { format!("{} wins (Space)", player_name(winner)) }
        }
        Phase::HuangZhuang => "Wall empty (Space)".into(),
        Phase::MatchOver { winner } => {
            if winner == 0 { "MATCH — You win!".into() }
            else { format!("MATCH — {} wins", player_name(winner)) }
        }
    }
}

fn draw_info_stack(game: &Game, ctx: &GameContext) {
    if matches!(game.phase, Phase::MatchOver { .. } | Phase::HuangZhuang) {
        let mut cy = INFO_Y;
        draw_text(
            format!("Turn: {}", player_name(game.turn)),
            INFO_X,
            cy,
            18.0,
            ctx.colors.text,
        );
        cy += 24.0;
        draw_text(phase_label(game), INFO_X, cy, 16.0, ctx.colors.text_dim);
        return;
    }

    let concealed = &game.players[0].concealed;
    let melds = &game.players[0].melds;
    let s = hand::shanten(concealed, melds);

    let (text, color) = if s < 0 {
        ("Ready".to_string(), ctx.colors.highlight)
    } else if s == 0 {
        ("TENPAI".to_string(), ctx.colors.highlight)
    } else if s == 1 {
        ("1-shanten".to_string(), ctx.colors.text)
    } else {
        (format!("{s}-shanten"), ctx.colors.text_dim)
    };

    let mut cy = INFO_Y;

    draw_text(&text, INFO_X, cy, 22.0, color);
    cy += 28.0;

    if s == 0 {
        let waits = waiting_tiles(concealed, melds);
        if !waits.is_empty() {
            let mut wx = INFO_X;
            for t in waits {
                let lbl = t.label();
                let dim = measure_text(&lbl, None, 18, 1.0);
                draw_text(&lbl, wx, cy, 18.0, ctx.colors.highlight);
                wx += dim.width + 10.0;
            }
            cy += 26.0;
        }
    }

    draw_text(
        format!(
            "Hand {} / {}",
            (game.hands_played + 1).min(game.match_length),
            game.match_length
        ),
        INFO_X,
        cy,
        18.0,
        ctx.colors.text_dim,
    );
    cy += 30.0;

    draw_text(
        format!("Turn: {}", player_name(game.turn)),
        INFO_X,
        cy,
        18.0,
        ctx.colors.text,
    );
    cy += 24.0;

    draw_text(phase_label(game), INFO_X, cy, 16.0, ctx.colors.text_dim);
}

fn draw_avatars(game: &Game, ctx: &GameContext) {
    let size = AVATAR_SIZE;
    let layouts: [(usize, f32, f32); 4] = [
        (0, AVATAR_HUMAN_X, AVATAR_HUMAN_Y),
        (1, AVATAR_LEFT_X, AVATAR_LEFT_Y),
        (2, avatar_top_x(), MARGIN),
        (3, avatar_right_x(), AVATAR_LEFT_Y),
    ];
    for (seat, x, y) in layouts {
        let is_current = game.turn == seat;
        draw_avatar(seat, x, y, size, is_current, ctx);

        let center_x = x + size * 0.5;
        let name_y = y + size + AVATAR_NAME_OFFSET;
        let tiles_y = name_y + AVATAR_TILES_OFFSET;
        let score_y = tiles_y + AVATAR_SCORE_OFFSET;

        let p = &game.players[seat];
        let tile_count = p.concealed.len() + p.drawn.is_some() as usize;

        draw_centered(player_name(seat), center_x, name_y, 16, ctx.colors.text);
        draw_centered(
            &format!("{tile_count}"),
            center_x,
            tiles_y,
            14,
            ctx.colors.text_dim,
        );
        draw_centered(
            &format!("{}", p.score),
            center_x,
            score_y,
            20,
            if seat == 0 { ctx.colors.highlight } else { ctx.colors.text },
        );
    }
}

fn avatar_color(idx: usize) -> Color {
    match idx {
        0 => Color::new(0.30, 0.55, 0.90, 1.0),
        1 => Color::new(0.88, 0.35, 0.35, 1.0),
        2 => Color::new(0.95, 0.75, 0.30, 1.0),
        3 => Color::new(0.35, 0.75, 0.45, 1.0),
        _ => WHITE,
    }
}

fn avatar_initial(idx: usize) -> &'static str {
    match idx {
        0 => "Y",
        1 => "1",
        2 => "2",
        3 => "3",
        _ => "?",
    }
}

fn draw_avatar(idx: usize, x: f32, y: f32, size: f32, is_current: bool, ctx: &GameContext) {
    let fill = avatar_color(idx);
    draw_rectangle(x, y, size, size, fill);
    let border_color = if is_current { ctx.colors.highlight } else { Color::new(0.08, 0.08, 0.08, 1.0) };
    let border_w = if is_current { 4.0 } else { 2.0 };
    draw_rectangle_lines(x, y, size, size, border_w, border_color);

    let label = avatar_initial(idx);
    let dim = measure_text(label, None, 28, 1.0);
    draw_text(
        label,
        x + (size - dim.width) * 0.5,
        y + size * 0.5 + dim.height * 0.35,
        28.0,
        WHITE,
    );
}

enum Zone {
    Top,
    Left,
    Right,
}

fn draw_opponent_zone(game: &Game, ctx: &GameContext, player: usize, zone: Zone) {
    let p = &game.players[player];
    let melds_line = if p.melds.is_empty() {
        None
    } else {
        Some(melds_summary(&p.melds))
    };

    match zone {
        Zone::Top => {
            let cx = scr_cx();
            let avatar_bottom = MARGIN + AVATAR_SIZE
                + AVATAR_NAME_OFFSET + AVATAR_TILES_OFFSET + AVATAR_SCORE_OFFSET;
            let melds_y = avatar_bottom + 22.0;
            let river_y = melds_y + 22.0;

            if let Some(line) = &melds_line {
                draw_centered(line, cx, melds_y, 16, ctx.colors.highlight);
            }
            let block_w = river_block_width(p.discards.len(), TOP_RIVER_COLS, RIVER_ROWS_MAX);
            let x_left = cx - block_w * 0.5;
            draw_river_block(
                p.discards.as_slice(),
                x_left,
                river_y,
                TOP_RIVER_COLS,
                RIVER_ROWS_MAX,
                ctx,
            );
        }
        Zone::Left => {
            let label_x = AVATAR_LEFT_X + AVATAR_SIZE + 12.0;
            let anchor_y = AVATAR_LEFT_Y + AVATAR_SIZE * 0.5;
            if let Some(line) = &melds_line {
                draw_text(line, label_x, anchor_y + 2.0, 16.0, ctx.colors.highlight);
            }
            let block_h = river_block_height(p.discards.len(), SIDE_RIVER_COLS, RIVER_ROWS_MAX);
            let y_top = anchor_y - block_h * 0.5 + 12.0;
            draw_river_block(
                p.discards.as_slice(),
                label_x,
                y_top,
                SIDE_RIVER_COLS,
                RIVER_ROWS_MAX,
                ctx,
            );
        }
        Zone::Right => {
            let label_x = avatar_right_x() - 12.0;
            let anchor_y = AVATAR_LEFT_Y + AVATAR_SIZE * 0.5;
            if let Some(line) = &melds_line {
                draw_right_aligned(line, label_x, anchor_y + 2.0, 16, ctx.colors.highlight);
            }
            let block_w = river_block_width(p.discards.len(), SIDE_RIVER_COLS, RIVER_ROWS_MAX);
            let block_h = river_block_height(p.discards.len(), SIDE_RIVER_COLS, RIVER_ROWS_MAX);
            let x_left = label_x - block_w;
            let y_top = anchor_y - block_h * 0.5 + 12.0;
            draw_river_block(
                p.discards.as_slice(),
                x_left,
                y_top,
                SIDE_RIVER_COLS,
                RIVER_ROWS_MAX,
                ctx,
            );
        }
    }
}

fn melds_summary(melds: &[Meld]) -> String {
    melds
        .iter()
        .map(|m| match m {
            Meld::Peng { tile, .. } => format!("P{}", tile.label()),
            Meld::Gang { tile, .. } => format!("G{}", tile.label()),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn river_rows(count: usize, cols: usize, rows_max: usize) -> usize {
    if count == 0 { return 0; }
    count.div_ceil(cols).min(rows_max)
}

fn river_block_width(count: usize, cols: usize, rows_max: usize) -> f32 {
    if count == 0 { return 0.0; }
    let rows = river_rows(count, cols, rows_max);
    let max_cols = if rows <= 1 { count.min(cols) } else { cols };
    max_cols as f32 * TILE_W + (max_cols.saturating_sub(1)) as f32 * TILE_GAP
}

fn river_block_height(count: usize, cols: usize, rows_max: usize) -> f32 {
    let rows = river_rows(count, cols, rows_max);
    if rows == 0 { return 0.0; }
    rows as f32 * TILE_H + (rows - 1) as f32 * TILE_GAP
}

fn draw_river_block(
    tiles_slice: &[Tile],
    x_left: f32,
    y_top: f32,
    cols: usize,
    rows_max: usize,
    ctx: &GameContext,
) {
    let tile_size = TileSize::new(TILE_W, TILE_H);
    let max = cols * rows_max;
    for (i, &t) in tiles_slice.iter().enumerate().take(max) {
        let col = i % cols;
        let row = i / cols;
        let x = x_left + col as f32 * (TILE_W + TILE_GAP);
        let y = y_top + row as f32 * (TILE_H + TILE_GAP);
        tiles::draw_tile(t, x, y, tile_size, &ctx.colors);
    }
}

fn draw_human_melds(game: &Game, ctx: &GameContext) {
    let melds = &game.players[0].melds;
    if melds.is_empty() {
        return;
    }
    let tile_size = TileSize::new(TILE_W, TILE_H);
    let mut total_w = 0.0;
    for (i, m) in melds.iter().enumerate() {
        let count = match m {
            Meld::Peng { .. } => 3,
            Meld::Gang { .. } => 4,
        };
        total_w += count as f32 * TILE_W + (count - 1) as f32 * 1.0;
        if i + 1 < melds.len() { total_w += MELD_GROUP_GAP; }
    }
    let mut x = (scr_w() - total_w) * 0.5;
    for (i, m) in melds.iter().enumerate() {
        let (tile, count) = match m {
            Meld::Peng { tile, .. } => (*tile, 3),
            Meld::Gang { tile, .. } => (*tile, 4),
        };
        for _ in 0..count {
            tiles::draw_tile(tile, x, MELDS_Y, tile_size, &ctx.colors);
            x += TILE_W + 1.0;
        }
        if i + 1 < melds.len() { x += MELD_GROUP_GAP - 1.0; }
    }
}

fn draw_human_river(game: &Game, ctx: &GameContext) {
    let tiles_slice = &game.players[0].discards;
    if tiles_slice.is_empty() { return; }
    let block_w = river_block_width(tiles_slice.len(), HUMAN_RIVER_COLS, RIVER_ROWS_MAX);
    let x_left = (scr_w() - block_w) * 0.5;
    draw_river_block(
        tiles_slice,
        x_left,
        HUMAN_RIVER_Y,
        HUMAN_RIVER_COLS,
        RIVER_ROWS_MAX,
        ctx,
    );
}

fn draw_human_hand(game: &Game, ctx: &GameContext, mouse: Vec2) {
    let discardable = matches!(game.phase, Phase::AwaitingDiscard { player: 0 });
    let (drawn_origin, concealed_start) = human_hand_layout(game);
    let tile_size = TileSize::new(TILE_W, TILE_H);

    for (i, tile) in game.players[0].concealed.iter().enumerate() {
        let x = concealed_start + i as f32 * (TILE_W + TILE_GAP);
        let hovered = discardable && rect_contains(x, HAND_Y, TILE_W, TILE_H, mouse);
        if hovered {
            tiles::draw_tile_highlighted(*tile, x, HAND_Y, tile_size, &ctx.colors);
        } else {
            tiles::draw_tile(*tile, x, HAND_Y, tile_size, &ctx.colors);
        }
    }

    if let (Some(drawn), Some(o)) = (game.players[0].drawn, drawn_origin) {
        let hovered = discardable && rect_contains(o.x, o.y, TILE_W, TILE_H, mouse);
        if hovered {
            tiles::draw_tile_highlighted(drawn, o.x, o.y, tile_size, &ctx.colors);
        } else {
            tiles::draw_tile(drawn, o.x, o.y, tile_size, &ctx.colors);
        }
        let mx = o.x + TILE_W * 0.5;
        let my = o.y - 8.0;
        draw_triangle(
            Vec2::new(mx - 7.0, my - 7.0),
            Vec2::new(mx + 7.0, my - 7.0),
            Vec2::new(mx, my),
            ctx.colors.highlight,
        );
    }
}

fn waiting_tiles(concealed: &[Tile], melds: &[Meld]) -> Vec<Tile> {
    let mut waits = Vec::new();
    for suit in Suit::ALL {
        for rank in 1..=9 {
            let t = Tile::new(suit, rank);
            let mut test = concealed.to_vec();
            test.push(t);
            test.sort();
            if hand::is_winning_hand(&test, melds) {
                waits.push(t);
            }
        }
    }
    waits
}

fn draw_claim_buttons(game: &Game, ctx: &GameContext, mouse: Vec2) {
    for (action, (x, y, w, h)) in claim_button_rects(game) {
        let hovered = rect_contains(x, y, w, h, mouse);
        let is_pass = matches!(action, ClaimAction::Pass);
        let bg = if hovered {
            if is_pass { ctx.colors.danger } else { ctx.colors.highlight }
        } else if is_pass {
            Color::new(0.38, 0.22, 0.22, 1.0)
        } else {
            ctx.colors.tile_face
        };
        draw_rectangle(x, y, w, h, bg);
        draw_rectangle_lines(x, y, w, h, 3.0, ctx.colors.tile_text);
        let label = match action {
            ClaimAction::Claim(ClaimKind::Peng) => "PENG",
            ClaimAction::Claim(ClaimKind::Gang) => "GANG",
            ClaimAction::Claim(ClaimKind::Hu) => "HU!",
            ClaimAction::Pass => "PASS",
        };
        let dim = measure_text(label, None, 28, 1.0);
        draw_text(
            label,
            x + (w - dim.width) * 0.5,
            y + h * 0.5 + 10.0,
            28.0,
            ctx.colors.tile_text,
        );
    }
}

fn draw_own_turn_buttons(game: &Game, ctx: &GameContext, mouse: Vec2) {
    for (action, (x, y, w, h)) in own_turn_button_rects(game) {
        let hovered = rect_contains(x, y, w, h, mouse);
        let bg = if hovered { ctx.colors.highlight } else { ctx.colors.tile_face };
        draw_rectangle(x, y, w, h, bg);
        draw_rectangle_lines(x, y, w, h, 3.0, ctx.colors.tile_text);
        let label = match action {
            OwnAction::Zimo => "ZIMO!",
            OwnAction::AnGang(_) => "An Gang",
        };
        let dim = measure_text(label, None, 24, 1.0);
        draw_text(
            label,
            x + (w - dim.width) * 0.5,
            y + h * 0.5 + 8.0,
            24.0,
            ctx.colors.tile_text,
        );
    }
}

// ─── Text helpers ──────────────────────────────────────────────────────────

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