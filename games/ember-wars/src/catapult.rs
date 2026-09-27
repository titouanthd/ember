//! Catapulte — tir parabolique, structure destructible.
//!
//! Le joueur vise un **point de chute** (la souris). On résout la
//! vélocité initiale pour qu'un projectile balistique passe par ce
//! point. Portée et HP configurables via les upgrades.

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

// ---------- Constantes de base ----------

pub const CATAPULT_HP: f32 = 300.0;
pub const CATAPULT_REBUILD_TIME: f32 = 10.0;
pub const CATAPULT_OFFSET_X: f32 = 150.0;

pub const CATAPULT_MIN_RANGE: f32 = 150.0;
pub const CATAPULT_MAX_RANGE: f32 = 1000.0;

pub const CATAPULT_FIRE_COOLDOWN: f32 = 0.9;
pub const CATAPULT_BASE_DAMAGE: f32 = 14.0;

/// Vitesse horizontale fixe — détermine le temps de vol.
pub const CATAPULT_H_SPEED: f32 = 650.0;
/// Gravité appliquée aux projectiles de catapulte.
pub const CATAPULT_GRAVITY: f32 = 900.0;

/// Écart latéral entre deux tirs du multi-shot (en px).
pub const MULTI_SHOT_SPREAD_PX: f32 = 20.0;

/// Dimensions de la hitbox (AABB, base au sol).
pub const CATAPULT_HITBOX_W: f32 = 50.0;
pub const CATAPULT_HITBOX_H: f32 = 60.0;

/// Hauteur du point de lancement au-dessus du sol.
pub const LAUNCH_HEIGHT: f32 = 45.0;

#[derive(Debug, Clone)]
pub struct Catapult {
    /// Bas-centre, posé sur le sol.
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub cooldown: f32,
    pub shot_kind: ShotKind,
    pub fire_rate_mult: f32,
    pub multi_shot: u32,
    /// 0 = vivante ; > 0 = détruite, décompte avant reconstruction.
    pub rebuild_timer: f32,
    /// Temps total de reconstruction (configurable via upgrades).
    pub rebuild_time: f32,
    /// Portée max (configurable via upgrades).
    pub max_range: f32,
    /// Point de chute visé.
    pub aim_target: Vec2,
}

impl Catapult {
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos,
            hp: CATAPULT_HP,
            max_hp: CATAPULT_HP,
            cooldown: 0.0,
            shot_kind: ShotKind::Basic,
            fire_rate_mult: 1.0,
            multi_shot: 1,
            rebuild_timer: 0.0,
            rebuild_time: CATAPULT_REBUILD_TIME,
            max_range: CATAPULT_MAX_RANGE,
            aim_target: pos + Vec2::new(CATAPULT_MIN_RANGE, 0.0),
        }
    }

    /// Applique les upgrades (portée, HP, temps de reconstruction).
    /// À appeler une seule fois, à la création de la partie.
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

    /// Rect (x, y, w, h) de la hitbox.
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        (
            self.pos.x - CATAPULT_HITBOX_W * 0.5,
            self.pos.y - CATAPULT_HITBOX_H,
            CATAPULT_HITBOX_W,
            CATAPULT_HITBOX_H,
        )
    }

    /// Point de départ des projectiles (haut du bras).
    pub fn launch_origin(&self) -> Vec2 {
        self.pos - Vec2::new(0.0, LAUNCH_HEIGHT)
    }

    /// Cible le point de chute (clampé à la portée).
    pub fn aim(&mut self, mouse_world: Vec2) {
        let dx = (mouse_world.x - self.pos.x)
            .clamp(CATAPULT_MIN_RANGE, self.max_range);
        self.aim_target = Vec2::new(self.pos.x + dx, mouse_world.y);
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

    /// Résout une trajectoire balistique vers `target`. Retourne la
    /// vélocité initiale, ou `None` si la cible est derrière / trop
    /// proche.
    pub fn solve_trajectory(&self, target: Vec2) -> Option<Vec2> {
        let start = self.launch_origin();
        let dx = target.x - start.x;
        if dx <= 1.0 {
            return None;
        }
        let dy = target.y - start.y;
        let vx = CATAPULT_H_SPEED;
        let t = dx / vx;
        if t <= 1e-3 {
            return None;
        }
        let g = CATAPULT_GRAVITY;
        // y(t) = dy = vy*t + 0.5*g*t²  →  vy = (dy - 0.5*g*t²) / t
        let vy = (dy - 0.5 * g * t * t) / t;
        Some(Vec2::new(vx, vy))
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
                damage: CATAPULT_BASE_DAMAGE * damage_mult * damage_mul,
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

        self.cooldown = CATAPULT_FIRE_COOLDOWN * self.fire_rate_mult;
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

    #[test]
    fn new_catapult_is_alive_and_ready() {
        let c = mk();
        assert!(c.is_alive());
        assert!(c.is_ready());
        assert_eq!(c.hp, CATAPULT_HP);
        assert!((c.max_range - CATAPULT_MAX_RANGE).abs() < 1e-6);
        assert!((c.rebuild_time - CATAPULT_REBUILD_TIME).abs() < 1e-6);
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
    fn configure_changes_rebuild_time() {
        let mut c = mk();
        c.configure(1000.0, 300.0, 6.0);
        assert!((c.rebuild_time - 6.0).abs() < 1e-6);
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
        assert!((c.rebuild_time - 0.5).abs() < 1e-6);
    }

    #[test]
    fn aim_clamps_to_min_range() {
        let mut c = mk();
        c.aim(Vec2::new(0.0, 500.0));
        assert!((c.aim_target.x - (c.pos.x + CATAPULT_MIN_RANGE)).abs() < 1e-6);
    }

    #[test]
    fn aim_clamps_to_max_range_default() {
        let mut c = mk();
        c.aim(Vec2::new(99_999.0, 500.0));
        assert!((c.aim_target.x - (c.pos.x + CATAPULT_MAX_RANGE)).abs() < 1e-6);
    }

    #[test]
    fn aim_clamps_to_configured_range() {
        let mut c = mk();
        c.configure(1500.0, 300.0, 10.0);
        c.aim(Vec2::new(99_999.0, 500.0));
        assert!((c.aim_target.x - (c.pos.x + 1500.0)).abs() < 1e-6);
    }

    #[test]
    fn aim_keeps_target_y() {
        let mut c = mk();
        c.aim(Vec2::new(500.0, 420.0));
        assert!((c.aim_target.y - 420.0).abs() < 1e-6);
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

    #[test]
    fn take_damage_reduces_hp() {
        let mut c = mk();
        c.take_damage(50.0);
        assert!((c.hp - 250.0).abs() < 1e-6);
    }

    #[test]
    fn destruction_uses_configured_rebuild_time() {
        let mut c = mk();
        c.configure(1000.0, 300.0, 6.0);
        c.take_damage(400.0);
        assert!((c.rebuild_timer - 6.0).abs() < 1e-6);
    }

    #[test]
    fn destruction_starts_rebuild_timer() {
        let mut c = mk();
        c.take_damage(400.0);
        assert!(!c.is_alive());
        assert!(c.is_rebuilding());
        assert!((c.rebuild_timer - CATAPULT_REBUILD_TIME).abs() < 1e-6);
    }

    #[test]
    fn rebuild_completes_after_timer() {
        let mut c = mk();
        c.take_damage(400.0);
        c.tick(CATAPULT_REBUILD_TIME + 0.1);
        assert!(c.is_alive());
        assert!((c.hp - CATAPULT_HP).abs() < 1e-6);
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
        assert!((c.cooldown - CATAPULT_FIRE_COOLDOWN).abs() < 1e-6);
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

    #[test]
    fn fire_rate_mult_affects_cooldown() {
        let mut c = mk();
        c.fire_rate_mult = 0.5;
        c.try_fire_multi(Team::Player, 1.0).unwrap();
        assert!((c.cooldown - CATAPULT_FIRE_COOLDOWN * 0.5).abs() < 1e-6);
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

    #[test]
    fn preview_points_are_monotonic_in_x() {
        let c = mk();
        let pts = c.preview_points(10);
        assert!(!pts.is_empty());
        for w in pts.windows(2) {
            assert!(w[1].x >= w[0].x);
        }
    }
}