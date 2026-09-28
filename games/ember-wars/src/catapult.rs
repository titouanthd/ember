//! Catapulte — tir parabolique, structure destructible.
//!
//! Remplace l'ancienne `Turret`. Peut être orientée à droite (joueur)
//! ou à gauche (ennemi) via le champ `facing`.

use glam::Vec2;

use crate::components::{Projectile, ProjectileKind, Team};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
    Basic,
    Piercing,
    Explosive,
}

impl ShotKind {
    pub fn label(self) -> &'static str {
        match self {
            ShotKind::Basic => "Basic",
            ShotKind::Piercing => "Piercing",
            ShotKind::Explosive => "Explosive",
        }
    }
}

// ---------- Constantes de base (joueur) ----------

pub const CATAPULT_HP: f32 = 300.0;
pub const CATAPULT_REBUILD_TIME: f32 = 10.0;
pub const CATAPULT_OFFSET_X: f32 = 150.0;

pub const CATAPULT_MIN_RANGE: f32 = 150.0;
pub const CATAPULT_MAX_RANGE: f32 = 1000.0;

pub const CATAPULT_FIRE_COOLDOWN: f32 = 0.9;
pub const CATAPULT_BASE_DAMAGE: f32 = 14.0;

pub const CATAPULT_H_SPEED: f32 = 650.0;
pub const CATAPULT_GRAVITY: f32 = 900.0;

pub const MULTI_SHOT_SPREAD_PX: f32 = 20.0;

pub const CATAPULT_HITBOX_W: f32 = 50.0;
pub const CATAPULT_HITBOX_H: f32 = 60.0;

pub const LAUNCH_HEIGHT: f32 = 45.0;

// ---------- Constantes ennemi (plus faibles) ----------

pub const ENEMY_CATAPULT_HP: f32 = 250.0;
pub const ENEMY_CATAPULT_REBUILD_TIME: f32 = 12.0;
pub const ENEMY_CATAPULT_MAX_RANGE: f32 = 700.0;
pub const ENEMY_CATAPULT_FIRE_COOLDOWN: f32 = 1.4;
pub const ENEMY_CATAPULT_DAMAGE: f32 = 10.0;

#[derive(Debug, Clone)]
pub struct Catapult {
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub cooldown: f32,
    pub shot_kind: ShotKind,
    pub fire_rate_mult: f32,
    pub multi_shot: u32,
    pub rebuild_timer: f32,
    pub rebuild_time: f32,
    pub max_range: f32,
    pub aim_target: Vec2,
    /// 1.0 = tire vers la droite (joueur), -1.0 = tire vers la gauche (ennemi).
    pub facing: f32,
    /// Dégâts de base (différents pour joueur / ennemi).
    pub base_damage: f32,
}

impl Catapult {
    /// Catapulte du joueur (tire à droite, stats joueur).
    pub fn new(pos: Vec2) -> Self {
        Self::with_facing(pos, 1.0)
    }

    /// Catapulte orientée. Utilise les stats joueur si `facing > 0`,
    /// les stats ennemi sinon (HP, portée, rebuild, dégâts).
    pub fn with_facing(pos: Vec2, facing: f32) -> Self {
        let facing = if facing >= 0.0 { 1.0 } else { -1.0 };
        let (hp, max_range, rebuild_time, base_damage) = if facing > 0.0 {
            (
                CATAPULT_HP,
                CATAPULT_MAX_RANGE,
                CATAPULT_REBUILD_TIME,
                CATAPULT_BASE_DAMAGE,
            )
        } else {
            (
                ENEMY_CATAPULT_HP,
                ENEMY_CATAPULT_MAX_RANGE,
                ENEMY_CATAPULT_REBUILD_TIME,
                ENEMY_CATAPULT_DAMAGE,
            )
        };
        Self {
            pos,
            hp,
            max_hp: hp,
            cooldown: 0.0,
            shot_kind: ShotKind::Basic,
            fire_rate_mult: 1.0,
            multi_shot: 1,
            rebuild_timer: 0.0,
            rebuild_time,
            max_range,
            aim_target: pos + Vec2::new(CATAPULT_MIN_RANGE * facing, 0.0),
            facing,
            base_damage,
        }
    }

    /// Configure la catapulte avec les paramètres d'upgrade (joueur).
    pub fn configure(&mut self, max_range: f32, max_hp: f32, rebuild_time: f32) {
        self.max_range = max_range.max(CATAPULT_MIN_RANGE);
        self.max_hp = max_hp.max(1.0);
        self.hp = self.max_hp;
        self.rebuild_time = rebuild_time.max(0.5);
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0.0 && self.rebuild_timer <= 0.0
    }

    pub fn is_rebuilding(&self) -> bool {
        self.rebuild_timer > 0.0
    }

    pub fn hp_fraction(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.hp / self.max_hp).clamp(0.0, 1.0)
        }
    }

    pub fn rect(&self) -> (f32, f32, f32, f32) {
        (
            self.pos.x - CATAPULT_HITBOX_W * 0.5,
            self.pos.y - CATAPULT_HITBOX_H,
            CATAPULT_HITBOX_W,
            CATAPULT_HITBOX_H,
        )
    }

    pub fn launch_origin(&self) -> Vec2 {
        self.pos - Vec2::new(0.0, LAUNCH_HEIGHT)
    }

    /// Cible le point de chute (clampé à la portée dans la direction facing).
    pub fn aim(&mut self, mouse_world: Vec2) {
        let dx_signed = mouse_world.x - self.pos.x;
        let dx_forward = dx_signed * self.facing;
        let dx_clamped = dx_forward.clamp(CATAPULT_MIN_RANGE, self.max_range);
        let target_x = self.pos.x + dx_clamped * self.facing;
        self.aim_target = Vec2::new(target_x, mouse_world.y);
    }

    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        if self.cooldown > 0.0 {
            self.cooldown = (self.cooldown - dt).max(0.0);
        }
        if self.rebuild_timer > 0.0 {
            self.rebuild_timer = (self.rebuild_timer - dt).max(0.0);
            if self.rebuild_timer <= 0.0 {
                self.hp = self.max_hp;
            }
        }
    }

    pub fn is_ready(&self) -> bool {
        self.is_alive() && self.cooldown <= 0.0
    }

    pub fn take_damage(&mut self, amount: f32) {
        if !self.is_alive() {
            return;
        }
        self.hp -= amount;
        if self.hp <= 0.0 {
            self.hp = 0.0;
            self.rebuild_timer = self.rebuild_time;
        }
    }

    /// Résout la vélocité balistique initiale pour atteindre `target`.
    pub fn solve_trajectory(&self, target: Vec2) -> Option<Vec2> {
        let start = self.launch_origin();
        let dx_signed = target.x - start.x;
        let dx_forward = dx_signed * self.facing;
        if dx_forward <= 1.0 {
            return None;
        }
        let dy = target.y - start.y;
        let vx_abs = CATAPULT_H_SPEED;
        let t = dx_forward / vx_abs;
        if t <= 1e-3 {
            return None;
        }
        let g = CATAPULT_GRAVITY;
        let vy = (dy - 0.5 * g * t * t) / t;
        Some(Vec2::new(vx_abs * self.facing, vy))
    }

    pub fn launch_velocity(&self) -> Option<Vec2> {
        self.solve_trajectory(self.aim_target)
    }

    pub fn preview_points(&self, steps: usize) -> Vec<Vec2> {
        let vel = match self.launch_velocity() {
            Some(v) => v,
            None => return Vec::new(),
        };
        let start = self.launch_origin();
        let g = CATAPULT_GRAVITY;
        let dt = 0.06;
        let mut points = Vec::with_capacity(steps);
        let mut t = 0.0;
        for _ in 0..steps {
            t += dt;
            let p = start + Vec2::new(vel.x * t, vel.y * t + 0.5 * g * t * t);
            points.push(p);
        }
        points
    }

    pub fn try_fire_multi(
        &mut self,
        team: Team,
        damage_mult: f32,
    ) -> Option<Vec<Projectile>> {
        if !self.is_ready() {
            return None;
        }

        let n = self.multi_shot.max(1);
        let start = self.launch_origin();
        let mut shots = Vec::with_capacity(n as usize);

        for i in 0..n {
            let offset = (i as f32 - (n - 1) as f32 * 0.5) * MULTI_SHOT_SPREAD_PX;
            let target = self.aim_target + Vec2::new(offset, 0.0);
            let vel = match self.solve_trajectory(target) {
                Some(v) => v,
                None => continue,
            };
            let (kind, pierce, damage_mul) = match self.shot_kind {
                ShotKind::Basic => (ProjectileKind::Basic, 0, 1.0),
                ShotKind::Piercing => (ProjectileKind::Basic, 3, 1.0),
                ShotKind::Explosive => (ProjectileKind::Explosive, 0, 1.5),
            };
            shots.push(Projectile {
                pos: start,
                vel,
                gravity: CATAPULT_GRAVITY,
                damage: self.base_damage * damage_mult * damage_mul,
                radius: 7.0,
                team,
                kind,
                pierce_remaining: pierce,
                color: macroquad::prelude::Color::new(0.95, 0.70, 0.35, 1.0),
            });
        }

        if shots.is_empty() {
            return None;
        }

        let cd = if team == Team::Enemy {
            ENEMY_CATAPULT_FIRE_COOLDOWN
        } else {
            CATAPULT_FIRE_COOLDOWN
        };
        self.cooldown = cd * self.fire_rate_mult;
        Some(shots)
    }

    pub fn cycle_shot_kind(&mut self) {
        self.shot_kind = match self.shot_kind {
            ShotKind::Basic => ShotKind::Piercing,
            ShotKind::Piercing => ShotKind::Explosive,
            ShotKind::Explosive => ShotKind::Basic,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk() -> Catapult {
        Catapult::new(Vec2::new(200.0, 500.0))
    }

    fn mk_enemy() -> Catapult {
        Catapult::with_facing(Vec2::new(800.0, 500.0), -1.0)
    }

    #[test]
    fn new_catapult_is_alive_and_ready() {
        let c = mk();
        assert!(c.is_alive());
        assert!(c.is_ready());
        assert_eq!(c.hp, CATAPULT_HP);
        assert_eq!(c.facing, 1.0);
    }

    #[test]
    fn enemy_catapult_faces_left() {
        let c = mk_enemy();
        assert_eq!(c.facing, -1.0);
    }

    #[test]
    fn enemy_catapult_uses_enemy_stats() {
        let c = mk_enemy();
        assert_eq!(c.hp, ENEMY_CATAPULT_HP);
        assert_eq!(c.max_range, ENEMY_CATAPULT_MAX_RANGE);
        assert_eq!(c.rebuild_time, ENEMY_CATAPULT_REBUILD_TIME);
        assert_eq!(c.base_damage, ENEMY_CATAPULT_DAMAGE);
    }

    #[test]
    fn player_aim_clamps_to_max_range() {
        let mut c = mk();
        c.aim(Vec2::new(99_999.0, 500.0));
        assert!((c.aim_target.x - (c.pos.x + CATAPULT_MAX_RANGE)).abs() < 1e-6);
    }

    #[test]
    fn enemy_aim_clamps_to_max_range_leftward() {
        let mut c = mk_enemy();
        c.aim(Vec2::new(0.0, 500.0));
        assert!((c.aim_target.x - (c.pos.x - ENEMY_CATAPULT_MAX_RANGE)).abs() < 1e-6);
    }

    #[test]
    fn player_aim_rejects_left_target() {
        let mut c = mk();
        c.aim(Vec2::new(0.0, 500.0));
        assert!((c.aim_target.x - (c.pos.x + CATAPULT_MIN_RANGE)).abs() < 1e-6);
    }

    #[test]
    fn enemy_aim_rejects_right_target() {
        let mut c = mk_enemy();
        c.aim(Vec2::new(99_999.0, 500.0));
        assert!((c.aim_target.x - (c.pos.x - CATAPULT_MIN_RANGE)).abs() < 1e-6);
    }

    #[test]
    fn player_solve_trajectory_rightward() {
        let c = mk();
        let v = c.solve_trajectory(Vec2::new(600.0, 500.0)).unwrap();
        assert!(v.x > 0.0, "vx should be positive for player: {}", v.x);
    }

    #[test]
    fn enemy_solve_trajectory_leftward() {
        let c = mk_enemy();
        let v = c.solve_trajectory(Vec2::new(400.0, 500.0)).unwrap();
        assert!(v.x < 0.0, "vx should be negative for enemy: {}", v.x);
    }

    #[test]
    fn enemy_rejects_rightward_target() {
        let c = mk_enemy();
        assert!(c.solve_trajectory(Vec2::new(1500.0, 500.0)).is_none());
    }

    #[test]
    fn enemy_fire_produces_leftward_projectiles() {
        let mut c = mk_enemy();
        let shots = c.try_fire_multi(Team::Enemy, 1.0).unwrap();
        assert!(shots[0].vel.x < 0.0);
    }

    #[test]
    fn enemy_catapult_uses_reduced_damage() {
        let mut c = mk_enemy();
        let shots = c.try_fire_multi(Team::Enemy, 1.0).unwrap();
        assert!((shots[0].damage - ENEMY_CATAPULT_DAMAGE).abs() < 1e-6);
    }

    #[test]
    fn destruction_starts_rebuild_timer() {
        let mut c = mk();
        c.take_damage(400.0);
        assert!(!c.is_alive());
        assert!(c.is_rebuilding());
    }

    #[test]
    fn rebuild_completes_after_timer() {
        let mut c = mk();
        c.take_damage(400.0);
        c.tick(CATAPULT_REBUILD_TIME + 0.1);
        assert!(c.is_alive());
    }

    #[test]
    fn configure_changes_stats() {
        let mut c = mk();
        c.configure(1500.0, 500.0, 8.0);
        assert!((c.max_range - 1500.0).abs() < 1e-6);
        assert!((c.max_hp - 500.0).abs() < 1e-6);
        assert!((c.rebuild_time - 8.0).abs() < 1e-6);
    }
}