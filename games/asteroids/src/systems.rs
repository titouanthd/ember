// games/asteroids/src/systems.rs
//! Logique du jeu : input, physique, spawn, collisions.

use crate::components::{Asteroid, Bullet, Ship};
use ember_stdlib::collider::collides;
use glam::Vec2;
use macroquad::prelude::Color;
use ember_core::app::GameState;
use std::fs;
use std::path::PathBuf;

// ============================================================================
// GameWorld — owns all mutable game state
// ============================================================================

/// All mutable state for one Asteroids run.
///
/// This is the 3rd "state struct" convention in the workspace (after Pong's
/// `MatchState` and Snake's `SnakeWorld`). Extraction is deferred per the
/// Rule of Three: we codify the *shape* (one struct per game owns all
/// mutable state, exposed to `main.rs` as a single `&mut`), not a shared type.
pub struct GameWorld {
    pub ship: Ship,
    pub bullets: Vec<Bullet>,
    pub asteroids: Vec<Asteroid>,
    pub score: i32,
    pub lives: u32,
    pub wave: u32,
    pub invincible_until: f32,
    pub shoot_cooldown: f32,
    pub state: GameState,
    /// Seconds remaining in the LevelCleared transition.
    pub level_cleared_timer: f32,
    pub high_score: i32,
}

impl GameWorld {
    /// Fresh world, in `Start` state, centered ship, no entities.
    pub fn new(ctx: &GameContext) -> Self {
        Self {
            ship: Ship::new(
                Vec2::new(ctx.screen_w / 2.0, ctx.screen_h / 2.0),
                ctx.ship_radius,
                ctx.ship_color,
            ),
            bullets: Vec::new(),
            asteroids: Vec::new(),
            score: 0,
            lives: ctx.lives_start,
            wave: 1,
            invincible_until: 0.0,
            shoot_cooldown: 0.0,
            state: GameState::Start,
            level_cleared_timer: 0.0,
            high_score: load_high_score(),
        }
    }

    /// Reset everything and start wave 1. Sets state to `Playing`.
    pub fn start_new_game(&mut self, ctx: &GameContext) {
        self.ship = Ship::new(
            Vec2::new(ctx.screen_w / 2.0, ctx.screen_h / 2.0),
            ctx.ship_radius,
            ctx.ship_color,
        );
        self.bullets.clear();
        self.asteroids.clear();
        self.score = 0;
        self.lives = ctx.lives_start;
        self.wave = 1;
        self.invincible_until = ctx.invincibility_secs;
        self.shoot_cooldown = 0.0;
        self.level_cleared_timer = 0.0;
        spawn_asteroid_wave(&mut self.asteroids, ctx.asteroids_for_wave(1), ctx);
        self.state = GameState::Playing;
    }

    /// Prepare `self.wave` (already incremented by the caller). Sets state
    /// back to `Playing`.
    pub fn start_next_wave(&mut self, ctx: &GameContext) {
        self.ship = Ship::new(
            Vec2::new(ctx.screen_w / 2.0, ctx.screen_h / 2.0),
            ctx.ship_radius,
            ctx.ship_color,
        );
        self.bullets.clear();
        self.asteroids.clear();
        self.invincible_until = ctx.invincibility_secs;
        self.shoot_cooldown = 0.0;
        spawn_asteroid_wave(&mut self.asteroids, ctx.asteroids_for_wave(self.wave), ctx);
        self.state = GameState::Playing;
    }

    /// Player lost a life. Returns true if this was the last life.
    /// Otherwise respawns the ship and grants invincibility.
    pub fn on_ship_hit(&mut self, ctx: &GameContext) -> bool {
        self.lives = self.lives.saturating_sub(1);
        if self.lives == 0 {
            if self.score > self.high_score {
                self.high_score = self.score;
                save_high_score(self.high_score);
            }
            self.state = GameState::GameOver;
            return true;
        }
        self.ship = Ship::new(
            Vec2::new(ctx.screen_w / 2.0, ctx.screen_h / 2.0),
            ctx.ship_radius,
            ctx.ship_color,
        );
        self.invincible_until = ctx.invincibility_secs;
        self.shoot_cooldown = 0.0;
        false
    }

    /// Called when the last asteroid is destroyed. Starts the wave-clear
    /// timer, or transitions to `Win` if this was the last wave.
    pub fn on_wave_cleared(&mut self, ctx: &GameContext) {
        if self.wave >= ctx.max_waves {
            if self.score > self.high_score {
                self.high_score = self.score;
                save_high_score(self.high_score);
            }
            self.state = GameState::Win;
        } else {
            self.state = GameState::LevelCleared;
            self.level_cleared_timer = ctx.wave_transition_secs;
        }
    }

    /// Tick the LevelCleared timer; if expired, advance the wave (or Win).
    pub fn tick_level_cleared(&mut self, ctx: &GameContext, dt: f32) {
        self.level_cleared_timer -= dt;
        if self.level_cleared_timer <= 0.0 {
            self.wave += 1;
            self.start_next_wave(ctx);
        }
    }

    /// Full reset back to the Start screen (R key from GameOver/Win).
    pub fn reset_to_start(&mut self, ctx: &GameContext) {
        let high = self.high_score;
        *self = Self::new(ctx);
        self.high_score = high;
    }
}

// ============================================================================
// GameContext
// ============================================================================

pub struct GameContext {
    pub screen_w: f32,
    pub screen_h: f32,

    pub ship_radius: f32,
    pub ship_rotation_speed: f32,
    pub ship_acceleration: f32,
    pub ship_max_speed: f32,
    pub invincibility_secs: f32,

    pub bullet_radius: f32,
    pub bullet_speed: f32,
    pub bullet_lifetime: f32,
    pub bullet_cooldown_ms: u32,

    pub asteroid_count_start: u32,
    pub asteroid_speed_min: f32,
    pub asteroid_speed_max: f32,

    pub lives_start: u32,
    pub max_waves: u32,
    pub wave_transition_secs: f32,

    pub ship_color: Color,
    pub bullet_color: Color,
    pub asteroid_color: Color,
    pub bg_color: Color,
    pub star_color: Color,
    pub ui_text_color: Color,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            screen_w: 900.0,
            screen_h: 700.0,
            ship_radius: 14.0,
            ship_rotation_speed: 4.0,
            ship_acceleration: 400.0,
            ship_max_speed: 400.0,
            invincibility_secs: 1.5,
            bullet_radius: 3.0,
            bullet_speed: 600.0,
            bullet_lifetime: 1.0,
            bullet_cooldown_ms: 250,
            asteroid_count_start: 4,
            asteroid_speed_min: 50.0,
            asteroid_speed_max: 150.0,
            lives_start: 3,
            max_waves: 5,
            wave_transition_secs: 1.5,
            ship_color: Color::new(0.9, 0.95, 1.0, 1.0),
            bullet_color: Color::new(1.0, 0.95, 0.5, 1.0),
            asteroid_color: Color::new(0.7, 0.65, 0.6, 1.0),
            bg_color: Color::new(0.04, 0.04, 0.07, 1.0),
            star_color: Color::new(0.5, 0.5, 0.6, 0.8),
            ui_text_color: Color::new(0.85, 0.88, 0.92, 1.0),
        }
    }
}

impl GameContext {
    /// Nombre d'astéroïdes pour une vague donnée (1-indexée).
    /// Vague 1 : 4, vague 2 : 6, vague 3 : 8, etc.
    pub fn asteroids_for_wave(&self, wave: u32) -> u32 {
        self.asteroid_count_start + (wave.saturating_sub(1)) * 2
    }
}

// ============================================================================
// Input
// ============================================================================

#[derive(Debug, Clone, Copy, Default)]
pub struct ShipInput {
    pub rotate_left: bool,
    pub rotate_right: bool,
    pub thrust: bool,
    pub shoot: bool,
}

// ============================================================================
// Événements
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateEvent {
    None,
    ShipDestroyed,
    LevelCleared,
}

// ============================================================================
// Helpers
// ============================================================================

/// Wrap une position dans [0, screen_w] × [0, screen_h].
pub fn wrap_position(p: Vec2, w: f32, h: f32) -> Vec2 {
    Vec2::new(
        ((p.x % w) + w) % w,
        ((p.y % h) + h) % h,
    )
}

fn random_angle() -> f32 {
    macroquad::rand::gen_range(0.0, std::f32::consts::TAU)
}

// ============================================================================
// Couleurs par taille
// ============================================================================

/// Couleur d'un astéroïde selon sa taille.
/// 3 = grand (clair), 2 = moyen, 1 = petit (foncé).
pub fn asteroid_color_for_size(size: u8, base: Color) -> Color {
    let factor = match size {
        3 => 1.0,
        2 => 0.85,
        _ => 0.7,
    };
    Color::new(
        base.r * factor,
        base.g * factor,
        base.b * factor,
        base.a,
    )
}

// ============================================================================
// High score (persistance)
// ============================================================================

/// Chemin du fichier high score.
fn high_score_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("highscore.txt")
}

/// Charge le high score depuis le fichier. Retourne 0 si absent/invalide.
pub fn load_high_score() -> i32 {
    fs::read_to_string(high_score_path())
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Sauvegarde le high score. Ignore les erreurs silencieusement.
pub fn save_high_score(score: i32) {
    let _ = fs::write(high_score_path(), score.to_string());
}

// ============================================================================
// Mise à jour du vaisseau
// ============================================================================

pub fn update_ship(ship: &mut Ship, input: &ShipInput, ctx: &GameContext, dt: f32) {
    if input.rotate_left {
        ship.transform.rotation -= ctx.ship_rotation_speed * dt;
    }
    if input.rotate_right {
        ship.transform.rotation += ctx.ship_rotation_speed * dt;
    }

    if input.thrust {
        let dir = ship.forward();
        ship.velocity += dir * ctx.ship_acceleration * dt;
        let speed = ship.velocity.length();
        if speed > ctx.ship_max_speed {
            ship.velocity *= ctx.ship_max_speed / speed;
        }
    }

    ship.transform.position += ship.velocity * dt;
    ship.transform.position = wrap_position(
        ship.transform.position,
        ctx.screen_w,
        ctx.screen_h,
    );
}

// ============================================================================
// Mise à jour des balles
// ============================================================================

pub fn update_bullets(bullets: &mut Vec<Bullet>, ctx: &GameContext, dt: f32) {
    for b in bullets.iter_mut() {
        b.transform.position += b.velocity * dt;
        b.transform.position = wrap_position(
            b.transform.position,
            ctx.screen_w,
            ctx.screen_h,
        );
        b.lifetime -= dt;
    }
    bullets.retain(|b| b.is_alive());
}

/// Tire une balle depuis le vaisseau. Ne gère PAS le cooldown.
pub fn try_shoot(
    ship: &Ship,
    bullets: &mut Vec<Bullet>,
    ctx: &GameContext,
) {
    let dir = ship.forward();
    let spawn = ship.nose();
    let velocity = dir * ctx.bullet_speed + ship.velocity * 0.5;

    bullets.push(Bullet::new(
        spawn,
        velocity,
        ctx.bullet_radius,
        ctx.bullet_lifetime,
        ctx.bullet_color,
    ));
}

// ============================================================================
// Mise à jour des astéroïdes
// ============================================================================

pub fn update_asteroids(asteroids: &mut [Asteroid], ctx: &GameContext, dt: f32) {
    for a in asteroids.iter_mut() {
        a.transform.position += a.velocity * dt;
        a.transform.position = wrap_position(
            a.transform.position,
            ctx.screen_w,
            ctx.screen_h,
        );
        a.transform.rotation += a.spin * dt;
    }
}

// ============================================================================
// Spawn d'astéroïdes
// ============================================================================

pub fn asteroid_radius(size: u8) -> f32 {
    match size {
        3 => 40.0,
        2 => 20.0,
        _ => 10.0,
    }
}

/// Spawn `count` astéroïdes de taille 3 (grands) sur les bords de l'écran.
pub fn spawn_asteroid_wave(
    asteroids: &mut Vec<Asteroid>,
    count: u32,
    ctx: &GameContext,
) {
    use macroquad::rand::gen_range;

    for _ in 0..count {
        let (x, y) = match gen_range(0, 4) {
            0 => (gen_range(0.0, ctx.screen_w), -50.0),
            1 => (gen_range(0.0, ctx.screen_w), ctx.screen_h + 50.0),
            2 => (-50.0, gen_range(0.0, ctx.screen_h)),
            _ => (ctx.screen_w + 50.0, gen_range(0.0, ctx.screen_h)),
        };

        let angle = random_angle();
        let speed = gen_range(ctx.asteroid_speed_min, ctx.asteroid_speed_max);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
        let spin = gen_range(-1.0, 1.0);

        asteroids.push(Asteroid::new(
            Vec2::new(x, y),
            velocity,
            spin,
            3,
            asteroid_radius(3),
            asteroid_color_for_size(3, ctx.asteroid_color),
        ));
    }
}

// ============================================================================
// Collisions
// ============================================================================

/// Détecte et résout les collisions balle ↔ astéroïde.
pub fn resolve_bullet_asteroid_collisions(
    bullets: &mut Vec<Bullet>,
    asteroids: &mut Vec<Asteroid>,
    ctx: &GameContext,
) -> u32 {
    let mut destroyed = 0;

    let mut bullets_to_remove = Vec::new();
    let mut asteroids_to_split = Vec::new();

    for (bi, b) in bullets.iter().enumerate() {
        for (ai, a) in asteroids.iter().enumerate() {
            if collides(
                b.transform.position,
                &b.collider,
                a.transform.position,
                &a.collider,
            ) {
                bullets_to_remove.push(bi);
                asteroids_to_split.push(ai);
                break;
            }
        }
    }

    asteroids_to_split.sort_unstable();
    asteroids_to_split.dedup();
    for ai in asteroids_to_split.iter().rev() {
        let a = asteroids.remove(*ai);
        destroyed += 1;

        if a.size > 1 {
            for k in 0..2 {
                let new_size = a.size - 1;
                let base_angle = a.velocity.y.atan2(a.velocity.x);
                let split_angle = base_angle + if k == 0 { 0.5 } else { -0.5 };
                let speed = a.velocity.length().max(60.0) * 1.2;
                let new_vel = Vec2::new(split_angle.cos(), split_angle.sin()) * speed;
                let spin = -a.spin;
                asteroids.push(Asteroid::new(
                    a.transform.position,
                    new_vel,
                    spin,
                    new_size,
                    asteroid_radius(new_size),
                    asteroid_color_for_size(new_size, ctx.asteroid_color),
                ));
            }
        }
    }

    bullets_to_remove.sort_unstable();
    bullets_to_remove.dedup();
    for bi in bullets_to_remove.iter().rev() {
        bullets.remove(*bi);
    }

    destroyed
}

/// Détecte les collisions vaisseau ↔ astéroïde.
pub fn resolve_ship_asteroid_collision(
    ship: &Ship,
    asteroids: &[Asteroid],
) -> bool {
    for a in asteroids {
        if collides(
            ship.transform.position,
            &ship.collider,
            a.transform.position,
            &a.collider,
        ) {
            return true;
        }
    }
    false
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Asteroid, Bullet, Ship};
    use macroquad::prelude::Color;

    fn ctx() -> GameContext {
        GameContext::default()
    }

    fn color() -> Color {
        Color::new(1.0, 1.0, 1.0, 1.0)
    }

    // --- wrap_position ---

    #[test]
    fn test_wrap_inside_stays() {
        let p = wrap_position(Vec2::new(100.0, 200.0), 800.0, 600.0);
        assert_eq!(p, Vec2::new(100.0, 200.0));
    }

    #[test]
    fn test_wrap_negative_wraps_to_max() {
        let p = wrap_position(Vec2::new(-10.0, 100.0), 800.0, 600.0);
        assert!((p.x - 790.0).abs() < 1e-4);
    }

    #[test]
    fn test_wrap_above_max_wraps_to_zero() {
        let p = wrap_position(Vec2::new(810.0, 100.0), 800.0, 600.0);
        assert!((p.x - 10.0).abs() < 1e-4);
    }

    // --- ship ---

    #[test]
    fn test_ship_rotate_left() {
        let mut ship = Ship::new(Vec2::ZERO, 10.0, color());
        let input = ShipInput { rotate_left: true, ..Default::default() };
        update_ship(&mut ship, &input, &ctx(), 0.1);
        assert!(ship.transform.rotation < 0.0);
    }

    #[test]
    fn test_ship_thrust_accelerates() {
        let mut ship = Ship::new(Vec2::ZERO, 10.0, color());
        ship.transform.rotation = 0.0;
        let input = ShipInput { thrust: true, ..Default::default() };
        update_ship(&mut ship, &input, &ctx(), 0.1);
        assert!(ship.velocity.x > 0.0);
    }

    #[test]
    fn test_ship_velocity_clamped() {
        let cx = ctx();
        let mut ship = Ship::new(Vec2::ZERO, 10.0, color());
        ship.velocity = Vec2::new(10_000.0, 0.0);
        let input = ShipInput { thrust: true, ..Default::default() };
        update_ship(&mut ship, &input, &cx, 0.1);
        assert!(ship.velocity.length() <= cx.ship_max_speed + 1e-3);
    }

    // --- bullets ---

    #[test]
    fn test_bullet_moves_and_dies() {
        let cx = ctx();
        let mut bullets = vec![Bullet::new(
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            3.0,
            0.05,
            color(),
        )];
        update_bullets(&mut bullets, &cx, 0.1);
        assert_eq!(bullets.len(), 0);
    }

    #[test]
    fn test_try_shoot_spawns_bullet() {
        let cx = ctx();
        let ship = Ship::new(Vec2::new(100.0, 100.0), 10.0, color());
        let mut bullets = Vec::new();
        try_shoot(&ship, &mut bullets, &cx);
        assert_eq!(bullets.len(), 1);
    }

    // --- wave / progression ---

    #[test]
    fn test_asteroids_for_wave() {
        let cx = ctx(); // asteroid_count_start = 4
        assert_eq!(cx.asteroids_for_wave(1), 4);
        assert_eq!(cx.asteroids_for_wave(2), 6);
        assert_eq!(cx.asteroids_for_wave(3), 8);
        assert_eq!(cx.asteroids_for_wave(5), 12);
    }

    #[test]
    fn test_asteroid_color_for_size_darkens() {
        let base = Color::new(1.0, 1.0, 1.0, 1.0);
        let c3 = asteroid_color_for_size(3, base);
        let c2 = asteroid_color_for_size(2, base);
        let c1 = asteroid_color_for_size(1, base);
        assert!(c3.r >= c2.r);
        assert!(c2.r >= c1.r);
    }

    // --- collisions ---

    #[test]
    fn test_bullet_destroys_small_asteroid() {
        let cx = ctx();
        let mut bullets = vec![Bullet::new(
            Vec2::new(100.0, 100.0),
            Vec2::new(0.0, 0.0),
            3.0,
            10.0,
            color(),
        )];
        let mut asteroids = vec![Asteroid::new(
            Vec2::new(100.0, 100.0),
            Vec2::ZERO,
            0.0,
            1,
            10.0,
            color(),
        )];
        let destroyed = resolve_bullet_asteroid_collisions(&mut bullets, &mut asteroids, &cx);
        assert_eq!(destroyed, 1);
        assert_eq!(bullets.len(), 0);
        assert_eq!(asteroids.len(), 0);
    }

    #[test]
    fn test_big_asteroid_splits_in_two() {
        let cx = ctx();
        let mut bullets = vec![Bullet::new(
            Vec2::new(100.0, 100.0),
            Vec2::new(0.0, 0.0),
            3.0,
            10.0,
            color(),
        )];
        let mut asteroids = vec![Asteroid::new(
            Vec2::new(100.0, 100.0),
            Vec2::new(100.0, 0.0),
            0.0,
            3,
            40.0,
            color(),
        )];
        let destroyed = resolve_bullet_asteroid_collisions(&mut bullets, &mut asteroids, &cx);
        assert_eq!(destroyed, 1);
        assert_eq!(asteroids.len(), 2);
        assert!(asteroids.iter().all(|a| a.size == 2));
    }

    #[test]
    fn test_ship_asteroid_collision() {
        let ship = Ship::new(Vec2::new(100.0, 100.0), 10.0, color());
        let asteroids = vec![Asteroid::new(
            Vec2::new(105.0, 100.0),
            Vec2::ZERO,
            0.0,
            1,
            10.0,
            color(),
        )];
        assert!(resolve_ship_asteroid_collision(&ship, &asteroids));
    }
}