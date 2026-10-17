//! Procedurally-drawn bag for Archie's back.
//!
//! Five fill levels, index 0 (empty) to 4 (full). The bag grows from
//! a small pouch into a modest satchel. Bottom-anchored: the base
//! stays at `BAG_BOTTOM_Y` and the top rises as it fills.
//!
//! Cells are 32×32 — same shape as the CraftPix sheet, so the bag
//! shares Archie's sprite offset and can be drawn behind him at the
//! exact same on-screen anchor.
//!
//! On the CraftPix sheet, the cat's body sits roughly at cell
//! x = 14..26; the tail and empty margin occupy the left side. The
//! bag hugs that body edge — pushed right into the cell so it
//! visually touches the back instead of floating beside it.

use crate::atlas::{AtlasBuilder, SpriteRect};
use crate::pixel::{Canvas, Rgba};

/// Number of fill levels. `Game::draw` maps `fill_ratio()` to an
/// index in `0..LEVELS`.
pub const LEVELS: usize = 5;

/// Sprite cell size, matching `sprites::archie::SHEET_CELL`.
pub const CELL: u16 = 32;

/// Bag body's horizontal center within the cell. Tuned against the
/// CraftPix cat so the bag's right edge overlaps the cat's back.
const BAG_CENTER_X: i32 = 13;
/// Bottom of the bag in cell coordinates. Mid-back, roughly level
/// with the cat's chest.
const BAG_BOTTOM_Y: i32 = 26;

// ---------- Palette ----------

pub const BAG_MAIN: Rgba = [0x8C, 0x5F, 0x32, 0xFF];
pub const BAG_DARK: Rgba = [0x3A, 0x24, 0x14, 0xFF];
pub const BAG_LIGHT: Rgba = [0xB8, 0x82, 0x48, 0xFF];
pub const BAG_STRAP: Rgba = [0x4A, 0x30, 0x18, 0xFF];

// ---------- Geometry ----------

/// Width and height of the bag body at `level ∈ 0..LEVELS`.
fn bag_size(level: usize) -> (i32, i32) {
    let w = 4 + (level as i32) / 2;
    let h = 4 + level as i32;
    (w, h)
}

fn render(level: usize) -> Canvas {
    let mut c = Canvas::new(CELL, CELL);
    let (w, h) = bag_size(level);
    let x = BAG_CENTER_X - w / 2;
    let y = BAG_BOTTOM_Y - h;
    draw_bag(&mut c, x, y, w, h);
    c
}

fn draw_bag(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    // Strap: a short diagonal from the bag's top toward the cat's
    // shoulder. Two pixels of thickness.
    let sx = x + w / 2;
    let sy = y - 1;
    c.set(sx, sy, BAG_STRAP);
    c.set(sx - 1, sy - 1, BAG_STRAP);
    c.set(sx - 2, sy - 2, BAG_STRAP);
    c.set(sx, sy - 1, BAG_STRAP);
    c.set(sx - 1, sy - 2, BAG_STRAP);

    // Body outline.
    c.rect_outline(x - 1, y - 1, w + 2, h + 2, BAG_DARK);

    // Body.
    c.rect(x, y, w, h, BAG_MAIN);

    // Top flap.
    c.rect(x, y, w, 2, BAG_LIGHT);
    c.hline(x, x + w - 1, y + 2, BAG_DARK);

    // Clasp: a single dark pixel centered on the flap seam.
    let cx = x + w / 2;
    c.set(cx, y + 3, BAG_DARK);
}

// ---------- Bake ----------

/// Bake all `LEVELS` fill levels, R + mirrored L, into the atlas.
/// Names: `bag_{0..LEVELS-1}_{R|L}`.
pub fn bake_into(builder: &mut AtlasBuilder) -> (Vec<SpriteRect>, Vec<SpriteRect>) {
    let mut r = Vec::with_capacity(LEVELS);
    let mut l = Vec::with_capacity(LEVELS);
    for level in 0..LEVELS {
        let canvas = render(level);
        r.push(builder.add(&format!("bag_{level}_R"), canvas.clone()));
        l.push(builder.add(&format!("bag_{level}_L"), canvas.mirrored_x()));
    }
    (r, l)
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_produces_non_empty_canvas() {
        for level in 0..LEVELS {
            assert!(
                !render(level).is_empty(),
                "bag level {level} is empty"
            );
        }
    }

    #[test]
    fn levels_differ() {
        let a = render(0);
        let b = render(LEVELS - 1);
        assert_ne!(a.pixels, b.pixels, "empty and full bags look identical");
    }

    #[test]
    fn size_grows_with_level() {
        for level in 1..LEVELS {
            let (w0, h0) = bag_size(level - 1);
            let (w1, h1) = bag_size(level);
            assert!(w1 >= w0 && h1 >= h0, "bag shrank at level {level}");
        }
    }

    #[test]
    fn bottom_stays_anchored() {
        for level in 0..LEVELS {
            let (_, h) = bag_size(level);
            let y = BAG_BOTTOM_Y - h;
            assert_eq!(y + h, BAG_BOTTOM_Y, "level {level} moved the base");
        }
    }

    #[test]
    fn bag_stays_within_cell() {
        let (w, h) = bag_size(LEVELS - 1);
        let x = BAG_CENTER_X - w / 2;
        let y = BAG_BOTTOM_Y - h;
        assert!(x >= 1 && y >= 1, "bag clips the cell top/left");
        assert!(x + w <= CELL as i32 - 1, "bag clips the cell right");
        assert!(y + h <= CELL as i32 - 1, "bag clips the cell bottom");
    }

    #[test]
    fn bag_is_not_taller_than_the_cat() {
        let (_, h) = bag_size(LEVELS - 1);
        assert!(h <= 10, "bag h = {h}, too tall");
    }

    #[test]
    fn mirrored_differs_from_original() {
        let a = render(2);
        assert_ne!(a.pixels, a.mirrored_x().pixels);
    }
}