//! State struct `Game` + machine à états `Phase`.

use std::collections::HashMap;

use ember_stdlib::Rng;
use glam::Vec2;

use crate::ai;
use crate::camera::Camera2D;
use crate::combat;
use crate::components::{Projectile, Team, Tower, Unit, UnitIdGen};
use crate::config::{self, Balance, GameContext, LevelConfig, Objective};
use crate::juice::Juice;
use crate::mana::ManaPool;
use crate::progress::Progress;
use crate::textures::{self, Palette};
use crate::tower::Turret;
use crate::units::{can_spawn, spawn_unit, unit_stats};
use crate::upgrades::UpgradeTree;

pub const TOWER_OFFSET_X: f32 = 60.0;
pub const TURRET_HEIGHT: f32 = 70.0;
pub const UNIT_SPAWN_OFFSET_X: f32 = 40.0;
pub const GROUND_Y_RATIO: f32 = 0.68;

pub const DEFAULT_UNLOCKED_UNITS: &[&str] = &["grunt", "brute"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Default, Clone)]
pub struct GameStats {
    pub kills: u32,
    pub gold_earned: f32,
    pub elapsed: f32,
}

pub struct Game {
    pub phase: Phase,
    pub player_tower: Tower,
    pub enemy_tower: Tower,
    pub units: Vec<Unit>,
    pub projectiles: Vec<Projectile>,
    pub player_mana: ManaPool,
    pub enemy_mana: ManaPool,
    pub player_upgrades: UpgradeTree,
    pub turret: Turret,
    pub camera: Camera2D,
    pub player_cooldowns: HashMap<String, f32>,
    pub enemy_cooldowns: HashMap<String, f32>,
    pub id_gen: UnitIdGen,
    pub rng: Rng,
    pub level: &'static LevelConfig,
    pub palette: &'static Palette,
    pub visual_seed: u32,
    pub balance: &'static Balance,
    pub stats: GameStats,
    pub ground_y: f32,
    pub juice: Juice,
}

impl Game {
    pub fn new(
        ctx: &GameContext,
        level_id: &str,
        seed: u32,
        progress: &Progress,
    ) -> Result<Self, String> {
        let level = config::level_by_id(level_id)
            .ok_or_else(|| format!("unknown level id: {level_id}"))?;
        let palette = textures::palette_by_id(&level.palette)
            .ok_or_else(|| format!("unknown palette: {}", level.palette))?;
        let balance = config::balance();

        let ground_y = ctx.viewport_h * GROUND_Y_RATIO;
        let visual_seed = seed
            .wrapping_mul(0x9E37_79B9)
            .wrapping_add(0x85EB_CA6B);

        let player_upgrades = progress.tree.clone();
        let tower_hp = level.tower_hp * player_upgrades.tower_hp_mult();

        let player_tower = Tower {
            team: Team::Player,
            hp: tower_hp,
            max_hp: tower_hp,
            x: TOWER_OFFSET_X,
        };
        let enemy_tower = Tower {
            team: Team::Enemy,
            hp: level.tower_hp,
            max_hp: level.tower_hp,
            x: level.width - TOWER_OFFSET_X,
        };

        let player_mana = ManaPool::new(
            balance.mana_start,
            balance.mana_max * player_upgrades.mana_cap_mult(),
            balance.mana_regen * player_upgrades.mana_regen_mult(),
        );
        let enemy_mana = ManaPool::new(
            balance.mana_start,
            balance.mana_max,
            balance.mana_regen * level.ai.mana_regen_mult,
        );

        let turret = Turret::new(Vec2::new(player_tower.x, ground_y - TURRET_HEIGHT));
        let mut camera = Camera2D::new(ctx.viewport_w, level.width);
        camera.x = 0.0;

        Ok(Self {
            phase: Phase::Playing,
            player_tower,
            enemy_tower,
            units: Vec::new(),
            projectiles: Vec::new(),
            player_mana,
            enemy_mana,
            player_upgrades,
            turret,
            camera,
            player_cooldowns: HashMap::new(),
            enemy_cooldowns: HashMap::new(),
            id_gen: UnitIdGen::new(),
            rng: Rng::new(seed),
            level,
            palette,
            visual_seed,
            balance,
            stats: GameStats::default(),
            ground_y,
            juice: Juice::new(),
        })
    }

    pub fn rng_state(&self) -> u32 {
        self.rng.state()
    }

    pub fn unlocked_units(&self) -> std::collections::HashSet<String> {
        self.player_upgrades.unlocked_units(DEFAULT_UNLOCKED_UNITS)
    }

    pub fn is_unit_unlocked(&self, kind: &str) -> bool {
        self.unlocked_units().contains(kind)
    }

    pub fn survive_remaining(&self) -> Option<f32> {
        match &self.level.objective {
            Objective::Survive(secs) => Some((secs - self.stats.elapsed).max(0.0)),
            Objective::Destroy => None,
        }
    }

    pub fn tick(
        &mut self,
        dt: f32,
        mouse_world: Vec2,
        mouse_left_pressed: bool,
        camera_scroll: f32,
    ) {
        if self.phase != Phase::Playing {
            return;
        }

        self.player_mana.tick(dt);
        self.enemy_mana.tick(dt);

        let tower_regen = self.player_upgrades.tower_regen();
        if tower_regen > 0.0 && self.player_tower.hp < self.player_tower.max_hp {
            self.player_tower.hp =
                (self.player_tower.hp + tower_regen * dt).min(self.player_tower.max_hp);
        }

        tick_cooldowns(&mut self.player_cooldowns, dt);
        tick_cooldowns(&mut self.enemy_cooldowns, dt);
        self.turret.tick(dt);

        self.camera.scroll(dt, camera_scroll);

        self.turret.aim(mouse_world);
        if mouse_left_pressed {
            let damage_mult = self.player_upgrades.turret_damage_mult();
            let fire_rate_mult = self.player_upgrades.turret_fire_rate_mult();
            let multi = self.player_upgrades.multi_shot_count();
            self.turret.fire_rate_mult = fire_rate_mult;
            self.turret.multi_shot = multi;
            if let Some(shots) = self.turret.try_fire_multi(Team::Player, damage_mult) {
                self.juice.shake(1.5, 0.05);
                for p in shots {
                    if self.projectiles.len() < combat::MAX_PROJECTILES {
                        self.projectiles.push(p);
                    }
                }
            }
        }

        self.tick_ai();

        combat::resolve_attacks(
            &mut self.units,
            &mut self.player_tower,
            &mut self.enemy_tower,
            &mut self.projectiles,
            &mut self.juice,
            dt,
        );
        combat::resolve_projectiles(
            &mut self.projectiles,
            &mut self.units,
            &mut self.player_tower,
            &mut self.enemy_tower,
            &mut self.juice,
            dt,
        );

        self.cleanup_dead_units();
        self.juice.tick(dt);

        self.stats.elapsed += dt;

        let was_playing = self.phase == Phase::Playing;
        self.check_end_condition();
        if was_playing && self.phase != Phase::Playing {
            let color = match self.phase {
                Phase::Won => macroquad::prelude::Color::new(0.4, 1.0, 0.5, 1.0),
                Phase::Lost => macroquad::prelude::Color::new(1.0, 0.3, 0.3, 1.0),
                Phase::Playing => unreachable!(),
            };
            self.juice.flash(color, 0.5);
            self.juice.shake(8.0, 0.4);
        }
    }

    pub fn try_player_spawn(&mut self, kind: &str) -> bool {
        if !self.is_unit_unlocked(kind) {
            return false;
        }
        let stats = match unit_stats(kind) {
            Some(s) => s,
            None => return false,
        };
        if !can_spawn(kind, Team::Player, &self.units) {
            return false;
        }
        let cd = self.player_cooldowns.get(kind).copied().unwrap_or(0.0);
        if cd > 0.0 {
            return false;
        }
        if !self.player_mana.try_spend(stats.cost) {
            return false;
        }

        let x = self.player_tower.x + UNIT_SPAWN_OFFSET_X;
        let hp_mult = self.player_upgrades.unit_hp_mult();
        let damage_mult = self.player_upgrades.unit_damage_mult();

        match spawn_unit(
            &mut self.id_gen,
            kind,
            Team::Player,
            Vec2::new(x, self.ground_y),
            hp_mult,
            damage_mult,
        ) {
            Some(unit) => {
                self.juice
                    .spawn_hit_spark(Vec2::new(x, self.ground_y - 20.0), stats.color());
                self.units.push(unit);
                self.player_cooldowns
                    .insert(kind.to_string(), stats.cooldown);
                true
            }
            None => false,
        }
    }

    pub fn tower_hp_fraction(&self) -> f32 {
        self.player_tower.hp_fraction()
    }

    pub fn check_end_condition(&mut self) {
        if self.enemy_tower.is_destroyed() {
            self.phase = Phase::Won;
            return;
        }
        if self.player_tower.is_destroyed() {
            self.phase = Phase::Lost;
            return;
        }
        if let Objective::Survive(secs) = self.level.objective
            && self.stats.elapsed >= secs
        {
            self.phase = Phase::Won;
        }
    }

    fn tick_ai(&mut self) {
        let ai_units: Vec<Unit> = self
            .units
            .iter()
            .filter(|u| u.team == Team::Enemy)
            .cloned()
            .collect();
        let player_units: Vec<Unit> = self
            .units
            .iter()
            .filter(|u| u.team == Team::Player)
            .cloned()
            .collect();

        let kind = match ai::decide_spawn(
            &self.enemy_mana,
            &ai_units,
            &player_units,
            &self.level.ai,
            self.stats.elapsed,
            &self.enemy_cooldowns,
        ) {
            Some(k) => k,
            None => return,
        };

        let stats = match unit_stats(&kind) {
            Some(s) => s,
            None => return,
        };
        if !self.enemy_mana.try_spend(stats.cost) {
            return;
        }

        let x = self.enemy_tower.x - UNIT_SPAWN_OFFSET_X;
        if let Some(unit) = spawn_unit(
            &mut self.id_gen,
            &kind,
            Team::Enemy,
            Vec2::new(x, self.ground_y),
            1.0,
            1.0,
        ) {
            self.units.push(unit);
            self.enemy_cooldowns.insert(kind, stats.cooldown);
        }
    }

    fn cleanup_dead_units(&mut self) {
        let enemy_kills = self
            .units
            .iter()
            .filter(|u| u.team == Team::Enemy && !u.is_alive())
            .count() as u32;
        let player_kills = self
            .units
            .iter()
            .filter(|u| u.team == Team::Player && !u.is_alive())
            .count() as u32;

        if enemy_kills > 0 {
            self.stats.kills += enemy_kills;
            let gold_per_kill =
                self.balance.gold_per_kill + self.player_upgrades.gold_per_kill_add();
            let gold = enemy_kills as f32 * gold_per_kill;
            self.player_upgrades.gold += gold;
            self.stats.gold_earned += gold;
            let mana = enemy_kills as f32 * self.balance.mana_per_kill;
            self.player_mana.gain(mana);
        }
        if player_kills > 0 {
            let mana = player_kills as f32 * self.balance.mana_per_kill;
            self.enemy_mana.gain(mana);
        }

        self.units.retain(|u| u.is_alive());
    }
}

fn tick_cooldowns(map: &mut HashMap<String, f32>, dt: f32) {
    for cd in map.values_mut() {
        *cd = (*cd - dt).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::components::UnitId;

    fn ctx() -> GameContext {
        GameContext::from_env()
    }

    fn default_progress() -> Progress {
        Progress::default()
    }

    fn new_game(seed: u32) -> Game {
        Game::new(&ctx(), "sh_02", seed, &default_progress()).expect("game creates")
    }

    fn new_game_survive(seed: u32) -> Game {
        Game::new(&ctx(), "sh_01", seed, &default_progress()).expect("game creates")
    }

    fn disable_ai(g: &mut Game) {
        g.enemy_mana.current = 0.0;
        g.enemy_mana.regen = 0.0;
    }

    #[test]
    fn new_game_starts_in_playing_phase() {
        let g = new_game(0);
        assert_eq!(g.phase, Phase::Playing);
    }

    #[test]
    fn new_game_loads_level_width() {
        let g = new_game(0);
        assert!((g.level.width - 1800.0).abs() < 1e-6);
    }

    #[test]
    fn new_game_places_towers_at_edges() {
        let g = new_game(0);
        assert!((g.player_tower.x - TOWER_OFFSET_X).abs() < 1e-6);
        assert!((g.enemy_tower.x - (1800.0 - TOWER_OFFSET_X)).abs() < 1e-6);
    }

    #[test]
    fn new_game_uses_progress_tree() {
        let mut p = Progress::default();
        p.tree.gold = 100.0;
        p.tree.grant("tower_hp_1");
        let g = Game::new(&ctx(), "sh_02", 0, &p).expect("creates");
        assert!((g.player_tower.max_hp - 600.0).abs() < 1e-3);
    }

    #[test]
    fn new_game_returns_err_for_unknown_level() {
        assert!(Game::new(&ctx(), "nope", 0, &default_progress()).is_err());
    }

    #[test]
    fn ground_y_follows_ratio() {
        let g = new_game(0);
        let expected = ctx().viewport_h * GROUND_Y_RATIO;
        assert!((g.ground_y - expected).abs() < 1e-3);
    }

    #[test]
    fn new_game_sets_palette_matching_level() {
        let g = new_game(0);
        assert_eq!(g.palette.id, "shanghai");
    }

    #[test]
    fn new_game_sets_palette_for_guiyang_level() {
        let g = Game::new(&ctx(), "gy_01", 0, &default_progress()).expect("creates");
        assert_eq!(g.palette.id, "guiyang");
    }

    #[test]
    fn visual_seed_is_deterministic() {
        let a = new_game(42);
        let b = new_game(42);
        assert_eq!(a.visual_seed, b.visual_seed);
    }

    #[test]
    fn visual_seed_changes_with_game_seed() {
        let a = new_game(1);
        let b = new_game(2);
        assert_ne!(a.visual_seed, b.visual_seed);
    }

    #[test]
    fn survive_objective_reports_remaining() {
        let g = new_game_survive(0);
        assert!((g.survive_remaining().unwrap() - 45.0).abs() < 1e-6);
    }

    #[test]
    fn destroy_objective_has_no_remaining() {
        let g = new_game(0);
        assert!(g.survive_remaining().is_none());
    }

    #[test]
    fn survive_objective_wins_at_timeout() {
        let mut g = new_game_survive(0);
        disable_ai(&mut g);
        let mut t = 0.0;
        while t < 46.0 {
            g.tick(0.5, Vec2::ZERO, false, 0.0);
            t += 0.5;
        }
        assert_eq!(g.phase, Phase::Won);
    }

    #[test]
    fn default_unlocked_units_are_grunt_and_brute() {
        let g = new_game(0);
        assert!(g.is_unit_unlocked("grunt"));
        assert!(g.is_unit_unlocked("brute"));
        assert!(!g.is_unit_unlocked("archer"));
    }

    #[test]
    fn unlocking_archer_via_progress_enables_spawn() {
        let mut p = Progress::default();
        p.tree.grant("unit_hp_1");
        p.tree.grant("unlock_archer");
        let mut g = Game::new(&ctx(), "sh_02", 0, &p).expect("creates");
        g.player_mana.current = 100.0;
        assert!(g.try_player_spawn("archer"));
    }

    #[test]
    fn tick_advances_elapsed() {
        let mut g = new_game(0);
        g.tick(0.5, Vec2::ZERO, false, 0.0);
        assert!((g.stats.elapsed - 0.5).abs() < 1e-6);
    }

    #[test]
    fn tick_does_nothing_outside_playing() {
        let mut g = new_game(0);
        g.phase = Phase::Won;
        let before = g.stats.elapsed;
        g.tick(0.5, Vec2::ZERO, false, 0.0);
        assert_eq!(g.stats.elapsed, before);
    }

    #[test]
    fn tick_spawns_ai_after_wave_start() {
        let mut g = new_game(0);
        g.enemy_mana.current = 100.0;
        g.tick(0.05, Vec2::ZERO, false, 0.0);
        assert!(g.units.iter().any(|u| u.team == Team::Enemy));
    }

    #[test]
    fn try_player_spawn_spends_mana() {
        let mut g = new_game(0);
        let before = g.player_mana.current;
        assert!(g.try_player_spawn("grunt"));
        assert!(g.player_mana.current < before);
    }

    #[test]
    fn kills_credit_bank_gold() {
        let mut g = new_game(0);
        disable_ai(&mut g);
        let gold_before = g.player_upgrades.gold;

        let mut dead = spawn_unit(
            &mut g.id_gen,
            "grunt",
            Team::Enemy,
            Vec2::new(500.0, g.ground_y),
            1.0,
            1.0,
        )
        .unwrap();
        dead.hp = 0.0;
        g.units.push(dead);
        g.tick(0.0, Vec2::ZERO, false, 0.0);

        assert!(g.player_upgrades.gold > gold_before);
    }

    #[test]
    fn tower_hp_fraction_reflects_current_hp() {
        let mut g = new_game(0);
        g.player_tower.hp = g.player_tower.max_hp * 0.5;
        assert!((g.tower_hp_fraction() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ai_spawns_start_at_full_hp() {
        let mut g = new_game(0);
        g.enemy_mana.current = 100.0;
        g.tick(0.05, Vec2::ZERO, false, 0.0);
        let enemy = g.units.iter().find(|u| u.team == Team::Enemy).unwrap();
        assert_eq!(enemy.hp, enemy.max_hp);
        assert!(enemy.id >= UnitId(0));
    }
}