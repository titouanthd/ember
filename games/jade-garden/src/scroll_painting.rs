//! Shan shui (ink landscape) background with slow parallax.
//!
//! The background is drawn in 4 layers (from farthest to nearest):
//! 1. Gradient sky (paper top → paper bottom).
//! 2. Distant mountains (parallax 0.15, heavily faded).
//! 3. Mid mountains (parallax 0.30, sharper contours).
//! 4. Near rocks (parallax 0.55, marked ink, occasional pines).
//!
//! Mountains are not simple triangles. Each chunk generates a small
//! set of peaks at jittered x positions and varied heights; the
//! silhouette is the **maximum** of triangular contributions from
//! those peaks, which produces an asymmetric natural ridge. Pines on
//! the near layer reinforce the shan shui feel.
//!
//! No player interaction: the background slowly drifts to the left,
//! giving a contemplative sense of an unrolling scroll.
//!
//! Layers use increasing alpha to recover the "diluted ink" effect
//! of shan shui scrolls: the distant mountain is almost a wash, the
//! near one has more presence but stays translucent — the paper
//! always shows through.
//!
//! ## Chapter palettes
//!
//! Each chapter renders the same mountains (same seed) under a
//! different palette — the garden is the same, but the light changes:
//!
//! - Chapter I — *Arrival*: cool damp dawn, moss-covered stone.
//! - Chapter II — *Silence of Stones*: cold mineral memory, starker
//!   contrast, lavender-grey ink.
//! - Chapter III — *Breath of Spring*: warm amber, first bird, gold
//!   mist.
//! - Chapter IV — *Awakening*: jade-green glow, mystical, breathable
//!   emerald.
//!
//! The `height_*`, `hash_u32`, `build_peaks` and `ridge_height_at`
//! functions are pure and testable. The `draw_*` functions use
//! macroquad and are not tested.

use glam::Vec2;
use macroquad::prelude::*;

// ---------- Fixed rendering constants ----------

pub const SKY_STRIPS: usize = 24;
pub const FAR_CHUNK_W: f32 = 260.0;
pub const MID_CHUNK_W: f32 = 180.0;
pub const NEAR_CHUNK_W: f32 = 140.0;
pub const FAR_PARALLAX: f32 = 0.15;
pub const MID_PARALLAX: f32 = 0.30;
pub const NEAR_PARALLAX: f32 = 0.55;

/// Automatic background drift speed, in px/s.
pub const DRIFT_SPEED: f32 = 2.5;

/// Full ink — reserved for crisp outlines, not chapter-dependent.
pub const COLOR_INK: Color = Color::new(0.03, 0.025, 0.02, 1.0);

/// Number of horizontal strips used to fill each ridge polygon.
const RIDGE_STRIPS: usize = 40;
/// Number of horizontal strips used to draw the mist gradient.
const MIST_STRIPS: usize = 12;

// Compile-time invariants on the fixed constants.
const _: () = assert!(FAR_CHUNK_W > 0.0);
const _: () = assert!(MID_CHUNK_W > 0.0);
const _: () = assert!(NEAR_CHUNK_W > 0.0);
const _: () = assert!(FAR_PARALLAX < MID_PARALLAX);
const _: () = assert!(MID_PARALLAX < NEAR_PARALLAX);
const _: () = assert!(NEAR_PARALLAX <= 1.0);
const _: () = assert!(DRIFT_SPEED > 0.0);
const _: () = assert!(DRIFT_SPEED < 10.0);

// ---------- Chapter palettes ----------

/// Colour set for one chapter's background.
///
/// Every palette must satisfy (checked at compile time):
/// - increasing alpha toward the front (`far.a < mid.a < near.a`),
///   which gives the depth sensation of a shan shui scroll;
/// - `near.a < 1.0`, so the paper always shows through the ink.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub paper_top: Color,
    pub paper_bottom: Color,
    pub far: Color,
    pub mid: Color,
    pub near: Color,
    pub mist: Color,
}

impl Palette {
    /// Returns the palette for a given chapter index (0-3).
    /// Out-of-range values fall back to chapter 0.
    pub const fn for_chapter(chapter: u32) -> Self {
        match chapter {
            0 => Self::ARRIVAL,
            1 => Self::SILENCE_OF_STONES,
            2 => Self::BREATH_OF_SPRING,
            3 => Self::AWAKENING,
            _ => Self::ARRIVAL,
        }
    }

    /// Chapter I — cool damp dawn, moss-covered stone, discovery.
    pub const ARRIVAL: Self = Self {
        paper_top:    Color::new(0.075, 0.085, 0.100, 1.0),
        paper_bottom: Color::new(0.040, 0.048, 0.058, 1.0),
        far:          Color::new(0.40, 0.46, 0.44, 0.22),
        mid:          Color::new(0.26, 0.32, 0.30, 0.42),
        near:         Color::new(0.14, 0.19, 0.18, 0.58),
        mist:         Color::new(0.72, 0.82, 0.78, 0.16),
    };

    /// Chapter II — mineral silence, cold memory, starker contrast.
    pub const SILENCE_OF_STONES: Self = Self {
        paper_top:    Color::new(0.080, 0.078, 0.090, 1.0),
        paper_bottom: Color::new(0.036, 0.034, 0.044, 1.0),
        far:          Color::new(0.36, 0.35, 0.42, 0.20),
        mid:          Color::new(0.22, 0.21, 0.27, 0.46),
        near:         Color::new(0.10, 0.10, 0.13, 0.64),
        mist:         Color::new(0.68, 0.70, 0.78, 0.12),
    };

    /// Chapter III — warm spring, first bird, gold light.
    pub const BREATH_OF_SPRING: Self = Self {
        paper_top:    Color::new(0.105, 0.080, 0.058, 1.0),
        paper_bottom: Color::new(0.058, 0.042, 0.030, 1.0),
        far:          Color::new(0.52, 0.44, 0.30, 0.22),
        mid:          Color::new(0.36, 0.29, 0.19, 0.42),
        near:         Color::new(0.22, 0.16, 0.10, 0.58),
        mist:         Color::new(0.92, 0.82, 0.62, 0.20),
    };

    /// Chapter IV — jade awakening, mysticism, emerald glow.
    pub const AWAKENING: Self = Self {
        paper_top:    Color::new(0.058, 0.088, 0.076, 1.0),
        paper_bottom: Color::new(0.028, 0.048, 0.040, 1.0),
        far:          Color::new(0.36, 0.54, 0.46, 0.26),
        mid:          Color::new(0.22, 0.38, 0.30, 0.46),
        near:         Color::new(0.10, 0.22, 0.16, 0.60),
        mist:         Color::new(0.72, 0.92, 0.80, 0.22),
    };

    /// Compile-time invariants.
    const fn assert_valid(self) {
        assert!(self.far.a < self.mid.a);
        assert!(self.mid.a < self.near.a);
        assert!(self.near.a < 1.0);
    }
}

const _: () = Palette::ARRIVAL.assert_valid();
const _: () = Palette::SILENCE_OF_STONES.assert_valid();
const _: () = Palette::BREATH_OF_SPRING.assert_valid();
const _: () = Palette::AWAKENING.assert_valid();

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

/// Base height of a chunk in the distant layer, in pixels.
/// Range: `[140, 300]`.
pub fn height_far(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed, chunk as u32);
    140.0 + (h % 160) as f32
}

/// Base height of a chunk in the mid layer.
/// Range: `[90, 200]`.
pub fn height_mid(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed.wrapping_add(0x1234_5678), chunk as u32);
    90.0 + (h % 110) as f32
}

/// Base height of a chunk in the near layer.
/// Range: `[50, 130]`.
pub fn height_near(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed.wrapping_add(0xCAFE_BABE), chunk as u32);
    50.0 + (h % 80) as f32
}

// ---------- Ridge generation (pure, testable) ----------

/// A peak is a `(x_fraction, height_px)` pair. `x_fraction` is in
/// `[0, 1]` along the chunk's width; `height_px` is above the horizon.
pub type Peak = (f32, f32);

/// Builds `n` peaks for the given chunk, forming an asymmetric ridge.
///
/// One peak is designated "main" and takes the full `base_h`. The
/// others are shorter (40-80% of `base_h`) and jittered around their
/// base positions. Deterministic in `seed` and `chunk`.
pub fn build_peaks(seed: u32, chunk: i32, base_h: f32, n: u32) -> Vec<Peak> {
    if n == 0 {
        return Vec::new();
    }

    // Reinterpret the chunk index as u32 (deterministic across runs),
    // then use wrapping arithmetic — negative chunk indices must not
    // overflow during hashing.
    let cu = chunk as u32;

    // Pick which index is the main peak (deterministic).
    let main_idx =
        (hash_u32(seed, cu.wrapping_mul(5).wrapping_add(99)) % n) as usize;

    let mut peaks = Vec::with_capacity(n as usize);
    for k in 0..n {
        let salt_x = cu.wrapping_mul(17).wrapping_add(k.wrapping_mul(3)).wrapping_add(1);
        let salt_h = salt_x.wrapping_add(1);

        let h_x = hash_u32(seed, salt_x);
        let h_h = hash_u32(seed, salt_h);

        // Base x position spread evenly along the chunk, with jitter.
        let base_frac = (k as f32 + 0.5) / n as f32;
        let jitter = ((h_x % 40) as f32 / 100.0 - 0.20) / n as f32;
        let x = (base_frac + jitter).clamp(0.05, 0.95);

        // Main peak gets full base_h; others 40-80%.
        let y = if k as usize == main_idx {
            base_h
        } else {
            base_h * (0.40 + (h_h % 40) as f32 / 100.0)
        };

        peaks.push((x, y));
    }
    peaks
}

/// Height of the ridge at normalized position `t ∈ [0, 1]`.
///
/// Uses the **maximum** of triangular contributions from each peak
/// (not the sum): overlapping peaks form a silhouette, they don't
/// stack. `half_width` is the normalized half-width of a peak's
/// footprint, in the same units as `t`.
pub fn ridge_height_at(peaks: &[Peak], t: f32, half_width: f32) -> f32 {
    if half_width <= 0.0 {
        return 0.0;
    }
    let mut h = 0.0f32;
    for &(x0, h0) in peaks {
        let d = (t - x0).abs();
        if d >= half_width {
            continue;
        }
        // Parabolic falloff: rounded top, sharp base.
        let n = d / half_width;
        let factor = 1.0 - n * n;
        h = h.max(h0 * factor);
    }
    h
}

// ---------- Rendering ----------

/// Draws the vertical gradient sky for the given palette.
pub fn draw_sky(ground_y: f32, palette: Palette) {
    let vw = screen_width();
    let strip_h = (ground_y / SKY_STRIPS as f32).max(1.0);
    let top = palette.paper_top;
    let bottom = palette.paper_bottom;

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
    /// Number of peaks generated per chunk.
    peaks_per_chunk: u32,
    /// Normalized half-width of each peak (fraction of chunk width).
    peak_half_width: f32,
    /// If true, draw small pines on some near-layer peaks.
    with_pines: bool,
}

/// Draws one layer of stylised shan shui mountains.
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

    let strip_w = layer.chunk_w / RIDGE_STRIPS as f32;

    for i in first..=last {
        let base_x = i as f32 * layer.chunk_w - layer_x;
        let base_h = (layer.height_fn)(seed, i);
        let peaks = build_peaks(seed, i, base_h, layer.peaks_per_chunk);

        // Silhouette: one vertical strip per column, at the ridge height.
        for s in 0..RIDGE_STRIPS {
            let t = (s as f32 + 0.5) / RIDGE_STRIPS as f32;
            let h = ridge_height_at(&peaks, t, layer.peak_half_width);
            if h < 0.5 {
                continue;
            }
            let x = base_x + s as f32 * strip_w;
            draw_rectangle(x, ground_y - h, strip_w + 0.6, h, layer.color);
        }

        // Pines on some peaks of the near layer.
        if layer.with_pines {
            let pine_color = Color::new(
                layer.color.r * 0.55,
                layer.color.g * 0.55,
                layer.color.b * 0.55,
                (layer.color.a * 1.5).min(0.92),
            );
            for (k, &(x_frac, h_peak)) in peaks.iter().enumerate() {
                if h_peak < 40.0 {
                    continue;
                }
                let h_pine = hash_u32(
                    seed,
                    (i as u32)
                        .wrapping_mul(31)
                        .wrapping_add((k as u32) * 7),
                );
                if !h_pine.is_multiple_of(3) {
                    continue;
                }
                let px = base_x + x_frac * layer.chunk_w;
                let py = ground_y - h_peak;
                draw_pine(px, py, h_peak * 0.22, pine_color);
            }
        }
    }
}

/// Draws the three mountain layers plus interleaved mist.
pub fn draw_mountains(cam_x: f32, ground_y: f32, seed: u32, palette: Palette) {
    // Distant layer — many small rounded peaks, no pines.
    draw_mountain_layer(
        cam_x,
        ground_y,
        seed,
        MountainLayer {
            chunk_w: FAR_CHUNK_W,
            parallax: FAR_PARALLAX,
            height_fn: height_far,
            color: palette.far,
            peaks_per_chunk: 4,
            peak_half_width: 0.30,
            with_pines: false,
        },
    );

    draw_mist(ground_y, 0.45, palette.mist);

    // Mid layer — fewer, slightly taller peaks.
    draw_mountain_layer(
        cam_x,
        ground_y,
        seed,
        MountainLayer {
            chunk_w: MID_CHUNK_W,
            parallax: MID_PARALLAX,
            height_fn: height_mid,
            color: palette.mid,
            peaks_per_chunk: 3,
            peak_half_width: 0.22,
            with_pines: false,
        },
    );

    draw_mist(ground_y, 0.70, palette.mist);

    // Near layer — 3 distinct peaks per chunk, occasional pines.
    draw_mountain_layer(
        cam_x,
        ground_y,
        seed,
        MountainLayer {
            chunk_w: NEAR_CHUNK_W,
            parallax: NEAR_PARALLAX,
            height_fn: height_near,
            color: palette.near,
            peaks_per_chunk: 3,
            peak_half_width: 0.18,
            with_pines: true,
        },
    );
}

/// Draws a horizontal band of mist with a soft bell-shaped alpha
/// gradient (transparent at the top and bottom edges).
fn draw_mist(ground_y: f32, height_frac: f32, color: Color) {
    let vw = screen_width();
    let band_h = 140.0 * height_frac;
    let y0 = ground_y - band_h * 0.55;
    let strip_h = band_h / MIST_STRIPS as f32;

    for i in 0..MIST_STRIPS {
        let t = i as f32 / (MIST_STRIPS - 1) as f32;
        // Bell curve: 0 at the edges, 1 in the middle.
        let bell = (1.0 - (2.0 * t - 1.0).abs()).powf(0.7);
        let c = Color::new(color.r, color.g, color.b, color.a * bell);
        draw_rectangle(0.0, y0 + i as f32 * strip_h, vw, strip_h + 0.5, c);
    }
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

    // ---------- Base heights ----------

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
        let f = height_far(1, 5);
        let m = height_mid(1, 5);
        let n = height_near(1, 5);
        assert!(f != m || m != n || f != n);
    }

    // ---------- build_peaks ----------

    #[test]
    fn build_peaks_is_deterministic() {
        assert_eq!(
            build_peaks(1, 5, 100.0, 3),
            build_peaks(1, 5, 100.0, 3),
        );
    }

    #[test]
    fn build_peaks_varies_by_chunk() {
        assert_ne!(
            build_peaks(1, 5, 100.0, 3),
            build_peaks(1, 6, 100.0, 3),
        );
    }

    #[test]
    fn build_peaks_varies_by_seed() {
        assert_ne!(
            build_peaks(1, 5, 100.0, 3),
            build_peaks(2, 5, 100.0, 3),
        );
    }

    #[test]
    fn build_peaks_returns_requested_count() {
        for n in 1..=6 {
            let p = build_peaks(42, 7, 100.0, n);
            assert_eq!(p.len(), n as usize, "n = {n}");
        }
    }

    #[test]
    fn build_peaks_empty_on_zero_count() {
        assert!(build_peaks(1, 1, 100.0, 0).is_empty());
    }

    #[test]
    fn build_peaks_x_within_chunk() {
        for chunk in -5..5 {
            for n in 2..=5 {
                let p = build_peaks(42, chunk, 100.0, n);
                for &(x, _) in &p {
                    assert!(
                        (0.0..=1.0).contains(&x),
                        "chunk {chunk}, n {n}: x = {x}"
                    );
                }
            }
        }
    }

    #[test]
    fn build_peaks_heights_positive() {
        for chunk in -5..5 {
            let p = build_peaks(42, chunk, 100.0, 3);
            for &(_, h) in &p {
                assert!(h > 0.0, "chunk {chunk}: h = {h}");
            }
        }
    }

    #[test]
    fn build_peaks_has_exactly_one_main_peak() {
        for chunk in -5..5 {
            let p = build_peaks(42, chunk, 100.0, 4);
            let count = p.iter().filter(|&&(_, h)| (h - 100.0).abs() < 1e-3).count();
            assert_eq!(count, 1, "chunk {chunk}: expected 1 main peak, got {count}");
        }
    }

    #[test]
    fn build_peaks_secondary_below_main() {
        let p = build_peaks(42, 3, 100.0, 4);
        for &(_, h) in &p {
            assert!(h <= 100.0 + 1e-3);
        }
    }

    // ---------- ridge_height_at ----------

    #[test]
    fn ridge_height_zero_on_empty_peaks() {
        assert_eq!(ridge_height_at(&[], 0.5, 0.3), 0.0);
    }

    #[test]
    fn ridge_height_max_at_peak_center() {
        let peaks = vec![(0.5, 100.0)];
        assert!((ridge_height_at(&peaks, 0.5, 0.3) - 100.0).abs() < 1e-3);
    }

    #[test]
    fn ridge_height_zero_beyond_half_width() {
        let peaks = vec![(0.5, 100.0)];
        assert_eq!(ridge_height_at(&peaks, 0.0, 0.3), 0.0);
        assert_eq!(ridge_height_at(&peaks, 1.0, 0.3), 0.0);
    }

    #[test]
    fn ridge_height_uses_max_not_sum() {
        // Two identical peaks shouldn't stack.
        let single = vec![(0.5, 100.0)];
        let double = vec![(0.5, 100.0), (0.5, 100.0)];
        assert!(
            (ridge_height_at(&single, 0.5, 0.3)
                - ridge_height_at(&double, 0.5, 0.3))
            .abs()
                < 1e-3
        );
    }

    #[test]
    fn ridge_height_is_non_negative() {
        let peaks = build_peaks(42, 3, 100.0, 4);
        for i in 0..=50 {
            let t = i as f32 / 50.0;
            let h = ridge_height_at(&peaks, t, 0.25);
            assert!(h >= 0.0, "t = {t}, h = {h}");
        }
    }

    #[test]
    fn ridge_height_below_max_peak_height() {
        let peaks = build_peaks(42, 3, 100.0, 4);
        let max_h = peaks.iter().map(|&(_, h)| h).fold(0.0f32, f32::max);
        for i in 0..=50 {
            let t = i as f32 / 50.0;
            let h = ridge_height_at(&peaks, t, 0.25);
            assert!(h <= max_h + 1e-3);
        }
    }

    #[test]
    fn ridge_height_zero_when_half_width_zero() {
        let peaks = vec![(0.5, 100.0)];
        assert_eq!(ridge_height_at(&peaks, 0.5, 0.0), 0.0);
    }

    // ---------- Palettes ----------

    #[test]
    fn palettes_are_in_unit_range() {
        for palette in [
            Palette::ARRIVAL,
            Palette::SILENCE_OF_STONES,
            Palette::BREATH_OF_SPRING,
            Palette::AWAKENING,
        ] {
            for c in [
                palette.paper_top,
                palette.paper_bottom,
                palette.far,
                palette.mid,
                palette.near,
                palette.mist,
            ] {
                for v in [c.r, c.g, c.b, c.a] {
                    assert!((0.0..=1.0).contains(&v));
                }
            }
        }
    }

    #[test]
    fn every_chapter_has_a_palette() {
        for chapter in 0..4 {
            let p = Palette::for_chapter(chapter);
            assert!(p.paper_top.r + p.paper_top.g + p.paper_top.b > 0.05);
        }
    }

    #[test]
    fn out_of_range_chapter_falls_back_to_arrival() {
        let p = Palette::for_chapter(99);
        assert_eq!(p.paper_top.r, Palette::ARRIVAL.paper_top.r);
        assert_eq!(p.paper_top.g, Palette::ARRIVAL.paper_top.g);
    }

    #[test]
    fn palettes_differ_between_chapters() {
        let a = Palette::for_chapter(0);
        let b = Palette::for_chapter(2);
        let diff = (a.paper_top.r - b.paper_top.r).abs()
            + (a.paper_top.g - b.paper_top.g).abs()
            + (a.paper_top.b - b.paper_top.b).abs();
        assert!(diff > 0.05, "chapters I and III look too similar");
    }

    #[test]
    fn each_palette_respects_depth_invariants() {
        for palette in [
            Palette::ARRIVAL,
            Palette::SILENCE_OF_STONES,
            Palette::BREATH_OF_SPRING,
            Palette::AWAKENING,
        ] {
            assert!(palette.far.a < palette.mid.a);
            assert!(palette.mid.a < palette.near.a);
            assert!(palette.near.a < 1.0);
        }
    }

    // ---------- Constants ----------

    #[test]
    fn fixed_colors_are_in_unit_range() {
        for v in [COLOR_INK.r, COLOR_INK.g, COLOR_INK.b, COLOR_INK.a] {
            assert!((0.0..=1.0).contains(&v));
        }
    }
}