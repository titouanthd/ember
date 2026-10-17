//! Small overlays drawn at native resolution, anchored to world
//! positions. Currently: the bag readout panel.
//!
//! The readout is *attached* to Archie's bag (follows the sprite,
//! hides when the bag is empty) rather than being a fixed HUD corner.
//! This keeps the information perceptually part of the character —
//! the jam restriction is on input, not on rendering, but a fixed
//! corner HUD would still feel like a break from the world.

use macroquad::prelude::*;

use crate::bag::Bag;
use crate::components::ResourceKind;
use crate::config::GameContext;

/// Small colored square next to each resource name, in native px.
const CHIP: f32 = 12.0;
/// Vertical step between rows, in native px.
const ROW_H: f32 = 24.0;
/// Panel padding, in native px.
const PAD: f32 = 10.0;
/// Gap between chip and name, and between name and count.
const GAP: f32 = 10.0;
/// Text size for names and counts.
const TEXT_SIZE: f32 = 20.0;
/// Minimum distance between panel bottom and sprite top, in native px.
const GAP_ABOVE: f32 = 12.0;

/// Draw the bag contents as a small panel centered above Archie.
///
/// `center_x` and `top_y` are the sprite's horizontal center and top
/// edge in **native screen pixels** — the caller converts from world
/// coordinates with the same `sx` / `sy` closures used for the sprite.
///
/// Draws nothing if the bag is empty.
pub fn draw_bag_readout(
    bag: &Bag,
    center_x: f32,
    top_y: f32,
    ctx: &GameContext,
) {
    let entries = bag.contents_in_order();
    if entries.is_empty() {
        return;
    }

    // Width of the widest row: chip + gap + longest name + gap + widest count.
    let mut max_row_w = 0.0_f32;
    for (res, count) in &entries {
        let name_w = measure_text(res.name(), None, TEXT_SIZE as u16, 1.0).width;
        let count_str = format!("x{count}");
        let count_w = measure_text(&count_str, None, TEXT_SIZE as u16, 1.0).width;
        let w = CHIP + GAP + name_w + GAP + count_w;
        if w > max_row_w {
            max_row_w = w;
        }
    }

    let n = entries.len() as f32;
    let panel_w = max_row_w + PAD * 2.0;
    let panel_h = n * ROW_H + PAD * 2.0 - (ROW_H - TEXT_SIZE);

    let px = center_x - panel_w * 0.5;
    let py = (top_y - panel_h - GAP_ABOVE).max(8.0);

    // Panel background — dark, warm, no outline.
    draw_rectangle(px, py, panel_w, panel_h, Color::new(0.06, 0.04, 0.02, 0.78));

    for (i, (res, count)) in entries.iter().enumerate() {
        let row_y = py + PAD + (i as f32) * ROW_H;

        // Chip.
        let (r, g, b) = resource_color(*res);
        let chip_y = row_y + (TEXT_SIZE - CHIP) * 0.5;
        draw_rectangle(
            px + PAD,
            chip_y,
            CHIP,
            CHIP,
            Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0),
        );

        // Name.
        let name_x = px + PAD + CHIP + GAP;
        draw_text(
            res.name(),
            name_x,
            row_y + TEXT_SIZE - 2.0,
            TEXT_SIZE,
            ctx.colors.text,
        );

        // Count, right-aligned.
        let count_str = format!("x{count}");
        let count_w = measure_text(&count_str, None, TEXT_SIZE as u16, 1.0).width;
        let count_x = px + panel_w - PAD - count_w;
        draw_text(
            &count_str,
            count_x,
            row_y + TEXT_SIZE - 2.0,
            TEXT_SIZE,
            ctx.colors.gold,
        );
    }
}

/// Chip color for the readout. Matches the on-ground drop color so
/// the player can visually link "what I picked up" to "what I'm
/// carrying".
fn resource_color(r: ResourceKind) -> (u8, u8, u8) {
    use ResourceKind::*;
    match r {
        Dirt => (0x8B, 0x5A, 0x3A),
        Stone => (0x7A, 0x6A, 0x5A),
        Coal => (0x1E, 0x20, 0x2A),
        Iron => (0xB8, 0x9A, 0x6A),
        Rock => (0x7A, 0x4E, 0x3A),
        Crystal => (0x5F, 0xD4, 0xD4),
    }
}