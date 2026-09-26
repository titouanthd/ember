//! Tourelle du joueur : visée souris + multi-projectiles.

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

pub const TURRET_ANGLE_MIN: f32 = -80.0_f32.to_radians();
pub const TURRET_ANGLE_MAX: f32 = 80.0_f32.to_radians();
pub const TURRET_FIRE_COOLDOWN: f32 = 0.45;
pub const TURRET_BASE_DAMAGE: f32 = 10.0;
pub const TURRET_PROJECTILE_SPEED: f32 = 700.0;
pub const MULTI_SHOT_SPREAD_RAD: f32 = 15.0_f32.to_radians();

#[derive(Debug, Clone)]
pub struct Turret {
    pub pos: Vec2,
    pub angle: f32,
    pub cooldown: f32,
    pub shot_kind: ShotKind,
    /// Multiplicateur sur le cooldown (1.0 par défaut, <1 = plus rapide).
    pub fire_rate_mult: f32,
    /// Nombre de projectiles par tir (>= 1).
    pub multi_shot: u32,
}

impl Turret {
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos,
            angle: 0.0,
            cooldown: 0.0,
            shot_kind: ShotKind::Basic,
            fire_rate_mult: 1.0,
            multi_shot: 1,
        }
    }

    pub fn aim(&mut self, mouse_world: Vec2) {
        let delta = mouse_world - self.pos;
        if delta.length_squared() > 1e-6 {
            self.angle = delta.y.atan2(delta.x).clamp(TURRET_ANGLE_MIN, TURRET_ANGLE_MAX);
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if dt > 0.0 {
            self.cooldown = (self.cooldown - dt).max(0.0);
        }
    }

    pub fn is_ready(&self) -> bool {
        self.cooldown <= 0.0
    }

    /// Tire un projectile unique (compat ascendante).
    pub fn try_fire(&mut self, team: Team, damage_mult: f32) -> Option<Projectile> {
        self.try_fire_multi(team, damage_mult)
            .and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) })
    }

    /// Tire N projectiles en éventail, selon `multi_shot`.
    pub fn try_fire_multi(
        &mut self,
        team: Team,
        damage_mult: f32,
    ) -> Option<Vec<Projectile>> {
        if !self.is_ready() {
            return None;
        }
        let base_dir = Vec2::new(self.angle.cos(), self.angle.sin());
        let n = self.multi_shot.max(1);
        let mut shots = Vec::with_capacity(n as usize);

        for i in 0..n {
            let offset = if n == 1 {
                0.0
            } else {
                // Éventail centré : de -spread/2 à +spread/2.
                let t = (i as f32 / (n - 1) as f32) - 0.5;
                t * MULTI_SHOT_SPREAD_RAD
            };
            let dir = rotate(base_dir, offset);

            let (kind, pierce, damage_mul) = match self.shot_kind {
                ShotKind::Basic => (ProjectileKind::Basic, 0, 1.0),
                ShotKind::Piercing => (ProjectileKind::Basic, 3, 1.0),
                ShotKind::Explosive => (ProjectileKind::Explosive, 0, 1.5),
            };

            shots.push(Projectile {
                pos: self.pos,
                vel: dir * TURRET_PROJECTILE_SPEED,
                damage: TURRET_BASE_DAMAGE * damage_mult * damage_mul,
                radius: 6.0,
                team,
                kind,
                pierce_remaining: pierce,
                color: macroquad::prelude::WHITE,
            });
        }

        // Cooldown ajusté par le fire_rate_mult.
        self.cooldown = TURRET_FIRE_COOLDOWN * self.fire_rate_mult;
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

fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let c = angle.cos();
    let s = angle.sin();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_turret_points_right_and_is_ready() {
        let t = Turret::new(Vec2::ZERO);
        assert_eq!(t.angle, 0.0);
        assert!(t.is_ready());
        assert_eq!(t.shot_kind, ShotKind::Basic);
        assert_eq!(t.multi_shot, 1);
        assert!((t.fire_rate_mult - 1.0).abs() < 1e-6);
    }

    #[test]
    fn aim_toward_positive_x() {
        let mut t = Turret::new(Vec2::ZERO);
        t.aim(Vec2::new(100.0, 0.0));
        assert!(t.angle.abs() < 1e-6);
    }

    #[test]
    fn aim_clamps_upward_at_80_degrees() {
        let mut t = Turret::new(Vec2::ZERO);
        t.aim(Vec2::new(1.0, 1000.0));
        assert!((t.angle - TURRET_ANGLE_MAX).abs() < 1e-5);
    }

    #[test]
    fn aim_clamps_downward_at_minus_80_degrees() {
        let mut t = Turret::new(Vec2::ZERO);
        t.aim(Vec2::new(1.0, -1000.0));
        assert!((t.angle - TURRET_ANGLE_MIN).abs() < 1e-5);
    }

    #[test]
    fn aim_ignores_mouse_at_origin() {
        let mut t = Turret::new(Vec2::ZERO);
        t.angle = 0.5;
        t.aim(Vec2::ZERO);
        assert!((t.angle - 0.5).abs() < 1e-6);
    }

    #[test]
    fn tick_decrements_cooldown_and_clamps() {
        let mut t = Turret::new(Vec2::ZERO);
        t.cooldown = 0.2;
        t.tick(0.5);
        assert_eq!(t.cooldown, 0.0);
    }

    #[test]
    fn try_fire_blocked_when_cooldown_running() {
        let mut t = Turret::new(Vec2::ZERO);
        t.cooldown = 0.3;
        assert!(t.try_fire(Team::Player, 1.0).is_none());
    }

    #[test]
    fn try_fire_sets_cooldown() {
        let mut t = Turret::new(Vec2::new(10.0, 20.0));
        let p = t.try_fire(Team::Player, 1.0).unwrap();
        assert_eq!(p.pos, Vec2::new(10.0, 20.0));
        assert!((t.cooldown - TURRET_FIRE_COOLDOWN).abs() < 1e-6);
    }

    #[test]
    fn try_fire_applies_damage_multiplier() {
        let mut t = Turret::new(Vec2::ZERO);
        let p = t.try_fire(Team::Player, 2.0).unwrap();
        assert!((p.damage - TURRET_BASE_DAMAGE * 2.0).abs() < 1e-6);
    }

    #[test]
    fn piercing_shot_has_pierce_remaining() {
        let mut t = Turret::new(Vec2::ZERO);
        t.shot_kind = ShotKind::Piercing;
        let p = t.try_fire(Team::Player, 1.0).unwrap();
        assert_eq!(p.pierce_remaining, 3);
    }

    #[test]
    fn explosive_shot_deals_more_damage() {
        let mut t = Turret::new(Vec2::ZERO);
        t.shot_kind = ShotKind::Explosive;
        let p = t.try_fire(Team::Player, 1.0).unwrap();
        assert!(p.damage > TURRET_BASE_DAMAGE);
        assert_eq!(p.kind, ProjectileKind::Explosive);
    }

    #[test]
    fn cycle_shot_kind_wraps_around() {
        let mut t = Turret::new(Vec2::ZERO);
        assert_eq!(t.shot_kind, ShotKind::Basic);
        t.cycle_shot_kind();
        assert_eq!(t.shot_kind, ShotKind::Piercing);
        t.cycle_shot_kind();
        assert_eq!(t.shot_kind, ShotKind::Explosive);
        t.cycle_shot_kind();
        assert_eq!(t.shot_kind, ShotKind::Basic);
    }

    #[test]
    fn fire_rate_mult_affects_cooldown() {
        let mut t = Turret::new(Vec2::ZERO);
        t.fire_rate_mult = 0.5;
        t.try_fire(Team::Player, 1.0).unwrap();
        assert!((t.cooldown - TURRET_FIRE_COOLDOWN * 0.5).abs() < 1e-6);
    }

    #[test]
    fn multi_shot_produces_two_projectiles() {
        let mut t = Turret::new(Vec2::ZERO);
        t.multi_shot = 2;
        let shots = t.try_fire_multi(Team::Player, 1.0).unwrap();
        assert_eq!(shots.len(), 2);
    }

    #[test]
    fn multi_shot_spread_diverges() {
        let mut t = Turret::new(Vec2::ZERO);
        t.multi_shot = 2;
        let shots = t.try_fire_multi(Team::Player, 1.0).unwrap();
        // Les deux projectiles ne sont pas colinéaires.
        let dot = shots[0].vel.normalize().dot(shots[1].vel.normalize());
        assert!(dot < 1.0 - 1e-3, "expected spread, dot = {dot}");
    }
}