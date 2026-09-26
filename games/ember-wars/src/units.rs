//! Chargement des stats d'unités (`assets/units.ron`) et spawn.

use std::path::Path;
use std::sync::OnceLock;

use ember_core::io::load_from_file;
use glam::Vec2;
use macroquad::prelude::Color;
use serde::Deserialize;

use crate::components::{rgba, Shape, Team, Unit, UnitIdGen};

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectileStats {
    pub speed: f32,
    pub damage: f32,
    pub radius: f32,
    pub color: (f32, f32, f32, f32),
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnitStats {
    pub id: String,
    pub name: String,
    pub cost: f32,
    pub cooldown: f32,
    pub hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub shape: Shape,
    pub color: (f32, f32, f32, f32),
    pub size: (f32, f32),

    #[serde(default)]
    pub projectile: Option<ProjectileStats>,
    #[serde(default)]
    pub heal: Option<f32>,
    #[serde(default)]
    pub heal_range: Option<f32>,
    #[serde(default)]
    pub heal_cooldown: Option<f32>,
    #[serde(default)]
    pub suicide: bool,
    #[serde(default)]
    pub explosion_radius: Option<f32>,
    #[serde(default)]
    pub max_alive: Option<u32>,
}

impl UnitStats {
    pub fn color(&self) -> Color {
        rgba(self.color)
    }

    pub fn size_vec(&self) -> Vec2 {
        Vec2::new(self.size.0, self.size.1)
    }

    pub fn is_healer(&self) -> bool {
        self.heal.is_some()
    }

    pub fn is_ranged(&self) -> bool {
        self.projectile.is_some()
    }

    pub fn is_suicide(&self) -> bool {
        self.suicide
    }
}

static UNITS: OnceLock<Vec<UnitStats>> = OnceLock::new();

fn units() -> &'static Vec<UnitStats> {
    UNITS.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("units.ron");
        load_from_file(&path).expect("assets/units.ron should load and parse")
    })
}

pub fn all_units() -> &'static [UnitStats] {
    units().as_slice()
}

pub fn unit_stats(id: &str) -> Option<&'static UnitStats> {
    units().iter().find(|u| u.id == id)
}

pub fn spawn_unit(
    id_gen: &mut UnitIdGen,
    kind: &str,
    team: Team,
    pos: Vec2,
    hp_mult: f32,
    damage_mult: f32,
) -> Option<Unit> {
    let stats = unit_stats(kind)?;
    let max_hp = stats.hp * hp_mult;
    Some(Unit {
        id: id_gen.next_id(),
        kind: kind.to_owned(),
        team,
        pos,
        hp: max_hp,
        max_hp,
        damage: stats.damage * damage_mult,
        attack_cd: 0.0,
        heal_cd: 0.0,
        hit_flash: 0.0,
        target: None,
    })
}

pub fn alive_count(kind: &str, team: Team, units: &[Unit]) -> u32 {
    units
        .iter()
        .filter(|u| u.kind == kind && u.team == team && u.is_alive())
        .count() as u32
}

pub fn can_spawn(kind: &str, team: Team, units: &[Unit]) -> bool {
    let stats = match unit_stats(kind) {
        Some(s) => s,
        None => return false,
    };
    match stats.max_alive {
        Some(max) => alive_count(kind, team, units) < max,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_ron_loads_six_entries() {
        assert_eq!(all_units().len(), 6);
    }

    #[test]
    fn unit_stats_lookup_by_id() {
        assert!(unit_stats("grunt").is_some());
        assert!(unit_stats("hero").is_some());
        assert!(unit_stats("nope").is_none());
    }

    #[test]
    fn all_ids_are_unique() {
        let mut ids: Vec<&str> = all_units().iter().map(|u| u.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before);
    }

    #[test]
    fn all_units_have_positive_hp() {
        for u in all_units() {
            assert!(u.hp > 0.0);
        }
    }

    #[test]
    fn spawn_unit_returns_none_for_unknown_kind() {
        let mut g = UnitIdGen::new();
        assert!(spawn_unit(&mut g, "nope", Team::Player, Vec2::ZERO, 1.0, 1.0).is_none());
    }

    #[test]
    fn spawn_unit_assigns_unique_monotonic_ids() {
        let mut g = UnitIdGen::new();
        let a = spawn_unit(&mut g, "grunt", Team::Player, Vec2::ZERO, 1.0, 1.0).unwrap();
        let b = spawn_unit(&mut g, "grunt", Team::Player, Vec2::ZERO, 1.0, 1.0).unwrap();
        assert_ne!(a.id, b.id);
        assert!(a.id < b.id);
    }

    #[test]
    fn spawn_unit_starts_at_full_hp() {
        let mut g = UnitIdGen::new();
        let u = spawn_unit(&mut g, "brute", Team::Player, Vec2::ZERO, 1.0, 1.0).unwrap();
        assert_eq!(u.hp, u.max_hp);
        assert!((u.hp - 120.0).abs() < 1e-6);
    }

    #[test]
    fn spawn_unit_applies_hp_mult() {
        let mut g = UnitIdGen::new();
        let u = spawn_unit(&mut g, "brute", Team::Player, Vec2::ZERO, 1.5, 1.0).unwrap();
        assert!((u.max_hp - 180.0).abs() < 1e-6);
    }

    #[test]
    fn spawn_unit_applies_damage_mult() {
        let mut g = UnitIdGen::new();
        let u = spawn_unit(&mut g, "grunt", Team::Player, Vec2::ZERO, 1.0, 2.0).unwrap();
        assert!((u.damage - 10.0).abs() < 1e-6);
    }

    #[test]
    fn spawn_unit_preserves_team_and_kind() {
        let mut g = UnitIdGen::new();
        let u = spawn_unit(&mut g, "archer", Team::Enemy, Vec2::ZERO, 1.0, 1.0).unwrap();
        assert_eq!(u.team, Team::Enemy);
        assert_eq!(u.kind, "archer");
    }

    #[test]
    fn spawn_unit_preserves_position() {
        let mut g = UnitIdGen::new();
        let p = Vec2::new(123.0, 456.0);
        let u = spawn_unit(&mut g, "grunt", Team::Player, p, 1.0, 1.0).unwrap();
        assert_eq!(u.pos, p);
    }

    #[test]
    fn spawn_unit_has_no_target_and_zero_cooldowns() {
        let mut g = UnitIdGen::new();
        let u = spawn_unit(&mut g, "grunt", Team::Player, Vec2::ZERO, 1.0, 1.0).unwrap();
        assert!(u.target.is_none());
        assert_eq!(u.attack_cd, 0.0);
        assert_eq!(u.heal_cd, 0.0);
        assert_eq!(u.hit_flash, 0.0);
    }

    #[test]
    fn archer_is_ranged_and_not_healer() {
        let s = unit_stats("archer").unwrap();
        assert!(s.is_ranged());
        assert!(!s.is_healer());
        assert!(!s.is_suicide());
    }

    #[test]
    fn healer_has_heal_fields() {
        let s = unit_stats("healer").unwrap();
        assert!(s.is_healer());
        assert!(s.heal.unwrap() > 0.0);
        assert!(s.heal_range.unwrap() > 0.0);
        assert!(s.heal_cooldown.unwrap() > 0.0);
    }

    #[test]
    fn bomber_is_suicide() {
        let s = unit_stats("bomber").unwrap();
        assert!(s.is_suicide());
        assert!(s.explosion_radius.is_some());
    }

    #[test]
    fn hero_has_max_alive_one() {
        let s = unit_stats("hero").unwrap();
        assert_eq!(s.max_alive, Some(1));
    }

    #[test]
    fn can_spawn_respects_max_alive_per_team() {
        let mut g = UnitIdGen::new();
        let mut units = Vec::new();
        assert!(can_spawn("hero", Team::Player, &units));

        let hero = spawn_unit(&mut g, "hero", Team::Player, Vec2::ZERO, 1.0, 1.0).unwrap();
        units.push(hero);

        assert!(!can_spawn("hero", Team::Player, &units));
        assert!(can_spawn("hero", Team::Enemy, &units));
    }

    #[test]
    fn alive_count_filters_by_team_and_kind() {
        let mut id_gen = UnitIdGen::new();
        let grunt_p1 =
            spawn_unit(&mut id_gen, "grunt", Team::Player, Vec2::ZERO, 1.0, 1.0).unwrap();
        let grunt_p2 =
            spawn_unit(&mut id_gen, "grunt", Team::Player, Vec2::ZERO, 1.0, 1.0).unwrap();
        let grunt_e =
            spawn_unit(&mut id_gen, "grunt", Team::Enemy, Vec2::ZERO, 1.0, 1.0).unwrap();
        let brute_p =
            spawn_unit(&mut id_gen, "brute", Team::Player, Vec2::ZERO, 1.0, 1.0).unwrap();
        let units = vec![grunt_p1, grunt_p2, grunt_e, brute_p];

        assert_eq!(alive_count("grunt", Team::Player, &units), 2);
        assert_eq!(alive_count("grunt", Team::Enemy, &units), 1);
        assert_eq!(alive_count("brute", Team::Player, &units), 1);
        assert_eq!(alive_count("hero", Team::Player, &units), 0);
    }
}