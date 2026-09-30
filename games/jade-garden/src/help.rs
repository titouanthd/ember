//! Help modal — accessible from menu, intro, and gameplay.
//!
//! Shows: the six jades (with their Chinese names and meanings), how
//! to play, how objectives work, and the controls. Opens/closes with
//! Tab or F1.

use macroquad::prelude::*;

use crate::components::Jade;
use crate::config::GameContext;
use crate::fonts::PoemFont;
use crate::tile_render::draw_tile;

/// Width of the modal box.
const MODAL_W: f32 = 780.0;
/// Height of the modal box.
const MODAL_H: f32 = 700.0;
/// Padding inside the modal.
const PAD: f32 = 40.0;

/// Renders the help modal on top of whatever is currently on screen.
pub fn draw(ctx: &GameContext, poem_font: &PoemFont) {
    let vw = screen_width();
    let vh = screen_height();

    // Dim the whole screen.
    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.0, 0.0, 0.0, 0.78));

    let x = (vw - MODAL_W) * 0.5;
    let y = (vh - MODAL_H) * 0.5;

    // Modal background.
    draw_rectangle(x, y, MODAL_W, MODAL_H, Color::new(0.06, 0.05, 0.04, 0.98));

    // Double gold frame.
    draw_rectangle_lines(x, y, MODAL_W, MODAL_H, 1.5, ctx.colors.panel_border);
    draw_rectangle_lines(
        x + 5.0,
        y + 5.0,
        MODAL_W - 10.0,
        MODAL_H - 10.0,
        1.0,
        Color::new(0.28, 0.22, 0.14, 1.0),
    );

    // Colors (mirror the menu palette).
    let gold = ctx.colors.gold;
    let ivory = Color::new(0.96, 0.92, 0.82, 1.0);
    let sepia = Color::new(0.62, 0.56, 0.46, 1.0);
    let dim = Color::new(0.45, 0.40, 0.32, 1.0);

    let left = x + PAD;
    let mut py = y + 50.0;

    // ─── Title ─────────────────────────────────────────────
    let title = "HELP";
    let tw = measure_text(title, None, 26, 1.0).width;
    draw_text(title, x + (MODAL_W - tw) * 0.5, py, 26.0, gold);
    py += 40.0;

    // ─── Section: The Six Jades ────────────────────────────
    section_header(left, py, "THE SIX JADES", sepia);
    py += 26.0;

    // Two columns, three rows.
    let col_x = [left, x + MODAL_W * 0.5 + 20.0];
    let jade_order = [
        (Jade::Bai, Jade::Hong),
        (Jade::Bi, Jade::Huang),
        (Jade::Qing, Jade::Mo),
    ];
    for row in jade_order {
        for (col, jade) in [row.0, row.1].iter().enumerate() {
            let cx = col_x[col];
            draw_jade_entry(*jade, cx, py, poem_font, ivory, sepia);
        }
        py += 62.0;
    }
    py += 4.0;

    // ─── Section: How to Play ──────────────────────────────
    section_header(left, py, "HOW TO PLAY", sepia);
    py += 26.0;

    let rules = [
        "Click a jade, then click an adjacent jade to swap them.",
        "Align 3 or more identical jades to clear them.",
        "Falling jades that align again trigger cascades —",
        "cascades multiply your score.",
    ];
    for line in rules {
        draw_text(line, left, py, 14.0, ivory);
        py += 20.0;
    }
    py += 8.0;

    // ─── Section: Objectives ──────────────────────────────
    section_header(left, py, "OBJECTIVES", sepia);
    py += 26.0;

    let obj_lines = [
        ("Score N", "reach N points"),
        ("Clear N Jade", "remove N jades of that type"),
        ("Trigger N cascades", "chain N cascade reactions"),
    ];
    for (label, desc) in obj_lines {
        draw_text(label, left, py, 14.0, gold);
        let lw = measure_text(label, None, 14, 1.0).width;
        draw_text(desc, left + lw + 20.0, py, 14.0, ivory);
        py += 22.0;
    }
    py += 8.0;

    // ─── Section: Controls ────────────────────────────────
    section_header(left, py, "CONTROLS", sepia);
    py += 26.0;

    let controls = [
        ("Click", "select / swap a jade"),
        ("H", "show a hint (3 per level)"),
        ("Tab / F1", "open or close this help"),
        ("Esc", "back to menu"),
    ];
    for (key, desc) in controls {
        draw_text(key, left, py, 14.0, gold);
        let lw = measure_text(key, None, 14, 1.0).width;
        draw_text(desc, left + lw + 20.0, py, 14.0, ivory);
        py += 22.0;
    }

    // ─── Footer hint ──────────────────────────────────────
    let footer = "[Tab] or [F1] to close";
    let fw = measure_text(footer, None, 13, 1.0).width;
    draw_text(
        footer,
        x + (MODAL_W - fw) * 0.5,
        y + MODAL_H - 26.0,
        13.0,
        dim,
    );
}

/// Draws a horizontal separator with a section label above it.
fn section_header(x: f32, y: f32, label: &str, color: Color) {
    draw_text(label, x, y, 11.0, color);
}

/// Draws one jade row: icon, Chinese name (via poem font), Latin name,
/// and meaning.
fn draw_jade_entry(
    jade: Jade,
    x: f32,
    y: f32,
    poem_font: &PoemFont,
    ivory: Color,
    sepia: Color,
) {
    // Icon (44x44) using the shared draw_tile helper. Seed fixed per
    // jade so the veins always look the same.
    let icon_size = 44.0;
    let seed = 100 + jade.index() as u32 * 7;
    draw_tile(
        jade,
        x,
        y - icon_size * 0.5,
        icon_size,
        1.0,
        1.0,
        seed,
    );

    // Chinese name.
    let cx = x + icon_size + 14.0;
    poem_font.draw(jade.chinese_name(), cx, y + 4.0, 20.0, ivory);

    // Latin name, slightly to the right of the Chinese.
    let cw = poem_font.measure(jade.chinese_name(), 20.0).width;
    draw_text(jade.latin_name(), cx + cw + 12.0, y + 4.0, 16.0, sepia);

    // Meaning underneath.
    draw_text(meaning_of(jade), x + icon_size + 14.0, y + 22.0, 12.0, sepia);
}

/// One-line meaning per jade.
fn meaning_of(jade: Jade) -> &'static str {
    match jade {
        Jade::Bai => "White jade — purity, mourning",
        Jade::Bi => "Green jade — harmony, growth",
        Jade::Qing => "Blue jade — spirit, immortality",
        Jade::Hong => "Red jade — joy, celebration",
        Jade::Huang => "Yellow jade — earth, prosperity",
        Jade::Mo => "Black jade — mystery, ink",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_jade_has_a_non_empty_meaning() {
        for j in Jade::ALL {
            let m = meaning_of(j);
            assert!(!m.is_empty(), "{j:?} has no meaning");
            assert!(
                m.contains('—'),
                "{j:?} meaning missing the em-dash separator: {m:?}"
            );
        }
    }

    #[test]
    fn meanings_are_unique_per_jade() {
        let mut seen = HashSet::new();
        for j in Jade::ALL {
            assert!(seen.insert(meaning_of(j)), "{j:?} meaning is duplicated");
        }
    }

    #[test]
    fn layout_constants_are_positive() {
        assert!(MODAL_W > 0.0);
        assert!(MODAL_H > 0.0);
        assert!(PAD > 0.0);
        // Sanity: padding must fit inside the modal on both axes.
        assert!(PAD * 2.0 < MODAL_W);
        assert!(PAD * 2.0 < MODAL_H);
    }
}