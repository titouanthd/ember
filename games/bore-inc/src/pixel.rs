//! Tiny procedural pixel canvas. Sprites are drawn at startup into
//! `Canvas` buffers, then packed into a single texture atlas.
//!
//! **No files on disk.** Every visual is code.
//!
//! Coordinates are `(x, y)` with `(0, 0)` at the top-left. All drawing
//! primitives are bounds-checked and never panic.

use std::collections::HashMap;

/// An RGBA8 pixel.
pub type Rgba = [u8; 4];

/// Fully transparent — the "skip" value used by `overlay`.
pub const TRANSPARENT: Rgba = [0, 0, 0, 0];

/// An RGBA8 canvas. Row-major, top-left origin.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    pub fn byte_len(&self) -> usize {
        self.pixels.len()
    }

    /// True if every pixel is fully transparent (alpha == 0),
    /// regardless of its RGB values.
    ///
    /// Sheets commonly use `[255, 255, 255, 0]` (white with alpha 0)
    /// for "empty" cells. Those bytes are not all zero, but the cell
    /// is visually empty — checking alpha only is the correct
    /// semantic for "has no visible content".
    pub fn is_empty(&self) -> bool {
        self.pixels.chunks_exact(4).all(|p| p[3] == 0)
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some((y as usize * self.width as usize + x as usize) * 4)
    }

    pub fn set(&mut self, x: i32, y: i32, color: Rgba) {
        if let Some(i) = self.idx(x, y) {
            self.pixels[i..i + 4].copy_from_slice(&color);
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Rgba> {
        let i = self.idx(x, y)?;
        Some([
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ])
    }

    /// Set only if the target is currently fully transparent. Useful
    /// for stacking outlines without erasing detail.
    pub fn set_if_empty(&mut self, x: i32, y: i32, color: Rgba) {
        if let Some(i) = self.idx(x, y)
            && self.pixels[i + 3] == 0
        {
            self.pixels[i..i + 4].copy_from_slice(&color);
        }
    }

    pub fn fill(&mut self, color: Rgba) {
        for px in self.pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&color);
        }
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, color: Rgba) {
        let (a, b) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        for x in a..=b {
            self.set(x, y, color);
        }
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, color: Rgba) {
        let (a, b) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        for y in a..=b {
            self.set(x, y, color);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba) {
        if w <= 0 || h <= 0 {
            return;
        }
        for dy in 0..h {
            self.hline(x, x + w - 1, y + dy, color);
        }
    }

    pub fn rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba) {
        if w <= 0 || h <= 0 {
            return;
        }
        self.hline(x, x + w - 1, y, color);
        self.hline(x, x + w - 1, y + h - 1, color);
        self.vline(x, y, y + h - 1, color);
        self.vline(x + w - 1, y, y + h - 1, color);
    }

    /// Bresenham line.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgba) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.set(x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                if x == x1 {
                    break;
                }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y1 {
                    break;
                }
                err += dx;
                y += sy;
            }
        }
    }

    /// Filled ellipse via per-row scanlines.
    pub fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, color: Rgba) {
        if rx < 0 || ry < 0 {
            return;
        }
        if ry == 0 {
            self.hline(cx - rx, cx + rx, cy, color);
            return;
        }
        if rx == 0 {
            self.vline(cx, cy - ry, cy + ry, color);
            return;
        }
        let rxf = rx as f32;
        let ryf = ry as f32;
        for dy in -ry..=ry {
            let t = 1.0 - (dy as f32 / ryf).powi(2);
            if t < 0.0 {
                continue;
            }
            let half_w = (rxf * t.sqrt()).round() as i32;
            self.hline(cx - half_w, cx + half_w, cy + dy, color);
        }
    }

    /// Horizontal mirror.
    pub fn mirrored_x(&self) -> Self {
        let mut out = Self::new(self.width, self.height);
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                if let Some(c) = self.get(x, y) {
                    out.set(self.width as i32 - 1 - x, y, c);
                }
            }
        }
        out
    }

    /// Blit `other` into `self` at `(dx, dy)`, skipping transparent
    /// source pixels.
    pub fn overlay(&mut self, other: &Canvas, dx: i32, dy: i32) {
        for oy in 0..other.height as i32 {
            for ox in 0..other.width as i32 {
                if let Some(c) = other.get(ox, oy)
                    && c[3] > 0
                {
                    self.set(dx + ox, dy + oy, c);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ASCII sprite parser
// ---------------------------------------------------------------------------

/// A palette mapping single ASCII characters to RGBA colors.
///
/// Characters not in the map are treated as transparent, so you can
/// use `.` or any other filler. Whitespace is stripped by `from_ascii`
/// so the source art can stay visually aligned in the text file.
pub struct Palette {
    map: HashMap<char, Rgba>,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn with(mut self, ch: char, color: Rgba) -> Self {
        self.map.insert(ch, color);
        self
    }

    pub fn get(&self, ch: char) -> Option<Rgba> {
        self.map.get(&ch).copied()
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a multi-line ASCII string into a Canvas.
///
/// Every non-whitespace character is looked up in `palette`. Characters
/// not found are transparent. Rows are separated by `\n`; leading and
/// trailing whitespace is ignored.
///
/// The canvas dimensions are derived from the longest line × the number
/// of lines.
pub fn from_ascii(art: &str, palette: &Palette) -> Canvas {
    let rows: Vec<Vec<char>> = art
        .lines()
        .map(|line| line.chars().filter(|c| !c.is_whitespace()).collect())
        .filter(|row: &Vec<char>| !row.is_empty())
        .collect();

    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0).max(1) as u16;
    let height = rows.len().max(1) as u16;
    let mut c = Canvas::new(width, height);

    for (y, row) in rows.iter().enumerate() {
        for (x, &ch) in row.iter().enumerate() {
            if let Some(color) = palette.get(ch) {
                c.set(x as i32, y as i32, color);
            }
        }
    }
    c
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgba = [255, 0, 0, 255];
    const GREEN: Rgba = [0, 255, 0, 255];

    #[test]
    fn new_canvas_is_all_transparent() {
        let c = Canvas::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(c.get(x, y), Some([0, 0, 0, 0]));
            }
        }
        assert!(c.is_empty());
    }

    #[test]
    fn is_empty_ignores_rgb_when_alpha_is_zero() {
        let mut c = Canvas::new(2, 2);
        // White with alpha 0 — visually empty.
        c.set(0, 0, [255, 255, 255, 0]);
        assert!(c.is_empty());
    }

    #[test]
    fn is_empty_false_when_any_alpha_nonzero() {
        let mut c = Canvas::new(2, 2);
        c.set(1, 1, [0, 0, 0, 1]);
        assert!(!c.is_empty());
    }

    #[test]
    fn byte_len_matches_dimensions() {
        let c = Canvas::new(8, 16);
        assert_eq!(c.byte_len(), 8 * 16 * 4);
    }

    #[test]
    fn set_stores_rgba() {
        let mut c = Canvas::new(4, 4);
        c.set(1, 2, RED);
        assert_eq!(c.get(1, 2), Some(RED));
        assert!(!c.is_empty());
    }

    #[test]
    fn set_out_of_bounds_is_noop() {
        let mut c = Canvas::new(4, 4);
        c.set(-1, 0, RED);
        c.set(0, -1, RED);
        c.set(4, 0, RED);
        c.set(0, 4, RED);
        assert!(c.is_empty());
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let c = Canvas::new(4, 4);
        assert_eq!(c.get(-1, 0), None);
        assert_eq!(c.get(4, 0), None);
        assert_eq!(c.get(0, 4), None);
        assert_eq!(c.get(100, 100), None);
    }

    #[test]
    fn set_if_empty_skips_filled_pixels() {
        let mut c = Canvas::new(4, 4);
        c.set(1, 1, RED);
        c.set_if_empty(1, 1, GREEN);
        assert_eq!(c.get(1, 1), Some(RED));
    }

    #[test]
    fn set_if_empty_fills_transparent() {
        let mut c = Canvas::new(4, 4);
        c.set_if_empty(1, 1, GREEN);
        assert_eq!(c.get(1, 1), Some(GREEN));
    }

    #[test]
    fn fill_and_clear() {
        let mut c = Canvas::new(4, 4);
        c.fill(RED);
        assert_eq!(c.get(0, 0), Some(RED));
        assert_eq!(c.get(3, 3), Some(RED));
        c.clear();
        assert!(c.is_empty());
    }

    #[test]
    fn hline_draws_horizontally() {
        let mut c = Canvas::new(8, 4);
        c.hline(1, 5, 2, RED);
        for x in 1..=5 {
            assert_eq!(c.get(x, 2), Some(RED));
        }
        assert_eq!(c.get(0, 2), Some([0, 0, 0, 0]));
        assert_eq!(c.get(6, 2), Some([0, 0, 0, 0]));
    }

    #[test]
    fn hline_handles_reversed_args() {
        let mut c = Canvas::new(8, 4);
        c.hline(5, 1, 2, RED);
        for x in 1..=5 {
            assert_eq!(c.get(x, 2), Some(RED));
        }
    }

    #[test]
    fn vline_draws_vertically() {
        let mut c = Canvas::new(4, 8);
        c.vline(2, 1, 5, RED);
        for y in 1..=5 {
            assert_eq!(c.get(2, y), Some(RED));
        }
    }

    #[test]
    fn rect_fills_area() {
        let mut c = Canvas::new(8, 8);
        c.rect(2, 2, 4, 3, RED);
        for y in 2..5 {
            for x in 2..6 {
                assert_eq!(c.get(x, y), Some(RED));
            }
        }
        assert_eq!(c.get(1, 2), Some([0, 0, 0, 0]));
        assert_eq!(c.get(6, 2), Some([0, 0, 0, 0]));
    }

    #[test]
    fn rect_zero_or_negative_is_noop() {
        let mut c = Canvas::new(4, 4);
        c.rect(0, 0, 0, 4, RED);
        c.rect(0, 0, 4, 0, RED);
        c.rect(0, 0, -1, 4, RED);
        assert!(c.is_empty());
    }

    #[test]
    fn rect_outline_draws_only_border() {
        let mut c = Canvas::new(8, 8);
        c.rect_outline(2, 2, 4, 4, RED);
        assert_eq!(c.get(2, 2), Some(RED));
        assert_eq!(c.get(5, 2), Some(RED));
        assert_eq!(c.get(2, 5), Some(RED));
        assert_eq!(c.get(5, 5), Some(RED));
        assert_eq!(c.get(3, 3), Some([0, 0, 0, 0]));
        assert_eq!(c.get(4, 4), Some([0, 0, 0, 0]));
    }

    #[test]
    fn line_is_diagonal() {
        let mut c = Canvas::new(8, 8);
        c.line(0, 0, 4, 4, RED);
        for i in 0..=4 {
            assert_eq!(c.get(i, i), Some(RED));
        }
    }

    #[test]
    fn line_horizontal() {
        let mut c = Canvas::new(8, 8);
        c.line(1, 3, 5, 3, RED);
        for x in 1..=5 {
            assert_eq!(c.get(x, 3), Some(RED));
        }
    }

    #[test]
    fn line_single_point() {
        let mut c = Canvas::new(4, 4);
        c.line(2, 2, 2, 2, RED);
        assert_eq!(c.get(2, 2), Some(RED));
    }

    #[test]
    fn ellipse_fills_center() {
        let mut c = Canvas::new(16, 16);
        c.ellipse(8, 8, 4, 4, RED);
        assert_eq!(c.get(8, 8), Some(RED));
        assert_eq!(c.get(7, 7), Some(RED));
        assert_eq!(c.get(0, 0), Some([0, 0, 0, 0]));
        assert_eq!(c.get(15, 15), Some([0, 0, 0, 0]));
    }

    #[test]
    fn ellipse_touches_extremes() {
        let mut c = Canvas::new(20, 20);
        c.ellipse(10, 10, 8, 4, RED);
        assert_eq!(c.get(2, 10), Some(RED));
        assert_eq!(c.get(18, 10), Some(RED));
        assert_eq!(c.get(10, 6), Some(RED));
        assert_eq!(c.get(10, 14), Some(RED));
    }

    #[test]
    fn ellipse_ry_zero_is_a_horizontal_line() {
        let mut c = Canvas::new(8, 8);
        c.ellipse(4, 4, 3, 0, RED);
        for x in 1..=7 {
            assert_eq!(c.get(x, 4), Some(RED));
        }
        assert_eq!(c.get(4, 3), Some([0, 0, 0, 0]));
    }

    #[test]
    fn ellipse_rx_zero_is_a_vertical_line() {
        let mut c = Canvas::new(8, 8);
        c.ellipse(4, 4, 0, 3, RED);
        for y in 1..=7 {
            assert_eq!(c.get(4, y), Some(RED));
        }
    }

    #[test]
    fn mirrored_x_flips_horizontally() {
        let mut c = Canvas::new(4, 4);
        c.set(0, 0, RED);
        c.set(1, 0, GREEN);
        let m = c.mirrored_x();
        assert_eq!(m.get(3, 0), Some(RED));
        assert_eq!(m.get(2, 0), Some(GREEN));
        assert_eq!(m.get(0, 0), Some([0, 0, 0, 0]));
    }

    #[test]
    fn overlay_skips_transparent_source() {
        let mut base = Canvas::new(4, 4);
        base.fill(RED);
        let mut over = Canvas::new(4, 4);
        over.set(1, 1, GREEN);
        base.overlay(&over, 0, 0);
        assert_eq!(base.get(1, 1), Some(GREEN));
        assert_eq!(base.get(0, 0), Some(RED));
    }

    #[test]
    fn overlay_with_offset() {
        let mut base = Canvas::new(8, 8);
        let mut over = Canvas::new(2, 2);
        over.fill(GREEN);
        base.overlay(&over, 3, 3);
        assert_eq!(base.get(3, 3), Some(GREEN));
        assert_eq!(base.get(4, 4), Some(GREEN));
        assert_eq!(base.get(2, 2), Some([0, 0, 0, 0]));
    }

    // ---------- from_ascii ----------

    #[test]
    fn from_ascii_parses_a_simple_sprite() {
        let p = Palette::new().with('X', RED);
        let c = from_ascii("XX\nXX\n", &p);
        assert_eq!(c.width, 2);
        assert_eq!(c.height, 2);
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(c.get(x, y), Some(RED));
            }
        }
    }

    #[test]
    fn from_ascii_ignores_unknown_chars() {
        let p = Palette::new().with('X', RED);
        let c = from_ascii("X.X\n...\n", &p);
        assert_eq!(c.get(0, 0), Some(RED));
        assert_eq!(c.get(1, 0), Some([0, 0, 0, 0]));
        assert_eq!(c.get(2, 0), Some(RED));
    }

    #[test]
    fn from_ascii_ignores_whitespace_and_blank_lines() {
        let p = Palette::new().with('X', RED);
        let c = from_ascii("  X X  \n\n  X X  \n", &p);
        assert_eq!(c.width, 2);
        assert_eq!(c.height, 2);
        assert_eq!(c.get(0, 0), Some(RED));
        assert_eq!(c.get(1, 0), Some(RED));
    }

    #[test]
    fn from_ascii_empty_input_gives_1x1() {
        let c = from_ascii("", &Palette::new());
        assert_eq!(c.width, 1);
        assert_eq!(c.height, 1);
    }
}