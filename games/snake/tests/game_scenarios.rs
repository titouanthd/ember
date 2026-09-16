// games/snake/tests/game_scenarios.rs
//! Tests d'intégration : scénarios de jeu complets.
//!
//! Vérifie que les systèmes interagissent correctement dans des
//! situations réalistes (manger, mourir, finir un niveau).
//!
//! **Note** : les tests qui appellent `systems::tick(...)` directement
//! n'ont pas changé — `tick` reste une primitive sur les pièces
//! individuelles. Seuls les tests qui exercent `update` construisent
//! désormais un `SnakeWorld`.

use macroquad::prelude::Color;
use snake::GameState;
use snake::components::{Cell, Direction, Food, Snake, Wall};
use snake::systems::{self, GameContext, SnakeState, SnakeWorld, TickEvent};

/// Crée un contexte de test sur une petite grille.
fn test_ctx() -> GameContext {
    GameContext {
        grid_cols: 10,
        grid_rows: 10,
        cell_size: 10.0,
        initial_tick_ms: 100,
        min_tick_ms: 50,
        tick_speedup_ms: 5,
        food_per_speedup: 2,
        ..Default::default()
    }
}

/// Crée un serpent de longueur 1 (tête seule) à une position donnée.
fn snake_at(cx: i32, cy: i32, dir: Direction) -> Snake {
    Snake::new(
        Cell::new(cx, cy),
        1,
        dir,
        Color::new(0.0, 1.0, 0.0, 1.0),
        Color::new(0.0, 0.5, 0.0, 1.0),
    )
}

fn food_at(cx: i32, cy: i32) -> Food {
    Food::new(Cell::new(cx, cy), Color::new(1.0, 0.0, 0.0, 1.0))
}

/// Construit un `SnakeWorld` en `Playing`, prêt pour `update`.
///
/// `tick_ms` règle `ctx.initial_tick_ms`, ce qui garantit que
/// `SnakeWorld::new` (qui lit `ctx.initial_tick_ms`) et `update`
/// (qui lit `ctx.current_tick_secs`, donc `ctx.initial_tick_ms`) sont
/// d'accord sur la durée du tick.
///
/// **Important** : `update` réécrit le timer à chaque frame via
/// `ctx.current_tick_secs(...)`. Si on ne synchronise pas `tick_ms` avec
/// `ctx.initial_tick_ms`, le test passe un timer à 10ms mais `update`
/// le remet à 100ms et rien ne se déclenche.
fn playing_world(
    snake: Snake,
    food: Food,
    walls: Vec<Wall>,
    ctx: &mut GameContext,
    tick_ms: u32,
    target_score: u32,
) -> SnakeWorld {
    ctx.initial_tick_ms = tick_ms;
    let mut world = SnakeWorld::new(snake, food, walls, ctx);
    world.state = GameState::Playing;
    world.snake_state.target_score = target_score;
    world
}

// ============================================================================
// Scénarios : manger (primitives via `tick`)
// ============================================================================

#[test]
fn test_eating_food_increases_score_and_length() {
    let ctx = test_ctx();
    let mut snake = snake_at(5, 5, Direction::Right);
    let mut food = food_at(6, 5);
    let mut score = 0;
    let mut eaten = 0;

    let ev = systems::tick(&mut snake, &mut food, &[], &ctx, &mut score, &mut eaten);

    assert_eq!(ev, TickEvent::Ate);
    assert_eq!(score, 1);
    assert_eq!(eaten, 1);
    assert_eq!(snake.len(), 2, "le serpent doit avoir grandi");
    assert_eq!(snake.head(), Cell::new(6, 5));
}

#[test]
fn test_eating_food_respawns_it_elsewhere() {
    let ctx = test_ctx();
    let mut snake = snake_at(5, 5, Direction::Right);
    let mut food = food_at(6, 5);
    let old_pos = food.cell;
    let mut score = 0;
    let mut eaten = 0;

    systems::tick(&mut snake, &mut food, &[], &ctx, &mut score, &mut eaten);

    assert_ne!(
        food.cell, old_pos,
        "la nourriture doit être déplacée après avoir été mangée"
    );
    assert!(!snake.body.contains(&food.cell));
}

// ============================================================================
// Scénarios : mourir (primitives via `tick`)
// ============================================================================

#[test]
fn test_hitting_wall_kills_snake() {
    let ctx = test_ctx();
    let mut snake = snake_at(9, 5, Direction::Right);
    let mut food = food_at(0, 0);
    let mut score = 0;
    let mut eaten = 0;

    let ev = systems::tick(&mut snake, &mut food, &[], &ctx, &mut score, &mut eaten);

    assert_eq!(ev, TickEvent::Died);
}

#[test]
fn test_hitting_internal_wall_kills_snake() {
    let ctx = test_ctx();
    let mut snake = snake_at(5, 5, Direction::Right);
    let mut food = food_at(0, 0);
    let wall_color = Color::new(0.5, 0.5, 0.5, 1.0);
    let walls = vec![Wall::new(Cell::new(6, 5), wall_color)];
    let mut score = 0;
    let mut eaten = 0;

    let ev = systems::tick(&mut snake, &mut food, &walls, &ctx, &mut score, &mut eaten);

    assert_eq!(ev, TickEvent::Died);
}

#[test]
fn test_self_collision_kills_snake() {
    let ctx = test_ctx();
    let mut snake = Snake::new(
        Cell::new(5, 5),
        1,
        Direction::Left,
        Color::new(0.0, 1.0, 0.0, 1.0),
        Color::new(0.0, 0.5, 0.0, 1.0),
    );
    snake.body.clear();
    snake.body.push_back(Cell::new(5, 5));
    snake.body.push_back(Cell::new(4, 5));
    snake.body.push_back(Cell::new(4, 4));
    snake.body.push_back(Cell::new(5, 4));
    let mut food = food_at(9, 9);
    let mut score = 0;
    let mut eaten = 0;

    let ev = systems::tick(&mut snake, &mut food, &[], &ctx, &mut score, &mut eaten);

    assert_eq!(ev, TickEvent::Died);
}

// ============================================================================
// Scénarios : machine à états (via `SnakeWorld` + `update`)
// ============================================================================

#[test]
fn test_death_sets_game_over_state() {
    let mut ctx = test_ctx();
    // Serpent au bord droit, va à droite → meurt au premier tick.
    let mut world = playing_world(
        snake_at(9, 5, Direction::Right),
        food_at(0, 0),
        Vec::new(),
        &mut ctx,
        10, // tick très rapide
        10,
    );

    let ev = systems::update(&mut world, &ctx, 0.05);

    assert_eq!(ev, TickEvent::Died);
    assert_eq!(world.state, GameState::GameOver);
}

#[test]
fn test_reaching_target_score_clears_level() {
    let mut ctx = test_ctx();
    // Serpent juste devant la nourriture, objectif = 1.
    let mut world = playing_world(
        snake_at(5, 5, Direction::Right),
        food_at(6, 5),
        Vec::new(),
        &mut ctx,
        10,
        1,
    );

    let ev = systems::update(&mut world, &ctx, 0.05);

    assert_eq!(ev, TickEvent::Ate);
    assert_eq!(world.state, GameState::LevelCleared);
}

#[test]
fn test_no_tick_when_not_playing() {
    let ctx = test_ctx();
    let mut world = SnakeWorld::new(
        snake_at(5, 5, Direction::Right),
        food_at(6, 5),
        Vec::new(),
        &ctx,
    );
    world.state = GameState::GameOver;
    world.snake_state = SnakeState::new(0.01);

    let ev = systems::update(&mut world, &ctx, 0.05);

    assert_eq!(ev, TickEvent::None);
    assert_eq!(world.snake.head(), Cell::new(5, 5));
}

// ============================================================================
// Scénarios : pas fixe (TickTimer)
// ============================================================================

#[test]
fn test_timer_triggers_tick_after_accumulation() {
    let mut ctx = test_ctx();
    // 100 ms par tick : un appel à 50 ms ne déclenche rien, deux appels oui.
    let mut world = playing_world(
        snake_at(5, 5, Direction::Right),
        food_at(0, 0),
        Vec::new(),
        &mut ctx,
        100,
        10,
    );

    systems::update(&mut world, &ctx, 0.05);
    assert_eq!(world.snake.head(), Cell::new(5, 5), "pas encore bougé");

    systems::update(&mut world, &ctx, 0.05);
    assert_eq!(world.snake.head(), Cell::new(6, 5), "un tick déclenché");
}

#[test]
fn test_multiple_ticks_in_one_frame() {
    let mut ctx = test_ctx();
    let mut world = playing_world(
        snake_at(5, 5, Direction::Right),
        food_at(0, 0),
        Vec::new(),
        &mut ctx,
        100,
        10,
    );

    // dt = 0.25 → 2 ticks consommés.
    systems::update(&mut world, &ctx, 0.25);

    assert_eq!(
        world.snake.head(),
        Cell::new(7, 5),
        "deux ticks = deux cases"
    );
}

// ============================================================================
// Scénarios : niveaux
// ============================================================================

#[test]
fn test_reset_round_places_snake_at_center() {
    let ctx = test_ctx();
    let mut world = SnakeWorld::new(
        snake_at(0, 0, Direction::Up),
        food_at(5, 5),
        Vec::new(),
        &ctx,
    );
    world.snake_state.food_eaten = 3;

    world.reset_round(&ctx);

    assert_eq!(
        world.snake.head(),
        Cell::new(ctx.grid_cols as i32 / 2, ctx.grid_rows as i32 / 2)
    );
    assert_eq!(world.snake_state.food_eaten, 0, "compteur remis à zéro");
}

#[test]
fn test_spawn_food_never_on_snake_or_wall() {
    let ctx = test_ctx();
    let mut snake = snake_at(5, 5, Direction::Right);
    for _ in 0..4 {
        snake.advance(true);
    }
    let walls = vec![
        Wall::new(Cell::new(0, 0), Color::new(0.5, 0.5, 0.5, 1.0)),
        Wall::new(Cell::new(1, 1), Color::new(0.5, 0.5, 0.5, 1.0)),
    ];

    for _ in 0..50 {
        let f = systems::spawn_food_avoiding(&snake, &walls, &ctx);
        assert!(
            !snake.body.contains(&f.cell),
            "nourriture sur serpent : {:?}",
            f.cell
        );
        assert!(
            !walls.iter().any(|w| w.cell == f.cell),
            "nourriture sur mur : {:?}",
            f.cell
        );
        assert!(
            ctx.is_inside(f.cell),
            "nourriture hors grille : {:?}",
            f.cell
        );
    }
}
