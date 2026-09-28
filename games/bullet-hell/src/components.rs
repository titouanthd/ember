use glam::Vec2;
use macroquad::prelude::Color;
use serde::Deserialize;

use ember_core::rng::Rng;
use ember_stdlib::collider::{Collider, Shape};
use ember_stdlib::time::Cooldown;
use ember_stdlib::transform::Transform;

use crate::config::GameContext;
use crate::waves::{EnemyData, enemy_style, pos_vec};

// ---------------------------------------------------------------------------
// Player
// ---------------------------------------------------------------------------

/// The player ship.
///
/// Convention: `transform.position` is the **center** of the ship.
/// This differs from the engine default (top-left) — see `lib.rs`.
#[derive(Debug, Clone)]
pub struct Player {
    pub transform: Transform,
    pub hitbox: Collider,
    pub graze: Collider,
    /// Cooldown de tir. `is_ready()` = peut tirer.
    pub cooldown: Cooldown,
    pub focus: bool,
    /// Timestamp absolu d'invincibilité (`now + 2.0`). Pas un countdown —
    /// on compare à `now`. Voir `is_invincible`.
    pub invincible_until: f32,
    pub alive: bool,
}

impl Player {
    pub fn new(center: Vec2, ctx: &GameContext) -> Self {
        Self {
            transform: Transform {
                position: center,
                rotation: 0.0,
                scale: Vec2::splat(ctx.player_radius * 2.0),
            },
            hitbox: Collider {
                shape: Shape::Circle {
                    radius: ctx.player_hitbox_radius,
                },
                active: true,
            },
            graze: Collider {
                shape: Shape::Circle {
                    radius: ctx.player_graze_radius,
                },
                active: true,
            },
            cooldown: Cooldown::default(),
            focus: false,
            invincible_until: 0.0,
            alive: true,
        }
    }

    pub fn respawn(&mut self, center: Vec2, ctx: &GameContext, now: f32) {
        self.transform.position = center;
        self.cooldown.clear();
        self.focus = false;
        self.invincible_until = now + 2.0;
        self.alive = true;
        self.hitbox.shape = Shape::Circle {
            radius: ctx.player_hitbox_radius,
        };
        self.graze.shape = Shape::Circle {
            radius: ctx.player_graze_radius,
        };
    }

    /// Current effective move speed, accounting for focus mode.
    pub fn speed(&self, ctx: &GameContext) -> f32 {
        if self.focus {
            ctx.player_speed * ctx.player_focus_mult
        } else {
            ctx.player_speed
        }
    }

    pub fn is_invincible(&self, now: f32) -> bool {
        now < self.invincible_until
    }

    /// Color to render with, accounting for focus + invincibility blink.
    pub fn color(&self, ctx: &GameContext, now: f32) -> Color {
        if self.is_invincible(now) {
            // Blink: visible half the time.
            let t = (now * 12.0) as i32;
            if t % 6 >= 3 {
                return Color::new(1.0, 1.0, 1.0, 0.0);
            }
        }
        if self.focus {
            ctx.color_player_focus
        } else {
            ctx.color_player
        }
    }
}

// ---------------------------------------------------------------------------
// Bullets
// ---------------------------------------------------------------------------

/// A projectile. Position + velocity + TTL.
///
/// `ttl` est un `Cooldown` : `is_active()` = la balle est vivante.
#[derive(Debug, Clone)]
pub struct Bullet {
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
    pub ttl: Cooldown,
    pub from_player: bool,
    pub color: Color,
}

impl Bullet {
    pub fn player(pos: Vec2, vel: Vec2, ctx: &GameContext) -> Self {
        Self {
            pos,
            vel,
            radius: ctx.bullet_player_radius,
            ttl: Cooldown::running(ctx.bullet_ttl),
            from_player: true,
            color: ctx.color_bullet_player,
        }
    }

    pub fn enemy(pos: Vec2, vel: Vec2, ctx: &GameContext) -> Self {
        Self {
            pos,
            vel,
            radius: ctx.bullet_enemy_radius,
            ttl: Cooldown::running(ctx.bullet_ttl),
            from_player: false,
            color: ctx.color_bullet_enemy,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.ttl.is_active()
    }
}

// ---------------------------------------------------------------------------
// Enemies — data-driven
// ---------------------------------------------------------------------------

/// How an enemy enters and moves through the field.
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum EntryMotion {
    /// Sits at its spawn position forever.
    Static,
    /// Moves in a straight line. Bounces horizontally at playfield edges.
    Drift { vel: (f32, f32) },
    /// Oscillates horizontally around its spawn X, at a fixed Y.
    Sine {
        amplitude: f32,
        frequency: f32,
        base_y: f32,
    },
}

/// A bullet pattern. Each variant's `cooldown` is the seconds between bursts.
#[derive(Debug, Clone, Deserialize)]
pub enum Emitter {
    Radial {
        count: u32,
        speed: f32,
        cooldown: f32,
    },
    Aimed {
        count: u32,
        spread: f32,
        speed: f32,
        cooldown: f32,
    },
    Spiral {
        arms: u32,
        speed: f32,
        cooldown: f32,
        rotation_rate: f32,
    },
    Wall {
        gap_x: f32,
        gap_w: f32,
        speed: f32,
        cooldown: f32,
    },
}

/// A live enemy in the world. Built from [`EnemyData`] via [`Enemy::from_data`].
#[derive(Debug, Clone)]
pub struct Enemy {
    pub center: Vec2,
    pub base_center: Vec2,
    pub radius: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub emitter: Emitter,
    /// Cooldown avant la prochaine burst. `is_ready()` = peut tirer.
    pub fire_cooldown: Cooldown,
    /// Seconds since spawn. Drives `Sine` phase and is available to emitters.
    pub age: f32,
    /// Rotation accumulator used by `Spiral`, incremented per burst.
    pub phase: f32,
    pub entry: EntryMotion,
    pub color: Color,
    pub kind: String,
    /// White-flash timer. `is_active()` = l'ennemi clignote blanc.
    pub flash: Cooldown,
}

impl Enemy {
    pub fn from_data(data: &EnemyData, ctx: &GameContext) -> Self {
        let center = pos_vec(data.pos);
        let (radius, default_color) = enemy_style(&data.kind);
        let color = if data.kind == "grunt" {
            ctx.color_enemy
        } else {
            default_color
        };
        let cooldown = match &data.emitter {
            Emitter::Radial { cooldown, .. }
            | Emitter::Aimed { cooldown, .. }
            | Emitter::Spiral { cooldown, .. }
            | Emitter::Wall { cooldown, .. } => *cooldown,
        };

        Self {
            center,
            base_center: center,
            radius,
            hp: data.hp,
            max_hp: data.hp,
            emitter: data.emitter.clone(),
            // Grace period before the first shot, so the player can react
            // to the wave spawning.
            fire_cooldown: Cooldown::running(cooldown + 0.6),
            age: 0.0,
            phase: 0.0,
            entry: data.entry,
            color,
            kind: data.kind.clone(),
            flash: Cooldown::default(),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    pub fn render_color(&self) -> Color {
        if self.flash.is_active() {
            return Color::new(1.0, 1.0, 1.0, self.color.a);
        }
        let frac = (self.hp as f32 / self.max_hp as f32).clamp(0.0, 1.0);
        let brightness = 0.35 + 0.65 * frac;
        Color::new(
            self.color.r * brightness,
            self.color.g * brightness,
            self.color.b * brightness,
            self.color.a,
        )
    }
}

// ---------------------------------------------------------------------------
// Particles — juice only
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub ttl: f32,
    pub max_ttl: f32,
    pub radius: f32,
    pub color: Color,
}

impl Particle {
    pub fn burst(pos: Vec2, count: u32, color: Color, rng: &mut Rng) -> Vec<Particle> {
        let mut out = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let a = rng.next_f32() * std::f32::consts::TAU;
            let speed = 40.0 + rng.next_f32() * 120.0;
            let ttl = 0.3 + rng.next_f32() * 0.35;
            out.push(Particle {
                pos,
                vel: Vec2::new(a.cos(), a.sin()) * speed,
                ttl,
                max_ttl: ttl,
                radius: 1.5 + rng.next_f32() * 2.0,
                color,
            });
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bullet_player_alive_on_creation() {
        let ctx = crate::config::load_config();
        let b = Bullet::player(Vec2::ZERO, Vec2::new(0.0, -100.0), &ctx);
        assert!(b.is_alive());
        assert!(b.from_player);
    }

    #[test]
    fn test_bullet_dies_at_zero_ttl() {
        let ctx = crate::config::load_config();
        let mut b = Bullet::enemy(Vec2::ZERO, Vec2::ZERO, &ctx);
        b.ttl.clear();
        assert!(!b.is_alive());
    }

    #[test]
    fn test_bullet_dies_below_zero_ttl() {
        let ctx = crate::config::load_config();
        let mut b = Bullet::enemy(Vec2::ZERO, Vec2::ZERO, &ctx);
        // tick amène à 0 → clear automatique par clamp
        b.ttl.tick(ctx.bullet_ttl + 1.0);
        assert!(!b.is_alive());
    }

    #[test]
    fn test_particle_burst_count() {
        let mut rng = Rng::new(1);
        let ps = Particle::burst(Vec2::ZERO, 7, Color::new(1.0, 0.0, 0.0, 1.0), &mut rng);
        assert_eq!(ps.len(), 7);
    }

    #[test]
    fn test_particle_burst_all_alive() {
        let mut rng = Rng::new(12345);
        let ps = Particle::burst(Vec2::ZERO, 20, Color::new(1.0, 1.0, 1.0, 1.0), &mut rng);
        assert!(ps.iter().all(|p| p.ttl > 0.0));
        assert!(ps.iter().all(|p| p.max_ttl > 0.0));
    }
}