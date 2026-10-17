//! Integration tests for cross-module invariants.
//!
//! These avoid calling `AppState::new().await` (which would try to
//! load assets that may not exist on CI). They validate the pure
//! pieces: paths, progress roundtrip, tile kind contract, palette.
//!
//! Once `systems::AppState::tick` becomes testable headless (no
//! `screen_width()` calls in the tick path), screens will move in
//! here. For now, structural smoke tests only.

use bore_inc::components::{ResourceKind, Tile, TileKind, TILE_SIZE};
use bore_inc::config::{Colors, GameContext};
use bore_inc::paths;
use bore_inc::progress::Progress;

// ─────────────────────────────────────────────────────────────
// 1. Paths resolve
// ─────────────────────────────────────────────────────────────

#[test]
fn asset_path_joins_under_assets_dir() {
    let p = paths::asset("DejaVuSansMono.ttf");
    assert!(p.ends_with("DejaVuSansMono.ttf"));
    assert!(p.parent().unwrap().ends_with("assets"));
}

#[test]
fn asset_str_returns_a_usable_string() {
    let s = paths::asset_str("DejaVuSansMono.ttf");
    assert!(s.ends_with("DejaVuSansMono.ttf"));
}

// ─────────────────────────────────────────────────────────────
// 2. Palette invariants
// ─────────────────────────────────────────────────────────────

#[test]
fn all_default_colors_are_in_unit_range() {
    let c = Colors::defaults();
    for col in [
        c.bg_top,
        c.bg_bottom,
        c.panel_bg,
        c.panel_border,
        c.text,
        c.text_dim,
        c.gold,
        c.crystal,
        c.danger,
    ] {
        for v in [col.r, col.g, col.b, col.a] {
            assert!((0.0..=1.0).contains(&v), "{col:?} has {v}");
        }
    }
}

#[test]
fn default_context_matches_window_size() {
    let ctx = GameContext::defaults();
    assert_eq!(ctx.viewport_w, 1280.0);
    assert_eq!(ctx.viewport_h, 720.0);
}

// ─────────────────────────────────────────────────────────────
// 3. Tile kinds — DESIGN §9 contract
// ─────────────────────────────────────────────────────────────

#[test]
fn tile_kinds_match_design_hp_table() {
    assert_eq!(TileKind::Dirt.hp(), 1.0);
    assert_eq!(TileKind::Stone.hp(), 2.0);
    assert_eq!(TileKind::Coal.hp(), 3.0);
    assert_eq!(TileKind::Iron.hp(), 4.0);
    assert_eq!(TileKind::Crystal.hp(), 6.0);
    assert_eq!(TileKind::HardRock.hp(), 20.0);
    assert_eq!(TileKind::Capsule.hp(), 5.0);
    assert!(TileKind::Bedrock.hp().is_infinite());
}

#[test]
fn bedrock_never_breaks() {
    let mut t = Tile::new(TileKind::Bedrock);
    t.damage(1_000_000.0);
    assert!(!t.is_broken());
}

#[test]
fn crystal_yields_crystal_resource() {
    let mut t = Tile::new(TileKind::Crystal);
    assert_eq!(t.kind.drop(), Some(ResourceKind::Crystal));
    t.damage(6.0);
    assert!(t.is_broken());
}

// ─────────────────────────────────────────────────────────────
// 4. Progress roundtrip
// ─────────────────────────────────────────────────────────────

#[test]
fn progress_roundtrip_preserves_everything() {
    let mut p = Progress::default();
    p.add_dollars(500);
    p.buy_upgrade("pickaxe");
    p.buy_upgrade("pickaxe");
    p.buy_upgrade("bag");
    p.unlock_robot("digger");
    p.mark_level_completed("level-1", 42);
    p.map_truck_pos = (300.0, 120.0);

    let text = ron::ser::to_string(&p).expect("serialize");
    let back: Progress = ron::from_str(&text).expect("deserialize");

    assert_eq!(back.dollars, 500);
    assert_eq!(back.upgrade_level("pickaxe"), 2);
    assert_eq!(back.upgrade_level("bag"), 1);
    assert!(back.has_robot("digger"));
    assert_eq!(back.map_truck_pos, (300.0, 120.0));
    let lvl = back.level_save("level-1").expect("saved");
    assert_eq!(lvl.seed, 42);
    assert!(lvl.completed);
}

#[test]
fn progress_accepts_forward_compatible_ron() {
    // Only `dollars` — all other fields must have serde defaults.
    let p: Progress = ron::from_str("(dollars: 100)").expect("minimal ron");
    assert_eq!(p.dollars, 100);
    assert!(p.upgrades.is_empty());
    assert!(p.unlocked_robots.is_empty());
}

// ─────────────────────────────────────────────────────────────
// 5. Constants
// ─────────────────────────────────────────────────────────────

#[test]
fn tile_size_is_a_positive_pixel_value() {
    // Tile size must be a small positive integer in pixels. Pixel-
    // perfect rendering (DESIGN §19) will snap tile positions to
    // integers, so an integral value avoids drift.
    let size: f32 = TILE_SIZE;
    assert!(size > 0.0, "tile size must be positive, got {size}");
    assert!((size as i32) as f32 == size, "tile size must be integral");
}

#[test]
fn resource_values_match_balance_table() {
    assert_eq!(ResourceKind::Dirt.value(), 1);
    assert_eq!(ResourceKind::Stone.value(), 2);
    assert_eq!(ResourceKind::Coal.value(), 4);
    assert_eq!(ResourceKind::Iron.value(), 8);
    assert_eq!(ResourceKind::Rock.value(), 12);
    assert_eq!(ResourceKind::Crystal.value(), 25);
}

// ─────────────────────────────────────────────────────────────
// 6. Mining — Session 2
// ─────────────────────────────────────────────────────────────

use bore_inc::input::{Facing, GameInput};
use bore_inc::systems::Game;
use bore_inc::world::World;

/// Build a small world from a closure `(col, row) -> Option<TileKind>`.
/// Row 0 is the top.
fn build_world<F>(width: usize, height: usize, f: F) -> World
where
    F: Fn(usize, usize) -> Option<TileKind>,
{
    use bore_inc::components::Tile;
    let mut cells = Vec::with_capacity(width * height);
    for row in 0..height {
        for col in 0..width {
            cells.push(f(col, row).map(Tile::new));
        }
    }
    World::from_cells(width, height, cells, 0, 0)
}

/// Place Archie so his body is entirely inside tile row 1.
///
/// Row 1 spans pixels y ∈ [16, 32). Archie's box is 12 wide × 14
/// tall. At y = 18 his top is 2 px into row 1 and his bottom is 2 px
/// above the row 1 / row 2 boundary at y = 32 — so his bounding box
/// occupies exactly tile row 1 and nothing else.
fn place_archie_at_row1(game: &mut Game, x: f32) {
    game.archie.pos.x = x;
    game.archie.pos.y = 18.0;
}

#[test]
fn archie_moves_freely_through_air() {
    // All air. He should move right at exactly `move_speed * t`.
    let world = build_world(16, 3, |_, _| None);
    let mut game = Game::with_world(world);
    place_archie_at_row1(&mut game, 0.0);

    let input = GameInput { horizontal: 1.0, ..Default::default() };
    let dt = 1.0 / 60.0;
    for _ in 0..30 {
        game.tick(dt, input);
    }
    // 30 ticks × (1/60) s × 100 px/s = 50 px.
    assert!(
        (48.0..=52.0).contains(&game.archie.pos.x),
        "expected ~50 px, got {}",
        game.archie.pos.x,
    );
}

#[test]
fn archie_does_not_move_without_input() {
    let world = build_world(8, 3, |_, _| None);
    let mut game = Game::with_world(world);
    place_archie_at_row1(&mut game, 40.0);

    let input = GameInput::default();
    for _ in 0..60 {
        game.tick(1.0 / 60.0, input);
    }
    assert_eq!(game.archie.pos.x, 40.0);
}

#[test]
fn archie_is_blocked_by_bedrock() {
    // A vertical wall of bedrock at col 4. Everything else is air.
    let world = build_world(8, 4, |col, _row| {
        if col == 4 { Some(TileKind::Bedrock) } else { None }
    });
    let mut game = Game::with_world(world);
    place_archie_at_row1(&mut game, 0.0);

    // No mining (A not held): Archie just bumps the wall.
    let input = GameInput { horizontal: 1.0, ..Default::default() };
    let dt = 1.0 / 60.0;

    for _ in 0..300 {
        game.tick(dt, input);
    }

    // Bedrock intact.
    assert!(game.world.tile(4, 1).is_some());
    assert!(game.world.tile(4, 1).unwrap().kind.is_indestructible());

    // Archie stopped just before the wall.
    let right_edge = game.archie.pos.x + 12.0;
    assert!(
        right_edge <= 64.0 + 0.01,
        "Archie's right edge ({right_edge}) should not pass into the wall",
    );
    assert!(
        right_edge >= 62.0,
        "Archie should be right up against the wall, right edge = {right_edge}",
    );
}

#[test]
fn archie_mines_a_stone_tile() {
    // Gravity is active — Archie needs solid ground under his feet
    // while mining. Stone target at (2, 1), stone floor at row 2.
    let mut cells: Vec<Option<Tile>> = vec![None; 8 * 4];
    for col in 0..8 {
        cells[2 * 8 + col] = Some(Tile::new(TileKind::Stone)); // floor
    }
    cells[8 + 2] = Some(Tile::new(TileKind::Stone)); // the target
    let world = World::from_cells(8, 4, cells, 0, 0);
    let mut game = Game::with_world(world);

    // Right edge at pixel 32 (col 2 starts at x = 32) → pos.x = 20.
    // Feet at row 2 top (y = 32) → pos.y = 32 - 14 = 18.
    game.archie.pos = glam::Vec2::new(20.0, 18.0);
    game.archie.on_ground = true;

    assert_eq!(game.world.tile(2, 1).unwrap().hp, 2.0);

    // Hold A, facing right.
    let input = GameInput {
        horizontal: 1.0,
        facing_held: Some(Facing::Right),
        mine_held: true,
        ..Default::default()
    };
    let dt = 1.0 / 60.0;

    // 30 ticks = 0.5 s of contact = 0.5 HP of damage. Stone at 1.5 HP.
    for _ in 0..30 {
        game.tick(dt, input);
    }
    let hp = game.world.tile(2, 1).map(|t| t.hp).unwrap_or(0.0);
    assert!(
        (1.4..=1.6).contains(&hp),
        "expected ~1.5 HP after 0.5 s, got {hp}",
    );

    // 120 more ticks → 2 s more contact → stone breaks.
    for _ in 0..120 {
        game.tick(dt, input);
    }
    assert!(
        game.world.tile(2, 1).is_none(),
        "stone should have been mined",
    );
}

#[test]
fn tile_breaks_after_exact_damage_budget() {
    // Dirt target at (3, 1), stone floor at row 2 to keep Archie
    // stationary in row 1 while he mines.
    let mut cells: Vec<Option<Tile>> = vec![None; 6 * 4];
    for col in 0..6 {
        cells[2 * 6 + col] = Some(Tile::new(TileKind::Stone));
    }
    cells[6 + 3] = Some(Tile::new(TileKind::Dirt));
    let world = World::from_cells(6, 4, cells, 0, 0);
    let mut game = Game::with_world(world);

    // Right edge at pixel 48 (col 3 starts at x = 48) → pos.x = 36.
    // Feet at y = 32 → pos.y = 18.
    game.archie.pos = glam::Vec2::new(36.0, 18.0);
    game.archie.on_ground = true;

    let input = GameInput {
        horizontal: 1.0,
        facing_held: Some(Facing::Right),
        mine_held: true,
        ..Default::default()
    };
    let dt = 1.0 / 60.0;

    // 59 ticks = 59/60 s → hp = 1/60, still present.
    for _ in 0..59 {
        game.tick(dt, input);
    }
    assert!(
        game.world.tile(3, 1).is_some(),
        "dirt should survive at 59/60 s",
    );

    // 2 more ticks cross 1 s → breaks.
    game.tick(dt, input);
    game.tick(dt, input);
    assert!(
        game.world.tile(3, 1).is_none(),
        "dirt should break after 1 s",
    );
}

#[test]
fn depositing_into_truck_empties_archies_bag() {
    use bore_inc::components::ResourceKind;
    use bore_inc::input::GameInput;
    use bore_inc::systems::Game;
    use bore_inc::world::World;

    let world = World::test_world();
    let mut game = Game::with_world(world);

    // Put Archie just to the right of the truck, on the ground.
    game.archie.pos.x = game.truck.pos.x + game.truck.size.x + 1.0;
    game.archie.pos.y = game.truck.pos.y + (game.truck.size.y - game.archie.size.y);
    game.archie.bag.clear();
    game.archie.collect(ResourceKind::Stone, 5);
    assert_eq!(game.archie.bag_used(), 5);

    // Push left into the truck.
    let input = GameInput { horizontal: -1.0, ..Default::default() };
    for _ in 0..120 {
        game.tick(1.0 / 60.0, input);
        if game.archie.bag_used() == 0 { break; }
    }

    assert_eq!(game.archie.bag_used(), 0, "bag should be empty after depositing");
    assert_eq!(game.truck.storage_used(), 5, "truck should hold the 5 stone");
}

// ─────────────────────────────────────────────────────────────
// 7. Tunnel delta persistence — slice 6
// ─────────────────────────────────────────────────────────────

#[test]
fn world_delta_replays_after_regeneration() {
    use bore_inc::world::World;

    // Mine a few tiles, keep the delta.
    let mut w1 = World::test_world();
    w1.set(5, 13, None);
    w1.set(6, 13, None);
    let delta = w1.delta.clone();
    assert_eq!(delta.len(), 2);

    // Regenerate the same world; the tiles are back.
    let mut w2 = World::test_world();
    assert!(w2.tile(5, 13).is_some());
    assert!(w2.tile(6, 13).is_some());

    // Apply the delta; the tunnel returns.
    w2.apply_delta(&delta);
    assert!(w2.tile(5, 13).is_none());
    assert!(w2.tile(6, 13).is_none());
}

#[test]
fn delta_accumulates_across_sessions() {
    use bore_inc::world::World;

    // Session 1: mine one tile.
    let mut w1 = World::test_world();
    w1.set(5, 13, None);
    let delta1 = w1.delta.clone();

    // Session 2: load the saved delta, mine another tile, save.
    let mut w2 = World::test_world();
    w2.apply_delta(&delta1);
    assert!(w2.tile(5, 13).is_none());
    assert!(w2.tile(10, 13).is_some());
    w2.set(10, 13, None);
    let delta2 = w2.delta.clone();

    // Session 3: both tunnels are present.
    let mut w3 = World::test_world();
    w3.apply_delta(&delta2);
    assert!(w3.tile(5, 13).is_none());
    assert!(w3.tile(10, 13).is_none());
}

#[test]
fn set_level_seed_clears_saved_delta() {
    use bore_inc::progress::Progress;

    let mut p = Progress::default();
    p.set_level_seed("level-one", 42);
    p.set_level_delta("level-one", vec![(5, 13, None)]);
    assert_eq!(p.level_delta("level-one").len(), 1);

    p.set_level_seed("level-one", 99);
    assert!(p.level_delta("level-one").is_empty());
}

#[test]
fn next_level_progresses_one_to_two_to_three() {
    use bore_inc::levelgen::LevelKind;
    assert_eq!(LevelKind::One.next(), LevelKind::Two);
    assert_eq!(LevelKind::Two.next(), LevelKind::Three);
    assert_eq!(LevelKind::Three.next(), LevelKind::Three);
}