// games/pagoda/src/backdrop.rs
//! Chinese night backdrop: an ink-wash sky, a paper moon, drifting
//! cloud bands and layered mountain silhouettes.
//!
//! Everything is procedural and cheap: a stack of horizontal quads for
//! the gradient, a few dozen circles, and two sawtooth ridges. The
//! mountains drift down slowly as the tower climbs, which sells height
//! without needing a camera-following world.

use crate::config::{MOON, background_color};
use ember_core::Color;
use ember_core::math::Rect;
use ember_servers::RenderingServer;
use glam::{Vec2, vec2};
use std::f32::consts::TAU;

const SKY_BANDS: usize = 32;
const STAR_COUNT: usize = 52;

/// `time` is the seconds since the game started (was macroquad's
/// `get_time()`), for the twinkle and the drifting moon cloud.
pub fn draw(
    r: &mut dyn RenderingServer,
    camera_y: f32,
    height: u32,
    screen_w: f32,
    screen_h: f32,
    time: f32,
) {
    draw_sky(r, height, screen_w, screen_h);
    draw_stars(r, camera_y, screen_w, screen_h, time);
    draw_moon(r, camera_y, screen_w, screen_h, time);
    draw_clouds(r, camera_y, screen_w, screen_h);
    draw_mountains(r, camera_y, screen_w, screen_h);
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn draw_sky(r: &mut dyn RenderingServer, height: u32, screen_w: f32, screen_h: f32) {
    let base = background_color(height);
    let top = Color::new(
        (base.r + 0.045).min(1.0),
        (base.g + 0.035).min(1.0),
        (base.b + 0.070).min(1.0),
        1.0,
    );
    let bottom = Color::new(base.r * 0.52, base.g * 0.52, base.b * 0.60, 1.0);

    let band_h = screen_h / SKY_BANDS as f32;
    for i in 0..SKY_BANDS {
        let t = i as f32 / (SKY_BANDS - 1) as f32;
        r.draw_rect(
            Rect::new(0.0, i as f32 * band_h, screen_w, band_h + 1.0),
            mix(top, bottom, t),
        );
    }
}

/// Stable pseudo-random value in `[0, 1)`.
fn hash01(n: u32) -> f32 {
    let mut x = n.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    x = ((x >> ((x >> 28) + 4)) ^ x).wrapping_mul(277_803_737);
    x = (x >> 22) ^ x;
    (x & 0xFFFF) as f32 / 65_535.0
}

fn star_position(index: u32, camera_y: f32, screen_w: f32, screen_h: f32) -> Vec2 {
    let x = hash01(index.wrapping_mul(2).wrapping_add(1)) * screen_w;
    let raw_y = hash01(index.wrapping_mul(2).wrapping_add(2)) * screen_h;
    // Very distant parallax: stars drift down slowly as the tower rises.
    let y = (raw_y - camera_y * 0.015).rem_euclid(screen_h);
    vec2(x, y)
}

fn draw_stars(r: &mut dyn RenderingServer, camera_y: f32, screen_w: f32, screen_h: f32, time: f32) {
    for i in 0..STAR_COUNT as u32 {
        let pos = star_position(i, camera_y, screen_w, screen_h);
        let size = 0.7 + hash01(i.wrapping_add(500)) * 1.2;
        let phase = hash01(i.wrapping_add(900)) * TAU;
        let twinkle = 0.72 + 0.28 * (time * 1.6 + phase).sin();
        let alpha = (0.22 + hash01(i.wrapping_add(300)) * 0.45) * twinkle;

        r.draw_circle(
            Vec2::new(pos.x, pos.y),
            size,
            Color::new(0.92, 0.90, 0.78, alpha),
        );

        if i % 11 == 0 {
            let arm = size * 2.8;
            let c = Color::new(0.95, 0.92, 0.78, alpha * 0.55);
            r.draw_line(
                Vec2::new(pos.x - arm, pos.y),
                Vec2::new(pos.x + arm, pos.y),
                1.0,
                c,
            );
            r.draw_line(
                Vec2::new(pos.x, pos.y - arm),
                Vec2::new(pos.x, pos.y + arm),
                1.0,
                c,
            );
        }
    }
}

fn draw_moon(r: &mut dyn RenderingServer, camera_y: f32, screen_w: f32, screen_h: f32, time: f32) {
    let x = screen_w * 0.76;
    let y = screen_h * 0.20 + (-camera_y * 0.01).clamp(0.0, 24.0);
    let radius = 54.0;

    // Soft halo: many faint rings avoid visible banding.
    for i in 0..16 {
        let age = i as f32 / 15.0;
        let halo = radius + 4.0 + age * 116.0;
        let alpha = 0.012 * (1.0 - age).powf(1.3);
        r.draw_circle(
            Vec2::new(x, y),
            halo,
            Color::new(MOON.r, MOON.g, MOON.b, alpha),
        );
    }

    r.draw_circle(
        Vec2::new(x, y),
        radius,
        Color::new(MOON.r, MOON.g, MOON.b, 0.92),
    );
    r.draw_circle_outline(
        Vec2::new(x, y),
        radius + 1.0,
        1.0,
        Color::new(1.0, 0.85, 0.45, 0.16),
    );

    // A single ink cloud crossing the moon, the classic painted motif.
    let drift = (time * 4.0).rem_euclid(screen_w + 380.0) - 190.0;
    for i in 0..10 {
        let t = i as f32 / 9.0;
        let fade = (1.0 - (2.0 * t - 1.0).abs()).powf(1.4);
        let w = 260.0 * fade.sqrt();
        r.draw_rect(
            Rect::new(x - w / 2.0 + drift * 0.30, y + 12.0 + t * 24.0, w, 3.0),
            Color::new(0.10, 0.10, 0.16, 0.09 * fade),
        );
    }
}

/// Soft horizontal mist: a stack of thin bands whose alpha peaks in the
/// middle. Reads as haze rather than a row of circles.
fn draw_mist_band(
    r: &mut dyn RenderingServer,
    base_y: f32,
    screen_w: f32,
    strength: f32,
    height: f32,
) {
    let steps = 14;
    let line_h = height / steps as f32;

    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let fade = (1.0 - (2.0 * t - 1.0).abs()).powf(1.5);
        r.draw_rect(
            Rect::new(
                0.0,
                base_y - height / 2.0 + i as f32 * line_h,
                screen_w,
                line_h + 1.0,
            ),
            Color::new(0.70, 0.78, 0.82, strength * fade),
        );
    }
}

fn draw_clouds(r: &mut dyn RenderingServer, camera_y: f32, screen_w: f32, screen_h: f32) {
    let drift = (-camera_y * 0.035).clamp(0.0, 46.0);

    draw_mist_band(r, screen_h * 0.34 + drift, screen_w, 0.022, 34.0);
    draw_mist_band(r, screen_h * 0.47 + drift * 1.3, screen_w, 0.018, 42.0);
    draw_mist_band(r, screen_h * 0.63 + drift * 1.7, screen_w, 0.014, 50.0);
}

fn draw_mountains(r: &mut dyn RenderingServer, camera_y: f32, screen_w: f32, screen_h: f32) {
    let far_drift = (-camera_y * 0.035).clamp(0.0, 44.0);
    let near_drift = (-camera_y * 0.055).clamp(0.0, 62.0);

    // Far ridge: blue ink, softened with a mist band at its base.
    let far_base = screen_h * 0.80 + far_drift;
    draw_ridge(
        r,
        far_base,
        150.0,
        Color::new(0.085, 0.115, 0.155, 1.0),
        0xB10C,
        screen_w,
    );
    r.draw_rect(
        Rect::new(0.0, far_base - 18.0, screen_w, 28.0),
        Color::new(0.14, 0.18, 0.22, 0.055),
    );

    // Near ridge: near-black ink, grounded at the bottom edge.
    let near_base = screen_h * 0.95 + near_drift;
    draw_ridge(
        r,
        near_base,
        185.0,
        Color::new(0.030, 0.045, 0.070, 1.0),
        0x7EA2,
        screen_w,
    );

    // Low fog so the tower does not look pasted onto the ridge.
    r.draw_rect(
        Rect::new(0.0, near_base - 12.0, screen_w, 24.0),
        Color::new(0.10, 0.14, 0.18, 0.05),
    );
}

/// Jagged ink-wash silhouette: overlapping triangles plus a solid base.
///
/// Peak spacing, height and width all come from the hash, so the ridge
/// stays irregular instead of turning into a regular sawtooth.
fn draw_ridge(
    r: &mut dyn RenderingServer,
    base_y: f32,
    peak_height: f32,
    color: Color,
    seed: u32,
    screen_w: f32,
) {
    let mut i = 0u32;
    let mut x = -140.0;

    while x < screen_w + 140.0 {
        let h = peak_height * (0.30 + 0.70 * hash01(seed.wrapping_add(i)));
        let left = 55.0 + 70.0 * hash01(seed.wrapping_add(i).wrapping_add(31));
        let right = 55.0 + 70.0 * hash01(seed.wrapping_add(i).wrapping_add(67));
        let peak_x = x + (hash01(seed.wrapping_add(i).wrapping_add(113)) - 0.5) * 46.0;

        r.draw_triangle(
            vec2(peak_x - left, base_y),
            vec2(peak_x + right, base_y),
            vec2(peak_x, base_y - h),
            color,
        );

        x += 70.0 + 90.0 * hash01(seed.wrapping_add(i).wrapping_add(151));
        i += 1;
    }

    r.draw_rect(Rect::new(0.0, base_y, screen_w, 420.0), color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic_and_in_range() {
        for i in 0..64 {
            let a = hash01(i);
            let b = hash01(i);
            assert_eq!(a, b);
            assert!((0.0..1.0).contains(&a), "hash01({i}) = {a}");
        }
    }

    #[test]
    fn stars_stay_on_screen_at_any_camera_height() {
        for camera_y in [0.0, -500.0, -5_000.0, -250_000.0] {
            for i in 0..STAR_COUNT as u32 {
                let p = star_position(i, camera_y, 800.0, 600.0);
                assert!((0.0..=800.0).contains(&p.x));
                assert!((0.0..=600.0).contains(&p.y));
            }
        }
    }

    #[test]
    fn mix_interpolates_and_clamps() {
        let a = Color::new(0.0, 0.0, 0.0, 1.0);
        let b = Color::new(1.0, 0.5, 0.25, 1.0);
        let mid = mix(a, b, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-6);
        assert!((mid.g - 0.25).abs() < 1e-6);
        assert_eq!(mix(a, b, 5.0), b);
    }
}
