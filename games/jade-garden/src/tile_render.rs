//! Procedural rendering of jade tiles.
//!
//! Every jade is drawn as a **distinct silhouette** so it reads at a
//! glance even without colour:
//!
//! - Bai  (白玉) → round pebble
//! - Bi   (碧玉) → hexagonal cut with facet lines
//! - Qing (青玉) → diamond with a 4-spoke star
//! - Hong (红玉) → octagon with a soft inner ring
//! - Huang(黄玉) → pentagon with facet lines
//! - Mo   (墨玉) → squared block with two subtle veins
//!
//! Layers per tile:
//! 1. Drop shadow (offset silhouette).
//! 2. Dark base fill (whole shape).
//! 3. Mid base fill (scaled 0.92, nudged up).
//! 4. Top sheen (scaled 0.52, nudged up further).
//! 5. Pulsing core glow.
//! 6. Shape-specific facets / veins.
//! 7. White rim.
//! 8. Specular sparkle at the top-left.
//!
//! All `draw_*` functions use macroquad and are not unit-tested.
//! Pure helpers are testable headless.

use glam::Vec2;
use macroquad::prelude::*;

use crate::components::{Jade, JadeShape};

/// Logical tile size in pixels.
pub const TILE_SIZE: f32 = 72.0;
/// Horizontal and vertical spacing between two tiles.
pub const TILE_GAP: f32 = 6.0;
/// Tile size plus its gap (no grid).
pub const TILE_STRIDE: f32 = TILE_SIZE + TILE_GAP;

/// Number of veins generated per jade.
pub const VEINS_PER_TILE: usize = 3;
/// Number of points per vein (quadratic Bézier: 3 points).
pub const VEIN_POINTS: usize = 3;

/// Converts a base colour (0-255) into a macroquad `Color`.
pub fn jade_color_base(jade: Jade) -> Color {
    let (r, g, b) = jade.base_rgb();
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

/// Brighter colour (× 1.3), for highlights.
pub fn jade_color_light(jade: Jade) -> Color {
    let (r, g, b) = jade.base_rgb();
    let scale = 1.3;
    Color::new(
        (r as f32 / 255.0 * scale).min(1.0),
        (g as f32 / 255.0 * scale).min(1.0),
        (b as f32 / 255.0 * scale).min(1.0),
        1.0,
    )
}

/// Darker colour (× 0.7), for shadows.
pub fn jade_color_dark(jade: Jade) -> Color {
    let (r, g, b) = jade.base_rgb();
    Color::new(
        r as f32 / 255.0 * 0.7,
        g as f32 / 255.0 * 0.7,
        b as f32 / 255.0 * 0.7,
        1.0,
    )
}

/// (sides, rotation) of the polygon used to draw the jade's silhouette.
pub fn jade_shape_params(jade: Jade) -> (u8, f32) {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_6, FRAC_PI_8};
    match jade.shape() {
        JadeShape::Round => (16, 0.0),
        JadeShape::Hexagon => (6, FRAC_PI_6),
        JadeShape::Diamond => (4, 0.0),
        JadeShape::Octagon => (8, FRAC_PI_8),
        JadeShape::Pentagon => (5, -FRAC_PI_2),
        JadeShape::Square => (4, FRAC_PI_4),
    }
}

/// Deterministic hash (murmur-inspired), borrowed from `ember-wars`.
fn hash_u32(seed: u32, x: u32) -> u32 {
    let mut h = seed ^ x.wrapping_mul(0x9E37_79B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

/// Generates `VEINS_PER_TILE` quadratic Bézier veins for one tile.
///
/// Each vein is a triplet `[start, control, end]` of coordinates
/// **local** to the square `[0, size] × [0, size]` (top-left anchored).
/// Veins are generated deterministically from `seed`.
pub fn generate_veins(seed: u32, size: f32) -> Vec<[Vec2; VEIN_POINTS]> {
    let mut veins = Vec::with_capacity(VEINS_PER_TILE);
    for i in 0..VEINS_PER_TILE {
        let h1 = hash_u32(seed.wrapping_add(i as u32 * 0x9E37), 0x01);
        let h2 = hash_u32(seed.wrapping_add(i as u32 * 0x9E37), 0x02);
        let h3 = hash_u32(seed.wrapping_add(i as u32 * 0x9E37), 0x03);
        let h4 = hash_u32(seed.wrapping_add(i as u32 * 0x9E37), 0x04);
        let h5 = hash_u32(seed.wrapping_add(i as u32 * 0x9E37), 0x05);

        let start_x = (h1 % 10000) as f32 / 10000.0 * size * 0.8 + size * 0.1;
        let start_y = (h2 % 10000) as f32 / 10000.0 * size * 0.3 + size * 0.1;

        let ctrl_x = (h3 % 10000) as f32 / 10000.0 * size * 0.8 + size * 0.1;
        let ctrl_y = (h4 % 10000) as f32 / 10000.0 * size * 0.6 + size * 0.2;

        let end_x = (h5 % 10000) as f32 / 10000.0 * size * 0.8 + size * 0.1;
        let end_y = (h1 % 10000) as f32 / 10000.0 * size * 0.4 + size * 0.6;

        veins.push([
            Vec2::new(start_x, start_y),
            Vec2::new(ctrl_x, ctrl_y),
            Vec2::new(end_x, end_y),
        ]);
    }
    veins
}

/// Draws a jade tile at `(x, y)` (top-left of its bounding square).
///
/// - `size`: bounding square side in pixels.
/// - `scale`: size multiplier (1.0 = normal, 0.0 = invisible).
/// - `alpha`: global opacity.
/// - `vein_seed`: deterministic seed for facet/vein details.
pub fn draw_tile(
    jade: Jade,
    x: f32,
    y: f32,
    size: f32,
    scale: f32,
    alpha: f32,
    vein_seed: u32,
) {
    if scale <= 0.01 || alpha <= 0.01 {
        return;
    }

    let base = jade_color_base(jade);
    let dark = jade_color_dark(jade);
    let light = jade_color_light(jade);
    let deep = Color::new(
        base.r * 0.42,
        base.g * 0.42,
        base.b * 0.42,
        1.0,
    );

    let s = size * scale;
    let cx = x + size * 0.5;
    let cy = y + size * 0.5;
    let r = s * 0.5;
    let (sides, rot) = jade_shape_params(jade);

    // 1. Soft multi-layer drop shadow — 3 offset copies, decreasing alpha.
    for i in 0..3 {
        let f = i as f32;
        let off = s * (0.035 + f * 0.025);
        let a = 0.30 * alpha * (1.0 - f * 0.28);
        draw_poly(
            cx + off,
            cy + off,
            sides,
            r * (1.01 + f * 0.015),
            rot,
            Color::new(0.0, 0.0, 0.0, a),
        );
    }

    // 2. Deep rim (near-black copy of the shape).
    draw_poly(cx, cy, sides, r, rot, Color::new(deep.r, deep.g, deep.b, alpha));

    // 3. Body — nested polygons, each smaller and shifted up, giving a
    //    proper vertical gradient plus an implied bottom bevel.
    const LAYERS: usize = 7;
    for i in 0..LAYERS {
        let t = i as f32 / (LAYERS - 1) as f32;
        let rr = r * (0.94 - t * 0.36);
        let dy = -r * (0.01 + t * 0.20);
        let c = Color::new(
            dark.r * (1.0 - t) + base.r * t,
            dark.g * (1.0 - t) + base.g * t,
            dark.b * (1.0 - t) + base.b * t,
            alpha,
        );
        draw_poly(cx, cy + dy, sides, rr, rot, c);
    }

    // 4. Top sheen — a small bright polygon near the top of the body.
    let sheen = Color::new(light.r, light.g, light.b, 0.55 * alpha);
    draw_poly(cx - r * 0.05, cy - r * 0.30, sides, r * 0.36, rot, sheen);

    // 5. Soft inner glow — a very faint, larger bright polygon just
    //    inside the body, gives the "polished" feel.
    draw_poly(
        cx,
        cy - r * 0.04,
        sides,
        r * 0.86,
        rot,
        Color::new(1.0, 1.0, 1.0, 0.06 * alpha),
    );

    // 6. Pulsing core glow (subtle).
    let t_anim = get_time() as f32;
    let phase = (vein_seed & 0xFF) as f32 * 0.05;
    let core_r = r * 0.32 * (0.85 + 0.15 * (t_anim * 2.4 + phase).sin());
    let core_a = 0.16 * alpha * (0.7 + 0.3 * (t_anim * 1.8 + phase).sin());
    draw_circle(cx, cy, core_r, Color::new(1.0, 1.0, 1.0, core_a));

    // 7. Shape-specific facets and patterns.
    draw_jade_inner_lines(jade, cx, cy, r, sides, rot, light, alpha, vein_seed);

    // 8. Inner light rim (thin, just inside the shape edge).
    draw_poly_lines(
        cx,
        cy - r * 0.03,
        sides,
        r * 0.90,
        rot,
        1.0,
        Color::new(light.r, light.g, light.b, 0.35 * alpha),
    );

    // 9. Bright thin outer rim.
    draw_poly_lines(
        cx,
        cy,
        sides,
        r,
        rot,
        1.3 * scale,
        Color::new(1.0, 1.0, 1.0, 0.45 * alpha),
    );

    // 10. Main specular — bright dot with a soft halo, top-left.
    let sparkle_r = r * 0.11;
    let sx = cx - r * 0.30;
    let sy = cy - r * 0.36;
    draw_circle(sx, sy, sparkle_r * 2.2, Color::new(1.0, 1.0, 1.0, 0.12 * alpha));
    draw_circle(sx, sy, sparkle_r * 1.3, Color::new(1.0, 1.0, 1.0, 0.35 * alpha));
    draw_circle(sx, sy, sparkle_r, Color::new(1.0, 1.0, 1.0, 0.95 * alpha));

    // 11. Secondary sparkle — faint, bottom-right.
    let s2x = cx + r * 0.34;
    let s2y = cy + r * 0.30;
    draw_circle(s2x, s2y, sparkle_r * 0.55, Color::new(1.0, 1.0, 1.0, 0.45 * alpha));
}

/// Draws the shape-specific facet lines / veins / patterns.
#[allow(clippy::too_many_arguments)]
fn draw_jade_inner_lines(
    jade: Jade,
    cx: f32,
    cy: f32,
    r: f32,
    sides: u8,
    rot: f32,
    light: Color,
    alpha: f32,
    vein_seed: u32,
) {
    let c = Color::new(light.r, light.g, light.b, 0.60 * alpha);
    let c_soft = Color::new(light.r, light.g, light.b, 0.30 * alpha);

    match jade.shape() {
        JadeShape::Round => {
            // Concentric inner rings + a soft curved vein.
            draw_poly_lines(cx, cy, sides, r * 0.60, rot, 1.0, c_soft);
            draw_poly_lines(cx, cy, sides, r * 0.30, rot, 1.0, c_soft);
            let veins = generate_veins(vein_seed, r * 2.0);
            for vein in &veins {
                let [start, ctrl, end] = *vein;
                let s_pt = Vec2::new(cx + start.x - r, cy + start.y - r);
                let c_pt = Vec2::new(cx + ctrl.x - r, cy + ctrl.y - r);
                let e_pt = Vec2::new(cx + end.x - r, cy + end.y - r);
                const SEGMENTS: usize = 10;
                let mut prev = s_pt;
                for i in 1..=SEGMENTS {
                    let t = i as f32 / SEGMENTS as f32;
                    let om = 1.0 - t;
                    let p = Vec2::new(
                        om * om * s_pt.x + 2.0 * om * t * c_pt.x + t * t * e_pt.x,
                        om * om * s_pt.y + 2.0 * om * t * c_pt.y + t * t * e_pt.y,
                    );
                    draw_line(prev.x, prev.y, p.x, p.y, 1.0, c_soft);
                    prev = p;
                }
            }
        }
        JadeShape::Hexagon => {
            // Full facet lines connecting opposite vertices (3 lines).
            for i in 0..3u8 {
                let a = rot + (i as f32) * std::f32::consts::TAU / 6.0;
                let (sin_a, cos_a) = a.sin_cos();
                draw_line(
                    cx + cos_a * r * 0.86,
                    cy + sin_a * r * 0.86,
                    cx - cos_a * r * 0.86,
                    cy - sin_a * r * 0.86,
                    1.0,
                    c,
                );
            }
        }
        JadeShape::Diamond => {
            // Long diagonal rays from the center to each vertex.
            for i in 0..4u8 {
                let a = (i as f32) * std::f32::consts::FRAC_PI_2;
                let (sin_a, cos_a) = a.sin_cos();
                draw_line(
                    cx,
                    cy,
                    cx + cos_a * r * 0.80,
                    cy + sin_a * r * 0.80,
                    1.2,
                    c,
                );
            }
            // Small bright centre star.
            draw_poly_lines(cx, cy, 4, r * 0.18, 0.0, 1.0, c_soft);
        }
        JadeShape::Octagon => {
            // Inner ring + 4 cardinal accents.
            draw_poly_lines(cx, cy, sides, r * 0.68, rot, 1.0, c);
            for i in 0..4u8 {
                let a = rot + (i as f32) * std::f32::consts::FRAC_PI_2;
                let (sin_a, cos_a) = a.sin_cos();
                draw_line(
                    cx + cos_a * r * 0.68,
                    cy + sin_a * r * 0.68,
                    cx + cos_a * r * 0.88,
                    cy + sin_a * r * 0.88,
                    1.2,
                    c,
                );
            }
        }
        JadeShape::Pentagon => {
            // Flower: lines from the centre to each vertex.
            for i in 0..5u8 {
                let a = rot + (i as f32) * std::f32::consts::TAU / 5.0;
                let (sin_a, cos_a) = a.sin_cos();
                draw_line(
                    cx,
                    cy,
                    cx + cos_a * r * 0.80,
                    cy + sin_a * r * 0.80,
                    1.0,
                    c,
                );
            }
            // Small inner hexagon (like a coin stamp).
            draw_poly_lines(cx, cy, 6, r * 0.16, 0.0, 1.0, c_soft);
        }
        JadeShape::Square => {
            // Inkstone: subtle horizontal striations.
            let h = vein_seed as f32 * 0.0017;
            for i in 0..3u8 {
                let y_off = r * (-0.30 + i as f32 * 0.30 + h.sin() * 0.05);
                draw_line(
                    cx - r * 0.62,
                    cy + y_off,
                    cx + r * 0.62,
                    cy + y_off + h.cos() * r * 0.06,
                    0.9,
                    c_soft,
                );
            }
            // One bright diagonal accent.
            draw_line(
                cx - r * 0.38,
                cy - r * 0.38,
                cx + r * 0.38,
                cy + r * 0.38,
                1.1,
                c,
            );
        }
    }
}

/// Draws a shape-matched golden halo behind the tile (selection).
pub fn draw_tile_glow(jade: Jade, x: f32, y: f32, size: f32, intensity: f32) {
    if intensity <= 0.01 {
        return;
    }
    let cx = x + size * 0.5;
    let cy = y + size * 0.5;
    let (sides, rot) = jade_shape_params(jade);

    draw_poly(
        cx,
        cy,
        sides,
        size * 0.68,
        rot,
        Color::new(1.0, 0.92, 0.55, 0.15 * intensity),
    );
    draw_poly(
        cx,
        cy,
        sides,
        size * 0.56,
        rot,
        Color::new(1.0, 0.92, 0.55, 0.32 * intensity),
    );
}

/// Draws a selection indicator: 4 accented corners around the tile.
pub fn draw_selection_corners(x: f32, y: f32, size: f32, intensity: f32) {
    if intensity <= 0.01 {
        return;
    }
    let corner_len = size * 0.20;
    let thickness = 2.5;
    let c = Color::new(1.0, 0.95, 0.55, 0.85 * intensity);

    draw_line(x, y, x + corner_len, y, thickness, c);
    draw_line(x, y, x, y + corner_len, thickness, c);
    draw_line(x + size, y, x + size - corner_len, y, thickness, c);
    draw_line(x + size, y, x + size, y + corner_len, thickness, c);
    draw_line(x, y + size, x + corner_len, y + size, thickness, c);
    draw_line(x, y + size, x, y + size - corner_len, thickness, c);
    draw_line(x + size, y + size, x + size - corner_len, y + size, thickness, c);
    draw_line(x + size, y + size, x + size, y + size - corner_len, thickness, c);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Colors ----------

    #[test]
    fn jade_color_base_in_unit_range() {
        for j in Jade::ALL {
            let c = jade_color_base(j);
            for v in [c.r, c.g, c.b, c.a] {
                assert!((0.0..=1.0).contains(&v), "{j:?}: {v}");
            }
        }
    }

    #[test]
    fn jade_color_light_is_brighter_than_base() {
        for j in Jade::ALL {
            let base = jade_color_base(j);
            let light = jade_color_light(j);
            let base_sum = base.r + base.g + base.b;
            let light_sum = light.r + light.g + light.b;
            assert!(light_sum >= base_sum, "{j:?}: {base_sum} → {light_sum}");
        }
    }

    #[test]
    fn jade_color_dark_is_darker_than_base() {
        for j in Jade::ALL {
            let base = jade_color_base(j);
            let dark = jade_color_dark(j);
            let base_sum = base.r + base.g + base.b;
            let dark_sum = dark.r + dark.g + dark.b;
            assert!(dark_sum <= base_sum, "{j:?}: {base_sum} → {dark_sum}");
        }
    }

    // ---------- Shape params ----------

    #[test]
    fn every_jade_has_a_valid_polygon() {
        for j in Jade::ALL {
            let (sides, rot) = jade_shape_params(j);
            assert!(sides >= 3, "{j:?} has {sides} sides");
            assert!(rot.is_finite(), "{j:?} has non-finite rotation");
        }
    }

    #[test]
    fn jade_shapes_give_distinct_silhouettes() {
        // Not exhaustive (different rotations could coincide), but
        // catches accidental duplicates in the table.
        use std::collections::HashSet;
        let mut set: HashSet<(u8, i32)> = HashSet::new();
        for j in Jade::ALL {
            let (sides, rot) = jade_shape_params(j);
            set.insert((sides, (rot * 1000.0) as i32));
        }
        assert_eq!(set.len(), Jade::ALL.len());
    }

    // ---------- generate_veins ----------

    #[test]
    fn generate_veins_returns_expected_count() {
        let veins = generate_veins(42, 100.0);
        assert_eq!(veins.len(), VEINS_PER_TILE);
    }

    #[test]
    fn generate_veins_is_deterministic() {
        let a = generate_veins(12345, 100.0);
        let b = generate_veins(12345, 100.0);
        assert_eq!(a, b);
    }

    #[test]
    fn generate_veins_varies_with_seed() {
        let a = generate_veins(1, 100.0);
        let b = generate_veins(2, 100.0);
        assert_ne!(a, b);
    }

    #[test]
    fn generate_veins_points_within_bounds() {
        for seed in [1, 42, 12345, 0xDEAD_BEEF] {
            let veins = generate_veins(seed, 100.0);
            for vein in &veins {
                for p in vein {
                    assert!((0.0..=100.0).contains(&p.x), "seed {seed}: x = {}", p.x);
                    assert!((0.0..=100.0).contains(&p.y), "seed {seed}: y = {}", p.y);
                }
            }
        }
    }

    #[test]
    fn generate_veins_scales_with_size() {
        let small = generate_veins(42, 50.0);
        let big = generate_veins(42, 200.0);
        for (s_vein, b_vein) in small.iter().zip(big.iter()) {
            for (sp, bp) in s_vein.iter().zip(b_vein.iter()) {
                assert!((bp.x - sp.x * 4.0).abs() < 1e-3);
                assert!((bp.y - sp.y * 4.0).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn generate_veins_returns_three_points_per_vein() {
        let veins = generate_veins(1, 100.0);
        for vein in &veins {
            assert_eq!(vein.len(), VEIN_POINTS);
            assert_eq!(VEIN_POINTS, 3);
        }
    }

    // ---------- Hash ----------

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_u32(42, 100), hash_u32(42, 100));
    }

    #[test]
    fn hash_changes_with_seed() {
        assert_ne!(hash_u32(42, 100), hash_u32(43, 100));
    }

    #[test]
    fn hash_changes_with_input() {
        assert_ne!(hash_u32(42, 100), hash_u32(42, 101));
    }

    // ---------- Constants ----------

    #[test]
    fn tile_stride_is_size_plus_gap() {
        assert!((TILE_STRIDE - (TILE_SIZE + TILE_GAP)).abs() < 1e-6);
    }
}