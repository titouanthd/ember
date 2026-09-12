use glam::Vec2;
use macroquad::prelude::Color;
use serde::Deserialize;

use ember_stdlib::collider::{Collider, Shape};
use ember_stdlib::transform::Transform;

use crate::config::GameContext;
use crate::waves::{enemy_style, pos_vec, EnemyData};

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
    pub cooldown: f32,
    pub focus: bool,
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
            cooldown: 0.0,
            focus: false,
            invincible_until: 0.0,
            alive: true,
        }
    }

    pub fn respawn(&mut self, center: Vec2, ctx: &GameContext, now: f32) {
        self.transform.position = center;
        self.cooldown = 0.0;
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
/// Deliberately no `Transform` — bullets don't rotate or scale, so wrapping
/// one would be ceremony. Same reasoning as Snake skipping `Transform` on
/// grid logic.
#[derive(Debug, Clone)]
pub struct Bullet {
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
    pub ttl: f32,
    pub from_player: bool,
    pub color: Color,
}

impl Bullet {
    pub fn player(pos: Vec2, vel: Vec2, ctx: &GameContext) -> Self {
        Self {
            pos,
            vel,
            radius: ctx.bullet_player_radius,
            ttl: ctx.bullet_ttl,
            from_player: true,
            color: ctx.color_bullet_player,
        }
    }

    pub fn enemy(pos: Vec2, vel: Vec2, ctx: &GameContext) -> Self {
        Self {
            pos,
            vel,
            radius: ctx.bullet_enemy_radius,
            ttl: ctx.bullet_ttl,
            from_player: false,
            color: ctx.color_bullet_enemy,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.ttl > 0.0
    }
}

// ---------------------------------------------------------------------------
// Enemies — data-driven
// ---------------------------------------------------------------------------

/// How an enemy enters and moves through the field.
///
/// `Drift.vel` and `Sine` are in playfield-space pixels.
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
    /// Fires `count` bullets evenly around a full circle.
    Radial {
        count: u32,
        speed: f32,
        cooldown: f32,
    },
    /// Fires `count` bullets in a fan aimed at the player's current position.
    Aimed {
        count: u32,
        spread: f32,
        speed: f32,
        cooldown: f32,
    },
    /// Fires `arms` bullets radiating outward, rotating each burst by
    /// `rotation_rate * cooldown` radians.
    Spiral {
        arms: u32,
        speed: f32,
        cooldown: f32,
        rotation_rate: f32,
    },
    /// Fires a horizontal wall of bullets descending, with a gap centered at
    /// `gap_x` of width `gap_w`.
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
    /// Current position (center). Updated by entry motion each frame.
    pub center: Vec2,
    /// Spawn position. `Sine` oscillates around `base_center.x`.
    pub base_center: Vec2,
    pub radius: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub emitter: Emitter,
    /// Seconds until the next burst.
    pub fire_cooldown: f32,
    /// Seconds since spawn. Drives `Sine` phase and is available to emitters.
    pub age: f32,
    /// Rotation accumulator used by `Spiral`, incremented per burst.
    pub phase: f32,
    pub entry: EntryMotion,
    pub color: Color,
    /// Free-form tag from the RON, used for styling and (later) scoring.
    pub kind: String,
    /// White-flash timer, set on hit, decremented each frame.
    pub flash: f32,
}

impl Enemy {
    pub fn from_data(data: &EnemyData, ctx: &GameContext) -> Self {
        let center = pos_vec(data.pos);
        let (radius, default_color) = enemy_style(&data.kind);
        // "grunt" uses the config-tunable color; others get their style color.
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
            fire_cooldown: cooldown + 0.6,
            age: 0.0,
            phase: 0.0,
            entry: data.entry,
            color,
            kind: data.kind.clone(),
            flash: 0.0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Color to draw with, accounting for damage and the hit flash.
    pub fn render_color(&self) -> Color {
        if self.flash > 0.0 {
            return Color::new(1.0, 1.0, 1.0, self.color.a);
        }
        let frac = (self.hp as f32 / self.max_hp as f32).clamp(0.0, 1.0);
        // Keep a floor so near-dead enemies stay visible.
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
    /// Radial burst of `count` particles from `pos`.
    pub fn burst(pos: Vec2, count: u32, color: Color, rng: &mut u32) -> Vec<Particle> {
        let mut out = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let a = rand01(rng) * std::f32::consts::TAU;
            let speed = 40.0 + rand01(rng) * 120.0;
            let ttl = 0.3 + rand01(rng) * 0.35;
            out.push(Particle {
                pos,
                vel: Vec2::new(a.cos(), a.sin()) * speed,
                ttl,
                max_ttl: ttl,
                radius: 1.5 + rand01(rng) * 2.0,
                color,
            });
        }
        out
    }
}

/// Tiny xorshift32. Seeded per-run in `World::new`. Deterministic given
/// the same seed; used only for cosmetic jitter, never for gameplay.
pub fn rand01(state: &mut u32) -> f32 {
    let mut x = *state;
    if x == 0 {
        x = 0x1234_5678;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    // Top 24 bits → [0, 1).
    (x >> 8) as f32 / (1u32 << 24) as f32
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
        b.ttl = 0.0;
        assert!(!b.is_alive());
    }

    #[test]
    fn test_bullet_dies_below_zero_ttl() {
        let ctx = crate::config::load_config();
        let mut b = Bullet::enemy(Vec2::ZERO, Vec2::ZERO, &ctx);
        b.ttl = -0.1;
        assert!(!b.is_alive());
    }

    #[test]
    fn test_rand01_in_unit_range() {
        let mut state = 0xDEAD_BEEF;
        for _ in 0..1000 {
            let v = rand01(&mut state);
            assert!((0.0..1.0).contains(&v), "got {v}");
        }
    }

    #[test]
    fn test_rand01_nonzero_seed_is_stable() {
        let mut a = 42u32;
        let mut b = 42u32;
        assert_eq!(rand01(&mut a), rand01(&mut b));
    }

    #[test]
    fn test_particle_burst_count() {
        let mut rng = 1u32;
        let ps = Particle::burst(Vec2::ZERO, 7, Color::new(1.0, 0.0, 0.0, 1.0), &mut rng);
        assert_eq!(ps.len(), 7);
    }

    #[test]
    fn test_particle_burst_all_alive() {
        let mut rng = 12345u32;
        let ps = Particle::burst(Vec2::ZERO, 20, Color::new(1.0, 1.0, 1.0, 1.0), &mut rng);
        assert!(ps.iter().all(|p| p.ttl > 0.0));
        assert!(ps.iter().all(|p| p.max_ttl > 0.0));
    }
}