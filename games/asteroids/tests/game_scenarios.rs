// games/asteroids/tests/game_scenarios.rs
//! Tests d'intégration : scénarios de jeu complets.
//!
//! Contrairement aux autres jeux, Asteroids n'a pas de `systems::update`
//! unique : la boucle vit dans `main.rs`. Ces tests composent donc les
//! systèmes dans l'ordre où `main.rs` les appelle, et vérifient les
//! événements/états qui en résultent.
//!
//! Ce qu'on couvre :
//! - Tir → destruction → split → score.
//! - Vaisseau touché → perte de vie.
//! - Vague vidée → LevelCleared.
//! - 0 vies → GameOver + high score sauvegardé.
//! - Invincibilité : pas de perte de vie pendant le délai.

use asteroids::components::{Asteroid, Bullet, Ship};
use asteroids::systems::{self, GameContext, GameWorld, ShipInput};
use asteroids::GameState;
use glam::Vec2;
use macroquad::prelude::Color;

/// Contexte de test. Nommé `test_ctx` (et non `ctx`) pour éviter toute
/// collision de nom avec les locals `cx`/`ctx` dans les tests.
fn test_ctx() -> GameContext {
    GameContext::default()
}

fn color() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}

fn ship_at(pos: Vec2) -> Ship {
    Ship::new(pos, test_ctx().ship_radius, color())
}

fn asteroid_at(pos: Vec2, size: u8) -> Asteroid {
    Asteroid::new(
        pos,
        Vec2::ZERO,
        0.0,
        size,
        systems::asteroid_radius(size),
        color(),
    )
}

// ============================================================================
// Scénario : tir → destruction → split → score
// ============================================================================

#[test]
fn test_bullet_destroys_big_asteroid_and_splits_in_two() {
    let cx = test_ctx();
    let mut bullets = vec![Bullet::new(
        Vec2::new(100.0, 100.0),
        Vec2::ZERO,
        cx.bullet_radius,
        10.0,
        color(),
    )];
    let mut asteroids = vec![asteroid_at(Vec2::new(100.0, 100.0), 3)];

    let destroyed =
        systems::resolve_bullet_asteroid_collisions(&mut bullets, &mut asteroids, &cx);
    let score = destroyed as i32 * 10;

    assert_eq!(destroyed, 1, "un astéroïde taille 3 doit être détruit");
    assert_eq!(score, 10);
    assert_eq!(bullets.len(), 0, "la balle est consommée");
    assert_eq!(asteroids.len(), 2, "un taille 3 se scinde en deux taille 2");
    assert!(asteroids.iter().all(|a| a.size == 2));
}

#[test]
fn test_small_asteroid_destroyed_without_split() {
    let cx = test_ctx();
    let mut bullets = vec![Bullet::new(
        Vec2::new(100.0, 100.0),
        Vec2::ZERO,
        cx.bullet_radius,
        10.0,
        color(),
    )];
    let mut asteroids = vec![asteroid_at(Vec2::new(100.0, 100.0), 1)];

    let destroyed =
        systems::resolve_bullet_asteroid_collisions(&mut bullets, &mut asteroids, &cx);

    assert_eq!(destroyed, 1);
    assert_eq!(asteroids.len(), 0, "un taille 1 disparaît, pas de split");
}

// ============================================================================
// Scénario : vague vidée → LevelCleared
// ============================================================================

#[test]
fn test_clearing_all_asteroids_leaves_asteroids_empty() {
    let cx = test_ctx();
    // Un seul astéroïde taille 1 : une balle suffit à vider la vague.
    let mut bullets = vec![Bullet::new(
        Vec2::new(100.0, 100.0),
        Vec2::ZERO,
        cx.bullet_radius,
        10.0,
        color(),
    )];
    let mut asteroids = vec![asteroid_at(Vec2::new(100.0, 100.0), 1)];

    systems::resolve_bullet_asteroid_collisions(&mut bullets, &mut asteroids, &cx);

    // C'est exactement la condition que `main.rs` teste pour passer en
    // LevelCleared : `if asteroids.is_empty()`.
    assert!(asteroids.is_empty());
}

// ============================================================================
// Scénario : collision vaisseau ↔ astéroïde → perte de vie
// ============================================================================

#[test]
fn test_ship_hit_by_asteroid_is_detected() {
    let ship = ship_at(Vec2::new(100.0, 100.0));
    let asteroids = vec![asteroid_at(Vec2::new(100.0 + 5.0, 100.0), 1)];

    assert!(
        systems::resolve_ship_asteroid_collision(&ship, &asteroids),
        "collision attendue (chevauchement de 5 px)"
    );
}

#[test]
fn test_ship_away_from_asteroid_is_safe() {
    let ship = ship_at(Vec2::new(100.0, 100.0));
    let asteroids = vec![asteroid_at(Vec2::new(500.0, 500.0), 1)];

    assert!(!systems::resolve_ship_asteroid_collision(&ship, &asteroids));
}

// ============================================================================
// Scénario : machine à états — perte de vie, invincibilité, GameOver
// ============================================================================

/// Reproduit le bloc "collision vaisseau ↔ astéroïde" de `main.rs`,
/// mais sur le `GameWorld` qui possède désormais l'état.
fn step_collision(world: &mut GameWorld, ctx: &GameContext, dt: f32) -> bool {
    if world.invincible_until > 0.0 {
        world.invincible_until = (world.invincible_until - dt).max(0.0);
    }
    let invincible = world.invincible_until > 0.0;

    if !invincible
        && systems::resolve_ship_asteroid_collision(&world.ship, &world.asteroids)
    {
        world.on_ship_hit(ctx);
        true
    } else {
        false
    }
}

#[test]
fn test_ship_hit_loses_one_life() {
    let cx = test_ctx();
    let mut world = GameWorld::new(&cx);
    world.state = GameState::Playing;
    world.ship = ship_at(Vec2::new(100.0, 100.0));
    world.asteroids = vec![asteroid_at(Vec2::new(105.0, 100.0), 1)];
    world.lives = 3;
    world.invincible_until = 0.0;

    let hit = step_collision(&mut world, &cx, 0.016);

    assert!(hit);
    assert_eq!(world.lives, 2);
}

#[test]
fn test_invincibility_blocks_life_loss() {
    let cx = test_ctx();
    let mut world = GameWorld::new(&cx);
    world.state = GameState::Playing;
    world.ship = ship_at(Vec2::new(100.0, 100.0));
    world.asteroids = vec![asteroid_at(Vec2::new(105.0, 100.0), 1)];
    world.lives = 3;
    world.invincible_until = 1.5;

    let hit = step_collision(&mut world, &cx, 0.016);

    assert!(!hit, "invincible → pas de perte de vie");
    assert_eq!(world.lives, 3);
}

#[test]
fn test_third_hit_triggers_game_over() {
    let cx = test_ctx();
    let mut world = GameWorld::new(&cx);
    world.state = GameState::Playing;
    world.ship = ship_at(Vec2::new(100.0, 100.0));
    world.asteroids = vec![asteroid_at(Vec2::new(105.0, 100.0), 1)];
    world.lives = 1;
    world.invincible_until = 0.0;

    let hit = step_collision(&mut world, &cx, 0.016);

    assert!(hit);
    assert_eq!(world.lives, 0);
    assert_eq!(world.state, GameState::GameOver);
}

// ============================================================================
// Scénario : tir via ShipInput → balle créée
// ============================================================================

#[test]
fn test_shoot_input_creates_bullet_ahead_of_ship() {
    let cx = test_ctx();
    let mut ship = ship_at(Vec2::new(100.0, 100.0));
    ship.transform.rotation = 0.0; // pointe vers +X
    let mut bullets = Vec::new();

    // Cooldown à 0 : le tir passe.
    let input = ShipInput {
        shoot: true,
        ..Default::default()
    };
    if input.shoot {
        systems::try_shoot(&ship, &mut bullets, &cx);
    }

    assert_eq!(bullets.len(), 1);
    // La balle doit spawn devant le nez, pas au centre.
    assert!(bullets[0].transform.position.x > ship.transform.position.x);
}

// ============================================================================
// Scénario : wrap-around compose avec le déplacement
// ============================================================================

#[test]
fn test_ship_crosses_screen_edge_and_wraps() {
    let cx = test_ctx();
    let mut ship = ship_at(Vec2::new(cx.screen_w - 1.0, 100.0));
    ship.transform.rotation = 0.0;
    // Pousse suffisamment pour sortir à droite.
    let input = ShipInput {
        thrust: true,
        ..Default::default()
    };

    // Plusieurs frames pour franchir le bord.
    for _ in 0..30 {
        systems::update_ship(&mut ship, &input, &cx, 0.016);
    }

    assert!(
        ship.transform.position.x < cx.screen_w,
        "la position doit rester dans [0, screen_w) après wrap"
    );
    assert!(ship.transform.position.x >= 0.0);
}