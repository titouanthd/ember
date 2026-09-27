//! IA adverse — heuristique de spawn avec waves.

use std::collections::HashMap;

use crate::components::{Team, Unit};
use crate::config::AiConfig;
use crate::mana::ManaPool;
use crate::units::{can_spawn, unit_stats};

const DEFENSIVE_ORDER: &[&str] = &["archer", "grunt", "brute"];

pub fn decide_spawn(
    ai_mana: &ManaPool,
    ai_units: &[Unit],
    player_units: &[Unit],
    ai_config: &AiConfig,
    elapsed: f32,
    cooldowns: &HashMap<String, f32>,
) -> Option<String> {
    let active = ai_config.waves.iter().rfind(|w| w.start_at <= elapsed)?;

    let ai_alive = ai_units.iter().filter(|u| u.is_alive()).count() as i32;
    let player_alive = player_units.iter().filter(|u| u.is_alive()).count() as i32;
    let defensive = ai_alive + 2 < player_alive;

    let order: Vec<&str> = if defensive {
        DEFENSIVE_ORDER
            .iter()
            .filter(|k| active.priority.iter().any(|p| p == *k))
            .copied()
            .collect()
    } else {
        active.priority.iter().map(|s| s.as_str()).collect()
    };

    if order.is_empty() {
        return None;
    }

    let aggression = ai_config.aggression.max(0.01);
    let threshold_mult = 1.0 / aggression;

    for kind in order {
        if !can_spawn(kind, Team::Enemy, ai_units) {
            continue;
        }
        let stats = match unit_stats(kind) {
            Some(s) => s,
            None => continue,
        };
        if ai_mana.current < stats.cost * threshold_mult {
            continue;
        }
        let cd = cooldowns.get(kind).copied().unwrap_or(0.0);
        if cd > 0.0 {
            continue;
        }
        return Some(kind.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::Vec2;

    use crate::components::UnitId;
    use crate::config::AiWave;

    fn ai_simple() -> AiConfig {
        AiConfig {
            aggression: 1.0,
            mana_regen_mult: 1.0,
            waves: vec![AiWave {
                start_at: 0.0,
                priority: vec![
                    "hero".into(),
                    "brute".into(),
                    "archer".into(),
                    "bomber".into(),
                    "grunt".into(),
                ],
            }],
        }
    }

    fn ai_two_waves() -> AiConfig {
        AiConfig {
            aggression: 1.0,
            mana_regen_mult: 1.0,
            waves: vec![
                AiWave {
                    start_at: 0.0,
                    priority: vec!["grunt".into()],
                },
                AiWave {
                    start_at: 20.0,
                    priority: vec!["brute".into(), "grunt".into()],
                },
            ],
        }
    }

    fn ai_aggressive() -> AiConfig {
        AiConfig {
            aggression: 1.4,
            mana_regen_mult: 1.35,
            waves: vec![AiWave {
                start_at: 0.0,
                priority: vec![
                    "hero".into(),
                    "brute".into(),
                    "archer".into(),
                    "bomber".into(),
                    "grunt".into(),
                ],
            }],
        }
    }

    fn mk_unit(id: u32, kind: &str, team: Team) -> Unit {
        let stats = unit_stats(kind).expect("kind exists");
        Unit {
            id: UnitId(id),
            kind: kind.to_owned(),
            team,
            pos: Vec2::ZERO,
            hp: stats.hp,
            max_hp: stats.hp,
            damage: stats.damage,
            attack_cd: 0.0,
            attack_cd_max: stats.attack_cooldown,
            heal_cd: 0.0,
            hit_flash: 0.0,
            pose_phase: 0.0,
            target: None,
            pending_attack: None,
        }
    }

    fn no_cooldowns() -> HashMap<String, f32> {
        HashMap::new()
    }

    #[test]
    fn no_spawn_when_no_mana() {
        let mana = ManaPool::new(0.0, 100.0, 10.0);
        assert!(decide_spawn(&mana, &[], &[], &ai_simple(), 0.0, &no_cooldowns()).is_none());
    }

    #[test]
    fn spawns_cheapest_when_only_grunt_affordable() {
        let mana = ManaPool::new(20.0, 100.0, 10.0);
        let c = decide_spawn(&mana, &[], &[], &ai_simple(), 0.0, &no_cooldowns());
        assert_eq!(c.as_deref(), Some("grunt"));
    }

    #[test]
    fn prefers_hero_when_affordable() {
        let mana = ManaPool::new(100.0, 100.0, 10.0);
        let c = decide_spawn(&mana, &[], &[], &ai_simple(), 0.0, &no_cooldowns());
        assert_eq!(c.as_deref(), Some("hero"));
    }

    #[test]
    fn wave_1_restricts_to_grunt_even_with_mana() {
        let mana = ManaPool::new(100.0, 100.0, 10.0);
        let c = decide_spawn(&mana, &[], &[], &ai_two_waves(), 5.0, &no_cooldowns());
        assert_eq!(c.as_deref(), Some("grunt"));
    }

    #[test]
    fn wave_2_allows_brute_after_start_at() {
        let mana = ManaPool::new(50.0, 100.0, 10.0);
        let before = decide_spawn(&mana, &[], &[], &ai_two_waves(), 10.0, &no_cooldowns());
        assert_eq!(before.as_deref(), Some("grunt"));
        let after = decide_spawn(&mana, &[], &[], &ai_two_waves(), 25.0, &no_cooldowns());
        assert_eq!(after.as_deref(), Some("brute"));
    }

    #[test]
    fn defensive_filters_to_archer_grunt_brute() {
        let ai = AiConfig {
            aggression: 1.0,
            mana_regen_mult: 1.0,
            waves: vec![AiWave {
                start_at: 0.0,
                priority: vec!["bomber".into(), "archer".into(), "grunt".into()],
            }],
        };
        let player_units = vec![
            mk_unit(0, "grunt", Team::Player),
            mk_unit(1, "grunt", Team::Player),
            mk_unit(2, "grunt", Team::Player),
        ];
        let mana = ManaPool::new(100.0, 100.0, 10.0);
        let c = decide_spawn(&mana, &[], &player_units, &ai, 0.0, &no_cooldowns());
        assert_eq!(c.as_deref(), Some("archer"));
    }

    #[test]
    fn aggressive_threshold_is_lower() {
        let mana = ManaPool::new(30.0, 100.0, 10.0);
        let normal = decide_spawn(&mana, &[], &[], &ai_simple(), 0.0, &no_cooldowns());
        let hard = decide_spawn(&mana, &[], &[], &ai_aggressive(), 0.0, &no_cooldowns());
        assert!(normal.is_some());
        assert!(hard.is_some());
        assert_eq!(hard.as_deref(), Some("brute"));
        assert_ne!(normal.as_deref(), Some("brute"));
    }

    #[test]
    fn skips_kind_with_running_cooldown() {
        let mut cds = HashMap::new();
        cds.insert("hero".to_string(), 10.0);
        cds.insert("brute".to_string(), 5.0);
        let mana = ManaPool::new(100.0, 100.0, 10.0);
        let c = decide_spawn(&mana, &[], &[], &ai_simple(), 0.0, &cds);
        assert_eq!(c.as_deref(), Some("archer"));
    }

    #[test]
    fn no_spawn_before_first_wave() {
        let ai = AiConfig {
            aggression: 1.0,
            mana_regen_mult: 1.0,
            waves: vec![AiWave {
                start_at: 10.0,
                priority: vec!["grunt".into()],
            }],
        };
        let mana = ManaPool::new(100.0, 100.0, 10.0);
        assert!(decide_spawn(&mana, &[], &[], &ai, 5.0, &no_cooldowns()).is_none());
    }

    #[test]
    fn skips_hero_when_one_already_alive() {
        let hero = mk_unit(0, "hero", Team::Enemy);
        let ai_units = vec![hero];
        let mana = ManaPool::new(100.0, 100.0, 10.0);
        let c = decide_spawn(&mana, &ai_units, &[], &ai_simple(), 0.0, &no_cooldowns());
        assert_eq!(c.as_deref(), Some("brute"));
    }
}