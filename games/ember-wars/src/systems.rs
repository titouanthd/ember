//! State struct `Game` + machine à états `Phase`.

use std::collections::HashMap;

use ember_stdlib::Rng;
use glam::Vec2;
use macroquad::prelude::Color;

use crate::ai;
use crate::camera::Camera2D;
use crate::catapult::{
    Catapult, CATAPULT_HP, CATAPULT_MAX_RANGE, CATAPULT_MIN_RANGE, CATAPULT_OFFSET_X,
    CATAPULT_REBUILD_TIME,
};
use crate::combat;
use crate::components::{Projectile, Team, Tower, Unit, UnitIdGen};
use crate::config::{self, Balance, GameContext, LevelConfig, Objective};
use crate::juice::Juice;
use crate::mana::ManaPool;
use crate::progress::Progress;
use crate::textures::{self, Palette};
use crate::units::{can_spawn, spawn_unit, unit_stats};
use crate::upgrades::UpgradeTree;

pub const TOWER_OFFSET_X: f32 = 60.0;
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
    pub catapult: Catapult,
    pub enemy_catapult: Option<Catapult>,
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

        let mut catapult = Catapult::new(Vec2::new(
            player_tower.x + CATAPULT_OFFSET_X,
            ground_y,
        ));
        catapult.configure(
            CATAPULT_MAX_RANGE + player_upgrades.catapult_range_add(),
            CATAPULT_HP * player_upgrades.catapult_hp_mult(),
            CATAPULT_REBUILD_TIME * player_upgrades.catapult_rebuild_mult(),
        );

        let enemy_catapult = if level.enemy_catapult {
            Some(Catapult::with_facing(
                Vec2::new(enemy_tower.x - CATAPULT_OFFSET_X, ground_y),
                -1.0,
            ))
        } else {
            None
        };

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
            catapult,
            enemy_catapult,
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
        self.catapult.tick(dt);
        if let Some(ec) = &mut self.enemy_catapult {
            ec.tick(dt);
        }

        self.camera.scroll(dt, camera_scroll);

        self.catapult.aim(mouse_world);
        if mouse_left_pressed && self.catapult.is_ready() {
            let damage_mult = self.player_upgrades.turret_damage_mult();
            let fire_rate_mult = self.player_upgrades.turret_fire_rate_mult();
            let multi = self.player_upgrades.multi_shot_count();
            let crit_chance = self.player_upgrades.turret_crit_chance();
            self.catapult.fire_rate_mult = fire_rate_mult;
            self.catapult.multi_shot = multi;
            if let Some(mut shots) = self.catapult.try_fire_multi(Team::Player, damage_mult)
            {
                self.juice.shake(2.0, 0.06);
                for p in &mut shots {
                    if crit_chance > 0.0 && self.rng.next_f32() < crit_chance {
                        p.damage *= 2.0;
                        p.radius *= 1.4;
                        p.color = Color::new(1.0, 0.95, 0.4, 1.0);
                    }
                }
                for p in shots {
                    if self.projectiles.len() < combat::MAX_PROJECTILES {
                        self.projectiles.push(p);
                    }
                }
            }
        }

        self.tick_ai();
        self.tick_enemy_catapult_ai();

        let speed_mult = self.player_upgrades.unit_speed_mult();

        let Self {
            units,
            player_tower,
            enemy_tower,
            catapult,
            enemy_catapult,
            projectiles,
            juice,
            ground_y,
            ..
        } = self;
        let gy = *ground_y;

        combat::resolve_attacks(
            units,
            (player_tower, enemy_tower),
            (catapult, enemy_catapult),
            projectiles,
            juice,
            dt,
            speed_mult,
        );
        combat::resolve_projectiles(
            projectiles,
            units,
            (player_tower, enemy_tower),
            (catapult, enemy_catapult),
            gy,
            juice,
            dt,
        );

        self.cleanup_dead_units();
        self.juice.tick(dt);

        self.stats.elapsed += dt;

        let was_playing = self.phase == Phase::Playing;
        self.check_end_condition();
        if was_playing && self.phase != Phase::Playing {
            let color = match self.phase {
                Phase::Won => Color::new(0.4, 1.0, 0.5, 1.0),
                Phase::Lost => Color::new(1.0, 0.3, 0.3, 1.0),
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

    fn tick_enemy_catapult_ai(&mut self) {
        let Some(ec) = self.enemy_catapult.as_mut() else {
            return;
        };
        if !ec.is_ready() {
            return;
        }

        let mut best: Option<(f32, Vec2)> = None;
        for u in &self.units {
            if u.team != Team::Player || !u.is_alive() {
                continue;
            }
            let d = (u.pos.x - ec.pos.x) * ec.facing;
            if d < CATAPULT_MIN_RANGE || d > ec.max_range {
                continue;
            }
            if best.is_none_or(|(x, _)| u.pos.x > x) {
                best = Some((u.pos.x, u.pos));
            }
        }

        let Some((_, target)) = best else {
            return;
        };
        ec.aim_target = target;

        if let Some(shots) = ec.try_fire_multi(Team::Enemy, 1.0) {
            for p in shots {
                if self.projectiles.len() < combat::MAX_PROJECTILES {
                    self.projectiles.push(p);
                }
            }
            self.juice.shake(1.5, 0.05);
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
            let mana_per_kill =
                self.balance.mana_per_kill + self.player_upgrades.mana_on_kill_add();
            let mana = enemy_kills as f32 * mana_per_kill;
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
    use crate::catapult::{
        ENEMY_CATAPULT_HP, ENEMY_CATAPULT_MAX_RANGE, ENEMY_CATAPULT_REBUILD_TIME,
    };
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
    fn new_game_places_towers_at_edges() {
        let g = new_game(0);
        assert!((g.player_tower.x - TOWER_OFFSET_X).abs() < 1e-6);
        assert!((g.enemy_tower.x - (1800.0 - TOWER_OFFSET_X)).abs() < 1e-6);
    }

    #[test]
    fn player_catapult_is_placed_in_front_of_tower() {
        let g = new_game(0);
        assert!((g.catapult.pos.x - (TOWER_OFFSET_X + CATAPULT_OFFSET_X)).abs() < 1e-6);
    }

    #[test]
    fn sh_02_has_no_enemy_catapult() {
        let g = new_game(0);
        assert!(g.enemy_catapult.is_none());
    }

    #[test]
    fn cd_03_has_enemy_catapult() {
        let g = Game::new(&ctx(), "cd_03", 0, &default_progress()).expect("creates");
        assert!(g.enemy_catapult.is_some());
        let ec = g.enemy_catapult.as_ref().unwrap();
        assert_eq!(ec.facing, -1.0);
        assert!((ec.max_hp - ENEMY_CATAPULT_HP).abs() < 1e-6);
        assert!((ec.max_range - ENEMY_CATAPULT_MAX_RANGE).abs() < 1e-6);
        assert!((ec.rebuild_time - ENEMY_CATAPULT_REBUILD_TIME).abs() < 1e-6);
    }

    #[test]
    fn enemy_catapult_placed_in_front_of_enemy_tower() {
        let g = Game::new(&ctx(), "cd_03", 0, &default_progress()).expect("creates");
        let ec = g.enemy_catapult.as_ref().unwrap();
        assert!((ec.pos.x - (g.enemy_tower.x - CATAPULT_OFFSET_X)).abs() < 1e-6);
    }

    #[test]
    fn enemy_catapult_fires_at_player_units() {
        let p = Progress::default();
        let mut g = Game::new(&ctx(), "cd_03", 1, &p).expect("creates");
        disable_ai(&mut g);

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

        g.tick(1.0 / 60.0, Vec2::ZERO, false, 0.0);

        let has_enemy_proj = g
            .projectiles
            .iter()
            .any(|p| p.team == Team::Enemy && p.gravity > 0.0);
        assert!(has_enemy_proj, "enemy catapult didn't fire");
    }

    #[test]
    fn enemy_catapult_ignores_units_out_of_range() {
        let p = Progress::default();
        let mut g = Game::new(&ctx(), "cd_03", 1, &p).expect("creates");
        disable_ai(&mut g);

        let gy = g.ground_y;
        let grunt = spawn_unit(
            &mut g.id_gen,
            "grunt",
            Team::Player,
            Vec2::new(100.0, gy),
            1.0,
            1.0,
        )
        .unwrap();
        g.units.push(grunt);

        g.tick(1.0 / 60.0, Vec2::ZERO, false, 0.0);

        let has_enemy_proj = g
            .projectiles
            .iter()
            .any(|p| p.team == Team::Enemy && p.gravity > 0.0);
        assert!(!has_enemy_proj);
    }

    #[test]
    fn destroyed_catapult_does_not_fire() {
        let mut g = new_game(0);
        disable_ai(&mut g);
        g.catapult.take_damage(9999.0);
        let target = Vec2::new(g.catapult.pos.x + 500.0, g.ground_y);
        g.tick(0.05, target, true, 0.0);
        assert!(!g.projectiles.iter().any(|p| p.gravity > 0.0));
    }

    #[test]
    fn player_fire_spawns_parabolic_projectile() {
        let mut g = new_game(0);
        disable_ai(&mut g);
        let target = Vec2::new(g.catapult.pos.x + 500.0, g.ground_y);
        g.tick(0.05, target, true, 0.0);
        let proj = g
            .projectiles
            .iter()
            .find(|p| p.team == Team::Player && p.gravity > 0.0);
        assert!(proj.is_some());
    }

    #[test]
    fn new_game_applies_catapult_range_upgrade() {
        let mut p = Progress::default();
        p.tree.grant("catapult_range_1");
        let g = Game::new(&ctx(), "sh_02", 0, &p).expect("creates");
        assert!((g.catapult.max_range - (CATAPULT_MAX_RANGE + 200.0)).abs() < 1e-6);
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
    fn catapult_rebuilds_after_timer() {
        let mut g = new_game(0);
        disable_ai(&mut g);
        g.catapult.take_damage(9999.0);
        for _ in 0..700 {
            g.tick(1.0 / 60.0, Vec2::ZERO, false, 0.0);
        }
        assert!(g.catapult.is_alive());
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

    #[test]
    fn new_levels_load_without_crash() {
        for id in ["cd_01", "cd_02", "cd_03", "bj_01", "bj_02", "bj_03"] {
            let p = Progress::default();
            let g = Game::new(&ctx(), id, 1, &p).expect("level loads");
            assert_eq!(g.phase, Phase::Playing);
        }
    }
}