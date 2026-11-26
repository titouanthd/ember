//! Engine-typed juice (ADR-046): particles, floating text, shockwaves,
//! full-screen flashes and screen shake, on `ember_core::Color`, drawn
//! through a `RenderingServer`.
//!
//! The same data and logic as the macroquad [`crate::juice`], which the
//! unmigrated games still use and which retires in Phase 8. Porting is
//! mechanical: draw functions take the renderer first.
//!
//! Games with extra effects (falling pieces, vignettes…) wrap [`Juice`] in a
//! local struct that `Deref`s to it and keep only the game-specific parts.

use ember_core::Color;
use ember_core::rng::Rng;
use ember_servers::RenderingServer;
use glam::Vec2;
use std::f32::consts::TAU;

/// Radius an expanding shockwave ring starts from.
pub const SHOCKWAVE_START_RADIUS: f32 = 8.0;

/// Tunables for how effects move and live.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JuiceConfig {
    pub particle_gravity: f32,
    pub particle_drag: f32,
    pub particle_life: f32,
    pub text_rise_speed: f32,
    pub text_life: f32,
}

impl Default for JuiceConfig {
    fn default() -> Self {
        Self {
            particle_gravity: 300.0,
            particle_drag: 2.0,
            particle_life: 0.5,
            text_rise_speed: 45.0,
            text_life: 1.0,
        }
    }
}

/// A short-lived dot (`sides == 0`) or regular polygon.
#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
    pub size: f32,
    /// `0` draws a circle, otherwise a regular polygon with this many sides.
    pub sides: u8,
    /// Degrees, as in [`crate::juice`].
    pub rotation: f32,
    /// Degrees per second.
    pub spin: f32,
}

impl Particle {
    /// A round particle with no rotation.
    pub fn dot(pos: Vec2, vel: Vec2, life: f32, color: Color, size: f32) -> Self {
        Self {
            pos,
            vel,
            life,
            max_life: life,
            color,
            size,
            sides: 0,
            rotation: 0.0,
            spin: 0.0,
        }
    }
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

/// Expanding ring.
#[derive(Debug, Clone)]
pub struct Shockwave {
    pub pos: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub end_radius: f32,
    pub color: Color,
    pub thickness: f32,
}

/// Full-screen colour overlay that fades out.
#[derive(Debug, Clone)]
pub struct Flash {
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
}

/// Deterministic screen shake. Call [`ScreenShake::offset`] once per frame.
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

    /// Starts a shake unless a stronger one is already running.
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

    pub fn is_active(&self) -> bool {
        self.remaining > 0.0
    }

    pub fn offset(&mut self) -> Vec2 {
        if self.remaining <= 0.0 || self.duration <= 0.0 {
            return Vec2::ZERO;
        }
        let amp = self.intensity * (self.remaining / self.duration);

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

#[derive(Debug, Default)]
pub struct Juice {
    pub config: JuiceConfig,
    pub particles: Vec<Particle>,
    pub floating_texts: Vec<FloatingText>,
    pub shockwaves: Vec<Shockwave>,
    pub flashes: Vec<Flash>,
    pub screen_shake: ScreenShake,
}

impl Juice {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: JuiceConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let cfg = self.config;

        for p in &mut self.particles {
            // Gravity and drag first, then move with the new velocity.
            p.vel.y += cfg.particle_gravity * dt;
            p.vel *= 1.0 - (cfg.particle_drag * dt).min(1.0);
            p.pos += p.vel * dt;
            p.rotation += p.spin * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        for t in &mut self.floating_texts {
            t.pos.y -= cfg.text_rise_speed * dt;
            t.life -= dt;
        }
        self.floating_texts.retain(|t| t.life > 0.0);

        for s in &mut self.shockwaves {
            s.life -= dt;
        }
        self.shockwaves.retain(|s| s.life > 0.0);

        for f in &mut self.flashes {
            f.life -= dt;
        }
        self.flashes.retain(|f| f.life > 0.0);

        self.screen_shake.tick(dt);
    }

    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.screen_shake.trigger(intensity, duration);
    }

    pub fn shake_offset(&mut self) -> Vec2 {
        self.screen_shake.offset()
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
        let life = self.config.text_life;
        self.floating_texts.push(FloatingText {
            pos,
            text: text.into(),
            life,
            max_life: life,
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

    /// Spawns `count` round particles flying out in random directions.
    pub fn burst(&mut self, pos: Vec2, color: Color, count: u32, seed: u32) {
        let mut rng = Rng::new(seed ^ 0x9E37_79B9);
        let life = self.config.particle_life;

        for _ in 0..count {
            let angle = rng.next_f32_range(0.0, TAU);
            let speed = rng.next_f32_range(50.0, 180.0);
            let size = rng.next_f32_range(2.0, 4.5);
            let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
            self.particles
                .push(Particle::dot(pos, vel, life, color, size));
        }
    }

    /// Spawns a single short, upward spark.
    pub fn spark(&mut self, pos: Vec2, color: Color) {
        self.particles
            .push(Particle::dot(pos, Vec2::new(0.0, -80.0), 0.15, color, 3.0));
    }

    /// Spawns `count` particles evenly spread around a circle.
    pub fn ring_burst(&mut self, pos: Vec2, color: Color, count: u32, life: f32) {
        for i in 0..count {
            let angle = (i as f32 / count as f32) * TAU;
            let speed = 90.0 + (i as f32 * 7.3).sin() * 40.0;
            let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
            self.particles
                .push(Particle::dot(pos, vel, life, color, 4.0));
        }
    }
}

fn faded(color: Color, alpha: f32) -> Color {
    Color::new(color.r, color.g, color.b, color.a * alpha)
}

/// Draws particles; `offset` is added to world positions (camera + shake).
pub fn draw_particles(r: &mut dyn RenderingServer, juice: &Juice, offset: Vec2) {
    for p in &juice.particles {
        let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
        let pos = p.pos + offset;
        let color = faded(p.color, alpha);
        if p.sides == 0 {
            r.draw_circle(pos, p.size * alpha, color);
        } else {
            let rotation = p.rotation.to_radians();
            r.draw_regular_polygon(pos, p.sides, p.size * alpha, rotation, color);
        }
    }
}

pub fn draw_floating_texts(r: &mut dyn RenderingServer, juice: &Juice, offset: Vec2) {
    for t in &juice.floating_texts {
        let alpha = (t.life / t.max_life).clamp(0.0, 1.0);
        let pos = t.pos + offset;
        let dim = r.measure_text(&t.text, t.font_size);
        r.draw_text(
            &t.text,
            Vec2::new(pos.x - dim.width * 0.5, pos.y),
            t.font_size,
            faded(t.color, alpha),
        );
    }
}

pub fn draw_shockwaves(r: &mut dyn RenderingServer, juice: &Juice, offset: Vec2) {
    for s in &juice.shockwaves {
        let t = 1.0 - (s.life / s.max_life).clamp(0.0, 1.0);
        let radius = SHOCKWAVE_START_RADIUS + (s.end_radius - SHOCKWAVE_START_RADIUS) * t;
        let alpha = (1.0 - t).powf(1.5);
        let pos = s.pos + offset;
        r.draw_circle_outline(pos, radius, s.thickness, faded(s.color, alpha));
    }
}

/// Draws full-screen flashes over a `size` area (the viewport or a
/// virtual canvas).
pub fn draw_flashes(r: &mut dyn RenderingServer, juice: &Juice, size: Vec2) {
    for f in &juice.flashes {
        let alpha = (f.life / f.max_life).clamp(0.0, 1.0) * 0.35;
        let area = ember_core::math::Rect::new(0.0, 0.0, size.x, size.y);
        r.draw_rect(area, faded(f.color, alpha));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ember_servers::{MockRenderer, RecordedCall};

    #[test]
    fn new_juice_is_empty() {
        let j = Juice::new();
        assert!(j.particles.is_empty());
        assert!(j.floating_texts.is_empty());
        assert!(j.shockwaves.is_empty());
        assert!(j.flashes.is_empty());
        assert!(!j.screen_shake.is_active());
    }

    #[test]
    fn particle_life_decreases_then_expires() {
        let mut j = Juice::new();
        j.particles.push(Particle::dot(
            Vec2::ZERO,
            Vec2::ZERO,
            0.5,
            Color::WHITE,
            3.0,
        ));
        j.tick(0.2);
        assert!((j.particles[0].life - 0.3).abs() < 1e-6);
        j.tick(0.5);
        assert!(j.particles.is_empty());
    }

    #[test]
    fn config_controls_gravity() {
        let mut j = Juice::with_config(JuiceConfig {
            particle_gravity: 100.0,
            particle_drag: 0.0,
            ..JuiceConfig::default()
        });
        j.particles.push(Particle::dot(
            Vec2::ZERO,
            Vec2::ZERO,
            1.0,
            Color::WHITE,
            3.0,
        ));
        j.tick(0.1);
        assert!((j.particles[0].vel.y - 10.0).abs() < 1e-5);
    }

    #[test]
    fn particle_spin_rotates() {
        let mut j = Juice::new();
        let mut p = Particle::dot(Vec2::ZERO, Vec2::ZERO, 1.0, Color::WHITE, 3.0);
        p.spin = 2.0;
        j.particles.push(p);
        j.tick(0.5);
        assert!((j.particles[0].rotation - 1.0).abs() < 1e-6);
    }

    #[test]
    fn floating_text_rises_and_expires() {
        let mut j = Juice::new();
        j.spawn_floating_text(Vec2::new(0.0, 100.0), "hi", Color::WHITE, 20.0);
        j.tick(0.1);
        assert!(j.floating_texts[0].pos.y < 100.0);
        j.tick(2.0);
        assert!(j.floating_texts.is_empty());
    }

    #[test]
    fn shockwave_and_flash_expire() {
        let mut j = Juice::new();
        j.spawn_shockwave(Vec2::ZERO, 50.0, Color::WHITE, 0.2, 2.0);
        j.flash(Color::WHITE, 0.1);
        j.tick(0.5);
        assert!(j.shockwaves.is_empty());
        assert!(j.flashes.is_empty());
    }

    #[test]
    fn burst_spawns_requested_count() {
        let mut j = Juice::new();
        j.burst(Vec2::ZERO, Color::WHITE, 7, 42);
        assert_eq!(j.particles.len(), 7);
        j.ring_burst(Vec2::ZERO, Color::WHITE, 5, 0.5);
        assert_eq!(j.particles.len(), 12);
        j.spark(Vec2::ZERO, Color::WHITE);
        assert_eq!(j.particles.len(), 13);
    }

    #[test]
    fn stronger_shake_replaces_weaker_but_not_vice_versa() {
        let mut shake = ScreenShake::new();
        shake.trigger(3.0, 0.2);
        shake.trigger(8.0, 0.3);
        assert_eq!((shake.intensity, shake.duration), (8.0, 0.3));
        shake.trigger(2.0, 0.1);
        assert_eq!((shake.intensity, shake.duration), (8.0, 0.3));
    }

    #[test]
    fn shake_offset_is_zero_when_inactive_and_after_expiry() {
        let mut j = Juice::new();
        assert_eq!(j.shake_offset(), Vec2::ZERO);
        j.shake(6.0, 0.1);
        assert!(j.shake_offset().length() > 0.0);
        j.tick(0.2);
        assert_eq!(j.shake_offset(), Vec2::ZERO);
    }

    #[test]
    fn same_seed_same_burst() {
        let mut a = Juice::new();
        let mut b = Juice::new();
        a.burst(Vec2::ZERO, Color::WHITE, 4, 9);
        b.burst(Vec2::ZERO, Color::WHITE, 4, 9);
        for (p, q) in a.particles.iter().zip(&b.particles) {
            assert_eq!(p.vel, q.vel);
        }
    }

    #[test]
    fn drawing_goes_through_the_renderer() {
        let mut j = Juice::new();
        let mut poly = Particle::dot(Vec2::ZERO, Vec2::ZERO, 1.0, Color::WHITE, 3.0);
        poly.sides = 4;
        poly.rotation = 90.0;
        j.particles.push(poly);
        j.particles
            .push(Particle::dot(Vec2::ZERO, Vec2::ZERO, 1.0, Color::RED, 2.0));
        j.spawn_shockwave(Vec2::ZERO, 50.0, Color::WHITE, 1.0, 2.0);
        j.flash(Color::WHITE, 1.0);
        j.spawn_floating_text(Vec2::new(0.0, 10.0), "hi", Color::WHITE, 20.0);

        let mut r = MockRenderer::new();
        let offset = Vec2::new(5.0, 0.0);
        draw_particles(&mut r, &j, offset);
        draw_shockwaves(&mut r, &j, offset);
        draw_flashes(&mut r, &j, Vec2::new(800.0, 600.0));
        draw_floating_texts(&mut r, &j, offset);
        let calls = r.calls();
        assert!(matches!(
            calls[0],
            RecordedCall::DrawRegularPolygon { center, sides: 4, rotation, .. }
                if center == offset && (rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-6
        ));
        assert!(matches!(calls[1], RecordedCall::DrawCircle { .. }));
        assert!(matches!(calls[2], RecordedCall::DrawCircleOutline { .. }));
        assert!(matches!(
            calls[3],
            RecordedCall::DrawRect { rect, .. } if rect.w == 800.0 && rect.h == 600.0
        ));
        assert!(matches!(calls[4], RecordedCall::DrawText { .. }));
    }
}
