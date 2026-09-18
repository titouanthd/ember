//! Tile rendering primitives.
//!
//! Visual conventions follow classic Mahjong sets:
//! - Every tile has a small red rank digit in its top-left corner.
//! - WAN: black digit at top-center, red element at bottom-center
//!   (proxy for the black Chinese numeral + red 万 character, which we
//!   cannot draw without a CJK font).
//! - TIAO: vertical bars in green/red alternation.
//! - TONG: circles in green/red alternation; the 8 is all black.
//!
//! CJK limitation: we render Arabic numerals and abstract shapes
//! instead of the real characters 万 / 一 / 二 / ... because the
//! workspace's DejaVu Sans Mono font doesn't cover them. The color
//! placement (black top, red bottom) preserves the visual convention.

use macroquad::prelude::*;

use crate::components::{Suit, Tile};
use crate::config::Colors;

const RED: Color = Color::new(0.78, 0.15, 0.15, 1.0);
const GREEN: Color = Color::new(0.10, 0.48, 0.22, 1.0);
const BLACK: Color = Color::new(0.10, 0.10, 0.10, 1.0);

#[derive(Debug, Clone, Copy)]
pub struct TileSize {
    pub w: f32,
    pub h: f32,
}

impl TileSize {
    pub const fn new(w: f32, h: f32) -> Self {
        Self { w, h }
    }
}

pub fn draw_tile(tile: Tile, x: f32, y: f32, size: TileSize, colors: &Colors) {
    draw_rectangle(x, y, size.w, size.h, colors.tile_face);
    draw_rectangle_lines(x, y, size.w, size.h, 1.5, colors.tile_edge);
    draw_tile_face(tile, x, y, size);
}

pub fn draw_tile_highlighted(tile: Tile, x: f32, y: f32, size: TileSize, colors: &Colors) {
    draw_rectangle(x, y, size.w, size.h, colors.tile_face);
    draw_rectangle_lines(x, y, size.w, size.h, 3.0, colors.highlight);
    draw_tile_face(tile, x, y, size);
}

pub fn draw_tile_face(tile: Tile, x: f32, y: f32, size: TileSize) {
    draw_rank_corner(tile.rank, x, y, size);
    match tile.suit {
        Suit::Wan => draw_wan(tile.rank, x, y, size),
        Suit::Tiao => draw_tiao(tile.rank, x, y, size),
        Suit::Tong => draw_tong(tile.rank, x, y, size),
    }
}

/// Small red rank digit in the top-left corner, common to every tile.
fn draw_rank_corner(rank: u8, x: f32, y: f32, size: TileSize) {
    let text = rank.to_string();
    let fs = (size.h * 0.20).max(9.0) as u16;
    draw_text(&text, x + 4.0, y + fs as f32 + 2.0, fs as f32, RED);
}

fn draw_wan(rank: u8, x: f32, y: f32, size: TileSize) {
    // Black digit at top-center (proxy for the Chinese numeral).
    let digit = rank.to_string();
    let digit_size = (size.h * 0.38).max(12.0) as u16;
    let dim = measure_text(&digit, None, digit_size, 1.0);
    let cx = x + (size.w - dim.width) * 0.5;
    let cy = y + size.h * 0.42;
    draw_text(&digit, cx, cy, digit_size as f32, BLACK);

    // Red element at bottom-center (proxy for 万).
    let bar_h = (size.h * 0.13).max(3.5);
    let bar_w = size.w * 0.55;
    let bar_x = x + (size.w - bar_w) * 0.5;
    let bar_y = y + size.h * 0.62;
    draw_rectangle(bar_x, bar_y, bar_w, bar_h, RED);

    // Small horizontal accent above the bar to hint at 万's strokes.
    let accent_w = size.w * 0.32;
    let accent_h = (size.h * 0.04).max(1.5);
    let ax = x + (size.w - accent_w) * 0.5;
    let ay = bar_y - accent_h - 2.0;
    draw_rectangle(ax, ay, accent_w, accent_h, RED);
}

fn draw_tiao(rank: u8, x: f32, y: f32, size: TileSize) {
    let positions = grid_positions(rank as usize);
    let pad_x = size.w * 0.10;
    let pad_y_top = size.h * 0.28;
    let pad_y_bot = size.h * 0.08;
    let pattern_w = size.w - pad_x * 2.0;
    let pattern_h = size.h - pad_y_top - pad_y_bot;
    let px = x + pad_x;
    let py = y + pad_y_top;
    let cell_w = pattern_w / 3.0;
    let cell_h = pattern_h / 3.0;
    let bar_w = (cell_w * 0.34).max(2.5);
    let bar_h = (cell_h * 0.70).max(4.0);

    for (i, (col, row)) in positions.iter().enumerate() {
        // Alternate green/red by position index within the pattern.
        let color = if i % 2 == 0 { GREEN } else { RED };
        let cx = px + *col as f32 * cell_w + cell_w * 0.5;
        let cy = py + *row as f32 * cell_h + cell_h * 0.5;
        draw_rectangle(cx - bar_w * 0.5, cy - bar_h * 0.5, bar_w, bar_h, color);
    }
}

fn draw_tong(rank: u8, x: f32, y: f32, size: TileSize) {
    let positions = grid_positions(rank as usize);
    let pad_x = size.w * 0.10;
    let pad_y_top = size.h * 0.28;
    let pad_y_bot = size.h * 0.08;
    let pattern_w = size.w - pad_x * 2.0;
    let pattern_h = size.h - pad_y_top - pad_y_bot;
    let px = x + pad_x;
    let py = y + pad_y_top;
    let cell_w = pattern_w / 3.0;
    let cell_h = pattern_h / 3.0;
    let r = (cell_w.min(cell_h) * 0.30).max(2.0);

    // The 8-tong is traditionally an all-black design.
    let all_black = rank == 8;

    for (i, (col, row)) in positions.iter().enumerate() {
        let color = if all_black {
            BLACK
        } else if i % 2 == 0 {
            GREEN
        } else {
            RED
        };
        let cx = px + *col as f32 * cell_w + cell_w * 0.5;
        let cy = py + *row as f32 * cell_h + cell_h * 0.5;

        if rank == 1 {
            // 1-tong: one large circle with a concentric ring
            // (contrasts with the plain dots of 2-9).
            let r_big = (cell_w.min(cell_h) * 0.42).max(3.0);
            draw_circle(cx, cy, r_big, GREEN);
            draw_circle(cx, cy, r_big * 0.55, colors_white());
            draw_circle(cx, cy, r_big * 0.30, RED);
        } else {
            draw_circle(cx, cy, r, color);
        }
    }
}

fn colors_white() -> Color {
    Color::new(0.94, 0.93, 0.86, 1.0) // matches tile_face
}

fn grid_positions(n: usize) -> Vec<(usize, usize)> {
    match n {
        1 => vec![(1, 1)],
        2 => vec![(1, 0), (1, 2)],
        3 => vec![(1, 0), (1, 1), (1, 2)],
        4 => vec![(0, 0), (2, 0), (0, 2), (2, 2)],
        5 => vec![(0, 0), (2, 0), (1, 1), (0, 2), (2, 2)],
        6 => vec![(0, 0), (1, 0), (2, 0), (0, 2), (1, 2), (2, 2)],
        7 => vec![(0, 0), (1, 0), (2, 0), (0, 2), (1, 2), (2, 2), (1, 1)],
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

/// Draw the wall as a stacked pile with the count inside.
/// The pile offset scales with the wall width so a small wall doesn't
/// look chunky.
pub fn draw_wall(x: f32, y: f32, w: f32, h: f32, count: usize, colors: &Colors) {
    let layers = 3;
    let offset = (w * 0.055).max(3.0);
    for i in (1..=layers).rev() {
        let ox = x - i as f32 * offset;
        let oy = y - i as f32 * offset;
        let t = i as f32 / layers as f32;
        let c = Color::new(
            0.55 - 0.10 * t,
            0.48 - 0.08 * t,
            0.38 - 0.06 * t,
            1.0,
        );
        draw_rectangle(ox, oy, w, h, c);
        draw_rectangle_lines(ox, oy, w, h, 1.0, Color::new(0.32, 0.26, 0.20, 1.0));
    }

    draw_rectangle(x, y, w, h, colors.tile_face);
    draw_rectangle_lines(x, y, w, h, 2.0, colors.tile_edge);

    let label = "WALL";
    let dim = measure_text(label, None, 12, 1.0);
    draw_text(
        label,
        x + (w - dim.width) * 0.5,
        y + 14.0,
        12.0,
        colors.text_dim,
    );

    let text = count.to_string();
    let dim = measure_text(&text, None, 22, 1.0);
    draw_text(
        &text,
        x + (w - dim.width) * 0.5,
        y + h * 0.5 + dim.height * 0.35 + 4.0,
        22.0,
        colors.tile_text,
    );
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
}