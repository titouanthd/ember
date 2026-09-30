//! Shan shui (ink landscape) background with slow parallax.
//!
//! The background is drawn in 4 layers (from farthest to nearest):
//! 1. Gradient sky (pale grey at the top, straw yellow at the bottom).
//! 2. Distant mountains (parallax 0.15, heavily faded).
//! 3. Mid mountains (parallax 0.30, sharper contours).
//! 4. Near rocks (parallax 0.55, marked ink).
//!
//! No player interaction: the background slowly drifts to the left,
//! giving a contemplative sense of an unrolling scroll.
//!
//! Layers use increasing alpha to recover the "diluted ink" effect of
//! shan shui scrolls: the distant mountain is almost a wash, the near
//! one has more presence but stays translucent — the paper always
//! shows through.
//!
//! The `height_*` and `hash_u32` functions are pure and testable.
//! The `draw_*` functions use macroquad and are not tested.

use glam::Vec2;
use macroquad::prelude::*;

// Compile-time invariants — checked by the compiler, not by tests.
const _: () = assert!(FAR_CHUNK_W > 0.0);
const _: () = assert!(MID_CHUNK_W > 0.0);
const _: () = assert!(NEAR_CHUNK_W > 0.0);
const _: () = assert!(FAR_PARALLAX < MID_PARALLAX);
const _: () = assert!(MID_PARALLAX < NEAR_PARALLAX);
const _: () = assert!(NEAR_PARALLAX <= 1.0);
const _: () = assert!(DRIFT_SPEED > 0.0);
const _: () = assert!(DRIFT_SPEED < 10.0);
// Ink concentrates progressively toward the front: this is what gives
// the depth sensation of a scroll. The near layer must remain
// translucent to let the paper breathe.
const _: () = assert!(COLOR_FAR.a < COLOR_MID.a);
const _: () = assert!(COLOR_MID.a < COLOR_NEAR.a);
const _: () = assert!(COLOR_NEAR.a < 1.0);

// ---------- Default colors (overridden by .env at runtime) ----------

pub const COLOR_PAPER_TOP: Color = Color::new(0.09, 0.075, 0.060, 1.0);
pub const COLOR_PAPER_BOTTOM: Color = Color::new(0.05, 0.040, 0.035, 1.0);

/// Distant wash — barely a ghost on the paper.
pub const COLOR_FAR: Color = Color::new(0.42, 0.35, 0.26, 0.24);
/// Middle wash — a touch more present.
pub const COLOR_MID: Color = Color::new(0.30, 0.24, 0.18, 0.42);
/// Near wash — visible, still translucent.
pub const COLOR_NEAR: Color = Color::new(0.18, 0.14, 0.10, 0.58);
/// Full ink — reserved for crisp outlines (pines, pagodas).
pub const COLOR_INK: Color = Color::new(0.03, 0.025, 0.02, 1.0);
/// Mist interleaved between layers.
pub const COLOR_MIST: Color = Color::new(0.82, 0.74, 0.58, 0.16);

// ---------- Rendering constants ----------

pub const SKY_STRIPS: usize = 24;
pub const FAR_CHUNK_W: f32 = 220.0;
pub const MID_CHUNK_W: f32 = 160.0;
pub const NEAR_CHUNK_W: f32 = 120.0;
pub const FAR_PARALLAX: f32 = 0.15;
pub const MID_PARALLAX: f32 = 0.30;
pub const NEAR_PARALLAX: f32 = 0.55;

/// Automatic background drift speed, in px/s.
pub const DRIFT_SPEED: f32 = 2.5;

// ---------- Deterministic hash ----------

pub fn hash_u32(seed: u32, x: u32) -> u32 {
    let mut h = seed ^ x.wrapping_mul(0x9E37_79B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

// ---------- Height generators (pure, testable) ----------

/// Height of a peak in the distant layer, in pixels.
///
/// Range: `[140, 300]`.
pub fn height_far(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed, chunk as u32);
    140.0 + (h % 160) as f32
}

/// Height of a peak in the mid layer.
///
/// Range: `[90, 200]`.
pub fn height_mid(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed.wrapping_add(0x1234_5678), chunk as u32);
    90.0 + (h % 110) as f32
}

/// Height of a rock in the near layer.
///
/// Range: `[50, 130]`.
pub fn height_near(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed.wrapping_add(0xCAFE_BABE), chunk as u32);
    50.0 + (h % 80) as f32
}

// ---------- Rendering ----------

/// Draws the vertical gradient sky.
pub fn draw_sky(ground_y: f32) {
    let vw = screen_width();
    let strip_h = (ground_y / SKY_STRIPS as f32).max(1.0);
    let top = COLOR_PAPER_TOP;
    let bottom = COLOR_PAPER_BOTTOM;

    for i in 0..SKY_STRIPS {
        let t = i as f32 / (SKY_STRIPS - 1) as f32;
        let c = Color::new(
            top.r * (1.0 - t) + bottom.r * t,
            top.g * (1.0 - t) + bottom.g * t,
            top.b * (1.0 - t) + bottom.b * t,
            1.0,
        );
        draw_rectangle(0.0, i as f32 * strip_h, vw, strip_h + 1.0, c);
    }
}

/// Parameters for one mountain layer.
#[derive(Debug, Clone, Copy)]
struct MountainLayer {
    chunk_w: f32,
    parallax: f32,
    height_fn: fn(u32, i32) -> f32,
    color: Color,
    peaks_per_chunk: i32,
}

/// Draws one layer of stylised shan shui mountains (triangles).
///
/// Each chunk produces `layer.peaks_per_chunk` triangular peaks,
/// regularly spaced across the chunk width.
fn draw_mountain_layer(
    cam_x: f32,
    ground_y: f32,
    seed: u32,
    layer: MountainLayer,
) {
    let vw = screen_width();
    let layer_x = cam_x * layer.parallax;
    let first = (layer_x / layer.chunk_w).floor() as i32 - 1;
    let last = ((layer_x + vw) / layer.chunk_w).ceil() as i32 + 1;

    for i in first..=last {
        let base_x = i as f32 * layer.chunk_w - layer_x;

        for p in 0..layer.peaks_per_chunk {
            let sub_seed = seed
                .wrapping_add(i as u32)
                .wrapping_add((p as u32).wrapping_mul(0x9E37));
            let h = (layer.height_fn)(sub_seed, i);
            let offset_frac = (p as f32 + 0.5) / layer.peaks_per_chunk as f32;
            let peak_x = base_x + layer.chunk_w * offset_frac;
            let base_left = peak_x - layer.chunk_w * 0.6;
            let base_right = peak_x + layer.chunk_w * 0.6;
            let top_y = ground_y - h;

            let p1 = Vec2::new(base_left, ground_y);
            let p2 = Vec2::new(peak_x, top_y);
            let p3 = Vec2::new(base_right, ground_y);
            draw_triangle(p1, p2, p3, layer.color);
        }
    }
}

/// Draws the three mountain layers plus interleaved mist.
pub fn draw_mountains(cam_x: f32, ground_y: f32, seed: u32) {
    // Distant layer — 1 peak per chunk, very wide.
    draw_mountain_layer(
        cam_x,
        ground_y,
        seed,
        MountainLayer {
            chunk_w: FAR_CHUNK_W,
            parallax: FAR_PARALLAX,
            height_fn: height_far,
            color: COLOR_FAR,
            peaks_per_chunk: 1,
        },
    );

    draw_mist(ground_y, 0.45);

    // Mid layer — 1 peak per chunk.
    draw_mountain_layer(
        cam_x,
        ground_y,
        seed,
        MountainLayer {
            chunk_w: MID_CHUNK_W,
            parallax: MID_PARALLAX,
            height_fn: height_mid,
            color: COLOR_MID,
            peaks_per_chunk: 1,
        },
    );

    draw_mist(ground_y, 0.65);

    // Near layer — 2 peaks per chunk, closer together.
    draw_mountain_layer(
        cam_x,
        ground_y,
        seed,
        MountainLayer {
            chunk_w: NEAR_CHUNK_W,
            parallax: NEAR_PARALLAX,
            height_fn: height_near,
            color: COLOR_NEAR,
            peaks_per_chunk: 2,
        },
    );
}

/// Draws a horizontal band of mist above the ground.
fn draw_mist(ground_y: f32, height_frac: f32) {
    let vw = screen_width();
    let band_h = 120.0 * height_frac;
    let y = ground_y - band_h * 0.5;
    draw_rectangle(0.0, y, vw, band_h, COLOR_MIST);
}

/// Draws a stylised pine (trunk + 3 stacked triangles).
pub fn draw_pine(x: f32, y: f32, h: f32, color: Color) {
    let trunk_w = h * 0.08;
    let trunk_h = h * 0.20;
    draw_rectangle(x - trunk_w * 0.5, y - trunk_h, trunk_w, trunk_h, color);

    let base_y = y - trunk_h;
    for i in 0..3 {
        let layer_h = h * 0.30;
        let layer_w = h * 0.40 * (1.0 - i as f32 * 0.15);
        let layer_y = base_y - i as f32 * h * 0.20;
        let p1 = Vec2::new(x - layer_w * 0.5, layer_y);
        let p2 = Vec2::new(x, layer_y - layer_h);
        let p3 = Vec2::new(x + layer_w * 0.5, layer_y);
        draw_triangle(p1, p2, p3, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Hash ----------

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_u32(1, 100), hash_u32(1, 100));
        assert_eq!(hash_u32(0xDEAD, 5), hash_u32(0xDEAD, 5));
    }

    #[test]
    fn hash_changes_with_seed() {
        assert_ne!(hash_u32(1, 100), hash_u32(2, 100));
    }

    #[test]
    fn hash_changes_with_input() {
        assert_ne!(hash_u32(1, 100), hash_u32(1, 101));
    }

    // ---------- Heights ----------

    #[test]
    fn height_far_in_range() {
        for chunk in -10..10 {
            let h = height_far(1, chunk);
            assert!((140.0..=300.0).contains(&h), "far h = {h}");
        }
    }

    #[test]
    fn height_mid_in_range() {
        for chunk in -10..10 {
            let h = height_mid(1, chunk);
            assert!((90.0..=200.0).contains(&h), "mid h = {h}");
        }
    }

    #[test]
    fn height_near_in_range() {
        for chunk in -10..10 {
            let h = height_near(1, chunk);
            assert!((50.0..=130.0).contains(&h), "near h = {h}");
        }
    }

    #[test]
    fn heights_vary_by_chunk() {
        let mut heights: Vec<i32> = (0..20).map(|c| height_far(1, c) as i32).collect();
        heights.sort_unstable();
        heights.dedup();
        assert!(heights.len() >= 5, "only {} unique heights", heights.len());
    }

    #[test]
    fn heights_are_deterministic() {
        for chunk in 0..10 {
            assert_eq!(height_far(42, chunk), height_far(42, chunk));
            assert_eq!(height_mid(42, chunk), height_mid(42, chunk));
            assert_eq!(height_near(42, chunk), height_near(42, chunk));
        }
    }

    #[test]
    fn heights_differ_across_layers() {
        // Because of distinct seed offsets, the 3 layers must not
        // produce the same value for the same chunk.
        let f = height_far(1, 5);
        let m = height_mid(1, 5);
        let n = height_near(1, 5);
        // At least one difference.
        assert!(f != m || m != n || f != n);
    }

    // ---------- Constants ----------
    //
    // The increasing-alpha invariants are checked at compile time
    // (see the `const _: () = assert!(...)` block at the top of the
    // module): Clippy rejects `assert!` on constants inside a test.
    // This test keeps only the range-[0, 1] check per channel, which
    // iterates over an array — not constant as far as Clippy knows.
    #[test]
    fn colors_are_in_unit_range() {
        for c in [
            COLOR_PAPER_TOP,
            COLOR_PAPER_BOTTOM,
            COLOR_FAR,
            COLOR_MID,
            COLOR_NEAR,
            COLOR_INK,
            COLOR_MIST,
        ] {
            for v in [c.r, c.g, c.b, c.a] {
                assert!((0.0..=1.0).contains(&v));
            }
        }
    }
}