//! Jade Garden feedback: radial rays and the vignette pulse, layered on top
//! of the shared `ember_stdlib::fx::Juice` (ADR-046).

use ember::prelude::*;
use ember_stdlib::fx::{Juice, JuiceConfig, Particle};
use std::ops::{Deref, DerefMut};

pub const DEATH_PARTICLE_LIFE: f32 = 0.65;

/// One thin ray in a radial burst.
#[derive(Debug, Clone)]
pub struct RadialRay {
    pub pos: Vec2,
    pub angle: f32,
    pub length: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
}

/// Full-screen corner darkening. Only one active at a time.
#[derive(Debug, Clone)]
pub struct Vignette {
    pub life: f32,
    pub max_life: f32,
    pub intensity: f32,
    pub color: Color,
}

/// Shared juice plus rays and a vignette. Derefs to [`Juice`].
#[derive(Debug)]
pub struct Effects {
    base: Juice,
    pub rays: Vec<RadialRay>,
    pub vignette: Option<Vignette>,
}

impl Default for Effects {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Effects {
    type Target = Juice;

    fn deref(&self) -> &Juice {
        &self.base
    }
}

impl DerefMut for Effects {
    fn deref_mut(&mut self) -> &mut Juice {
        &mut self.base
    }
}

impl Effects {
    pub fn new() -> Self {
        Self {
            base: Juice::with_config(JuiceConfig {
                particle_gravity: 320.0,
                particle_drag: 2.5,
                particle_life: DEATH_PARTICLE_LIFE,
                text_rise_speed: 45.0,
                text_life: 1.0,
            }),
            rays: Vec::new(),
            vignette: None,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        for r in &mut self.rays {
            r.life -= dt;
        }
        self.rays.retain(|r| r.life > 0.0);

        if let Some(v) = self.vignette.as_mut() {
            v.life -= dt;
            if v.life <= 0.0 {
                self.vignette = None;
            }
        }

        self.base.tick(dt);
    }

    /// Spawns `count` rays evenly distributed around `pos`.
    pub fn spawn_radial_burst(
        &mut self,
        pos: Vec2,
        count: u32,
        length: f32,
        color: Color,
        seed: u32,
    ) {
        let mut s = seed;
        for _ in 0..count {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            let angle = (s >> 16) as f32 / 65535.0 * std::f32::consts::TAU;
            self.rays.push(RadialRay {
                pos,
                angle,
                length,
                life: 0.35,
                max_life: 0.35,
                color,
            });
        }
    }

    /// Pulses the full-screen vignette. Replaces any active pulse.
    pub fn vignette_pulse(&mut self, intensity: f32, duration: f32) {
        self.vignette = Some(Vignette {
            life: duration,
            max_life: duration,
            intensity,
            color: Color::new(0.02, 0.01, 0.0, 1.0),
        });
    }

    pub fn spawn_jade_shatter(&mut self, pos: Vec2, color: Color, count: u32, rng_seed: u32) {
        let mut seed = rng_seed;
        for _ in 0..count {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let angle = (seed >> 16) as f32 / 65535.0 * std::f32::consts::TAU;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let speed = 80.0 + ((seed >> 16) as f32 / 65535.0) * 140.0;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let size = 2.5 + ((seed >> 16) as f32 / 65535.0) * 3.0;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let spin = -4.0 + ((seed >> 16) as f32 / 65535.0) * 8.0;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let sides = if seed.is_multiple_of(2) { 3 } else { 4 };

            self.particles.push(Particle {
                pos,
                vel: Vec2::new(angle.cos(), angle.sin()) * speed,
                life: DEATH_PARTICLE_LIFE,
                max_life: DEATH_PARTICLE_LIFE,
                color,
                size,
                sides,
                rotation: 0.0,
                spin,
            });
        }
    }
}

// ---------- Drawing ----------

pub fn draw_radial_rays(gfx: &mut dyn RenderingServer, juice: &Effects, shake: Vec2) {
    for r in &juice.rays {
        let t = 1.0 - (r.life / r.max_life).clamp(0.0, 1.0);
        let alpha = (1.0 - t).powf(1.2) * r.color.a;
        let len = r.length * (0.3 + 0.7 * t);
        let dx = r.angle.cos() * len;
        let dy = r.angle.sin() * len;
        let c = Color::new(r.color.r, r.color.g, r.color.b, alpha);
        gfx.draw_line(
            Vec2::new(r.pos.x + shake.x, r.pos.y + shake.y),
            Vec2::new(r.pos.x + shake.x + dx, r.pos.y + shake.y + dy),
            2.0,
            c,
        );
    }
}

pub fn draw_vignette(gfx: &mut dyn RenderingServer, juice: &Effects) {
    let Some(v) = &juice.vignette else {
        return;
    };
    let t = (v.life / v.max_life).clamp(0.0, 1.0);
    let intensity = v.intensity * t;
    let vw = gfx.viewport().w;
    let vh = gfx.viewport().h;

    const BANDS: usize = 8;
    let max_band = vh * 0.28;
    let band_h = max_band / BANDS as f32;

    for i in 0..BANDS {
        let frac = i as f32 / (BANDS - 1) as f32;
        // Outer band (i=0) is darkest; inner bands fade to transparent.
        let a = intensity * (1.0 - frac).powf(1.6) * 0.55;
        let c = Color::new(v.color.r, v.color.g, v.color.b, a);

        let y_top = i as f32 * band_h;
        let y_bot = vh - (i as f32 + 1.0) * band_h;

        gfx.draw_rect(Rect::new(0.0, y_top, vw, band_h + 0.5), c);
        gfx.draw_rect(Rect::new(0.0, y_bot, vw, band_h + 0.5), c);

        let x_left = i as f32 * band_h;
        let x_right = vw - (i as f32 + 1.0) * band_h;
        gfx.draw_rect(Rect::new(x_left, 0.0, band_h + 0.5, vh), c);
        gfx.draw_rect(Rect::new(x_right, 0.0, band_h + 0.5, vh), c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// macroquad's `WHITE`, which the tests used.
    const WHITE: Color = Color::WHITE;

    #[test]
    fn jade_shatter_creates_particles() {
        let mut j = Effects::new();
        j.spawn_jade_shatter(Vec2::ZERO, WHITE, 12, 12345);
        assert_eq!(j.particles.len(), 12);
    }

    #[test]
    fn jade_shatter_is_deterministic() {
        let mut a = Effects::new();
        let mut b = Effects::new();
        a.spawn_jade_shatter(Vec2::ZERO, WHITE, 8, 42);
        b.spawn_jade_shatter(Vec2::ZERO, WHITE, 8, 42);
        for (pa, pb) in a.particles.iter().zip(b.particles.iter()) {
            assert_eq!(pa.vel, pb.vel);
            assert_eq!(pa.sides, pb.sides);
        }
    }

    #[test]
    fn jade_shatter_sides_are_triangles_or_quads() {
        let mut j = Effects::new();
        j.spawn_jade_shatter(Vec2::ZERO, WHITE, 20, 7);
        for p in &j.particles {
            assert!(p.sides == 3 || p.sides == 4);
        }
    }

    // ---------- Shockwaves ----------

    #[test]
    fn radial_burst_creates_expected_ray_count() {
        let mut j = Effects::new();
        j.spawn_radial_burst(Vec2::ZERO, 8, 60.0, WHITE, 42);
        assert_eq!(j.rays.len(), 8);
    }

    #[test]
    fn radial_burst_is_deterministic() {
        let mut a = Effects::new();
        let mut b = Effects::new();
        a.spawn_radial_burst(Vec2::ZERO, 6, 60.0, WHITE, 7);
        b.spawn_radial_burst(Vec2::ZERO, 6, 60.0, WHITE, 7);
        for (ra, rb) in a.rays.iter().zip(b.rays.iter()) {
            assert!((ra.angle - rb.angle).abs() < 1e-6);
        }
    }

    #[test]
    fn rays_expire() {
        let mut j = Effects::new();
        j.spawn_radial_burst(Vec2::ZERO, 6, 60.0, WHITE, 7);
        j.tick(0.5);
        assert!(j.rays.is_empty());
    }

    // ---------- Vignette ----------

    #[test]
    fn vignette_pulse_sets_vignette() {
        let mut j = Effects::new();
        j.vignette_pulse(0.6, 0.3);
        assert!(j.vignette.is_some());
        assert_eq!(j.vignette.as_ref().unwrap().intensity, 0.6);
    }

    #[test]
    fn vignette_expires() {
        let mut j = Effects::new();
        j.vignette_pulse(0.6, 0.3);
        j.tick(0.5);
        assert!(j.vignette.is_none());
    }

    #[test]
    fn vignette_pulse_replaces_previous() {
        let mut j = Effects::new();
        j.vignette_pulse(0.3, 0.3);
        j.vignette_pulse(0.7, 0.3);
        assert_eq!(j.vignette.as_ref().unwrap().intensity, 0.7);
    }
}
