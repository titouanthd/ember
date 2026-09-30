//! Visual feedback: particles, floating texts, shockwaves, radial
//! bursts, screen shake, vignette, flash.

use glam::Vec2;
use macroquad::prelude::*;

pub const DEATH_PARTICLE_LIFE: f32 = 0.65;
pub const SHOCKWAVE_START_RADIUS: f32 = 8.0;

#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
    pub size: f32,
    pub sides: u8,
    pub rotation: f32,
    pub spin: f32,
}

#[derive(Debug, Clone)]
pub struct FloatingText {
    pub pos: Vec2,
    pub text: String,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
    pub font_size: f32,
}

/// Expanding ring emitted from a match.
#[derive(Debug, Clone)]
pub struct Shockwave {
    pub pos: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub end_radius: f32,
    pub color: Color,
    pub thickness: f32,
}

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

#[derive(Debug, Clone)]
pub struct ScreenShake {
    pub intensity: f32,
    pub remaining: f32,
    pub duration: f32,
    pub seed: u32,
}

impl ScreenShake {
    pub fn new() -> Self {
        Self {
            intensity: 0.0,
            remaining: 0.0,
            duration: 0.0,
            seed: 0xDEAD_BEEF,
        }
    }

    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        if intensity > self.intensity || self.remaining <= 0.0 {
            self.intensity = intensity;
            self.remaining = duration;
            self.duration = duration;
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if self.remaining > 0.0 {
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }

    pub fn offset(&mut self) -> Vec2 {
        if self.remaining <= 0.0 || self.duration <= 0.0 {
            return Vec2::ZERO;
        }
        let t = self.remaining / self.duration;
        let amp = self.intensity * t;

        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let sx = ((self.seed >> 16) as f32 / 65535.0) - 0.5;
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let sy = ((self.seed >> 16) as f32 / 65535.0) - 0.5;

        Vec2::new(sx * 2.0 * amp, sy * 2.0 * amp)
    }
}

impl Default for ScreenShake {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Flash {
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Debug, Default)]
pub struct Juice {
    pub particles: Vec<Particle>,
    pub floating_texts: Vec<FloatingText>,
    pub shockwaves: Vec<Shockwave>,
    pub rays: Vec<RadialRay>,
    pub vignette: Option<Vignette>,
    pub screen_shake: ScreenShake,
    pub flashes: Vec<Flash>,
}

impl Juice {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        for p in self.particles.iter_mut() {
            p.vel.y += 320.0 * dt;
            p.vel *= 1.0 - (2.5 * dt).min(1.0);
            p.pos += p.vel * dt;
            p.rotation += p.spin * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        for t in self.floating_texts.iter_mut() {
            t.pos.y -= 45.0 * dt;
            t.life -= dt;
        }
        self.floating_texts.retain(|t| t.life > 0.0);

        for s in self.shockwaves.iter_mut() {
            s.life -= dt;
        }
        self.shockwaves.retain(|s| s.life > 0.0);

        for r in self.rays.iter_mut() {
            r.life -= dt;
        }
        self.rays.retain(|r| r.life > 0.0);

        if let Some(v) = self.vignette.as_mut() {
            v.life -= dt;
            if v.life <= 0.0 {
                self.vignette = None;
            }
        }

        self.screen_shake.tick(dt);

        for f in self.flashes.iter_mut() {
            f.life -= dt;
        }
        self.flashes.retain(|f| f.life > 0.0);
    }

    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.screen_shake.trigger(intensity, duration);
    }

    pub fn flash(&mut self, color: Color, duration: f32) {
        self.flashes.push(Flash {
            color,
            life: duration,
            max_life: duration,
        });
    }

    pub fn spawn_floating_text(
        &mut self,
        pos: Vec2,
        text: impl Into<String>,
        color: Color,
        font_size: f32,
    ) {
        self.floating_texts.push(FloatingText {
            pos,
            text: text.into(),
            life: 1.0,
            max_life: 1.0,
            color,
            font_size,
        });
    }

    /// Spawns an expanding ring from `pos`.
    pub fn spawn_shockwave(
        &mut self,
        pos: Vec2,
        end_radius: f32,
        color: Color,
        duration: f32,
        thickness: f32,
    ) {
        self.shockwaves.push(Shockwave {
            pos,
            life: duration,
            max_life: duration,
            end_radius,
            color,
            thickness,
        });
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

    /// Polygonal particle burst (jade shatter).
    pub fn spawn_jade_shatter(&mut self, pos: Vec2, color: Color, count: u32, rng_seed: u32) {
        let mut seed = rng_seed;
        for i in 0..count {
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

            let _ = i;
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

pub fn draw_particles(juice: &Juice, shake: Vec2) {
    for p in &juice.particles {
        let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
        let c = Color::new(p.color.r, p.color.g, p.color.b, p.color.a * alpha);
        let screen = p.pos + shake;
        draw_poly(
            screen.x,
            screen.y,
            p.sides,
            p.size * alpha,
            p.rotation,
            c,
        );
    }
}

pub fn draw_floating_texts(juice: &Juice, shake: Vec2) {
    for t in &juice.floating_texts {
        let alpha = (t.life / t.max_life).clamp(0.0, 1.0);
        let c = Color::new(t.color.r, t.color.g, t.color.b, t.color.a * alpha);
        let screen = t.pos + shake;
        let dim = measure_text(&t.text, None, t.font_size as u16, 1.0);
        draw_text(
            &t.text,
            screen.x - dim.width * 0.5,
            screen.y,
            t.font_size,
            c,
        );
    }
}

pub fn draw_shockwaves(juice: &Juice, shake: Vec2) {
    for s in &juice.shockwaves {
        let t = 1.0 - (s.life / s.max_life).clamp(0.0, 1.0);
        let r = SHOCKWAVE_START_RADIUS + (s.end_radius - SHOCKWAVE_START_RADIUS) * t;
        let alpha = (1.0 - t).powf(1.5) * s.color.a;
        let c = Color::new(s.color.r, s.color.g, s.color.b, alpha);
        draw_circle_lines(
            s.pos.x + shake.x,
            s.pos.y + shake.y,
            r,
            s.thickness,
            c,
        );
    }
}

pub fn draw_radial_rays(juice: &Juice, shake: Vec2) {
    for r in &juice.rays {
        let t = 1.0 - (r.life / r.max_life).clamp(0.0, 1.0);
        let alpha = (1.0 - t).powf(1.2) * r.color.a;
        let len = r.length * (0.3 + 0.7 * t);
        let dx = r.angle.cos() * len;
        let dy = r.angle.sin() * len;
        let c = Color::new(r.color.r, r.color.g, r.color.b, alpha);
        draw_line(
            r.pos.x + shake.x,
            r.pos.y + shake.y,
            r.pos.x + shake.x + dx,
            r.pos.y + shake.y + dy,
            2.0,
            c,
        );
    }
}

pub fn draw_vignette(juice: &Juice) {
    let Some(v) = &juice.vignette else { return; };
    let t = (v.life / v.max_life).clamp(0.0, 1.0);
    let intensity = v.intensity * t;
    let vw = screen_width();
    let vh = screen_height();

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

        draw_rectangle(0.0, y_top, vw, band_h + 0.5, c);
        draw_rectangle(0.0, y_bot, vw, band_h + 0.5, c);

        let x_left = i as f32 * band_h;
        let x_right = vw - (i as f32 + 1.0) * band_h;
        draw_rectangle(x_left, 0.0, band_h + 0.5, vh, c);
        draw_rectangle(x_right, 0.0, band_h + 0.5, vh, c);
    }
}

pub fn draw_flashes(juice: &Juice) {
    for f in &juice.flashes {
        let alpha = (f.life / f.max_life).clamp(0.0, 1.0) * 0.35;
        let c = Color::new(f.color.r, f.color.g, f.color.b, alpha);
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_juice_is_empty() {
        let j = Juice::new();
        assert!(j.particles.is_empty());
        assert!(j.floating_texts.is_empty());
        assert!(j.shockwaves.is_empty());
        assert!(j.rays.is_empty());
        assert!(j.vignette.is_none());
        assert!(j.flashes.is_empty());
        assert_eq!(j.screen_shake.remaining, 0.0);
    }

    #[test]
    fn particle_tick_reduces_life() {
        let mut j = Juice::new();
        j.particles.push(Particle {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            life: 0.5,
            max_life: 0.5,
            color: WHITE,
            size: 3.0,
            sides: 3,
            rotation: 0.0,
            spin: 0.0,
        });
        j.tick(0.2);
        assert!((j.particles[0].life - 0.3).abs() < 1e-6);
    }

    #[test]
    fn particle_dies_after_life() {
        let mut j = Juice::new();
        j.particles.push(Particle {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            life: 0.1,
            max_life: 0.1,
            color: WHITE,
            size: 3.0,
            sides: 3,
            rotation: 0.0,
            spin: 0.0,
        });
        j.tick(0.5);
        assert!(j.particles.is_empty());
    }

    #[test]
    fn particle_rotation_advances() {
        let mut j = Juice::new();
        j.particles.push(Particle {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            life: 1.0,
            max_life: 1.0,
            color: WHITE,
            size: 3.0,
            sides: 3,
            rotation: 0.0,
            spin: 2.0,
        });
        j.tick(0.5);
        assert!((j.particles[0].rotation - 1.0).abs() < 1e-4);
    }

    #[test]
    fn floating_text_rises() {
        let mut j = Juice::new();
        j.spawn_floating_text(Vec2::new(100.0, 100.0), "test", WHITE, 16.0);
        let y0 = j.floating_texts[0].pos.y;
        j.tick(0.1);
        assert!(j.floating_texts[0].pos.y < y0);
    }

    #[test]
    fn floating_text_dies_after_life() {
        let mut j = Juice::new();
        j.spawn_floating_text(Vec2::ZERO, "test", WHITE, 16.0);
        j.tick(1.5);
        assert!(j.floating_texts.is_empty());
    }

    #[test]
    fn shake_stronger_replaces_weaker() {
        let mut j = Juice::new();
        j.shake(3.0, 0.2);
        j.shake(8.0, 0.3);
        assert_eq!(j.screen_shake.intensity, 8.0);
    }

    #[test]
    fn shake_weaker_does_not_replace_stronger() {
        let mut j = Juice::new();
        j.shake(8.0, 0.3);
        j.shake(3.0, 0.2);
        assert_eq!(j.screen_shake.intensity, 8.0);
    }

    #[test]
    fn shake_offset_zero_when_not_active() {
        let mut j = Juice::new();
        assert_eq!(j.screen_shake.offset(), Vec2::ZERO);
    }

    #[test]
    fn shake_offset_nonzero_when_active() {
        let mut j = Juice::new();
        j.shake(5.0, 0.2);
        assert!(j.screen_shake.offset().length() > 0.0);
    }

    #[test]
    fn shake_offset_returns_to_zero_after_duration() {
        let mut j = Juice::new();
        j.shake(5.0, 0.2);
        j.tick(0.5);
        assert_eq!(j.screen_shake.offset(), Vec2::ZERO);
    }

    #[test]
    fn flash_expires() {
        let mut j = Juice::new();
        j.flash(Color::new(1.0, 0.9, 0.3, 1.0), 0.15);
        j.tick(0.2);
        assert!(j.flashes.is_empty());
    }

    #[test]
    fn jade_shatter_creates_particles() {
        let mut j = Juice::new();
        j.spawn_jade_shatter(Vec2::ZERO, WHITE, 12, 12345);
        assert_eq!(j.particles.len(), 12);
    }

    #[test]
    fn jade_shatter_is_deterministic() {
        let mut a = Juice::new();
        let mut b = Juice::new();
        a.spawn_jade_shatter(Vec2::ZERO, WHITE, 8, 42);
        b.spawn_jade_shatter(Vec2::ZERO, WHITE, 8, 42);
        for (pa, pb) in a.particles.iter().zip(b.particles.iter()) {
            assert_eq!(pa.vel, pb.vel);
            assert_eq!(pa.sides, pb.sides);
        }
    }

    #[test]
    fn jade_shatter_sides_are_triangles_or_quads() {
        let mut j = Juice::new();
        j.spawn_jade_shatter(Vec2::ZERO, WHITE, 20, 7);
        for p in &j.particles {
            assert!(p.sides == 3 || p.sides == 4);
        }
    }

    // ---------- Shockwaves ----------

    #[test]
    fn shockwave_is_spawned() {
        let mut j = Juice::new();
        j.spawn_shockwave(Vec2::ZERO, 80.0, WHITE, 0.3, 2.0);
        assert_eq!(j.shockwaves.len(), 1);
        assert_eq!(j.shockwaves[0].end_radius, 80.0);
    }

    #[test]
    fn shockwaves_expire() {
        let mut j = Juice::new();
        j.spawn_shockwave(Vec2::ZERO, 80.0, WHITE, 0.3, 2.0);
        j.tick(0.5);
        assert!(j.shockwaves.is_empty());
    }

    #[test]
    fn shockwave_life_decreases() {
        let mut j = Juice::new();
        j.spawn_shockwave(Vec2::ZERO, 80.0, WHITE, 0.4, 2.0);
        j.tick(0.1);
        assert!(j.shockwaves[0].life < 0.4);
        assert!(j.shockwaves[0].life > 0.0);
    }

    // ---------- Radial rays ----------

    #[test]
    fn radial_burst_creates_expected_ray_count() {
        let mut j = Juice::new();
        j.spawn_radial_burst(Vec2::ZERO, 8, 60.0, WHITE, 42);
        assert_eq!(j.rays.len(), 8);
    }

    #[test]
    fn radial_burst_is_deterministic() {
        let mut a = Juice::new();
        let mut b = Juice::new();
        a.spawn_radial_burst(Vec2::ZERO, 6, 60.0, WHITE, 7);
        b.spawn_radial_burst(Vec2::ZERO, 6, 60.0, WHITE, 7);
        for (ra, rb) in a.rays.iter().zip(b.rays.iter()) {
            assert!((ra.angle - rb.angle).abs() < 1e-6);
        }
    }

    #[test]
    fn rays_expire() {
        let mut j = Juice::new();
        j.spawn_radial_burst(Vec2::ZERO, 6, 60.0, WHITE, 7);
        j.tick(0.5);
        assert!(j.rays.is_empty());
    }

    // ---------- Vignette ----------

    #[test]
    fn vignette_pulse_sets_vignette() {
        let mut j = Juice::new();
        j.vignette_pulse(0.6, 0.3);
        assert!(j.vignette.is_some());
        assert_eq!(j.vignette.as_ref().unwrap().intensity, 0.6);
    }

    #[test]
    fn vignette_expires() {
        let mut j = Juice::new();
        j.vignette_pulse(0.6, 0.3);
        j.tick(0.5);
        assert!(j.vignette.is_none());
    }

    #[test]
    fn vignette_pulse_replaces_previous() {
        let mut j = Juice::new();
        j.vignette_pulse(0.3, 0.3);
        j.vignette_pulse(0.7, 0.3);
        assert_eq!(j.vignette.as_ref().unwrap().intensity, 0.7);
    }
}