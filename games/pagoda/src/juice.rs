// games/pagoda/src/juice.rs
//! Local visual feedback for Pagoda: falling slices, particles, shockwaves,
//! floating text, flash, and screen shake.
//!
//! This stays local to the game on purpose. The workspace `Juice` abstraction
//! is currently only at two occurrences and has not been unified yet.

use crate::config::PERFECT_GOLD;
use ember_core::rng::Rng;
use glam::Vec2;
use macroquad::prelude::*;
use std::f32::consts::TAU;

const PIECE_LIFE: f32 = 0.8;
const PARTICLE_LIFE: f32 = 0.5;
const SHOCKWAVE_START_RADIUS: f32 = 8.0;
const PIECE_GRAVITY: f32 = 900.0;
const PARTICLE_GRAVITY: f32 = 350.0;
const PARTICLE_DRAG: f32 = 2.0;
const TEXT_RISE_SPEED: f32 = 45.0;

/// A cut-off overhang falling away from the tower.
#[derive(Debug, Clone)]
pub struct FallingPiece {
    /// Center in world coordinates.
    pub pos: Vec2,
    pub size: Vec2,
    pub vel: Vec2,
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
    pub rotation: f32,
    pub spin: f32,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub color: Color,
    pub size: f32,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Debug, Clone)]
pub struct FloatingText {
    pub pos: Vec2,
    pub text: String,
    pub color: Color,
    pub font_size: f32,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Debug, Clone)]
pub struct Shockwave {
    pub pos: Vec2,
    pub end_radius: f32,
    pub thickness: f32,
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Debug, Clone)]
pub struct Flash {
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
}

/// Deterministic screen shake. Call [`Self::offset`] once per frame.
#[derive(Debug, Clone)]
pub struct ScreenShake {
    pub intensity: f32,
    pub remaining: f32,
    pub duration: f32,
    rng: Rng,
}

impl ScreenShake {
    pub fn new() -> Self {
        Self {
            intensity: 0.0,
            remaining: 0.0,
            duration: 0.0,
            rng: Rng::new(0x0BAD_C0DE),
        }
    }

    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        if intensity >= self.intensity || self.remaining <= 0.0 {
            self.intensity = intensity;
            self.remaining = duration;
            self.duration = duration;
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }

    pub fn offset(&mut self) -> Vec2 {
        if self.remaining <= 0.0 || self.duration <= 0.0 {
            return Vec2::ZERO;
        }

        let t = self.remaining / self.duration;
        let amplitude = self.intensity * t;
        Vec2::new(
            self.rng.next_f32_range(-1.0, 1.0) * amplitude,
            self.rng.next_f32_range(-1.0, 1.0) * amplitude,
        )
    }

    pub fn is_active(&self) -> bool {
        self.remaining > 0.0
    }
}

impl Default for ScreenShake {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct Juice {
    pub pieces: Vec<FallingPiece>,
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

    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        for piece in &mut self.pieces {
            piece.vel.y += PIECE_GRAVITY * dt;
            piece.pos += piece.vel * dt;
            piece.rotation += piece.spin * dt;
            piece.life -= dt;
        }
        self.pieces.retain(|piece| piece.life > 0.0);

        for particle in &mut self.particles {
            particle.vel.y += PARTICLE_GRAVITY * dt;
            particle.vel *= 1.0 - (PARTICLE_DRAG * dt).min(1.0);
            particle.pos += particle.vel * dt;
            particle.life -= dt;
        }
        self.particles.retain(|particle| particle.life > 0.0);

        for text in &mut self.floating_texts {
            text.pos.y -= TEXT_RISE_SPEED * dt;
            text.life -= dt;
        }
        self.floating_texts.retain(|text| text.life > 0.0);

        for shockwave in &mut self.shockwaves {
            shockwave.life -= dt;
        }
        self.shockwaves.retain(|shockwave| shockwave.life > 0.0);

        for flash in &mut self.flashes {
            flash.life -= dt;
        }
        self.flashes.retain(|flash| flash.life > 0.0);

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
        self.floating_texts.push(FloatingText {
            pos,
            text: text.into(),
            color,
            font_size,
            life: 1.0,
            max_life: 1.0,
        });
    }

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
            end_radius,
            thickness,
            color,
            life: duration,
            max_life: duration,
        });
    }

    /// Spawn a burst of round particles from `pos`.
    pub fn burst(&mut self, pos: Vec2, color: Color, count: u32, seed: u32) {
        let mut rng = Rng::new(seed ^ 0x9E37_79B9);

        for _ in 0..count {
            let angle = rng.next_f32_range(0.0, TAU);
            let speed = rng.next_f32_range(50.0, 180.0);
            let size = rng.next_f32_range(2.0, 4.5);

            self.particles.push(Particle {
                pos,
                vel: Vec2::new(angle.cos(), angle.sin()) * speed,
                color,
                size,
                life: PARTICLE_LIFE,
                max_life: PARTICLE_LIFE,
            });
        }
    }

    /// Spawn the falling overhang piece and a small edge burst.
    ///
    /// `top_left` and `size` describe the cut rectangle; `dir` is `-1.0`
    /// for a left cut and `1.0` for a right cut.
    pub fn spawn_slice_piece(
        &mut self,
        top_left: Vec2,
        size: Vec2,
        color: Color,
        dir: f32,
        seed: u32,
    ) {
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }

        let center = top_left + size * 0.5;
        self.pieces.push(FallingPiece {
            pos: center,
            size,
            vel: Vec2::new(dir * 60.0, -40.0),
            color,
            life: PIECE_LIFE,
            max_life: PIECE_LIFE,
            rotation: 0.0,
            spin: dir * 2.5,
        });

        let edge = if dir < 0.0 {
            Vec2::new(top_left.x + size.x, center.y)
        } else {
            Vec2::new(top_left.x, center.y)
        };
        self.burst(edge, color, 5, seed);
    }

    /// Small dust puff + landing shake for a normal (non-perfect) drop.
    pub fn spawn_land(&mut self, pos: Vec2, color: Color, seed: u32) {
        self.burst(pos, color, 3, seed);
        self.shake(1.2, 0.08);
    }

    /// Full perfect-drop feedback: gold burst, ring, label, flash, shake.
    pub fn spawn_perfect(&mut self, center: Vec2, combo: u32, seed: u32) {
        self.burst(center, PERFECT_GOLD, 24, seed);
        self.spawn_shockwave(center, 90.0, PERFECT_GOLD, 0.35, 3.0);

        let label = if combo > 1 {
            format!("PERFECT x{combo}")
        } else {
            "PERFECT".to_owned()
        };
        self.spawn_floating_text(center + Vec2::new(0.0, -28.0), label, PERFECT_GOLD, 28.0);

        self.flash(PERFECT_GOLD, 0.18);
        self.shake(4.0 + combo.min(4) as f32 * 0.75, 0.22);
    }

    /// Draw world-space effects. `camera_y` is the tower camera offset;
    /// `shake` is the current screen-shake offset.
    pub fn draw_world(&self, camera_y: f32, shake: Vec2) {
        for shockwave in &self.shockwaves {
            let t = 1.0 - (shockwave.life / shockwave.max_life).clamp(0.0, 1.0);
            let radius =
                SHOCKWAVE_START_RADIUS + (shockwave.end_radius - SHOCKWAVE_START_RADIUS) * t;
            let alpha = (1.0 - t).powf(1.5) * shockwave.color.a;

            draw_circle_lines(
                shockwave.pos.x + shake.x,
                shockwave.pos.y + camera_y + shake.y,
                radius,
                shockwave.thickness,
                Color::new(
                    shockwave.color.r,
                    shockwave.color.g,
                    shockwave.color.b,
                    alpha,
                ),
            );
        }

        for piece in &self.pieces {
            let alpha = (piece.life / piece.max_life).clamp(0.0, 1.0);
            draw_rectangle_ex(
                piece.pos.x + shake.x,
                piece.pos.y + camera_y + shake.y,
                piece.size.x,
                piece.size.y,
                DrawRectangleParams {
                    offset: vec2(0.5, 0.5),
                    rotation: piece.rotation,
                    color: Color::new(
                        piece.color.r,
                        piece.color.g,
                        piece.color.b,
                        piece.color.a * alpha,
                    ),
                },
            );
        }

        for particle in &self.particles {
            let alpha = (particle.life / particle.max_life).clamp(0.0, 1.0);
            draw_circle(
                particle.pos.x + shake.x,
                particle.pos.y + camera_y + shake.y,
                particle.size * alpha,
                Color::new(
                    particle.color.r,
                    particle.color.g,
                    particle.color.b,
                    particle.color.a * alpha,
                ),
            );
        }

        for text in &self.floating_texts {
            let alpha = (text.life / text.max_life).clamp(0.0, 1.0);
            ember_stdlib::graphics::text::draw_centered(
                &text.text,
                text.pos.x + shake.x,
                text.pos.y + camera_y + shake.y,
                text.font_size as u16,
                Color::new(
                    text.color.r,
                    text.color.g,
                    text.color.b,
                    text.color.a * alpha,
                ),
            );
        }
    }

    pub fn draw_flash(&self, screen_w: f32, screen_h: f32) {
        for flash in &self.flashes {
            let alpha = (flash.life / flash.max_life).clamp(0.0, 1.0) * 0.35;
            draw_rectangle(
                0.0,
                0.0,
                screen_w,
                screen_h,
                Color::new(
                    flash.color.r,
                    flash.color.g,
                    flash.color.b,
                    flash.color.a * alpha,
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_juice_is_empty() {
        let juice = Juice::new();
        assert!(juice.pieces.is_empty());
        assert!(juice.particles.is_empty());
        assert!(juice.floating_texts.is_empty());
        assert!(juice.shockwaves.is_empty());
        assert!(juice.flashes.is_empty());
        assert!(!juice.screen_shake.is_active());
    }

    #[test]
    fn slice_piece_falls_and_spawns_particles() {
        let mut juice = Juice::new();
        juice.spawn_slice_piece(
            Vec2::new(100.0, 100.0),
            Vec2::new(40.0, 40.0),
            WHITE,
            -1.0,
            7,
        );

        assert_eq!(juice.pieces.len(), 1);
        assert_eq!(juice.particles.len(), 5);
        let y0 = juice.pieces[0].pos.y;

        juice.tick(0.2);

        assert!(juice.pieces[0].pos.y > y0);
        assert!(juice.pieces[0].rotation < 0.0);
    }

    #[test]
    fn zero_width_slice_spawns_nothing() {
        let mut juice = Juice::new();
        juice.spawn_slice_piece(Vec2::new(100.0, 100.0), Vec2::new(0.0, 40.0), WHITE, 1.0, 1);
        assert!(juice.pieces.is_empty());
        assert!(juice.particles.is_empty());
    }

    #[test]
    fn floating_text_rises_and_expires() {
        let mut juice = Juice::new();
        juice.spawn_floating_text(Vec2::new(100.0, 100.0), "PERFECT", WHITE, 28.0);
        let y0 = juice.floating_texts[0].pos.y;

        juice.tick(0.1);
        assert!(juice.floating_texts[0].pos.y < y0);

        juice.tick(2.0);
        assert!(juice.floating_texts.is_empty());
    }

    #[test]
    fn shockwave_and_flash_expire() {
        let mut juice = Juice::new();
        juice.spawn_shockwave(Vec2::ZERO, 90.0, PERFECT_GOLD, 0.35, 3.0);
        juice.flash(PERFECT_GOLD, 0.18);

        juice.tick(1.0);

        assert!(juice.shockwaves.is_empty());
        assert!(juice.flashes.is_empty());
    }

    #[test]
    fn perfect_spawns_full_feedback() {
        let mut juice = Juice::new();
        juice.spawn_perfect(Vec2::new(400.0, 200.0), 2, 42);

        assert_eq!(juice.particles.len(), 24);
        assert_eq!(juice.shockwaves.len(), 1);
        assert_eq!(juice.floating_texts.len(), 1);
        assert_eq!(juice.floating_texts[0].text, "PERFECT x2");
        assert_eq!(juice.flashes.len(), 1);
        assert!(juice.screen_shake.is_active());
    }

    #[test]
    fn land_spawns_dust_and_shake() {
        let mut juice = Juice::new();
        juice.spawn_land(Vec2::new(400.0, 500.0), WHITE, 3);

        assert_eq!(juice.particles.len(), 3);
        assert!(juice.screen_shake.is_active());
    }

    #[test]
    fn stronger_shake_replaces_weaker() {
        let mut shake = ScreenShake::new();
        shake.trigger(3.0, 0.2);
        shake.trigger(8.0, 0.3);
        assert_eq!(shake.intensity, 8.0);
        assert_eq!(shake.duration, 0.3);
    }

    #[test]
    fn weaker_shake_does_not_replace_stronger() {
        let mut shake = ScreenShake::new();
        shake.trigger(8.0, 0.3);
        shake.trigger(3.0, 0.2);
        assert_eq!(shake.intensity, 8.0);
        assert_eq!(shake.duration, 0.3);
    }

    #[test]
    fn shake_offset_is_zero_when_inactive() {
        let mut juice = Juice::new();
        assert_eq!(juice.shake_offset(), Vec2::ZERO);
    }

    #[test]
    fn shake_offset_is_nonzero_and_then_expires() {
        let mut juice = Juice::new();
        juice.shake(5.0, 0.2);
        assert!(juice.shake_offset().length() > 0.0);

        juice.tick(1.0);
        assert_eq!(juice.shake_offset(), Vec2::ZERO);
    }
}
