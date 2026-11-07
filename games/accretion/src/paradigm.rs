//! Paradigm progression: conditions + advance.
//!
//! Advance is **explicit** (the player clicks Advance when ready).
//! Conditions are evaluated against the current state; the advance
//! action is a state mutation.
//!
//! "Planets" in DESIGN.md §9.4 is interpreted as `WaterCondenser`
//! (the closest building representing planets forming) — asserted in
//! tests so it can't silently drift.

use std::collections::HashMap;
use crate::format::format_number;
use crate::components::{BuildingId, Paradigm, ResourceId};
use crate::economy::BuildingInstance;

/// One paradigm transition rule.
pub struct Condition {
    /// The paradigm this condition unlocks.
    pub target: Paradigm,
    /// Minimum count of each required building.
    pub required_buildings: &'static [(BuildingId, u32)],
    /// Minimum *held* amount of each required resource.
    pub required_resources: &'static [(ResourceId, f64)],
}

/// The five transitions. Ordering matters: index 0 is the Dust →
/// Pebbles transition, index 4 is Gas Giant → Star. `Star → Endgame`
/// is a separate explicit choice handled by `systems::Game::try_endgame`.
pub const CONDITIONS: &[Condition] = &[
    Condition {
        target: Paradigm::Pebbles,
        required_buildings: &[],
        required_resources: &[(ResourceId::Dust, 100.0)],
    },
    Condition {
        target: Paradigm::Planetesimals,
        required_buildings: &[(BuildingId::DustCollector, 5)],
        required_resources: &[(ResourceId::Pebbles, 500.0)],
    },
    // Planetesimals → Planets : 2000 Pebbles → 1500
    Condition {
        target: Paradigm::Planets,
        required_buildings: &[(BuildingId::ResonanceEngine, 1)],
        required_resources: &[(ResourceId::Pebbles, 1500.0)],  // was 2000
    },
    // Planets → GasGiant : ajouter 500 Water
    Condition {
        target: Paradigm::GasGiant,
        required_buildings: &[
            (BuildingId::MetalFoundry, 5),
            (BuildingId::WaterCondenser, 3),
        ],
        required_resources: &[(ResourceId::Water, 500.0)],  // NEW
    },
    Condition {
        target: Paradigm::Star,
        required_buildings: &[(BuildingId::GasScoop, 10)],
        required_resources: &[(ResourceId::Gas, 10_000.0)],
    },
];

/// The condition that would advance *from* `current`. `None` when the
/// player is at the last paradigm.
pub fn next_condition(current: Paradigm) -> Option<&'static Condition> {
    let next = current.next()?;
    CONDITIONS.iter().find(|c| c.target == next)
}

/// Whether a condition is satisfied by the given state.
pub fn condition_met(
    cond: &Condition,
    buildings: &[BuildingInstance],
    resources: &HashMap<ResourceId, f64>,
) -> bool {
    for (id, needed) in cond.required_buildings {
        let owned = buildings
            .iter()
            .find(|b| b.id == *id)
            .map(|b| b.count)
            .unwrap_or(0);
        if owned < *needed {
            return false;
        }
    }
    for (r, needed) in cond.required_resources {
        let have = resources.get(r).copied().unwrap_or(0.0);
        if have < *needed {
            return false;
        }
    }
    true
}

/// Return the target paradigm if `current` can advance, else `None`.
pub fn check_advance(
    current: Paradigm,
    buildings: &[BuildingInstance],
    resources: &HashMap<ResourceId, f64>,
) -> Option<Paradigm> {
    let cond = next_condition(current)?;
    if condition_met(cond, buildings, resources) {
        Some(cond.target)
    } else {
        None
    }
}

pub fn progress_lines(
    cond: &Condition,
    buildings: &[BuildingInstance],
    resources: &HashMap<ResourceId, f64>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (id, needed) in cond.required_buildings {
        let owned = buildings
            .iter()
            .find(|b| b.id == *id)
            .map(|b| b.count)
            .unwrap_or(0);
        let name = crate::economy::id_to_def(*id).name;
        out.push(format!("{name}: {owned} / {needed}"));
    }
    for (r, needed) in cond.required_resources {
        let have = resources.get(r).copied().unwrap_or(0.0);
        out.push(format!("{}: {} / {}", r.name(), format_number(have), format_number(*needed)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::{BUILDINGS, BuildingInstance};

    fn empty_buildings() -> Vec<BuildingInstance> {
        BUILDINGS.iter().map(|d| BuildingInstance::new(d.id)).collect()
    }

    fn with_buildings(pairs: &[(BuildingId, u32)]) -> Vec<BuildingInstance> {
        let mut v = empty_buildings();
        for (id, n) in pairs {
            v.iter_mut().find(|b| b.id == *id).unwrap().count = *n;
        }
        v
    }

    fn res(pairs: &[(ResourceId, f64)]) -> HashMap<ResourceId, f64> {
        let mut m: HashMap<_, _> = ResourceId::ALL.iter().map(|r| (*r, 0.0)).collect();
        for (r, v) in pairs {
            m.insert(*r, *v);
        }
        m
    }

    #[test]
    fn conditions_table_has_five_entries() {
        assert_eq!(CONDITIONS.len(), 5);
    }

    #[test]
    fn conditions_chain_with_paradigm_next() {
        let mut p = Paradigm::Dust;
        for cond in CONDITIONS {
            let expected = p.next().expect("chain end");
            assert_eq!(cond.target, expected);
            p = cond.target;
        }
        assert_eq!(p, Paradigm::Star);
    }

    #[test]
    fn next_condition_from_dust_is_pebbles() {
        let c = next_condition(Paradigm::Dust).unwrap();
        assert_eq!(c.target, Paradigm::Pebbles);
    }

    #[test]
    fn next_condition_from_star_is_none() {
        assert!(next_condition(Paradigm::Star).is_none());
    }

    #[test]
    fn planets_transition_requires_water_condensers() {
        // Asserts the design interpretation: "3 Planets" → WaterCondenser.
        let c = next_condition(Paradigm::Planets).unwrap();
        assert_eq!(c.target, Paradigm::GasGiant);
        let has_water = c
            .required_buildings
            .iter()
            .any(|(id, n)| *id == BuildingId::WaterCondenser && *n == 3);
        assert!(has_water);
    }

    #[test]
    fn dust_transition_not_met_initially() {
        assert!(!condition_met(
            next_condition(Paradigm::Dust).unwrap(),
            &empty_buildings(),
            &res(&[(ResourceId::Dust, 0.0)]),
        ));
    }

    #[test]
    fn dust_transition_met_with_100_dust() {
        assert!(condition_met(
            next_condition(Paradigm::Dust).unwrap(),
            &empty_buildings(),
            &res(&[(ResourceId::Dust, 100.0)]),
        ));
    }

    #[test]
    fn pebbles_transition_needs_buildings_and_resources() {
        let cond = next_condition(Paradigm::Pebbles).unwrap();
        let only_res = res(&[(ResourceId::Pebbles, 500.0)]);
        let only_b = with_buildings(&[(BuildingId::DustCollector, 5)]);

        assert!(!condition_met(cond, &empty_buildings(), &only_res));
        assert!(!condition_met(cond, &only_b, &res(&[(ResourceId::Pebbles, 0.0)])));
        assert!(condition_met(cond, &only_b, &only_res));
    }

    #[test]
    fn building_requirement_is_lower_bound() {
        let cond = next_condition(Paradigm::Pebbles).unwrap();
        let b = with_buildings(&[(BuildingId::DustCollector, 10)]);
        assert!(condition_met(cond, &b, &res(&[(ResourceId::Pebbles, 500.0)])));
    }

    #[test]
    fn resource_requirement_is_lower_bound() {
        let cond = next_condition(Paradigm::Dust).unwrap();
        assert!(condition_met(
            cond,
            &empty_buildings(),
            &res(&[(ResourceId::Dust, 150.0)]),
        ));
    }

    #[test]
    fn check_advance_returns_none_when_not_met() {
        assert!(check_advance(
            Paradigm::Dust,
            &empty_buildings(),
            &res(&[(ResourceId::Dust, 0.0)]),
        )
        .is_none());
    }

    #[test]
    fn check_advance_returns_target_when_met() {
        assert_eq!(
            check_advance(
                Paradigm::Dust,
                &empty_buildings(),
                &res(&[(ResourceId::Dust, 100.0)]),
            ),
            Some(Paradigm::Pebbles),
        );
    }

    #[test]
    fn check_advance_at_star_returns_none() {
        assert!(check_advance(
            Paradigm::Star,
            &empty_buildings(),
            &res(&[]),
        )
        .is_none());
    }

    #[test]
    fn progress_lines_lists_all_requirements() {
        let cond = next_condition(Paradigm::Pebbles).unwrap();
        let lines = progress_lines(
            cond,
            &with_buildings(&[(BuildingId::DustCollector, 2)]),
            &res(&[(ResourceId::Pebbles, 100.0)]),
        );
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().any(|l| l.contains("Dust Collector: 2 / 5")));
        assert!(lines.iter().any(|l| l.contains("Pebbles: 100 / 500")));
    }
}