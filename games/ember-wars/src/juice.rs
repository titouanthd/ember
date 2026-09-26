//! Juice — feedback visuel : hit flash, particules, screen shake,
//! floating text, flash plein écran.

use glam::Vec2;
use macroquad::prelude::*;

/// Durée du flash blanc sur une unité touchée.
pub const HIT_FLASH_DURATION: f32 = 0.08;

/// Durée de vie de base d'un particle de mort.
pub const DEATH_PARTICLE_LIFE: f32 = 0.55;

#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
    pub size: f32,
}

#[derive(Debug, Clone)]
pub struct FloatingText {
    pub pos: Vec2,
    pub text: String,
    pub life: f32,
    pub max_life: f32,
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
    pub screen_shake: ScreenShake,
    pub flashes: Vec<Flash>,
}

impl Juice {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            floating_texts: Vec::new(),
            screen_shake: ScreenShake::new(),
            flashes: Vec::new(),
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        for p in self.particles.iter_mut() {
            // Drag + gravité appliqués avant le déplacement (physique
            // standard : on calcule la nouvelle vitesse, puis on bouge).
            p.vel.y += 280.0 * dt;
            p.vel *= 1.0 - (2.0 * dt).min(1.0);
            p.pos += p.vel * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        for t in self.floating_texts.iter_mut() {
            t.pos.y -= 40.0 * dt;
            t.life -= dt;
        }
        self.floating_texts.retain(|t| t.life > 0.0);

        self.screen_shake.tick(dt);

        for f in self.flashes.iter_mut() {
            f.life -= dt;
        }
        self.flashes.retain(|f| f.life > 0.0);
    }

    // ---------- Triggers ----------

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

    pub fn spawn_hit_spark(&mut self, pos: Vec2, color: Color) {
        self.particles.push(Particle {
            pos,
            vel: Vec2::new(0.0, -80.0),
            life: 0.15,
            max_life: 0.15,
            color,
            size: 3.0,
        });
    }

    pub fn spawn_death_burst(&mut self, pos: Vec2, color: Color, count: u32) {
        for i in 0..count {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let speed = 90.0 + (i as f32 * 7.3).sin() * 40.0;
            self.particles.push(Particle {
                pos,
                vel: Vec2::new(angle.cos(), angle.sin()) * speed,
                life: DEATH_PARTICLE_LIFE,
                max_life: DEATH_PARTICLE_LIFE,
                color,
                size: 4.0,
            });
        }
    }

    pub fn spawn_floating_text(&mut self, pos: Vec2, text: impl Into<String>, color: Color) {
        self.floating_texts.push(FloatingText {
            pos,
            text: text.into(),
            life: 0.9,
            max_life: 0.9,
            color,
        });
    }
}

/// Dessine tous les particles (espace monde + offset caméra + shake).
pub fn draw_particles(juice: &Juice, cam_x: f32, shake: Vec2) {
    for p in &juice.particles {
        let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
        let c = Color::new(p.color.r, p.color.g, p.color.b, p.color.a * alpha);
        let screen = Vec2::new(p.pos.x - cam_x, p.pos.y) + shake;
        draw_circle(screen.x, screen.y, p.size * alpha, c);
    }
}

/// Dessine les floating texts.
pub fn draw_floating_texts(juice: &Juice, cam_x: f32, shake: Vec2) {
    for t in &juice.floating_texts {
        let alpha = (t.life / t.max_life).clamp(0.0, 1.0);
        let c = Color::new(t.color.r, t.color.g, t.color.b, t.color.a * alpha);
        let screen = Vec2::new(t.pos.x - cam_x, t.pos.y) + shake;
        let dim = measure_text(&t.text, None, 16, 1.0);
        draw_text(&t.text, screen.x - dim.width * 0.5, screen.y, 16.0, c);
    }
}

/// Dessine les flashs plein écran (overlay coloré).
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
        });
        j.tick(0.5);
        assert!(j.particles.is_empty());
    }

    #[test]
    fn particle_moves_by_velocity() {
        let mut j = Juice::new();
        j.particles.push(Particle {
            pos: Vec2::ZERO,
            vel: Vec2::new(100.0, 0.0),
            life: 1.0,
            max_life: 1.0,
            color: WHITE,
            size: 3.0,
        });
        j.tick(0.1);
        // Après drag (×0.8) : vel.x = 80, déplacement = 8.0
        assert!(j.particles[0].pos.x > 5.0);
        assert!(j.particles[0].pos.x < 10.0);
    }

    #[test]
    fn floating_text_rises() {
        let mut j = Juice::new();
        j.spawn_floating_text(Vec2::new(100.0, 100.0), "test", WHITE);
        let y0 = j.floating_texts[0].pos.y;
        j.tick(0.1);
        assert!(j.floating_texts[0].pos.y < y0);
    }

    #[test]
    fn floating_text_dies_after_life() {
        let mut j = Juice::new();
        j.spawn_floating_text(Vec2::ZERO, "test", WHITE);
        j.tick(1.0);
        assert!(j.floating_texts.is_empty());
    }

    #[test]
    fn shake_trigger_sets_remaining() {
        let mut j = Juice::new();
        j.shake(5.0, 0.2);
        assert_eq!(j.screen_shake.remaining, 0.2);
        assert_eq!(j.screen_shake.intensity, 5.0);
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
        let offset = j.screen_shake.offset();
        assert_eq!(offset, Vec2::ZERO);
    }

    #[test]
    fn shake_offset_nonzero_when_active() {
        let mut j = Juice::new();
        j.shake(5.0, 0.2);
        let offset = j.screen_shake.offset();
        assert!(offset.length() > 0.0);
    }

    #[test]
    fn shake_offset_returns_to_zero_after_duration() {
        let mut j = Juice::new();
        j.shake(5.0, 0.2);
        j.tick(0.5);
        let offset = j.screen_shake.offset();
        assert_eq!(offset, Vec2::ZERO);
    }

    #[test]
    fn flash_expires() {
        let mut j = Juice::new();
        j.flash(Color::new(1.0, 0.9, 0.3, 1.0), 0.15);
        j.tick(0.2);
        assert!(j.flashes.is_empty());
    }

    #[test]
    fn spawn_death_burst_creates_particles() {
        let mut j = Juice::new();
        j.spawn_death_burst(Vec2::ZERO, RED, 6);
        assert_eq!(j.particles.len(), 6);
    }

    #[test]
    fn spawn_hit_spark_creates_one_particle() {
        let mut j = Juice::new();
        j.spawn_hit_spark(Vec2::ZERO, YELLOW);
        assert_eq!(j.particles.len(), 1);
    }
}