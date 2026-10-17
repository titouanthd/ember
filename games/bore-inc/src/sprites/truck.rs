//! Truck sprite: modern blue hauler, drawn pixel-by-pixel into a
//! 32×32 cell. Two frames for wheel spin.
//!
//! Facing right canonical; left is mirrored at bake time.

use crate::atlas::{AtlasBuilder, SpriteRect};
use crate::pixel::{Canvas, Rgba};

/// Cell size, in pixels. Matches the physics box.
pub const TRUCK_CELL: u16 = 32;

/// Wheel-spin frame count.
pub const FRAMES: usize = 2;

// ---------- Palette ----------

const OUTLINE: Rgba = [0x08, 0x0C, 0x18, 0xFF];
const BODY: Rgba = [0x3F, 0x66, 0x9C, 0xFF];
const BODY_LIGHT: Rgba = [0x60, 0x88, 0xBC, 0xFF];
const PLATEAU: Rgba = [0x30, 0x50, 0x80, 0xFF];
const PLATEAU_RAIL: Rgba = [0x1C, 0x32, 0x54, 0xFF];
const GLASS: Rgba = [0xA8, 0xC8, 0xE8, 0xFF];
const GLASS_DARK: Rgba = [0x5C, 0x7C, 0xA0, 0xFF];
const WHEEL: Rgba = [0x12, 0x12, 0x18, 0xFF];
const WHEEL_HUB: Rgba = [0x6E, 0x6E, 0x78, 0xFF];
const YELLOW: Rgba = [0xF5, 0xD8, 0x7A, 0xFF];
const HEADLIGHT: Rgba = [0xFF, 0xFC, 0xE0, 0xFF];
const TAILLIGHT: Rgba = [0xD0, 0x4A, 0x32, 0xFF];

// ---------- Baked sprites ----------

/// Baked truck sprites, ready for `Game::draw`.
#[derive(Debug, Clone)]
pub struct TruckSprites {
    pub r: Vec<SpriteRect>,
    pub l: Vec<SpriteRect>,
}

pub fn bake_into(builder: &mut AtlasBuilder) -> (Vec<SpriteRect>, Vec<SpriteRect>) {
    let mut r = Vec::with_capacity(FRAMES);
    let mut l = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        let canvas = render(frame);
        r.push(builder.add(&format!("truck_{frame}_R"), canvas.clone()));
        l.push(builder.add(&format!("truck_{frame}_L"), canvas.mirrored_x()));
    }
    (r, l)
}

fn render(frame: usize) -> Canvas {
    let mut c = Canvas::new(TRUCK_CELL, TRUCK_CELL);
    draw_truck(&mut c, frame);
    c
}

// ---------- Drawing ----------

fn draw_truck(c: &mut Canvas, frame: usize) {
    // Plateau (left, long low bed). Extended left to x=1 so the
    // truck reads as a proper hauler.
    c.rect(1, 9, 17, 11, PLATEAU);
    c.rect_outline(1, 9, 17, 11, OUTLINE);
    c.hline(2, 17, 10, PLATEAU_RAIL);
    c.hline(2, 17, 18, PLATEAU_RAIL);

    // Brand text on the side panel.
    draw_bore_text(c, 3, 12);

    // Cabine (right, tall + sloped windshield). Reaches x=30.
    c.rect(18, 4, 12, 16, BODY);
    c.rect_outline(18, 4, 12, 16, OUTLINE);
    c.hline(19, 29, 4, BODY_LIGHT);

    // Windshield with a sloped top-left corner.
    c.rect(20, 5, 9, 6, GLASS);
    c.rect(20, 5, 3, 2, BODY);
    c.set(23, 5, GLASS_DARK);
    c.hline(20, 28, 10, GLASS_DARK);
    c.vline(19, 5, 10, GLASS_DARK);

    // Door line + handle.
    c.vline(20, 11, 20, OUTLINE);
    c.set(26, 13, OUTLINE);

    // Brand stripe on the door.
    c.hline(21, 28, 15, YELLOW);
    c.hline(21, 28, 16, YELLOW);

    // Lights.
    c.set(29, 17, HEADLIGHT);
    c.set(1, 17, TAILLIGHT);

    // Wheels. Placed under the body, no chassis bar between them.
    draw_wheel(c, 6, 25, frame);
    draw_wheel(c, 24, 25, frame);

    // Outline pass — connects body to wheels, no shadow line.
    add_outline(c, OUTLINE);
}

fn draw_wheel(c: &mut Canvas, cx: i32, cy: i32, frame: usize) {
    c.hline(cx - 1, cx + 1, cy - 3, WHEEL);
    c.hline(cx - 2, cx + 2, cy - 2, WHEEL);
    c.hline(cx - 3, cx + 3, cy - 1, WHEEL);
    c.hline(cx - 3, cx + 3, cy, WHEEL);
    c.hline(cx - 3, cx + 3, cy + 1, WHEEL);
    c.hline(cx - 2, cx + 2, cy + 2, WHEEL);
    c.hline(cx - 1, cx + 1, cy + 3, WHEEL);

    let (hx, hy) = match frame {
        0 => (cx, cy),
        _ => (cx + 1, cy - 1),
    };
    c.hline(hx - 1, hx + 1, hy, WHEEL_HUB);
    c.vline(hx, hy - 1, hy + 1, WHEEL_HUB);
}

fn draw_bore_text(c: &mut Canvas, x: i32, y: i32) {
    // B
    c.hline(x, x + 2, y, YELLOW);
    c.set(x, y + 1, YELLOW);
    c.set(x + 2, y + 1, YELLOW);
    c.hline(x, x + 2, y + 2, YELLOW);
    c.set(x, y + 3, YELLOW);
    c.set(x + 2, y + 3, YELLOW);
    c.hline(x, x + 2, y + 4, YELLOW);

    // O
    let ox = x + 3;
    c.hline(ox, ox + 2, y, YELLOW);
    c.set(ox, y + 1, YELLOW);
    c.set(ox + 2, y + 1, YELLOW);
    c.set(ox, y + 2, YELLOW);
    c.set(ox + 2, y + 2, YELLOW);
    c.set(ox, y + 3, YELLOW);
    c.set(ox + 2, y + 3, YELLOW);
    c.hline(ox, ox + 2, y + 4, YELLOW);

    // R
    let rx = x + 6;
    c.hline(rx, rx + 2, y, YELLOW);
    c.set(rx, y + 1, YELLOW);
    c.set(rx + 2, y + 1, YELLOW);
    c.hline(rx, rx + 2, y + 2, YELLOW);
    c.set(rx, y + 3, YELLOW);
    c.set(rx + 1, y + 3, YELLOW);
    c.set(rx, y + 4, YELLOW);
    c.set(rx + 2, y + 4, YELLOW);

    // E
    let ex = x + 9;
    c.hline(ex, ex + 2, y, YELLOW);
    c.set(ex, y + 1, YELLOW);
    c.hline(ex, ex + 2, y + 2, YELLOW);
    c.set(ex, y + 3, YELLOW);
    c.hline(ex, ex + 2, y + 4, YELLOW);
}

fn add_outline(c: &mut Canvas, color: Rgba) {
    let snapshot = c.clone();
    for y in 0..c.height as i32 {
        for x in 0..c.width as i32 {
            let is_empty = snapshot.get(x, y).is_none_or(|p| p[3] == 0);
            if !is_empty {
                continue;
            }
            let has_solid_neighbor = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| {
                    snapshot.get(x + dx, y + dy).is_some_and(|p| p[3] > 0)
                });
            if has_solid_neighbor {
                c.set(x, y, color);
            }
        }
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_is_32() {
        assert_eq!(TRUCK_CELL, 32);
    }

    #[test]
    fn render_produces_non_empty_canvas() {
        for frame in 0..FRAMES {
            assert!(!render(frame).is_empty(), "frame {frame} is empty");
        }
    }

    #[test]
    fn frames_differ() {
        assert_ne!(render(0).pixels, render(1).pixels);
    }

    #[test]
    fn mirrored_differs_from_original() {
        let a = render(0);
        assert_ne!(a.pixels, a.mirrored_x().pixels);
    }

    #[test]
    fn bore_text_is_present() {
        let c = render(0);
        let mut yellow_count = 0;
        for y in 0..TRUCK_CELL as i32 {
            for x in 0..TRUCK_CELL as i32 {
                if c.get(x, y) == Some(YELLOW) {
                    yellow_count += 1;
                }
            }
        }
        assert!(yellow_count > 40, "yellow pixels = {yellow_count}");
    }
}