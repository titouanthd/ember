//! Game logic. The single entry point is [`update`].
//!
//! `World` owns all mutable state. This is the 3rd state struct in the
//! workspace (after Pong's `MatchState` and Snake's `SnakeWorld`) — a
//! convention, not a shared type. See `version.txt`.

use glam::Vec2;

use ember_core::app::GameState;
use ember_core::rng::Rng;

use crate::components::{Bullet, Emitter, Enemy, EntryMotion, Particle, Player};
use crate::config::GameContext;
use crate::waves::{WaveData, load_waves};
use macroquad::prelude::Color;

/// Input snapshot for one frame. Decoupled from macroquad so tests can drive
/// the world without a window.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShipInput {
    /// -1, 0, or +1.
    pub dx: f32,
    /// -1, 0, or +1.
    pub dy: f32,
    pub focus: bool,
    pub fire: bool,
}

/// All mutable game state.
pub struct World {
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub player_bullets: Vec<Bullet>,
    pub enemy_bullets: Vec<Bullet>,
    pub particles: Vec<Particle>,
    pub score: u32,
    pub lives: i32,
    pub wave: u32,
    pub waves: Vec<WaveData>,
    pub state: GameState,
    /// Seconds since world creation. Drives invincibility, blink, timers.
    pub now: f32,
    /// Seconds the current wave has been running.
    pub wave_elapsed: f32,
    /// Wall-clock time at which `LevelCleared` will transition to the next
    /// wave. Only valid while `state == LevelCleared`.
    pub level_cleared_until: f32,

    // --- Juice ---
    pub shake_magnitude: f32,
    pub shake_decay: f32,
    pub hitstop_until: f32,
    pub rng: Rng,

    // --- Scoring ---
    pub graze_count: u32,
    pub multiplier: f32,
}

impl World {
    pub fn new(ctx: &GameContext) -> Self {
        let center = Vec2::new(ctx.playfield_w() * 0.5, ctx.playfield_h() * 0.8);
        Self {
            player: Player::new(center, ctx),
            enemies: Vec::new(),
            player_bullets: Vec::new(),
            enemy_bullets: Vec::new(),
            particles: Vec::new(),
            score: 0,
            lives: 3,
            wave: 0,
            waves: load_waves(),
            state: GameState::Start,
            now: 0.0,
            wave_elapsed: 0.0,
            level_cleared_until: 0.0,
            shake_magnitude: 0.0,
            shake_decay: 6.0,
            hitstop_until: 0.0,
            rng: Rng::new(0xDEAD_BEEF),
            graze_count: 0,
            multiplier: 1.0,
        }
    }

    /// Reset to a fresh game, keeping `state` untouched.
    pub fn reset(&mut self, ctx: &GameContext) {
        let center = Vec2::new(ctx.playfield_w() * 0.5, ctx.playfield_h() * 0.8);
        self.player = Player::new(center, ctx);
        self.enemies.clear();
        self.player_bullets.clear();
        self.enemy_bullets.clear();
        self.particles.clear();
        self.score = 0;
        self.lives = 3;
        self.wave = 0;
        self.waves = load_waves();
        self.now = 0.0;
        self.wave_elapsed = 0.0;
        self.level_cleared_until = 0.0;
        self.shake_magnitude = 0.0;
        self.hitstop_until = 0.0;
        self.rng = Rng::new(0xDEAD_BEEF);
        self.graze_count = 0;
        self.multiplier = 1.0;
    }

    /// Start a run: reset and enter `Playing` with the first wave spawned.
    pub fn start_run(&mut self, ctx: &GameContext) {
        self.reset(ctx);
        self.state = GameState::Playing;
        spawn_wave(self, ctx, 0);
    }
}

// ---------------------------------------------------------------------------
// Top-level update
// ---------------------------------------------------------------------------

pub fn update(world: &mut World, input: ShipInput, ctx: &GameContext, dt: f32) {
    // Hitstop freezes the simulation but keeps time advancing so timers
    // don't get stuck. Shake decays regardless.
    if world.now < world.hitstop_until {
        world.now += dt;
        world.shake_magnitude = (world.shake_magnitude - world.shake_decay * dt).max(0.0);
        return;
    }

    world.now += dt;

    // Shake decay — applied every frame.
    if world.shake_magnitude > 0.0 {
        world.shake_magnitude = (world.shake_magnitude - world.shake_decay * dt).max(0.0);
    }

    // Multiplier decay — grazing must be continuous to maintain a high
    // multiplier. Losing multiplier is slow (0.5/s) so a single graze gap
    // doesn't erase a full chain.
    if world.multiplier > 1.0 {
        world.multiplier = (world.multiplier - 0.5 * dt).max(1.0);
    }

    match world.state {
        GameState::Start | GameState::GameOver | GameState::Win => {
            // No simulation.
        }
        GameState::Playing => {
            update_player(world, input, ctx, dt);
            try_fire_player(world, input, ctx);
            update_bullets(world, ctx, dt);
            update_enemies(world, ctx, dt);
            update_particles(world, dt);
            resolve_player_bullet_vs_enemies(world, ctx);
            resolve_enemy_bullet_vs_player(world, ctx);

            world.wave_elapsed += dt;

            // Wave cleared: all enemies dead. Small grace before advancing so
            // the last enemy's death particles finish.
            if world.enemies.is_empty() && world.wave_elapsed >= 1.5 {
                let next = world.wave + 1;
                if (next as usize) < world.waves.len() {
                    world.state = GameState::LevelCleared;
                    world.level_cleared_until = world.now + 1.5;
                } else {
                    world.state = GameState::Win;
                }
            }
        }
        GameState::LevelCleared => {
            // Player still moves; no enemies on screen.
            update_player(world, input, ctx, dt);
            try_fire_player(world, input, ctx);
            update_bullets(world, ctx, dt);
            update_particles(world, dt);
            resolve_enemy_bullet_vs_player(world, ctx);

            if world.now >= world.level_cleared_until {
                spawn_wave(world, ctx, world.wave + 1);
                world.state = GameState::Playing;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Player
// ---------------------------------------------------------------------------

pub fn update_player(world: &mut World, input: ShipInput, ctx: &GameContext, dt: f32) {
    if !world.player.alive {
        return;
    }
    world.player.focus = input.focus;

    // Normalize diagonal so 8-way movement isn't faster on diagonals.
    let mut dir = Vec2::new(input.dx, input.dy);
    if dir.length_squared() > 0.0 {
        dir = dir.normalize();
    }
    let speed = world.player.speed(ctx);
    world.player.transform.position += dir * speed * dt;

    // Clamp to playfield (hard walls, no wrap).
    let r = ctx.player_radius;
    let max_x = ctx.playfield_w() - r;
    let max_y = ctx.playfield_h() - r;
    world.player.transform.position.x = world.player.transform.position.x.clamp(r, max_x);
    world.player.transform.position.y = world.player.transform.position.y.clamp(r, max_y);

    if world.player.cooldown > 0.0 {
        world.player.cooldown -= dt;
    }
}

/// Fire a player bullet when `fire` is held and cooldown is ready.
pub fn try_fire_player(world: &mut World, input: ShipInput, ctx: &GameContext) {
    if !world.player.alive || !input.fire || world.player.cooldown > 0.0 {
        return;
    }
    let origin = world.player.transform.position + Vec2::new(0.0, -ctx.player_radius);
    let vel = Vec2::new(0.0, -ctx.bullet_player_speed);
    world.player_bullets.push(Bullet::player(origin, vel, ctx));
    world.player.cooldown = ctx.player_fire_rate;
}

// ---------------------------------------------------------------------------
// Bullets
// ---------------------------------------------------------------------------

pub fn update_bullets(world: &mut World, ctx: &GameContext, dt: f32) {
    for b in world
        .player_bullets
        .iter_mut()
        .chain(world.enemy_bullets.iter_mut())
    {
        b.pos += b.vel * dt;
        b.ttl -= dt;
    }

    let w = ctx.playfield_w();
    let h = ctx.playfield_h();
    // Give a small margin so bullets don't visually pop at the edge.
    let margin = 24.0;
    let in_bounds = |p: Vec2| -> bool {
        p.x >= -margin && p.x <= w + margin && p.y >= -margin && p.y <= h + margin
    };

    world
        .player_bullets
        .retain(|b| b.is_alive() && in_bounds(b.pos));
    world
        .enemy_bullets
        .retain(|b| b.is_alive() && in_bounds(b.pos));
}

// ---------------------------------------------------------------------------
// Enemies
// ---------------------------------------------------------------------------

pub fn update_enemies(world: &mut World, ctx: &GameContext, dt: f32) {
    let player_pos = world.player.transform.position;
    let playfield_w = ctx.playfield_w();

    // Collect spawn requests first to avoid borrowing `world.enemy_bullets`
    // while `world.enemies` is borrowed mutably.
    let mut to_spawn: Vec<(Vec2, Vec2)> = Vec::new();

    for e in world.enemies.iter_mut() {
        e.age += dt;
        e.fire_cooldown -= dt;
        if e.flash > 0.0 {
            e.flash = (e.flash - dt).max(0.0);
        }

        // --- Entry motion ---
        match e.entry {
            EntryMotion::Static => {}
            EntryMotion::Drift { vel } => {
                e.center += Vec2::new(vel.0, vel.1) * dt;
                // Bounce horizontally at edges.
                if e.center.x < e.radius {
                    e.center.x = e.radius;
                } else if e.center.x > playfield_w - e.radius {
                    e.center.x = playfield_w - e.radius;
                }
            }
            EntryMotion::Sine {
                amplitude,
                frequency,
                base_y,
            } => {
                e.center.y = base_y;
                e.center.x =
                    e.base_center.x + (e.age * frequency * std::f32::consts::TAU).sin() * amplitude;
            }
        }

        // --- Emitter ---
        if e.fire_cooldown > 0.0 {
            continue;
        }

        // Clone so we can move `e`'s mutable borrow around freely.
        match e.emitter.clone() {
            Emitter::Radial {
                count,
                speed,
                cooldown,
            } => {
                e.fire_cooldown = cooldown;
                let n = count.max(1);
                for i in 0..n {
                    let a = e.phase + (i as f32) * std::f32::consts::TAU / (n as f32);
                    to_spawn.push((e.center, Vec2::new(a.cos(), a.sin()) * speed));
                }
                // Small phase offset each burst so consecutive rings differ.
                e.phase += 0.13;
            }
            Emitter::Aimed {
                count,
                spread,
                speed,
                cooldown,
            } => {
                e.fire_cooldown = cooldown;
                let n = count.max(1);
                let base = (player_pos - e.center).to_angle();
                for i in 0..n {
                    let t = if n == 1 {
                        0.0
                    } else {
                        (i as f32) / ((n - 1) as f32) - 0.5
                    };
                    let a = base + t * spread;
                    to_spawn.push((e.center, Vec2::new(a.cos(), a.sin()) * speed));
                }
            }
            Emitter::Spiral {
                arms,
                speed,
                cooldown,
                rotation_rate,
            } => {
                e.fire_cooldown = cooldown;
                let n = arms.max(1);
                for i in 0..n {
                    let a = e.phase + (i as f32) * std::f32::consts::TAU / (n as f32);
                    to_spawn.push((e.center, Vec2::new(a.cos(), a.sin()) * speed));
                }
                e.phase += rotation_rate * cooldown;
            }
            Emitter::Wall {
                gap_x,
                gap_w,
                speed,
                cooldown,
            } => {
                e.fire_cooldown = cooldown;
                let spacing = 40.0;
                let mut x = 20.0;
                while x < playfield_w - 20.0 {
                    if (x - gap_x).abs() > gap_w * 0.5 {
                        to_spawn.push((Vec2::new(x, e.center.y), Vec2::new(0.0, speed)));
                    }
                    x += spacing;
                }
            }
        }
    }

    for (pos, vel) in to_spawn {
        world.enemy_bullets.push(Bullet::enemy(pos, vel, ctx));
    }

    world.enemies.retain(|e| e.is_alive());
}

// ---------------------------------------------------------------------------
// Collisions
// ---------------------------------------------------------------------------

pub fn resolve_player_bullet_vs_enemies(world: &mut World, _ctx: &GameContext) {
    let mut score_gain: u32 = 0;
    let mut died: Vec<(Vec2, Color)> = Vec::new();

    for e in world.enemies.iter_mut() {
        if !e.is_alive() {
            continue;
        }
        for b in world.player_bullets.iter_mut() {
            if !b.is_alive() {
                continue;
            }
            let r = e.radius + b.radius;
            if (e.center - b.pos).length_squared() <= r * r {
                e.hp -= 1;
                e.flash = 0.08;
                b.ttl = 0.0; // consume the bullet
                if !e.is_alive() {
                    score_gain += (100.0 * world.multiplier) as u32;
                    died.push((e.center, e.color));
                }
            }
        }
    }

    world.score += score_gain;

    // Death feedback: particles + a small shake.
    if !died.is_empty() {
        world.shake_magnitude = world.shake_magnitude.max(5.0);
        for (pos, color) in died {
            let burst = Particle::burst(pos, 14, color, &mut world.rng);
            world.particles.extend(burst);
        }
    }

    world.enemies.retain(|e| e.is_alive());
    world.player_bullets.retain(|b| b.is_alive());
}

pub fn resolve_enemy_bullet_vs_player(world: &mut World, ctx: &GameContext) {
    if !world.player.alive {
        return;
    }

    let pp = world.player.transform.position;
    let hit_r = ctx.player_hitbox_radius;
    let graze_r = ctx.player_graze_radius;
    let invincible = world.player.is_invincible(world.now);

    let mut hit = false;
    for b in world.enemy_bullets.iter_mut() {
        if !b.is_alive() {
            continue;
        }
        let d2 = (b.pos - pp).length_squared();
        let hit_d = hit_r + b.radius;
        let graze_d = graze_r + b.radius;

        if d2 <= hit_d * hit_d {
            b.ttl = 0.0;
            if !invincible {
                hit = true;
            }
        } else if !invincible && d2 <= graze_d * graze_d {
            world.graze_count += 1;
            world.multiplier = (world.multiplier + 0.1).min(8.0);
            let spark = Particle::burst(b.pos, 1, ctx.color_bullet_enemy, &mut world.rng);
            world.particles.extend(spark);
        }
    }

    if hit {
        on_player_hit(world, ctx);
    }
    world.enemy_bullets.retain(|b| b.is_alive());
}

fn on_player_hit(world: &mut World, ctx: &GameContext) {
    // Juice: strong shake + a brief hitstop.
    world.shake_magnitude = 14.0;
    world.hitstop_until = world.now + 0.08;

    // Death burst at the player position.
    let pos = world.player.transform.position;
    let burst = Particle::burst(pos, 20, ctx.color_player, &mut world.rng);
    world.particles.extend(burst);

    world.lives -= 1;
    if world.lives <= 0 {
        world.player.alive = false;
        world.state = GameState::GameOver;
        return;
    }

    // Respawn at bottom-center with invincibility.
    let center = Vec2::new(ctx.playfield_w() * 0.5, ctx.playfield_h() * 0.8);
    world.player.respawn(center, ctx, world.now);
    // Multiplier resets on death — the risk-reward loop restarts.
    world.multiplier = 1.0;
}

// ---------------------------------------------------------------------------
// Particles
// ---------------------------------------------------------------------------

pub fn update_particles(world: &mut World, dt: f32) {
    for p in world.particles.iter_mut() {
        p.pos += p.vel * dt;
        p.ttl -= dt;
        // Drag so particles slow down, feels less linear.
        p.vel *= 1.0 - 3.0 * dt;
    }
    world.particles.retain(|p| p.ttl > 0.0);
}

// ---------------------------------------------------------------------------
// Waves
// ---------------------------------------------------------------------------

/// Spawn the enemies for wave `wave_idx`, clearing any existing enemies and
/// enemy bullets. If `wave_idx` is out of range, transitions to `Win`.
pub fn spawn_wave(world: &mut World, ctx: &GameContext, wave_idx: u32) {
    let Some(data) = world.waves.get(wave_idx as usize).cloned() else {
        world.state = GameState::Win;
        return;
    };

    world.enemies.clear();
    world.enemy_bullets.clear();

    for ed in &data.enemies {
        world.enemies.push(Enemy::from_data(ed, ctx));
    }

    world.wave = wave_idx;
    world.wave_elapsed = 0.0;
}

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------

/// Full reset back to the Start screen.
pub fn reset(world: &mut World, ctx: &GameContext) {
    world.reset(ctx);
    world.state = GameState::Start;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> GameContext {
        crate::config::load_config()
    }

    fn play_world(ctx: &GameContext) -> World {
        let mut w = World::new(ctx);
        w.start_run(ctx);
        w
    }

    #[test]
    fn test_world_starts_in_start_state() {
        let ctx = ctx();
        let w = World::new(&ctx);
        assert_eq!(w.state, GameState::Start);
        assert_eq!(w.lives, 3);
        assert_eq!(w.score, 0);
        assert_eq!(w.multiplier, 1.0);
    }

    #[test]
    fn test_start_run_spawns_first_wave() {
        let ctx = ctx();
        let w = play_world(&ctx);
        assert_eq!(w.state, GameState::Playing);
        assert!(!w.enemies.is_empty());
        assert_eq!(w.wave, 0);
    }

    #[test]
    fn test_player_moves_with_input() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let start = w.player.transform.position;
        let input = ShipInput {
            dx: 1.0,
            dy: 0.0,
            focus: false,
            fire: false,
        };
        update(&mut w, input, &ctx, 0.1);
        assert!(w.player.transform.position.x > start.x);
    }

    #[test]
    fn test_focus_halves_speed() {
        let ctx = ctx();
        let mut w1 = play_world(&ctx);
        let mut w2 = play_world(&ctx);
        // Move both from a known position so no clamping interferes.
        let start = Vec2::new(ctx.playfield_w() * 0.5, ctx.playfield_h() * 0.5);
        w1.player.transform.position = start;
        w2.player.transform.position = start;

        let normal = ShipInput {
            dx: 1.0,
            dy: 0.0,
            focus: false,
            fire: false,
        };
        let focused = ShipInput {
            dx: 1.0,
            dy: 0.0,
            focus: true,
            fire: false,
        };

        update_player(&mut w1, normal, &ctx, 0.1);
        update_player(&mut w2, focused, &ctx, 0.1);

        let d1 = w1.player.transform.position.x - start.x;
        let d2 = w2.player.transform.position.x - start.x;
        assert!(d2 < d1);
        let ratio = d2 / d1;
        assert!((ratio - ctx.player_focus_mult).abs() < 0.05);
    }

    #[test]
    fn test_player_clamped_to_playfield() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        // Try to shove the player far outside.
        let input = ShipInput {
            dx: -1.0,
            dy: 0.0,
            focus: false,
            fire: false,
        };
        for _ in 0..1000 {
            update_player(&mut w, input, &ctx, 0.016);
        }
        assert!(w.player.transform.position.x >= ctx.player_radius - 0.01);
    }

    #[test]
    fn test_fire_spawns_bullet() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let input = ShipInput {
            dx: 0.0,
            dy: 0.0,
            focus: false,
            fire: true,
        };
        try_fire_player(&mut w, input, &ctx);
        assert_eq!(w.player_bullets.len(), 1);
    }

    #[test]
    fn test_fire_respects_cooldown() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let input = ShipInput {
            dx: 0.0,
            dy: 0.0,
            focus: false,
            fire: true,
        };
        try_fire_player(&mut w, input, &ctx);
        try_fire_player(&mut w, input, &ctx);
        assert_eq!(
            w.player_bullets.len(),
            1,
            "cooldown should block second shot"
        );
    }

    #[test]
    fn test_bullet_ttl_culls() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        w.player_bullets
            .push(Bullet::player(Vec2::ZERO, Vec2::ZERO, &ctx));
        w.player_bullets[0].ttl = 0.0;
        update_bullets(&mut w, &ctx, 0.016);
        assert!(w.player_bullets.is_empty());
    }

    #[test]
    fn test_bullet_off_screen_culls() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        w.player_bullets
            .push(Bullet::player(Vec2::new(-1000.0, 0.0), Vec2::ZERO, &ctx));
        update_bullets(&mut w, &ctx, 0.016);
        assert!(w.player_bullets.is_empty());
    }

    #[test]
    fn test_player_bullet_damages_enemy() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let e_pos = w.enemies[0].center;
        let hp_before = w.enemies[0].hp;
        w.player_bullets
            .push(Bullet::player(e_pos, Vec2::ZERO, &ctx));
        resolve_player_bullet_vs_enemies(&mut w, &ctx);
        // Either the enemy took damage, or it died and was removed.
        let hp_after = w.enemies.first().map(|e| e.hp).unwrap_or(0);
        assert!(hp_after < hp_before);
    }

    #[test]
    fn test_enemy_dies_at_zero_hp() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let e_pos = w.enemies[0].center;
        w.enemies[0].hp = 1;
        w.player_bullets
            .push(Bullet::player(e_pos, Vec2::ZERO, &ctx));
        resolve_player_bullet_vs_enemies(&mut w, &ctx);
        assert!(w.enemies.is_empty());
    }

    #[test]
    fn test_player_hit_loses_life() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let pp = w.player.transform.position;
        w.enemy_bullets.push(Bullet::enemy(pp, Vec2::ZERO, &ctx));
        resolve_enemy_bullet_vs_player(&mut w, &ctx);
        assert_eq!(w.lives, 2);
    }

    #[test]
    fn test_invincibility_blocks_hit() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        w.player.invincible_until = w.now + 10.0;
        let pp = w.player.transform.position;
        w.enemy_bullets.push(Bullet::enemy(pp, Vec2::ZERO, &ctx));
        resolve_enemy_bullet_vs_player(&mut w, &ctx);
        assert_eq!(w.lives, 3);
    }

    #[test]
    fn test_graze_increments_multiplier() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let pp = w.player.transform.position;
        // Place a bullet just outside hit radius but inside graze radius.
        let offset = Vec2::new(ctx.player_graze_radius * 0.7, 0.0);
        w.enemy_bullets
            .push(Bullet::enemy(pp + offset, Vec2::ZERO, &ctx));
        resolve_enemy_bullet_vs_player(&mut w, &ctx);
        assert!(w.graze_count >= 1);
        assert!(w.multiplier > 1.0);
        assert_eq!(w.lives, 3, "graze must not kill");
    }

    #[test]
    fn test_multiplier_caps_at_8() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        w.multiplier = 8.0;
        let pp = w.player.transform.position;
        let offset = Vec2::new(ctx.player_graze_radius * 0.7, 0.0);
        w.enemy_bullets
            .push(Bullet::enemy(pp + offset, Vec2::ZERO, &ctx));
        resolve_enemy_bullet_vs_player(&mut w, &ctx);
        assert!(w.multiplier <= 8.0);
    }

    #[test]
    fn test_game_over_at_zero_lives() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        w.lives = 1;
        let pp = w.player.transform.position;
        w.enemy_bullets.push(Bullet::enemy(pp, Vec2::ZERO, &ctx));
        resolve_enemy_bullet_vs_player(&mut w, &ctx);
        assert_eq!(w.state, GameState::GameOver);
        assert!(!w.player.alive);
    }

    #[test]
    fn test_wave_cleared_after_grace() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        w.enemies.clear();
        w.wave_elapsed = 2.0;
        // Advance without any input — just checking transition.
        let input = ShipInput::default();
        update(&mut w, input, &ctx, 0.016);
        assert!(matches!(w.state, GameState::LevelCleared | GameState::Win));
    }

    #[test]
    fn test_level_cleared_advances_wave() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        w.state = GameState::LevelCleared;
        w.level_cleared_until = w.now - 1.0; // already elapsed
        let input = ShipInput::default();
        update(&mut w, input, &ctx, 0.016);
        assert_eq!(w.state, GameState::Playing);
        assert_eq!(w.wave, 1);
    }

    #[test]
    fn test_reset_clears_state() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        w.score = 500;
        w.graze_count = 12;
        w.multiplier = 3.0;
        reset(&mut w, &ctx);
        assert_eq!(w.state, GameState::Start);
        assert_eq!(w.score, 0);
        assert_eq!(w.graze_count, 0);
        assert_eq!(w.multiplier, 1.0);
        assert!(w.enemies.is_empty());
        assert!(w.player_bullets.is_empty());
        assert!(w.enemy_bullets.is_empty());
        assert!(w.particles.is_empty());
    }

    #[test]
    fn test_sine_entry_oscillates() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        // Push a controlled sine enemy in.
        let mut e = Enemy::from_data(
            &crate::waves::EnemyData {
                kind: "grunt".into(),
                entry: EntryMotion::Sine {
                    amplitude: 50.0,
                    frequency: 1.0,
                    base_y: 100.0,
                },
                pos: (400.0, 100.0),
                hp: 5,
                emitter: Emitter::Radial {
                    count: 1,
                    speed: 50.0,
                    cooldown: 100.0,
                },
            },
            &ctx,
        );
        e.fire_cooldown = 100.0; // don't fire, we only care about motion
        w.enemies.clear();
        w.enemies.push(e);

        update_enemies(&mut w, &ctx, 0.25);
        let x1 = w.enemies[0].center.x;

        update_enemies(&mut w, &ctx, 0.25);
        let x2 = w.enemies[0].center.x;

        // Two samples a quarter-period apart should differ for a sine wave.
        assert!((x1 - x2).abs() > 0.5, "sine should move: {x1} vs {x2}");
    }

    #[test]
    fn test_drift_entry_moves_enemy() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let mut e = Enemy::from_data(
            &crate::waves::EnemyData {
                kind: "grunt".into(),
                entry: EntryMotion::Drift { vel: (100.0, 0.0) },
                pos: (400.0, 100.0),
                hp: 5,
                emitter: Emitter::Radial {
                    count: 1,
                    speed: 50.0,
                    cooldown: 100.0,
                },
            },
            &ctx,
        );
        e.fire_cooldown = 100.0;
        w.enemies.clear();
        w.enemies.push(e);

        let x0 = w.enemies[0].center.x;
        update_enemies(&mut w, &ctx, 0.1);
        assert!(w.enemies[0].center.x > x0);
    }

    #[test]
    fn test_radial_emitter_spawns_count_bullets() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let mut e = Enemy::from_data(
            &crate::waves::EnemyData {
                kind: "grunt".into(),
                entry: EntryMotion::Static,
                pos: (400.0, 100.0),
                hp: 100,
                emitter: Emitter::Radial {
                    count: 6,
                    speed: 80.0,
                    cooldown: 0.1,
                },
            },
            &ctx,
        );
        e.fire_cooldown = 0.0;
        w.enemies.clear();
        w.enemies.push(e);
        w.enemy_bullets.clear();

        update_enemies(&mut w, &ctx, 0.016);
        assert_eq!(w.enemy_bullets.len(), 6);
    }

    #[test]
    fn test_wall_emitter_leaves_gap() {
        let ctx = ctx();
        let mut w = play_world(&ctx);
        let mut e = Enemy::from_data(
            &crate::waves::EnemyData {
                kind: "tank".into(),
                entry: EntryMotion::Static,
                pos: (400.0, 100.0),
                hp: 100,
                emitter: Emitter::Wall {
                    gap_x: 400.0,
                    gap_w: 120.0,
                    speed: 100.0,
                    cooldown: 0.1,
                },
            },
            &ctx,
        );
        e.fire_cooldown = 0.0;
        w.enemies.clear();
        w.enemies.push(e);
        w.enemy_bullets.clear();

        update_enemies(&mut w, &ctx, 0.016);

        // No bullet should be within gap_w/2 of gap_x.
        for b in &w.enemy_bullets {
            let dist = (b.pos.x - 400.0).abs();
            assert!(dist > 60.0, "bullet at x={} inside gap", b.pos.x);
        }
    }
}
