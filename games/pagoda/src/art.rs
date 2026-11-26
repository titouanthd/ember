// games/pagoda/src/art.rs
//! World-space artwork: lacquered tower tiles and the drop guide.
//!
//! A block is still a rectangle at heart, but it is rendered as a
//! carved tier: a lit top face, a shadowed base, a thin gold seam and
//! a small diamond boss. The treatment is cheap (a handful of quads per
//! block) and gives the tower a Chinese timber-frame feel without
//! turning the game into a 3D renderer.

use crate::components::{Block, SlidingBlock};
use crate::config::{IVORY, PERFECT_GOLD};
use ember_core::Color;
use ember_core::math::Rect;
use ember_servers::RenderingServer;
use glam::{Vec2, vec2};

#[cfg(test)]
use crate::config::WHITE;

/// Move `color` towards white by `amount` (0.0–1.0).
pub fn lighten(color: Color, amount: f32) -> Color {
    let t = amount.clamp(0.0, 1.0);
    Color::new(
        color.r + (1.0 - color.r) * t,
        color.g + (1.0 - color.g) * t,
        color.b + (1.0 - color.b) * t,
        color.a,
    )
}

/// Move `color` towards black by `amount` (0.0–1.0).
pub fn darken(color: Color, amount: f32) -> Color {
    let t = 1.0 - amount.clamp(0.0, 1.0);
    Color::new(color.r * t, color.g * t, color.b * t, color.a)
}

pub fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::new(color.r, color.g, color.b, alpha.clamp(0.0, 1.0))
}

/// Filled diamond, built from two triangles.
pub fn draw_diamond(r: &mut dyn RenderingServer, cx: f32, cy: f32, radius: f32, color: Color) {
    let top = vec2(cx, cy - radius);
    let bottom = vec2(cx, cy + radius);
    let left = vec2(cx - radius, cy);
    let right = vec2(cx + radius, cy);
    r.draw_triangle(top, left, bottom, color);
    r.draw_triangle(top, right, bottom, color);
}

/// Diamond outline, used as a small carved motif.
pub fn diamond_outline(
    r: &mut dyn RenderingServer,
    cx: f32,
    cy: f32,
    radius: f32,
    thickness: f32,
    color: Color,
) {
    r.draw_line(
        Vec2::new(cx, cy - radius),
        Vec2::new(cx + radius, cy),
        thickness,
        color,
    );
    r.draw_line(
        Vec2::new(cx + radius, cy),
        Vec2::new(cx, cy + radius),
        thickness,
        color,
    );
    r.draw_line(
        Vec2::new(cx, cy + radius),
        Vec2::new(cx - radius, cy),
        thickness,
        color,
    );
    r.draw_line(
        Vec2::new(cx - radius, cy),
        Vec2::new(cx, cy - radius),
        thickness,
        color,
    );
}

/// Draw one tower tier at screen coordinates.
///
/// `glow` in `0.0..=1.0` adds a gold landing outline; it is used on the
/// newest block and decays over a few frames.
pub fn draw_tile(
    r: &mut dyn RenderingServer,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    glow: f32,
) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    r.draw_rect(Rect::new(x, y, w, h), color);

    // Lit top face / shadowed base: enough to read as a carved edge.
    let top_h = (h * 0.16).clamp(2.0, 7.0);
    r.draw_rect(Rect::new(x, y, w, top_h), lighten(color, 0.20));
    let base_h = (h * 0.13).clamp(2.0, 6.0);
    r.draw_rect(Rect::new(x, y + h - base_h, w, base_h), darken(color, 0.28));

    // A thin gold seam, like the joined edge of a lacquered beam.
    r.draw_rect(Rect::new(x, y, w, 1.5), with_alpha(PERFECT_GOLD, 0.34));

    // Carved diamond boss on wide-enough tiles.
    if w >= 42.0 && h >= 24.0 {
        let radius = (h * 0.14).clamp(3.0, 5.5);
        diamond_outline(
            r,
            x + w / 2.0,
            y + h / 2.0 + 1.0,
            radius,
            1.0,
            with_alpha(darken(color, 0.50), 0.34),
        );
    }

    if glow > 0.0 {
        r.draw_rect_outline(
            Rect::new(x - 1.0, y - 1.0, w + 2.0, h + 2.0),
            2.0,
            with_alpha(PERFECT_GOLD, glow.clamp(0.0, 1.0) * 0.85),
        );
    }
}

/// Draw a settled tower block.
pub fn draw_block(
    r: &mut dyn RenderingServer,
    block: &Block,
    camera_y: f32,
    shake: Vec2,
    glow: f32,
) {
    draw_tile(
        r,
        block.transform.position.x + shake.x,
        block.transform.position.y + camera_y + shake.y,
        block.transform.scale.x,
        block.transform.scale.y,
        block.color,
        glow,
    );
}

/// Draw the sliding block. When its centre is inside the perfect
/// window, a gold rim makes the alignment readable.
pub fn draw_sliding_block(
    r: &mut dyn RenderingServer,
    sliding: &SlidingBlock,
    y: f32,
    block_height: f32,
    shake: Vec2,
    aligned: bool,
) {
    let x = sliding.x + shake.x;
    let tile_y = y + shake.y;

    draw_tile(
        r,
        x,
        tile_y,
        sliding.width,
        block_height,
        sliding.color,
        0.0,
    );

    let (color, thickness) = if aligned {
        (PERFECT_GOLD, 2.5)
    } else {
        (with_alpha(IVORY, 0.55), 1.0)
    };
    r.draw_rect_outline(
        Rect::new(
            x - 1.0,
            tile_y - 1.0,
            sliding.width + 2.0,
            block_height + 2.0,
        ),
        thickness,
        with_alpha(color, if aligned { 0.95 } else { 0.35 }),
    );

    if aligned {
        draw_diamond(r, x + 7.0, tile_y + block_height / 2.0, 3.0, PERFECT_GOLD);
        draw_diamond(
            r,
            x + sliding.width - 7.0,
            tile_y + block_height / 2.0,
            3.0,
            PERFECT_GOLD,
        );
    }
}

/// Draw the alignment guide once the sliding block is over the tower.
///
/// The line tracks the sliding block's centre through the landing tier,
/// while the small target mark sits on the centre of the block below.
/// Both turn gold inside the perfect window.
pub fn draw_drop_guide(
    r: &mut dyn RenderingServer,
    slide_center_x: f32,
    target_center_x: f32,
    top_y: f32,
    block_height: f32,
    aligned: bool,
    time: f32,
) {
    let pulse = if aligned {
        0.72 + 0.22 * (time * 11.0).sin()
    } else {
        0.14
    };

    r.draw_line(
        Vec2::new(slide_center_x, top_y + 1.0),
        Vec2::new(slide_center_x, top_y + block_height * 2.4),
        if aligned { 2.5 } else { 1.0 },
        with_alpha(if aligned { PERFECT_GOLD } else { IVORY }, pulse),
    );

    // The target itself is always visible, faintly.
    let target = if aligned { 0.95 } else { 0.30 };
    r.draw_line(
        Vec2::new(target_center_x, top_y - 5.0),
        Vec2::new(target_center_x, top_y + 9.0),
        2.0,
        with_alpha(PERFECT_GOLD, target),
    );
    draw_diamond(
        r,
        target_center_x,
        top_y + 11.0,
        3.0,
        with_alpha(PERFECT_GOLD, target),
    );

    if aligned {
        // Short gold land marks frame the perfect spot.
        r.draw_line(
            Vec2::new(target_center_x - 20.0, top_y + 3.0),
            Vec2::new(target_center_x - 8.0, top_y + 3.0),
            2.0,
            with_alpha(PERFECT_GOLD, 0.85),
        );
        r.draw_line(
            Vec2::new(target_center_x + 8.0, top_y + 3.0),
            Vec2::new(target_center_x + 20.0, top_y + 3.0),
            2.0,
            with_alpha(PERFECT_GOLD, 0.85),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lighten_moves_towards_white() {
        let c = lighten(Color::new(0.2, 0.4, 0.6, 1.0), 0.5);
        assert!((c.r - 0.6).abs() < 1e-6);
        assert!((c.g - 0.7).abs() < 1e-6);
        assert!((c.b - 0.8).abs() < 1e-6);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn darken_moves_towards_black() {
        let c = darken(Color::new(0.2, 0.4, 0.6, 1.0), 0.5);
        assert!((c.r - 0.1).abs() < 1e-6);
        assert!((c.g - 0.2).abs() < 1e-6);
        assert!((c.b - 0.3).abs() < 1e-6);
    }

    #[test]
    fn lighten_and_darken_clamp() {
        let white = lighten(Color::new(0.9, 0.9, 0.9, 1.0), 10.0);
        assert_eq!(white, WHITE);

        let black = darken(Color::new(0.1, 0.1, 0.1, 1.0), 10.0);
        assert!((black.r).abs() < 1e-6);
        assert!((black.g).abs() < 1e-6);
        assert!((black.b).abs() < 1e-6);
    }

    #[test]
    fn with_alpha_clamps() {
        let c = with_alpha(WHITE, 2.0);
        assert_eq!(c.a, 1.0);
        let c = with_alpha(WHITE, -2.0);
        assert_eq!(c.a, 0.0);
    }
}
