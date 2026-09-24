//! Tile rendering: traditional Mahjong drawings + CJK glyphs.
//!
//! Layout per tile:
//! - Top-left: Arabic numeral 1-9 (default font, red)
//! - Top-right: suit character 條 / 筒 (CJK font, suit color)
//! - Center: traditional drawing
//!   - WAN:  Chinese numeral (black) + 萬 (red) stacked
//!   - TIAO: 1 Tiao is a chicken (幺鸡, permanent Ji).
//!     2-9 Tiao are bamboo bars with joints.
//!   - TONG: blue/red coin dots (custom for 1 and 8)
//!
//! Standing tiles get a 3D parallelepiped effect with directional
//! shadows from a fixed top-left light source. A colored outline
//! ("glow") can be applied around a tile to flag its status
//! (useful / Ji) without touching the face.

use macroquad::prelude::*;

use crate::components::{Suit, Tile};
use crate::config::Colors;
use crate::font::TileFont;
use crate::layout::TileSize;

// ─── Palettes ────────────────────────────────────────────────────────────

const WAN_RED: Color = Color::new(0.78, 0.15, 0.15, 1.0);

const TIAO_GREEN: Color = Color::new(0.10, 0.48, 0.22, 1.0);
const TIAO_RED: Color = Color::new(0.78, 0.15, 0.15, 1.0);

const TONG_BLUE: Color = Color::new(0.18, 0.32, 0.68, 1.0);
const TONG_RED: Color = Color::new(0.78, 0.15, 0.15, 1.0);
const TONG_BLACK: Color = Color::new(0.10, 0.10, 0.10, 1.0);

const RANK_RED: Color = Color::new(0.78, 0.15, 0.15, 1.0);
const BLACK: Color = Color::new(0.10, 0.10, 0.10, 1.0);

// ─── Chicken palette (from reference image) ──────────────────────────────

const CHK_BODY: Color = Color::new(0.97, 0.94, 0.87, 1.0);
const CHK_OUTLINE: Color = Color::new(0.28, 0.12, 0.08, 1.0);
const CHK_RED: Color = Color::new(0.86, 0.16, 0.14, 1.0);
const CHK_RED_DARK: Color = Color::new(0.42, 0.05, 0.05, 1.0);
const CHK_ORANGE: Color = Color::new(0.96, 0.68, 0.16, 1.0);
const CHK_ORANGE_DARK: Color = Color::new(0.52, 0.28, 0.05, 1.0);
const CHK_BLACK: Color = Color::new(0.05, 0.05, 0.05, 1.0);
const CHK_WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
const CHK_GROUND: Color = Color::new(0.10, 0.28, 0.15, 0.45);

// ─── Tile glow ───────────────────────────────────────────────────────────
//
// A status indicator rendered as a colored outline around a tile.
// Priority between types is decided by the caller (main.rs); the
// renderer just draws whatever glow it is given.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileGlow {
    /// No indicator.
    None,
    /// Soft green — the tile is useful for the player's hand
    /// (forms a pair or a partial sequence with another tile).
    Useful,
    /// Warm gold — the tile is a Ji (permanent or active primary).
    Ji,
}

impl TileGlow {
    fn color(self) -> Option<Color> {
        match self {
            TileGlow::None => None,
            TileGlow::Useful => Some(Color::new(0.40, 0.95, 0.55, 0.90)),
            TileGlow::Ji => Some(Color::new(1.00, 0.82, 0.30, 0.98)),
        }
    }
}

// ─── Top glyph band metrics ──────────────────────────────────────────────

/// Font size used for the corner glyphs (numeral + CJK char).
fn corner_font_size(size: TileSize) -> f32 {
    (size.h * 0.20).max(9.0)
}

/// Padding from the tile's top edge to the visual top of the corner
/// glyphs.
fn corner_padding_top(fs: f32) -> f32 {
    fs * 0.30
}

/// Vertical offset from the tile top to the center of the corner
/// glyph band. Used by `main.rs` to place status indicators
/// consistently with the glyphs.
///
/// We avoid `measure_text` here because it requires the macroquad
/// game thread (panics in unit tests). The 0.40 factor approximates
/// the center of the cap-height band for typical Latin/CJK fonts.
pub fn glyph_band_center_offset(size: TileSize) -> f32 {
    let fs = corner_font_size(size);
    let pad = corner_padding_top(fs);
    pad + fs * 0.40
}

// ─── Flat tiles (rivers, miniatures) ─────────────────────────────────────

pub fn draw_tile_flat(
    tile: Tile,
    x: f32,
    y: f32,
    size: TileSize,
    colors: &Colors,
    font: &TileFont,
) {
    let s = shadow_offset();
    draw_rectangle(
        x + s.x, y + s.y,
        size.w, size.h,
        Color::new(0.0, 0.0, 0.0, 0.25),
    );
    draw_rectangle(x, y, size.w, size.h, colors.tile_face);
    draw_rectangle_lines(x, y, size.w, size.h, 1.5, colors.tile_edge);
    draw_tile_face(tile, x, y, size, font);
}

// ─── Standing tiles: split base + face + optional glow ───────────────────

pub fn draw_tile_standing_base(x: f32, y: f32, size: TileSize) {
    draw_standing_base(x, y, size);
}

pub fn draw_tile_standing_face(
    tile: Tile,
    x: f32,
    y: f32,
    size: TileSize,
    colors: &Colors,
    font: &TileFont,
) {
    draw_rectangle(x, y, size.w, size.h, colors.tile_face);
    draw_rectangle_lines(x, y, size.w, size.h, 1.5, colors.tile_edge);
    draw_tile_face(tile, x, y, size, font);
}

pub fn draw_tile_standing(
    tile: Tile,
    x: f32,
    y: f32,
    size: TileSize,
    colors: &Colors,
    font: &TileFont,
) {
    draw_tile_standing_base(x, y, size);
    draw_tile_standing_face(tile, x, y, size, colors, font);
}

/// Same as `draw_tile_standing`, but draws a colored outline around
/// the tile to flag its status (useful, Ji). The glow is applied
/// after the face so it sits on top of the tile's own edge, giving
/// the illusion of a soft halo without needing a blur pass.
pub fn draw_tile_standing_with_glow(
    tile: Tile,
    x: f32,
    y: f32,
    size: TileSize,
    glow: TileGlow,
    colors: &Colors,
    font: &TileFont,
) {
    draw_tile_standing_base(x, y, size);
    draw_tile_standing_face(tile, x, y, size, colors, font);
    if let Some(color) = glow.color() {
        draw_glow_outline(x, y, size, color);
    }
}

/// Two-layer outline: a wide, low-alpha ring for the soft halo, and
/// a tight, high-alpha ring for the bright core. Macroquad has no
/// native blur, so we fake it with layered strokes.
fn draw_glow_outline(x: f32, y: f32, size: TileSize, color: Color) {
    let soft = Color::new(color.r, color.g, color.b, color.a * 0.45);
    draw_rectangle_lines(
        x - 3.5, y - 3.5,
        size.w + 7.0, size.h + 7.0,
        5.0, soft,
    );
    draw_rectangle_lines(
        x - 1.0, y - 1.0,
        size.w + 2.0, size.h + 2.0,
        1.8, color,
    );
}

pub fn draw_tile_standing_highlighted(
    tile: Tile,
    x: f32,
    y: f32,
    size: TileSize,
    colors: &Colors,
    font: &TileFont,
) {
    draw_standing_base(x, y, size);
    draw_rectangle(x, y, size.w, size.h, colors.tile_face);
    draw_rectangle_lines(x, y, size.w, size.h, 3.0, colors.highlight);
    draw_tile_face(tile, x, y, size, font);
}

pub fn draw_tile_back_standing(x: f32, y: f32, size: TileSize) {
    draw_standing_base(x, y, size);
    let fill = Color::new(0.55, 0.32, 0.20, 1.0);
    let edge = Color::new(0.35, 0.20, 0.12, 1.0);
    draw_rectangle(x, y, size.w, size.h, fill);
    draw_rectangle_lines(x, y, size.w, size.h, 1.5, edge);
    let inset = 4.0;
    draw_rectangle_lines(
        x + inset, y + inset,
        size.w - inset * 2.0, size.h - inset * 2.0,
        1.0, edge,
    );
}

fn draw_standing_base(x: f32, y: f32, size: TileSize) {
    let depth = (size.h * 0.10).clamp(4.0, 9.0);
    let skew = (size.w * 0.05).clamp(1.5, 3.5);

    let s = shadow_offset();
    draw_rectangle(
        x + s.x,
        y + depth + s.y,
        size.w,
        size.h + depth,
        Color::new(0.0, 0.0, 0.0, 0.28),
    );

    let depth_color = Color::new(0.42, 0.38, 0.30, 1.0);
    let depth_edge = Color::new(0.28, 0.24, 0.18, 1.0);

    let tl = Vec2::new(x, y + size.h);
    let tr = Vec2::new(x + size.w, y + size.h);
    let br = Vec2::new(x + size.w + skew, y + size.h + depth);
    let bl = Vec2::new(x - skew, y + size.h + depth);

    draw_triangle(tl, tr, br, depth_color);
    draw_triangle(tl, br, bl, depth_color);

    draw_line(bl.x, bl.y, br.x, br.y, 1.0, depth_edge);
    draw_line(tl.x, tl.y, bl.x, bl.y, 1.0, depth_edge);
    draw_line(tr.x, tr.y, br.x, br.y, 1.0, depth_edge);
}

fn shadow_offset() -> Vec2 {
    Vec2::new(5.0, 3.0)
}

// ─── Tile face ────────────────────────────────────────────────────────────

fn draw_tile_face(tile: Tile, x: f32, y: f32, size: TileSize, font: &TileFont) {
    draw_rank_corner(tile.rank, x, y, size);
    match tile.suit {
        Suit::Wan => draw_wan(tile.rank, x, y, size, font),
        Suit::Tiao => draw_tiao(tile.rank, x, y, size, font),
        Suit::Tong => draw_tong(tile.rank, x, y, size, font),
    }
}

fn draw_rank_corner(rank: u8, x: f32, y: f32, size: TileSize) {
    let text = rank.to_string();
    let fs = corner_font_size(size);
    let pad = corner_padding_top(fs);
    let dim = measure_text(&text, None, fs as u16, 1.0);
    let baseline = y + pad + dim.offset_y.abs();
    draw_text(&text, x + 4.0, baseline, fs, RANK_RED);
}

fn draw_suit_corner(suit: Suit, x: f32, y: f32, size: TileSize, font: &TileFont) {
    if !font.is_available() {
        return;
    }
    let (ch, color) = match suit {
        Suit::Tiao => ("條", TIAO_GREEN),
        Suit::Tong => ("筒", TONG_BLUE),
        Suit::Wan => return,
    };
    let fs = corner_font_size(size);
    let pad = corner_padding_top(fs);
    let dim = font.measure(ch, fs);
    let baseline = y + pad + dim.offset_y.abs();
    font.draw(ch, x + size.w - dim.width - 4.0, baseline, fs, color);
}

// ─── WAN ──────────────────────────────────────────────────────────────────

fn draw_wan(rank: u8, x: f32, y: f32, size: TileSize, font: &TileFont) {
    if !font.is_available() {
        let digit = rank.to_string();
        let big = (size.h * 0.42).max(12.0);
        let dim = measure_text(&digit, None, big as u16, 1.0);
        draw_text(
            &digit,
            x + (size.w - dim.width) * 0.5,
            y + size.h * 0.52,
            big,
            WAN_RED,
        );
        let bar_w = size.w * 0.55;
        let bar_h = (size.h * 0.13).max(3.5);
        let bar_x = x + (size.w - bar_w) * 0.5;
        let bar_y = y + size.h * 0.72;
        draw_rectangle(bar_x, bar_y, bar_w, bar_h, WAN_RED);
        return;
    }

    let numeral = chinese_numeral(rank);
    let big = size.h * 0.34;
    let small = size.h * 0.32;

    let top_y = y + size.h * 0.50;
    font.draw_centered(numeral, x + size.w * 0.5, top_y, big, BLACK);
    let bot_y = y + size.h * 0.86;
    font.draw_centered("萬", x + size.w * 0.5, bot_y, small, WAN_RED);
}

// ─── TIAO (bamboo + chicken for 1) ────────────────────────────────────────

fn draw_tiao(rank: u8, x: f32, y: f32, size: TileSize, font: &TileFont) {
    let pad_x = size.w * 0.12;
    let pad_y_top = size.h * 0.30;
    let pad_y_bot = size.h * 0.08;
    let pattern_w = size.w - pad_x * 2.0;
    let pattern_h = size.h - pad_y_top - pad_y_bot;
    let px = x + pad_x;
    let py = y + pad_y_top;

    if rank == 1 {
        draw_tiao_chicken(px, py, pattern_w, pattern_h);
    } else if rank == 8 {
        let cols: [f32; 2] = [0.35, 0.65];
        let rows: [f32; 4] = [0.12, 0.37, 0.62, 0.87];
        let bar_w = (pattern_w * 0.14).max(2.2);
        let bar_h = (pattern_h * 0.16).max(3.0);
        for (yi, &yf) in rows.iter().enumerate() {
            for (xi, &xf) in cols.iter().enumerate() {
                let color = if (yi + xi) % 2 == 0 { TIAO_GREEN } else { TIAO_RED };
                let cx = px + xf * pattern_w;
                let cy = py + yf * pattern_h;
                draw_bamboo_bar(cx, cy, bar_w, bar_h, color);
            }
        }
    } else {
        let positions = grid_positions(rank as usize);
        let cell_w = pattern_w / 3.0;
        let cell_h = pattern_h / 3.0;
        let bar_w = (cell_w * 0.38).max(2.5);
        let bar_h = (cell_h * 0.74).max(4.0);
        for (i, (col, row)) in positions.iter().enumerate() {
            let color = if i % 2 == 0 { TIAO_GREEN } else { TIAO_RED };
            let cx = px + *col as f32 * cell_w + cell_w * 0.5;
            let cy = py + *row as f32 * cell_h + cell_h * 0.5;
            draw_bamboo_bar(cx, cy, bar_w, bar_h, color);
        }
    }

    draw_suit_corner(Suit::Tiao, x, y, size, font);
}

/// Draws the 1 Tiao as a stylized chicken, facing right.
///
/// Matches the reference: cream body, red comb with 3 lobes, red
/// wattle, orange beak and legs, big cartoon eye, and a **fan of 3
/// pointed tail feathers** sweeping up-left from the back of the
/// body. There are no wings — the body is a smooth cream circle.
///
/// All coordinates are relative to the pattern box `(px, py, pw, ph)`
/// and scale with `s = min(pw, ph)`, so the bird stays proportional
/// on hand tiles (56×84), river tiles (30×46), and mini tiles (22×32).
fn draw_tiao_chicken(px: f32, py: f32, pw: f32, ph: f32) {
    let s = pw.min(ph);
    let outline_t = (s * 0.030).max(0.6);

    // ─── Geometry ────────────────────────────────────────────────
    let body_cx = px + pw * 0.56;
    let body_cy = py + ph * 0.62;
    let body_r = s * 0.28;

    let head_cx = px + pw * 0.63;
    let head_cy = py + ph * 0.32;
    let head_r = s * 0.17;

    let ground_y = py + ph * 0.96;

    // ─── 1. Ground shadow (behind everything) ────────────────────
    draw_ellipse(
        px + pw * 0.50, ground_y,
        pw * 0.36, s * 0.045,
        0.0, CHK_GROUND,
    );

    // ─── 2. Legs + feet ──────────────────────────────────────────
    let leg_top_y = body_cy + body_r * 0.72;
    let leg_bot_y = ground_y - s * 0.02;
    let leg_w = (s * 0.040).max(1.0);

    let leg1_x = body_cx - body_r * 0.30;
    let leg2_x = body_cx + body_r * 0.28;

    draw_line(leg1_x, leg_top_y, leg1_x, leg_bot_y, leg_w + outline_t, CHK_OUTLINE);
    draw_line(leg2_x, leg_top_y, leg2_x, leg_bot_y, leg_w + outline_t, CHK_OUTLINE);
    draw_line(leg1_x, leg_top_y, leg1_x, leg_bot_y, leg_w, CHK_ORANGE);
    draw_line(leg2_x, leg_top_y, leg2_x, leg_bot_y, leg_w, CHK_ORANGE);

    let toe_len = s * 0.075;
    for &leg_x in &[leg1_x, leg2_x] {
        draw_line(
            leg_x, leg_bot_y,
            leg_x + toe_len * 0.8, leg_bot_y + toe_len * 0.15,
            leg_w, CHK_ORANGE_DARK,
        );
        draw_line(
            leg_x, leg_bot_y,
            leg_x - toe_len * 0.8, leg_bot_y + toe_len * 0.15,
            leg_w, CHK_ORANGE_DARK,
        );
        draw_line(
            leg_x, leg_bot_y,
            leg_x, leg_bot_y + toe_len * 0.30,
            leg_w, CHK_ORANGE_DARK,
        );
    }

    // ─── 3. Tail feathers — fan of 3 pointed plumes up-left ──────
    let tail_base = Vec2::new(
        body_cx - body_r * 0.55,
        body_cy - body_r * 0.35,
    );
    let plume_len = s * 0.44;
    let plume_half_width = s * 0.045;
    let plume_angles_deg = [72.0_f32, 50.0, 28.0];
    for &deg in &plume_angles_deg {
        let rad = deg.to_radians();
        let tip = Vec2::new(
            tail_base.x - plume_len * rad.cos(),
            tail_base.y - plume_len * rad.sin(),
        );
        draw_tail_plume(
            tail_base,
            tip,
            plume_half_width,
            (CHK_BODY, CHK_OUTLINE),
            outline_t,
        );
    }

    // ─── 4. Body (with outline) ──────────────────────────────────
    outlined_circle(body_cx, body_cy, body_r, outline_t, CHK_BODY, CHK_OUTLINE);

    // ─── 5. Head (with outline) ──────────────────────────────────
    outlined_circle(head_cx, head_cy, head_r, outline_t, CHK_BODY, CHK_OUTLINE);

    // ─── 6. Comb — 3 red lobes on top of the head ────────────────
    let comb_lobes: [(f32, f32, f32); 3] = [
        (head_cx - head_r * 0.45, head_cy - head_r * 0.80, head_r * 0.32),
        (head_cx + head_r * 0.02, head_cy - head_r * 1.00, head_r * 0.38),
        (head_cx + head_r * 0.48, head_cy - head_r * 0.85, head_r * 0.30),
    ];
    for &(lx, ly, lr) in &comb_lobes {
        draw_circle(lx, ly, lr + outline_t * 0.45, CHK_RED_DARK);
        draw_circle(lx, ly, lr, CHK_RED);
    }

    // ─── 7. Beak — orange triangle pointing right ────────────────
    let beak_base_x = head_cx + head_r * 0.72;
    let beak_base_y_top = head_cy - head_r * 0.10;
    let beak_base_y_bot = head_cy + head_r * 0.32;
    let beak_tip_x = head_cx + head_r * 1.45;
    let beak_tip_y = head_cy + head_r * 0.12;

    let o = outline_t * 0.7;
    draw_triangle(
        Vec2::new(beak_base_x - o, beak_base_y_top - o),
        Vec2::new(beak_tip_x + o, beak_tip_y),
        Vec2::new(beak_base_x - o, beak_base_y_bot + o),
        CHK_ORANGE_DARK,
    );
    draw_triangle(
        Vec2::new(beak_base_x, beak_base_y_top),
        Vec2::new(beak_tip_x, beak_tip_y),
        Vec2::new(beak_base_x, beak_base_y_bot),
        CHK_ORANGE,
    );

    // ─── 8. Wattle — red teardrop below the beak ─────────────────
    let wattle_cx = head_cx + head_r * 0.68;
    let wattle_cy = head_cy + head_r * 0.68;
    let wattle_r = head_r * 0.42;
    draw_circle(wattle_cx, wattle_cy, wattle_r + outline_t * 0.45, CHK_RED_DARK);
    draw_circle(wattle_cx, wattle_cy, wattle_r, CHK_RED);

    // ─── 9. Eye — big cartoon eye (only on large enough tiles) ───
    if head_r >= 3.5 {
        let eye_cx = head_cx + head_r * 0.22;
        let eye_cy = head_cy - head_r * 0.18;
        let eye_r = head_r * 0.44;

        draw_circle(eye_cx, eye_cy, eye_r + outline_t * 0.35, CHK_OUTLINE);
        draw_circle(eye_cx, eye_cy, eye_r, CHK_WHITE);

        let pupil_cx = eye_cx + eye_r * 0.22;
        let pupil_cy = eye_cy + eye_r * 0.05;
        let pupil_r = eye_r * 0.58;
        draw_circle(pupil_cx, pupil_cy, pupil_r, CHK_BLACK);

        if eye_r >= 3.0 {
            draw_circle(
                pupil_cx - pupil_r * 0.35,
                pupil_cy - pupil_r * 0.40,
                pupil_r * 0.32,
                CHK_WHITE,
            );
        }
    }
}

/// Filled circle with a uniform outline of thickness `t`.
fn outlined_circle(cx: f32, cy: f32, r: f32, t: f32, fill: Color, stroke: Color) {
    if r <= 0.1 {
        return;
    }
    let outer = r + t * 0.5;
    let inner = (r - t * 0.5).max(0.05);
    draw_circle(cx, cy, outer, stroke);
    draw_circle(cx, cy, inner, fill);
}

/// Draws a single tail feather as a **pointed plume**: wide at the
/// base (on the body), tapering to a sharp tip.
fn draw_tail_plume(
    base: Vec2,
    tip: Vec2,
    base_half_width: f32,
    colors: (Color, Color),
    outline_t: f32,
) {
    let (fill, stroke) = colors;
    let delta = tip - base;
    let len = delta.length();
    if len < 0.1 {
        return;
    }
    let dir = delta / len;
    let perp = Vec2::new(-dir.y, dir.x);

    let base_l = base + perp * base_half_width;
    let base_r = base - perp * base_half_width;

    let base_l_o = base + perp * (base_half_width + outline_t);
    let base_r_o = base - perp * (base_half_width + outline_t);
    let tip_o = tip + dir * outline_t;

    draw_triangle(base_l_o, base_r_o, tip_o, stroke);
    draw_triangle(base_l, base_r, tip, fill);
}

/// Draws a bamboo stick: a filled bar with two subtle horizontal
/// joints when the bar is tall enough (bamboo is segmented).
fn draw_bamboo_bar(cx: f32, cy: f32, w: f32, h: f32, color: Color) {
    draw_rectangle(cx - w * 0.5, cy - h * 0.5, w, h, color);
    if h >= 10.0 {
        let joint = Color::new(0.0, 0.0, 0.0, 0.30);
        let half_w = w * 0.5;
        let y_top = cy - h * 0.18;
        let y_bot = cy + h * 0.18;
        draw_line(cx - half_w, y_top, cx + half_w, y_top, 1.0, joint);
        draw_line(cx - half_w, y_bot, cx + half_w, y_bot, 1.0, joint);
    }
}

// ─── TONG (coins) ─────────────────────────────────────────────────────────

fn draw_tong(rank: u8, x: f32, y: f32, size: TileSize, font: &TileFont) {
    let pad_x = size.w * 0.12;
    let pad_y_top = size.h * 0.30;
    let pad_y_bot = size.h * 0.08;
    let pattern_w = size.w - pad_x * 2.0;
    let pattern_h = size.h - pad_y_top - pad_y_bot;
    let px = x + pad_x;
    let py = y + pad_y_top;

    match rank {
        8 => {
            let cols: [f32; 2] = [0.32, 0.68];
            let rows: [f32; 4] = [0.12, 0.37, 0.62, 0.87];
            let r = (pattern_w * 0.16).min(pattern_h * 0.10).max(2.2);
            for &yf in &rows {
                for &xf in &cols {
                    let cx = px + xf * pattern_w;
                    let cy = py + yf * pattern_h;
                    draw_coin(cx, cy, r, TONG_BLACK);
                }
            }
        }
        1 => {
            let cell_w = pattern_w / 3.0;
            let cell_h = pattern_h / 3.0;
            let cx = px + cell_w * 1.5;
            let cy = py + cell_h * 1.5;
            let r_big = (cell_w.min(cell_h) * 0.46).max(3.5);
            draw_circle(cx, cy, r_big, TONG_BLUE);
            draw_circle(cx, cy, r_big * 0.62, colors_white());
            draw_circle(cx, cy, r_big * 0.30, TONG_RED);
        }
        _ => {
            let positions = grid_positions(rank as usize);
            let cell_w = pattern_w / 3.0;
            let cell_h = pattern_h / 3.0;
            let r = (cell_w.min(cell_h) * 0.34).max(2.5);
            for (col, row) in positions {
                let is_center = col == 1 && row == 1;
                let color = if is_center { TONG_RED } else { TONG_BLUE };
                let cx = px + col as f32 * cell_w + cell_w * 0.5;
                let cy = py + row as f32 * cell_h + cell_h * 0.5;
                draw_coin(cx, cy, r, color);
            }
        }
    }

    draw_suit_corner(Suit::Tong, x, y, size, font);
}

fn draw_coin(cx: f32, cy: f32, r: f32, color: Color) {
    draw_circle(cx, cy, r, color);
    if r >= 4.5 {
        let highlight = Color::new(1.0, 1.0, 1.0, 0.35);
        draw_circle_lines(cx, cy, r * 0.62, 1.0, highlight);
    }
}

fn colors_white() -> Color {
    Color::new(0.96, 0.94, 0.86, 1.0)
}

fn chinese_numeral(rank: u8) -> &'static str {
    match rank {
        1 => "一",
        2 => "二",
        3 => "三",
        4 => "四",
        5 => "五",
        6 => "六",
        7 => "七",
        8 => "八",
        9 => "九",
        _ => "?",
    }
}

fn grid_positions(n: usize) -> Vec<(usize, usize)> {
    match n {
        1 => vec![(1, 1)],
        2 => vec![(1, 0), (1, 2)],
        3 => vec![(1, 0), (1, 1), (1, 2)],
        4 => vec![(0, 0), (2, 0), (0, 2), (2, 2)],
        5 => vec![(0, 0), (2, 0), (1, 1), (0, 2), (2, 2)],
        6 => vec![(0, 0), (1, 0), (2, 0), (0, 2), (1, 2), (2, 2)],
        7 => vec![(1, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        8 => vec![
            (0, 0), (1, 0), (2, 0),
            (0, 1),         (2, 1),
            (0, 2), (1, 2), (2, 2),
        ],
        9 => vec![
            (0, 0), (1, 0), (2, 0),
            (0, 1), (1, 1), (2, 1),
            (0, 2), (1, 2), (2, 2),
        ],
        _ => vec![],
    }
}

// ─── Wall ─────────────────────────────────────────────────────────────────

pub fn draw_wall(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    count: usize,
    colors: &Colors,
) {
    const TILES: usize = 4;
    let gap = 3.0;
    let tile_h = h;
    let tile_w = tile_h * (30.0 / 46.0);
    let tiles_w = TILES as f32 * tile_w + (TILES - 1) as f32 * gap;
    let text_w = (w - tiles_w - 10.0).max(30.0);

    let text_cx = x + text_w * 0.5;

    let label = "WALL";
    let dim = measure_text(label, None, 12, 1.0);
    draw_text(
        label,
        text_cx - dim.width * 0.5,
        y + h * 0.42,
        12.0,
        colors.wall_gold,
    );

    let count_str = format!("{count}");
    let dim = measure_text(&count_str, None, 26, 1.0);
    let cx = text_cx - dim.width * 0.5;
    let cy = y + h * 0.94;
    draw_text(
        &count_str,
        cx + 1.5,
        cy + 1.5,
        26.0,
        Color::new(0.0, 0.0, 0.0, 0.65),
    );
    draw_text(&count_str, cx, cy, 26.0, colors.tile_face);

    let tiles_x = x + text_w + 10.0;
    let size = TileSize::new(tile_w, tile_h);
    for i in 0..TILES {
        let tx = tiles_x + i as f32 * (tile_w + gap);
        draw_tile_back_standing(tx, y, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_positions_counts() {
        for n in 1..=9 {
            assert_eq!(grid_positions(n).len(), n, "n = {n}");
        }
    }

    #[test]
    fn test_grid_positions_in_bounds() {
        for n in 1..=9 {
            for (col, row) in grid_positions(n) {
                assert!(col < 3);
                assert!(row < 3);
            }
        }
    }

    #[test]
    fn test_chinese_numerals_cover_all_ranks() {
        for r in 1..=9 {
            assert_ne!(chinese_numeral(r), "?");
        }
    }

    #[test]
    fn test_tong_primary_is_blue() {
        const _: () = assert!(
            TONG_BLUE.b > TONG_BLUE.g,
            "Tong primary should be blue-dominant",
        );
    }

    #[test]
    fn test_tiao_primary_is_green() {
        const _: () = assert!(
            TIAO_GREEN.g > TIAO_GREEN.b,
            "Tiao primary should be green-dominant",
        );
    }

    #[test]
    fn test_tong_and_tiao_palettes_are_distinct() {
        let gap = (TONG_BLUE.b - TIAO_GREEN.b).abs();
        assert!(
            gap > 0.2,
            "Tong and Tiao primaries should differ enough (gap={})",
            gap,
        );
    }

    #[test]
    fn test_glow_colors_are_distinct() {
        let useful = TileGlow::Useful.color().unwrap();
        let ji = TileGlow::Ji.color().unwrap();
        // Useful should be green-dominant, Ji should be red/orange-dominant.
        assert!(useful.g > useful.r);
        assert!(ji.r > ji.b);
        // Both should be at least somewhat opaque.
        assert!(useful.a > 0.5);
        assert!(ji.a > 0.5);
    }

    #[test]
    fn test_glow_none_has_no_color() {
        assert!(TileGlow::None.color().is_none());
    }

    #[test]
    fn test_chicken_geometry_fits_pattern_box() {
        for (tw, th) in [
            (56.0_f32, 84.0_f32),
            (30.0, 46.0),
            (22.0, 32.0),
        ] {
            let pw = tw * 0.76;
            let ph = th * 0.62;
            let s = pw.min(ph);
            let outline_t = (s * 0.030).max(0.6);

            let body_cx = pw * 0.56;
            let body_r = s * 0.28;
            let head_cx = pw * 0.63;
            let head_cy = ph * 0.32;
            let head_r = s * 0.17;

            let comb_top = head_cy - head_r * 1.00 - head_r * 0.38 - outline_t * 0.45;
            let beak_tip = head_cx + head_r * 1.45 + outline_t * 0.7;
            let wattle_right = head_cx + head_r * 0.68 + head_r * 0.42 + outline_t * 0.45;
            let head_right = head_cx + head_r + outline_t * 0.5;
            let body_right = body_cx + body_r + outline_t * 0.5;
            let ground_y = ph * 0.96;

            assert!(
                comb_top >= -1.0,
                "comb top out of box at {tw}x{th}: {}",
                comb_top,
            );
            assert!(
                beak_tip <= pw + 1.0,
                "beak overflows at {tw}x{th}: {} > {}",
                beak_tip, pw,
            );
            assert!(
                wattle_right <= pw + 1.0,
                "wattle overflows at {tw}x{th}: {} > {}",
                wattle_right, pw,
            );
            assert!(
                head_right <= pw + 1.0,
                "head overflows at {tw}x{th}",
            );
            assert!(
                body_right <= pw + 1.0,
                "body overflows at {tw}x{th}",
            );
            assert!(
                ground_y <= ph + 1.0,
                "ground overflows at {tw}x{th}",
            );
        }
    }

    #[test]
    fn test_chicken_tail_fits_pattern_box() {
        for (tw, th) in [
            (56.0_f32, 84.0_f32),
            (30.0, 46.0),
            (22.0, 32.0),
        ] {
            let pw = tw * 0.76;
            let ph = th * 0.62;
            let s = pw.min(ph);

            let body_cx = pw * 0.56;
            let body_cy = ph * 0.62;
            let body_r = s * 0.28;

            let tail_base = Vec2::new(
                body_cx - body_r * 0.55,
                body_cy - body_r * 0.35,
            );
            let plume_len = s * 0.44;
            for &deg in &[72.0_f32, 50.0, 28.0] {
                let rad = deg.to_radians();
                let tip = Vec2::new(
                    tail_base.x - plume_len * rad.cos(),
                    tail_base.y - plume_len * rad.sin(),
                );
                assert!(
                    tip.x >= -1.0,
                    "tail tip overflows left at {tw}x{th} ({deg}°): x={}",
                    tip.x,
                );
                assert!(
                    tip.y >= -1.0,
                    "tail tip overflows top at {tw}x{th} ({deg}°): y={}",
                    tip.y,
                );
            }
        }
    }

    #[test]
    fn test_glyph_band_offset_is_positive_and_small() {
        for (tw, th) in [(56.0_f32, 84.0_f32), (30.0, 46.0), (22.0, 32.0)] {
            let size = TileSize::new(tw, th);
            let off = glyph_band_center_offset(size);
            assert!(off > 0.0, "offset must be positive at {tw}x{th}");
            assert!(
                off < th * 0.30,
                "offset {} must be < {} at {tw}x{th}",
                off, th * 0.30,
            );
        }
    }
}