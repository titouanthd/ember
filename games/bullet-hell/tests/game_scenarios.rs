//! End-to-end scenarios for Bullet Hell.
//!
//! These run the full `World` through `update()` with scripted input, and
//! assert on the resulting state. They complement the unit tests inside
//! `systems.rs` by exercising cross-system interactions (fire → hit →
//! score → multiplier → wave advance) rather than single functions.

use bullet_hell::GameState;
use bullet_hell::config::load_config;
use bullet_hell::systems::{ShipInput, World, reset, update};
use glam::Vec2;

/// Build a world in `Playing` state with wave 0 spawned.
///
/// Invariant: the returned world has non-empty `enemies`, empty bullets,
/// `lives == 3`, `multiplier == 1.0`, `score == 0`, `state == Playing`.
/// Callers that need a different starting shape must mutate explicitly.
fn play_world() -> (World, bullet_hell::config::GameContext) {
    let ctx = load_config();
    let mut world = World::new(&ctx);
    world.start_run(&ctx);
    (world, ctx)
}

#[test]
fn test_world_starts_in_start_state() {
    let ctx = load_config();
    let world = World::new(&ctx);
    assert_eq!(world.state, GameState::Start);
    assert!(world.enemies.is_empty());
    assert!(world.player_bullets.is_empty());
    assert_eq!(world.lives, 3);
    assert_eq!(world.multiplier, 1.0);
    assert_eq!(world.score, 0);
}

#[test]
fn test_start_run_spawns_wave_1() {
    let (world, _ctx) = play_world();
    assert_eq!(world.state, GameState::Playing);
    assert!(!world.enemies.is_empty(), "wave 1 must have enemies");
    assert_eq!(world.wave, 0);
}

#[test]
fn test_fire_input_creates_player_bullet() {
    let (mut world, ctx) = play_world();
    let input = ShipInput {
        fire: true,
        ..Default::default()
    };
    update(&mut world, input, &ctx, 0.016);
    assert!(!world.player_bullets.is_empty());
}

#[test]
fn test_bullet_hits_enemy_and_damages() {
    let (mut world, ctx) = play_world();
    // Move player bullet to enemy position next frame.
    let e_pos = world.enemies[0].center;
    let hp_before = world.enemies[0].hp;
    world
        .player_bullets
        .push(bullet_hell::components::Bullet::player(
            e_pos,
            Vec2::ZERO,
            &ctx,
        ));
    update(&mut world, ShipInput::default(), &ctx, 0.016);
    let hp_after = world.enemies.first().map(|e| e.hp).unwrap_or(0);
    assert!(hp_after < hp_before);
}

#[test]
fn test_killing_last_enemy_transitions_to_level_cleared() {
    let (mut world, ctx) = play_world();
    world.enemies.clear();
    world.wave_elapsed = 2.0;
    update(&mut world, ShipInput::default(), &ctx, 0.016);
    assert_eq!(world.state, GameState::LevelCleared);
}

#[test]
fn test_level_cleared_advances_to_next_wave() {
    let (mut world, ctx) = play_world();
    world.state = GameState::LevelCleared;
    world.level_cleared_until = world.now - 1.0;
    update(&mut world, ShipInput::default(), &ctx, 0.016);
    assert_eq!(world.state, GameState::Playing);
    assert_eq!(world.wave, 1);
    assert!(!world.enemies.is_empty());
}

#[test]
fn test_player_hit_costs_life_and_grants_invincibility() {
    let (mut world, ctx) = play_world();
    let pp = world.player.transform.position;
    world
        .enemy_bullets
        .push(bullet_hell::components::Bullet::enemy(pp, Vec2::ZERO, &ctx));
    update(&mut world, ShipInput::default(), &ctx, 0.016);
    assert_eq!(world.lives, 2);
    assert!(world.player.is_invincible(world.now));
    assert_eq!(world.multiplier, 1.0, "death resets multiplier");
}

#[test]
fn test_zero_lives_triggers_game_over() {
    let (mut world, ctx) = play_world();
    world.lives = 1;
    let pp = world.player.transform.position;
    world
        .enemy_bullets
        .push(bullet_hell::components::Bullet::enemy(pp, Vec2::ZERO, &ctx));
    update(&mut world, ShipInput::default(), &ctx, 0.016);
    assert_eq!(world.lives, 0);
    assert_eq!(world.state, GameState::GameOver);
    assert!(!world.player.alive);
}

#[test]
fn test_graze_builds_multiplier_over_multiple_frames() {
    let (mut world, ctx) = play_world();
    // Park a bullet just outside the hitbox but inside graze.
    let pp = world.player.transform.position;
    // Just outside the hitbox (hitbox + bullet_radius), well inside the
    // graze ring (hitbox + graze_radius + bullet_radius).
    let offset = Vec2::new(
        ctx.player_hitbox_radius + ctx.bullet_enemy_radius + 2.0,
        0.0,
    );
    // Several frames of the bullet sitting there — but bullets move. Use
    // a stationary bullet (zero velocity) so it grazes every frame.
    for _ in 0..5 {
        world
            .enemy_bullets
            .push(bullet_hell::components::Bullet::enemy(
                pp + offset,
                Vec2::ZERO,
                &ctx,
            ));
        update(&mut world, ShipInput::default(), &ctx, 0.016);
    }
    assert!(world.graze_count >= 1);
    assert!(world.multiplier > 1.0);
}

#[test]
fn test_reset_clears_everything() {
    let (mut world, ctx) = play_world();
    world.score = 1234;
    world.graze_count = 42;
    world.multiplier = 5.0;
    reset(&mut world, &ctx);
    assert_eq!(world.state, GameState::Start);
    assert_eq!(world.score, 0);
    assert_eq!(world.graze_count, 0);
    assert_eq!(world.multiplier, 1.0);
    assert!(world.enemies.is_empty());
    assert!(world.player_bullets.is_empty());
    assert!(world.enemy_bullets.is_empty());
    assert!(world.particles.is_empty());
}

#[test]
fn test_player_cannot_leave_playfield_via_full_loop() {
    let (mut world, ctx) = play_world();
    let input = ShipInput {
        dx: -1.0,
        dy: 0.0,
        focus: false,
        fire: false,
    };
    for _ in 0..500 {
        update(&mut world, input, &ctx, 0.016);
    }
    assert!(world.player.transform.position.x >= ctx.player_radius - 0.01);
    assert!(world.player.transform.position.y >= ctx.player_radius - 0.01);
}
