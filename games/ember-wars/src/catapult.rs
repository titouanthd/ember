//! Catapulte — tir parabolique, structure destructible.
//!
//! Remplace l'ancienne `Turret`. Peut être orientée à droite (joueur)
//! ou à gauche (ennemi) via le champ `facing`.

use ember_stdlib::time::Cooldown;
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
    /// Cooldown entre deux tirs.
    pub cooldown: Cooldown,
    pub shot_kind: ShotKind,
    pub fire_rate_mult: f32,
    pub multi_shot: u32,
    /// Timer de reconstruction. `is_active()` = détruite.
    pub rebuild: Cooldown,
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
            cooldown: Cooldown::new(CATAPULT_FIRE_COOLDOWN),
            shot_kind: ShotKind::Basic,
            fire_rate_mult: 1.0,
            multi_shot: 1,
            rebuild: Cooldown::new(rebuild_time),
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
        self.rebuild = Cooldown::new(rebuild_time.max(0.5));
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0.0 && self.rebuild.is_ready()
    }

    pub fn is_rebuilding(&self) -> bool {
        self.rebuild.is_active()
    }

    /// Temps restant avant reconstruction (0 si vivante).
    pub fn rebuild_remaining(&self) -> f32 {
        self.rebuild.remaining()
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
        self.cooldown.tick(dt);
        // tick_returning nous dit si le rebuild VIENT de finir.
        if self.rebuild.tick_returning(dt) {
            self.hp = self.max_hp;
        }
    }

    pub fn is_ready(&self) -> bool {
        self.is_alive() && self.cooldown.is_ready()
    }

    pub fn take_damage(&mut self, amount: f32) {
        if !self.is_alive() {
            return;
        }
        self.hp -= amount;
        if self.hp <= 0.0 {
            self.hp = 0.0;
            // reset() utilise la durée stockée dans `rebuild`.
            self.rebuild.reset();
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
        self.cooldown.trigger(cd * self.fire_rate_mult);
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

    // ---------- Construction / orientation ----------

    #[test]
    fn new_catapult_is_alive_and_ready() {
        let c = mk();
        assert!(c.is_alive());
        assert!(c.is_ready());
        assert_eq!(c.hp, CATAPULT_HP);
        assert_eq!(c.facing, 1.0);
        assert!(c.cooldown.is_ready());
        assert!(c.rebuild.is_ready());
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
        assert_eq!(c.rebuild.duration(), ENEMY_CATAPULT_REBUILD_TIME);
        assert_eq!(c.base_damage, ENEMY_CATAPULT_DAMAGE);
    }

    // ---------- Configure ----------

    #[test]
    fn configure_changes_stats() {
        let mut c = mk();
        c.configure(1500.0, 500.0, 8.0);
        assert!((c.max_range - 1500.0).abs() < 1e-6);
        assert!((c.max_hp - 500.0).abs() < 1e-6);
        assert!((c.rebuild.duration() - 8.0).abs() < 1e-6);
    }

    #[test]
    fn configure_changes_range() {
        let mut c = mk();
        c.configure(1500.0, 500.0, 8.0);
        assert!((c.max_range - 1500.0).abs() < 1e-6);
    }

    #[test]
    fn configure_changes_hp() {
        let mut c = mk();
        c.configure(1000.0, 500.0, 8.0);
        assert!((c.max_hp - 500.0).abs() < 1e-6);
        assert!((c.hp - 500.0).abs() < 1e-6);
    }

    #[test]
    fn configure_clamps_range_to_min() {
        let mut c = mk();
        c.configure(50.0, 300.0, 10.0);
        assert!((c.max_range - CATAPULT_MIN_RANGE).abs() < 1e-6);
    }

    #[test]
    fn configure_clamps_rebuild_to_min() {
        let mut c = mk();
        c.configure(1000.0, 300.0, 0.0);
        assert!((c.rebuild.duration() - 0.5).abs() < 1e-6);
    }

    // ---------- Aim ----------

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
    fn aim_keeps_target_y() {
        let mut c = mk();
        c.aim(Vec2::new(500.0, 420.0));
        assert!((c.aim_target.y - 420.0).abs() < 1e-6);
    }

    // ---------- Trajectoire ----------

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
    fn solve_trajectory_returns_none_for_left_target() {
        let c = mk();
        assert!(c.solve_trajectory(Vec2::new(100.0, 500.0)).is_none());
    }

    #[test]
    fn solve_trajectory_returns_positive_vx() {
        let c = mk();
        let v = c.solve_trajectory(Vec2::new(600.0, 500.0)).unwrap();
        assert!(v.x > 0.0);
    }

    #[test]
    fn solve_trajectory_launches_upward_for_same_height() {
        let c = mk();
        // launch_origin.y = 500 - 45 = 455. Target y = 500 (au sol).
        let v = c.solve_trajectory(Vec2::new(900.0, 500.0)).unwrap();
        assert!(v.y < 0.0);
    }

    #[test]
    fn solve_trajectory_long_range_has_taller_arc() {
        let c = mk();
        let v_short = c.solve_trajectory(Vec2::new(400.0, 500.0)).unwrap();
        let v_long = c.solve_trajectory(Vec2::new(1000.0, 500.0)).unwrap();
        assert!(v_long.y < v_short.y);
    }

    // ---------- Preview ----------

    #[test]
    fn preview_points_are_monotonic_in_x() {
        let c = mk();
        let pts = c.preview_points(10);
        assert!(!pts.is_empty());
        for w in pts.windows(2) {
            assert!(w[1].x >= w[0].x);
        }
    }

    // ---------- Damage / rebuild ----------

    #[test]
    fn take_damage_reduces_hp() {
        let mut c = mk();
        c.take_damage(50.0);
        assert!((c.hp - 250.0).abs() < 1e-6);
    }

    #[test]
    fn destruction_starts_rebuild_timer() {
        let mut c = mk();
        c.take_damage(400.0);
        assert!(!c.is_alive());
        assert!(c.is_rebuilding());
        assert!((c.rebuild.remaining() - CATAPULT_REBUILD_TIME).abs() < 1e-6);
    }

    #[test]
    fn destruction_uses_configured_rebuild_time() {
        let mut c = mk();
        c.configure(1000.0, 300.0, 6.0);
        c.take_damage(400.0);
        assert!((c.rebuild.remaining() - 6.0).abs() < 1e-6);
    }

    #[test]
    fn rebuild_completes_after_timer() {
        let mut c = mk();
        c.take_damage(400.0);
        c.tick(CATAPULT_REBUILD_TIME + 0.1);
        assert!(c.is_alive());
        assert!(c.rebuild.is_ready());
    }

    #[test]
    fn rebuild_uses_configured_max_hp() {
        let mut c = mk();
        c.configure(1000.0, 500.0, 6.0);
        c.take_damage(9999.0);
        c.tick(7.0);
        assert!((c.hp - 500.0).abs() < 1e-6);
    }

    #[test]
    fn hp_fraction_is_clamped() {
        let mut c = mk();
        c.hp = 600.0;
        assert!((c.hp_fraction() - 1.0).abs() < 1e-6);
        c.hp = 150.0;
        assert!((c.hp_fraction() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn rect_has_correct_position() {
        let c = mk();
        let (x, y, w, h) = c.rect();
        assert!((x - (200.0 - CATAPULT_HITBOX_W * 0.5)).abs() < 1e-6);
        assert!((y - (500.0 - CATAPULT_HITBOX_H)).abs() < 1e-6);
        assert!((w - CATAPULT_HITBOX_W).abs() < 1e-6);
        assert!((h - CATAPULT_HITBOX_H).abs() < 1e-6);
    }

    // ---------- Fire ----------

    #[test]
    fn destroyed_catapult_cannot_fire() {
        let mut c = mk();
        c.take_damage(400.0);
        assert!(c.try_fire_multi(Team::Player, 1.0).is_none());
    }

    #[test]
    fn fire_sets_cooldown() {
        let mut c = mk();
        let shots = c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert_eq!(shots.len(), 1);
        assert!((c.cooldown.remaining() - CATAPULT_FIRE_COOLDOWN).abs() < 1e-6);
    }

    #[test]
    fn fire_blocked_while_cooldown() {
        let mut c = mk();
        c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert!(c.try_fire_multi(Team::Player, 1.0).is_none());
    }

    #[test]
    fn projectiles_are_parabolic() {
        let mut c = mk();
        let shots = c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert_eq!(shots[0].gravity, CATAPULT_GRAVITY);
        assert!(shots[0].gravity > 0.0);
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
    fn fire_rate_mult_affects_cooldown() {
        let mut c = mk();
        c.fire_rate_mult = 0.5;
        c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert!((c.cooldown.remaining() - CATAPULT_FIRE_COOLDOWN * 0.5).abs() < 1e-6);
    }

    // ---------- Multi-shot ----------

    #[test]
    fn multi_shot_produces_n_projectiles() {
        let mut c = mk();
        c.multi_shot = 3;
        let shots = c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert_eq!(shots.len(), 3);
    }

    #[test]
    fn multi_shot_velocities_are_close() {
        let mut c = mk();
        c.multi_shot = 3;
        let shots = c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert!((shots[0].vel.x - shots[1].vel.x).abs() < 1e-6);
        assert!((shots[1].vel.x - shots[2].vel.x).abs() < 1e-6);
        let dy01 = (shots[0].vel.y - shots[1].vel.y).abs();
        assert!(dy01 < 100.0, "vy trop différents: {}", dy01);
    }

    #[test]
    fn multi_shot_spread_moves_targets_apart() {
        let mut c = mk();
        c.multi_shot = 3;
        let shots = c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert!(shots[2].vel.y < shots[0].vel.y);
    }

    // ---------- Shot kinds ----------

    #[test]
    fn piercing_shot_has_pierce_remaining() {
        let mut c = mk();
        c.shot_kind = ShotKind::Piercing;
        let shots = c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert_eq!(shots[0].pierce_remaining, 3);
    }

    #[test]
    fn explosive_shot_deals_more_damage() {
        let mut c = mk();
        c.shot_kind = ShotKind::Explosive;
        let shots = c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert!(shots[0].damage > CATAPULT_BASE_DAMAGE);
        assert_eq!(shots[0].kind, ProjectileKind::Explosive);
    }

    #[test]
    fn cycle_shot_kind_wraps_around() {
        let mut c = mk();
        assert_eq!(c.shot_kind, ShotKind::Basic);
        c.cycle_shot_kind();
        assert_eq!(c.shot_kind, ShotKind::Piercing);
        c.cycle_shot_kind();
        assert_eq!(c.shot_kind, ShotKind::Explosive);
        c.cycle_shot_kind();
        assert_eq!(c.shot_kind, ShotKind::Basic);
    }
}