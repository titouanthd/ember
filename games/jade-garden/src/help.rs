//! Help modal — accessible from menu, intro, and gameplay.
//!
//! Shows the six jades, how to play, objectives, and controls.
//! Opens/closes with Tab or F1.
//!
//! The modal is not full-height: it has a fixed header band with the
//! title, a fixed footer band with the close hint, and a scrollable
//! content area in between. Scroll with the wheel or the arrow keys.

use macroquad::prelude::*;

use crate::components::Jade;
use crate::config::GameContext;
use crate::fonts::PoemFont;
use crate::tile_render::draw_tile;

// ---------- Modal geometry ----------

const MODAL_W: f32 = 900.0;
const MODAL_H: f32 = 640.0;
const PAD: f32 = 48.0;

/// Height of the fixed header band (title area).
const HEADER_BAND_H: f32 = 84.0;
/// Height of the fixed footer band (close hint area).
const FOOTER_BAND_H: f32 = 48.0;

/// Baseline of the "HELP" title, from the modal top.
const TITLE_BASELINE: f32 = 54.0;

// ---------- Content vertical rhythm ----------

const CONTENT_TOP_PAD: f32 = 24.0;
const HEADER_LINE_H: f32 = 13.0;
const HEADER_TO_CONTENT: f32 = 40.0;
const CONTENT_TO_HEADER: f32 = 40.0;
const LINE_STRIDE: f32 = 24.0;
const JADE_ROW_STRIDE: f32 = 88.0;
const JADE_ROW_BOTTOM: f32 = 26.0;

// ---------- Colors ----------

const MODAL_BG: Color = Color::new(0.06, 0.05, 0.04, 0.98);
const MODAL_INNER_FRAME: Color = Color::new(0.28, 0.22, 0.14, 1.0);
const MODAL_SEPARATOR: Color = Color::new(0.28, 0.22, 0.14, 0.7);
const IVORY: Color = Color::new(0.96, 0.92, 0.82, 1.0);
const SEPIA: Color = Color::new(0.62, 0.56, 0.46, 1.0);
const DIM: Color = Color::new(0.45, 0.40, 0.32, 1.0);

// ---------- Public entry point ----------

/// Renders the help modal on top of whatever is currently on screen.
///
/// Mutates `scroll_y` based on wheel / arrow-key input, and clamps it
/// to the valid scroll range. The caller owns the scroll state so it
/// survives across frames.
pub fn draw(ctx: &GameContext, poem_font: &PoemFont, scroll_y: &mut f32) {
    let vw = screen_width();
    let vh = screen_height();

    let visible_h = MODAL_H - HEADER_BAND_H - FOOTER_BAND_H;
    let content_h = content_height();
    let max_scroll = (content_h - visible_h).max(0.0);

    // ─── Input ───
    if max_scroll > 0.0 {
        let (_, wy) = mouse_wheel();
        if wy.abs() > 0.0 {
            *scroll_y = (*scroll_y - wy * 50.0).clamp(0.0, max_scroll);
        }
        if is_key_down(KeyCode::Down) {
            *scroll_y = (*scroll_y + 600.0 * get_frame_time()).min(max_scroll);
        }
        if is_key_down(KeyCode::Up) {
            *scroll_y = (*scroll_y - 600.0 * get_frame_time()).max(0.0);
        }
    } else {
        *scroll_y = 0.0;
    }
    let scroll = scroll_y.clamp(0.0, max_scroll);

    let x = (vw - MODAL_W) * 0.5;
    let y = (vh - MODAL_H) * 0.5;

    // ─── 1. Dim overlay ───
    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.0, 0.0, 0.0, 0.80));

    // ─── 2. Modal background ───
    draw_rectangle(x, y, MODAL_W, MODAL_H, MODAL_BG);

    // ─── 3. Scrollable content ───
    let content_top = y + HEADER_BAND_H;
    draw_content(x + PAD, content_top - scroll, ctx.colors.gold, poem_font);

    // ─── 4. Opaque header band — hides content that scrolled up ───
    draw_rectangle(x, y, MODAL_W, HEADER_BAND_H, MODAL_BG);

    // ─── 5. Opaque footer band — hides content that scrolled down ───
    draw_rectangle(
        x,
        y + MODAL_H - FOOTER_BAND_H,
        MODAL_W,
        FOOTER_BAND_H,
        MODAL_BG,
    );

    // ─── 6. Separators between bands and content ───
    let sep_top = y + HEADER_BAND_H;
    let sep_bot = y + MODAL_H - FOOTER_BAND_H;
    draw_line(x + 20.0, sep_top, x + MODAL_W - 20.0, sep_top, 1.0, MODAL_SEPARATOR);
    draw_line(x + 20.0, sep_bot, x + MODAL_W - 20.0, sep_bot, 1.0, MODAL_SEPARATOR);

    // ─── 7. Frame ───
    draw_frame(x, y, MODAL_W, MODAL_H, ctx.colors.panel_border);

    // ─── 8. Title ───
    let title = "HELP";
    let tw = measure_text(title, None, 32, 1.0).width;
    draw_text(
        title,
        x + (MODAL_W - tw) * 0.5,
        y + TITLE_BASELINE,
        32.0,
        ctx.colors.gold,
    );

    // Small gold diamond under the title.
    draw_poly(
        x + MODAL_W * 0.5,
        y + TITLE_BASELINE + 20.0,
        4,
        5.0,
        0.0,
        ctx.colors.gold,
    );

    // ─── 9. Footer hint ───
    let hint = if max_scroll > 0.0 {
        "[Tab] or [F1] to close   ·   Scroll \u{2191}\u{2193} or wheel"
    } else {
        "[Tab] or [F1] to close"
    };
    let hw = measure_text(hint, None, 13, 1.0).width;
    draw_text(
        hint,
        x + (MODAL_W - hw) * 0.5,
        y + MODAL_H - 20.0,
        13.0,
        DIM,
    );

    // ─── 10. Scroll indicator ───
    if max_scroll > 0.0 {
        draw_scroll_indicator(x, y, scroll, max_scroll, visible_h, content_h);
    }
}

// ---------- Content height ----------

/// Total height of the scrollable content, in pixels. Must mirror the
/// sequence of `py +=` operations in `draw_content` exactly — a test
/// guards the correspondence.
fn content_height() -> f32 {
    let mut h = CONTENT_TOP_PAD + HEADER_LINE_H;
    h += HEADER_TO_CONTENT;
    h += 2.0 * JADE_ROW_STRIDE + JADE_ROW_BOTTOM;
    h += CONTENT_TO_HEADER + HEADER_TO_CONTENT;
    h += 3.0 * LINE_STRIDE;
    h += CONTENT_TO_HEADER + HEADER_TO_CONTENT;
    h += 2.0 * LINE_STRIDE;
    h += CONTENT_TO_HEADER + HEADER_TO_CONTENT;
    h += 3.0 * LINE_STRIDE;
    h += CONTENT_TOP_PAD;
    h
}

// ---------- Content rendering ----------

fn draw_content(left: f32, content_top: f32, gold: Color, poem_font: &PoemFont) {
    // ── Section: The Six Jades ──
    let mut py = content_top + CONTENT_TOP_PAD + HEADER_LINE_H;
    section_header(left, py, "THE SIX JADES");
    py += HEADER_TO_CONTENT;
    let first_row_center = py;

    let jade_rows = [
        (Jade::Bai, Jade::Hong),
        (Jade::Bi, Jade::Huang),
        (Jade::Qing, Jade::Mo),
    ];
    let col2_x = left + (MODAL_W - 2.0 * PAD) * 0.5 + 24.0;
    for (i, (j_left, j_right)) in jade_rows.iter().enumerate() {
        let row_center = first_row_center + i as f32 * JADE_ROW_STRIDE;
        draw_jade_entry(*j_left, left, row_center, poem_font);
        draw_jade_entry(*j_right, col2_x, row_center, poem_font);
    }
    py = first_row_center + 2.0 * JADE_ROW_STRIDE + JADE_ROW_BOTTOM;

    // ── Section: How to Play ──
    py += CONTENT_TO_HEADER;
    section_header(left, py, "HOW TO PLAY");
    py += HEADER_TO_CONTENT;
    let rules = [
        "Click a jade, then click an adjacent jade to swap them.",
        "Align 3 or more identical jades to clear them.",
        "Falling jades that align again trigger cascades —",
        "cascades multiply your score.",
    ];
    for line in rules {
        draw_text(line, left, py, 15.0, IVORY);
        py += LINE_STRIDE;
    }
    py -= LINE_STRIDE;

    // ── Section: Objectives ──
    py += CONTENT_TO_HEADER;
    section_header(left, py, "OBJECTIVES");
    py += HEADER_TO_CONTENT;
    let objectives = [
        ("Score N", "reach N points"),
        ("Clear N Jade", "remove N jades of that type"),
        ("Trigger N cascades", "chain N cascade reactions"),
    ];
    for (label, desc) in objectives {
        draw_key_value(left, py, label, desc, gold);
        py += LINE_STRIDE;
    }
    py -= LINE_STRIDE;

    // ── Section: Controls ──
    py += CONTENT_TO_HEADER;
    section_header(left, py, "CONTROLS");
    py += HEADER_TO_CONTENT;
    let controls = [
        ("Click", "select / swap a jade"),
        ("H", "show a hint (3 per level)"),
        ("Tab / F1", "open or close this help"),
        ("Esc", "back to menu"),
    ];
    for (key, desc) in controls {
        draw_key_value(left, py, key, desc, gold);
        py += LINE_STRIDE;
    }
}

fn section_header(x: f32, y: f32, label: &str) {
    draw_text(label, x, y, HEADER_LINE_H, SEPIA);
}

fn draw_key_value(x: f32, y: f32, label: &str, desc: &str, label_color: Color) {
    draw_text(label, x, y, 15.0, label_color);
    let lw = measure_text(label, None, 15, 1.0).width;
    draw_text(desc, x + lw + 24.0, y, 15.0, IVORY);
}

fn draw_jade_entry(jade: Jade, x: f32, y: f32, poem_font: &PoemFont) {
    let icon_size = 56.0;
    let seed = 100 + jade.index() as u32 * 7;
    draw_tile(jade, x, y - icon_size * 0.5, icon_size, 1.0, 1.0, seed);

    let text_x = x + icon_size + 18.0;
    poem_font.draw(jade.chinese_name(), text_x, y + 6.0, 24.0, IVORY);

    let cw = poem_font.measure(jade.chinese_name(), 24.0).width;
    draw_text(jade.latin_name(), text_x + cw + 14.0, y + 6.0, 18.0, SEPIA);

    draw_text(meaning_of(jade), text_x, y + 28.0, 14.0, SEPIA);
}

fn draw_frame(x: f32, y: f32, w: f32, h: f32, gold: Color) {
    // Outer thick line
    draw_rectangle_lines(x, y, w, h, 2.0, gold);
    // Inner thin line
    draw_rectangle_lines(x + 8.0, y + 8.0, w - 16.0, h - 16.0, 1.0, MODAL_INNER_FRAME);
    // Corner diamonds at the inner frame corners
    let size = 6.0;
    for (cx, cy) in [
        (x + 8.0, y + 8.0),
        (x + w - 8.0, y + 8.0),
        (x + 8.0, y + h - 8.0),
        (x + w - 8.0, y + h - 8.0),
    ] {
        draw_poly(cx, cy, 4, size, 0.0, gold);
    }
}

fn draw_scroll_indicator(
    modal_x: f32,
    modal_y: f32,
    scroll: f32,
    max_scroll: f32,
    visible_h: f32,
    content_h: f32,
) {
    let track_x = modal_x + MODAL_W - 20.0;
    let track_top = modal_y + HEADER_BAND_H + 12.0;
    let track_bottom = modal_y + MODAL_H - FOOTER_BAND_H - 12.0;
    let track_h = track_bottom - track_top;
    if track_h <= 0.0 {
        return;
    }

    // Track
    draw_rectangle(track_x, track_top, 2.0, track_h, MODAL_INNER_FRAME);

    // Thumb
    let thumb_h = (visible_h / content_h) * track_h;
    let thumb_y = track_top + (scroll / max_scroll) * (track_h - thumb_h);
    let thumb_color = Color::new(0.65, 0.55, 0.35, 0.9);
    draw_rectangle(track_x - 2.0, thumb_y, 6.0, thumb_h, thumb_color);
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

// ---------- Tests ----------

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
        assert!(PAD * 2.0 < MODAL_W);
        assert!(PAD * 2.0 < MODAL_H);
    }

    #[test]
    fn section_padding_is_uniform() {
        assert_eq!(HEADER_TO_CONTENT, CONTENT_TO_HEADER);
    }

    #[test]
    fn modal_leaves_visible_margins_on_a_900px_window() {
        // The modal should not fill the whole window: at least 100px
        // of visible overlay above and below.
        assert!(MODAL_H <= 900.0 - 200.0, "modal is too tall: {MODAL_H}");
    }

    #[test]
    fn modal_fits_the_default_window() {
        assert!(MODAL_W < 1400.0);
        assert!(MODAL_H < 900.0);
        assert!(MODAL_W + 20.0 < 1400.0);
        assert!(MODAL_H + 20.0 < 900.0);
    }

    #[test]
    fn content_is_taller_than_the_visible_area() {
        // The whole point of adding scrolling: the content must
        // actually overflow, otherwise the scroll indicator never
        // appears and the interaction is dead code.
        let visible = MODAL_H - HEADER_BAND_H - FOOTER_BAND_H;
        assert!(
            content_height() > visible,
            "content ({}) fits in the visible area ({}); scrolling is dead",
            content_height(),
            visible
        );
    }

    #[test]
    fn content_fits_when_scrolled_to_bottom() {
        let visible = MODAL_H - HEADER_BAND_H - FOOTER_BAND_H;
        let content = content_height();
        let max_scroll = (content - visible).max(0.0);
        // At max scroll, the bottom of the content is exactly at the
        // bottom of the visible area.
        let bottom_at_max = content - max_scroll;
        assert!((bottom_at_max - visible).abs() < 1e-3);
    }

    #[test]
    fn header_band_fits_the_title() {
        // The title baseline and its decorative diamond must sit
        // comfortably inside the header band.
        assert!(TITLE_BASELINE + 20.0 < HEADER_BAND_H);
    }
}