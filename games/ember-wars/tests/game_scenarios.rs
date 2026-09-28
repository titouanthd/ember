//! Tests d'intégration — scénarios de jeu complets.

use ember_wars::components::Team;
use ember_wars::config::GameContext;
use ember_wars::progress::{stars_for_victory, Progress};
use ember_wars::systems::{Game, Phase, GROUND_Y_RATIO};
use ember_wars::units::spawn_unit;

use glam::Vec2;

const SIM_DT: f32 = 1.0 / 60.0;

fn ctx() -> GameContext {
    GameContext::from_env()
}

fn new_game(seed: u32) -> Game {
    Game::new(&ctx(), "sh_02", seed, &Progress::default()).expect("game creates")
}

fn new_game_survive(seed: u32) -> Game {
    Game::new(&ctx(), "sh_01", seed, &Progress::default()).expect("game creates")
}

fn sim_until<F: FnMut(&Game) -> bool>(g: &mut Game, max_seconds: f32, mut stop: F) -> bool {
    let mut t = 0.0;
    while t < max_seconds {
        g.tick(SIM_DT, Vec2::ZERO, false, 0.0);
        if stop(g) {
            return true;
        }
        t += SIM_DT;
    }
    false
}

fn disable_ai(g: &mut Game) {
    g.enemy_mana.current = 0.0;
    g.enemy_mana.regen = 0.0;
}

fn ground_y_for_test() -> f32 {
    ctx().viewport_h * GROUND_Y_RATIO
}

// ---------------------------------------------------------------------

#[test]
fn player_wins_by_destroying_enemy_tower() {
    let mut p = Progress::default();
    p.tree.grant("unit_hp_1");
    p.tree.grant("unlock_archer");
    p.tree.grant("unlock_bomber");
    p.tree.grant("unlock_hero");

    let mut g = Game::new(&ctx(), "sh_02", 1, &p).expect("creates");
    disable_ai(&mut g);
    g.enemy_tower.hp = 10.0;

    g.player_mana.current = 1000.0;
    g.player_mana.max = 1000.0;
    assert!(g.try_player_spawn("hero"));

    let won = sim_until(&mut g, 90.0, |g| g.phase == Phase::Won);
    assert!(won, "phase = {:?}", g.phase);
}

#[test]
fn player_wins_by_surviving() {
    let mut g = new_game_survive(1);
    disable_ai(&mut g);
    let won = sim_until(&mut g, 60.0, |g| g.phase == Phase::Won);
    assert!(won);
}

#[test]
fn player_loses_when_tower_destroyed() {
    let mut g = new_game(1);
    g.player_tower.hp = 30.0;
    g.enemy_mana.current = 1000.0;
    g.enemy_mana.max = 1000.0;
    g.enemy_mana.regen = 50.0;
    let lost = sim_until(&mut g, 180.0, |g| g.phase == Phase::Lost);
    assert!(lost);
}

#[test]
fn phase_stays_playing_while_both_towers_alive() {
    let mut g = new_game(1);
    disable_ai(&mut g);
    for _ in 0..(60 * 5) {
        g.tick(SIM_DT, Vec2::ZERO, false, 0.0);
    }
    assert_eq!(g.phase, Phase::Playing);
}

#[test]
fn turret_kill_credits_bank_gold() {
    let mut g = new_game(1);
    disable_ai(&mut g);
    let enemy_pos = Vec2::new(g.catapult.pos.x + 400.0, g.ground_y);
    let mut grunt = spawn_unit(&mut g.id_gen, "grunt", Team::Enemy, enemy_pos, 1.0, 1.0).unwrap();
    grunt.hp = 5.0;
    grunt.max_hp = 5.0;
    g.units.push(grunt);

    let gold_before = g.player_upgrades.gold;
    let mut t = 0.0;
    while t < 6.0 && g.units.iter().any(|u| u.team == Team::Enemy) {
        let target = g
            .units
            .iter()
            .find(|u| u.team == Team::Enemy)
            .map(|u| u.pos)
            .unwrap_or(enemy_pos);
        g.tick(SIM_DT, target, true, 0.0);
        t += SIM_DT;
    }
    assert!(g.player_upgrades.gold > gold_before);
}

#[test]
fn progress_tree_grants_unlock_at_start() {
    let mut p = Progress::default();
    p.tree.grant("unit_hp_1");
    p.tree.grant("unlock_archer");
    let mut g = Game::new(&ctx(), "sh_02", 1, &p).expect("creates");
    disable_ai(&mut g);
    g.player_mana.current = 100.0;
    g.player_mana.max = 100.0;
    assert!(g.try_player_spawn("archer"));
}

#[test]
fn bomber_explodes_and_damages_neighbors() {
    let mut g = new_game(1);
    disable_ai(&mut g);
    let gy = ground_y_for_test();
    let bomber = spawn_unit(&mut g.id_gen, "bomber", Team::Player, Vec2::new(200.0, gy), 1.0, 1.0).unwrap();
    let e1 = spawn_unit(&mut g.id_gen, "grunt", Team::Enemy, Vec2::new(215.0, gy), 1.0, 1.0).unwrap();
    let e2 = spawn_unit(&mut g.id_gen, "grunt", Team::Enemy, Vec2::new(240.0, gy), 1.0, 1.0).unwrap();
    let e3 = spawn_unit(&mut g.id_gen, "grunt", Team::Enemy, Vec2::new(500.0, gy), 1.0, 1.0).unwrap();
    g.units.extend([bomber, e1, e2, e3]);

    for _ in 0..18 {
        g.tick(1.0 / 60.0, Vec2::ZERO, false, 0.0);
    }

    assert!(!g.units.iter().any(|u| u.kind == "bomber"));
    assert_eq!(g.stats.kills, 2);
    assert!(g.units.iter().any(|u| u.team == Team::Enemy));
}

#[test]
fn levels_load_and_run() {
    for level_id in [
        "sh_01", "sh_02", "sh_03",
        "gy_01", "gy_02", "gy_03",
        "hk_01", "hk_02", "hk_03",
        "cd_01", "cd_02", "cd_03",
        "bj_01", "bj_02", "bj_03",
    ] {
        let p = Progress::default();
        let mut g = Game::new(&ctx(), level_id, 1, &p).expect("level exists");
        for _ in 0..30 {
            g.tick(SIM_DT, Vec2::ZERO, false, 0.0);
        }
        assert_eq!(g.phase, Phase::Playing, "level {level_id} crashed");
    }
}

#[test]
fn progress_unlocks_next_level_after_completion() {
    let mut p = Progress::default();
    assert!(!p.is_level_unlocked("sh_02"));
    p.record_win("sh_01", 1, 0.0);
    assert!(p.is_level_unlocked("sh_02"));
}

#[test]
fn chapter_2_unlocked_only_after_chapter_1_cleared() {
    let mut p = Progress::default();
    assert!(!p.is_chapter_unlocked(1));
    p.record_win("sh_01", 1, 0.0);
    p.record_win("sh_02", 1, 0.0);
    assert!(!p.is_chapter_unlocked(1));
    p.record_win("sh_03", 1, 0.0);
    assert!(p.is_chapter_unlocked(1));
}

#[test]
fn chapter_3_unlocked_only_after_chapter_2_cleared() {
    let mut p = Progress::default();
    assert!(!p.is_chapter_unlocked(2));

    p.record_win("sh_01", 1, 0.0);
    p.record_win("sh_02", 1, 0.0);
    p.record_win("sh_03", 1, 0.0);
    assert!(!p.is_chapter_unlocked(2));

    p.record_win("gy_01", 1, 0.0);
    p.record_win("gy_02", 1, 0.0);
    assert!(!p.is_chapter_unlocked(2));

    p.record_win("gy_03", 1, 0.0);
    assert!(p.is_chapter_unlocked(2));
}

#[test]
fn chapter_4_unlocked_only_after_chapter_3_cleared() {
    let mut p = Progress::default();
    assert!(!p.is_chapter_unlocked(3));

    for id in ["sh_01", "sh_02", "sh_03", "gy_01", "gy_02", "gy_03", "hk_01", "hk_02"] {
        p.record_win(id, 1, 0.0);
    }
    assert!(!p.is_chapter_unlocked(3));

    p.record_win("hk_03", 1, 0.0);
    assert!(p.is_chapter_unlocked(3));
}

#[test]
fn chapter_5_unlocked_only_after_chapter_4_cleared() {
    let mut p = Progress::default();
    assert!(!p.is_chapter_unlocked(4));

    for id in [
        "sh_01", "sh_02", "sh_03",
        "gy_01", "gy_02", "gy_03",
        "hk_01", "hk_02", "hk_03",
        "cd_01", "cd_02",
    ] {
        p.record_win(id, 1, 0.0);
    }
    assert!(!p.is_chapter_unlocked(4));

    p.record_win("cd_03", 1, 0.0);
    assert!(p.is_chapter_unlocked(4));
}

#[test]
fn stars_scale_with_tower_hp() {
    assert_eq!(stars_for_victory(1.0), 3);
    assert_eq!(stars_for_victory(0.75), 2);
    assert_eq!(stars_for_victory(0.25), 1);
}

#[test]
fn chapters_ron_loads() {
    use ember_wars::config::all_chapters;
    assert_eq!(all_chapters().len(), 5);
    for c in all_chapters() {
        assert_eq!(c.levels.len(), 3);
    }
}

#[test]
fn catapult_never_reaches_enemy_tower() {
    use ember_wars::catapult::CATAPULT_MAX_RANGE;
    use ember_wars::systems::TOWER_OFFSET_X;

    for level_id in [
        "sh_01", "sh_02", "sh_03",
        "gy_01", "gy_02", "gy_03",
        "hk_01", "hk_02", "hk_03",
        "cd_01", "cd_02", "cd_03",
        "bj_01", "bj_02", "bj_03",
    ] {
        let p = Progress::default();
        let g = Game::new(&ctx(), level_id, 1, &p).expect("level exists");
        let catapult_x = g.catapult.pos.x;
        let tower_x = g.enemy_tower.x;
        let distance = tower_x - catapult_x;
        assert!(
            distance > CATAPULT_MAX_RANGE,
            "level {} : distance {} <= portée max {} — la tour est attaquable",
            level_id,
            distance,
            CATAPULT_MAX_RANGE,
        );
        assert!(catapult_x > TOWER_OFFSET_X);
    }
}

#[test]
fn upgrade_tree_ron_loads() {
    use ember_wars::upgrades::load_defs;
    assert_eq!(load_defs().nodes.len(), 28);
}

#[test]
fn units_ron_loads() {
    use ember_core::io::load_from_file;
    use ember_wars::units::UnitStats;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("units.ron");
    let data: Vec<UnitStats> = load_from_file(&path).expect("units.ron");
    assert_eq!(data.len(), 6);
}

#[test]
fn enemy_catapult_present_only_on_flagged_levels() {
    // Niveaux sans catapulte ennemie.
    for id in ["sh_01", "sh_02", "gy_01", "hk_01", "cd_01"] {
        let p = Progress::default();
        let g = Game::new(&ctx(), id, 1, &p).expect("level exists");
        assert!(g.enemy_catapult.is_none(), "level {id} should NOT have enemy catapult");
    }
    // Niveaux avec catapulte ennemie.
    for id in ["cd_03", "bj_02", "bj_03"] {
        let p = Progress::default();
        let g = Game::new(&ctx(), id, 1, &p).expect("level exists");
        assert!(g.enemy_catapult.is_some(), "level {id} should have enemy catapult");
    }
}

#[test]
fn enemy_catapult_fires_when_player_unit_in_range() {
    let p = Progress::default();
    let mut g = Game::new(&ctx(), "cd_03", 1, &p).expect("creates");
    g.enemy_mana.current = 0.0;
    g.enemy_mana.regen = 0.0;

    let ec_x = g.enemy_catapult.as_ref().unwrap().pos.x;
    let gy = g.ground_y;
    let grunt = spawn_unit(
        &mut g.id_gen,
        "grunt",
        Team::Player,
        Vec2::new(ec_x - 300.0, gy),
        1.0,
        1.0,
    )
    .unwrap();
    g.units.push(grunt);

    g.tick(SIM_DT, Vec2::ZERO, false, 0.0);

    let has_enemy_proj = g
        .projectiles
        .iter()
        .any(|p| p.team == Team::Enemy && p.gravity > 0.0);
    assert!(has_enemy_proj, "enemy catapult should fire at player unit");
}