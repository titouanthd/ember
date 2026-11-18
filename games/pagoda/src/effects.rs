// games/pagoda/src/effects.rs
//! Pagoda-specific feedback: falling slices and the perfect-drop combo,
//! layered on top of the shared `ember_stdlib::juice::Juice`.

use crate::config::PERFECT_GOLD;
use ember_stdlib::juice::{self, Juice, JuiceConfig};
use glam::Vec2;
use macroquad::prelude::*;
use std::ops::{Deref, DerefMut};

const PIECE_LIFE: f32 = 0.8;
const PIECE_GRAVITY: f32 = 900.0;

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

/// Shared juice plus falling pieces. Derefs to [`Juice`].
#[derive(Debug)]
pub struct Effects {
    base: Juice,
    pub pieces: Vec<FallingPiece>,
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
                particle_gravity: 350.0,
                particle_drag: 2.0,
                particle_life: 0.5,
                text_rise_speed: 45.0,
                text_life: 1.0,
            }),
            pieces: Vec::new(),
        }
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

        self.base.tick(dt);
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
        let offset = shake + Vec2::new(0.0, camera_y);

        juice::draw_shockwaves(&self.base, offset);

        for piece in &self.pieces {
            let alpha = (piece.life / piece.max_life).clamp(0.0, 1.0);
            draw_rectangle_ex(
                piece.pos.x + offset.x,
                piece.pos.y + offset.y,
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

        juice::draw_particles(&self.base, offset);
        juice::draw_floating_texts(&self.base, offset);
    }

    pub fn draw_flash(&self, screen_w: f32, screen_h: f32) {
        juice::draw_flashes_sized(&self.base, screen_w, screen_h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_piece_falls_and_spawns_particles() {
        let mut juice = Effects::new();
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
        let mut juice = Effects::new();
        juice.spawn_slice_piece(Vec2::new(100.0, 100.0), Vec2::new(0.0, 40.0), WHITE, 1.0, 1);
        assert!(juice.pieces.is_empty());
        assert!(juice.particles.is_empty());
    }

    #[test]
    fn perfect_spawns_full_feedback() {
        let mut juice = Effects::new();
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
        let mut juice = Effects::new();
        juice.spawn_land(Vec2::new(400.0, 500.0), WHITE, 3);

        assert_eq!(juice.particles.len(), 3);
        assert!(juice.screen_shake.is_active());
    }
}
