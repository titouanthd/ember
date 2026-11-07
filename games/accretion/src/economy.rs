//! Production, costs, and building ticks.
//!
//! All functions are pure (take `&mut` to state, return produced
//! amounts) so they're testable headless — no `macroquad` context
//! required.

use std::collections::HashMap;

use ember_core::time::Cooldown;
use serde::{Deserialize, Serialize};

use crate::components::{BuildingId, Paradigm, ResourceId};

/// A building's static definition. Loaded from `assets/buildings.ron`
/// in a later session; hardcoded for now.
#[derive(Debug, Clone, Copy)]
pub struct BuildingDef {
    pub id: BuildingId,
    pub name: &'static str,
    pub paradigm: Paradigm,
    /// Base cost. Each entry scales by `1.15^count`.
    pub base_cost: &'static [(ResourceId, f64)],
    /// Production per tick. `None` for passive buildings.
    pub production: Option<(ResourceId, f64)>,
    /// Tick duration in seconds. `0.0` for passive buildings.
    pub cooldown: f32,
    /// Passive effect applied on every production tick.
    pub passive_effect: Option<PassiveEffect>,
    pub description: &'static str,
}

/// The effect of a passive building, applied multiplicatively per
/// instance.
#[derive(Debug, Clone, Copy)]
pub enum PassiveEffect {
    /// Multiply a single resource's production by `value^count`.
    MultiplyResource(ResourceId, f64),
    /// Multiply all resources' production by `value^count`.
    MultiplyAll(f64),
}

/// Base costs use `Dust` for the two first producers so the
/// Pebbles → Planetesimals transition is self-consistent.
/// See the note at the top of `DESIGN.md` (Session 12 addendum).
pub const BUILDINGS: &[BuildingDef] = &[
    BuildingDef {
        id: BuildingId::DustCollector,
        name: "Dust Collector",
        paradigm: Paradigm::Pebbles,
        base_cost: &[(ResourceId::Dust, 10.0)],
        production: Some((ResourceId::Dust, 1.0)),
        cooldown: 1.0,
        passive_effect: None,
        description: "Attracts dust from the surrounding disk.",
    },
    BuildingDef {
        id: BuildingId::AccretionRing,
        name: "Accretion Ring",
        paradigm: Paradigm::Pebbles,
        base_cost: &[(ResourceId::Pebbles, 50.0)],
        production: None,
        cooldown: 0.0,
        passive_effect: Some(PassiveEffect::MultiplyResource(ResourceId::Dust, 1.10)),
        description: "Channels the disk's rotation to enrich dust capture.",
    },
    BuildingDef {
        id: BuildingId::PebbleFoundry,
        name: "Pebble Foundry",
        paradigm: Paradigm::Pebbles,
        base_cost: &[(ResourceId::Dust, 100.0)],
        production: Some((ResourceId::Pebbles, 1.0)),
        cooldown: 1.0,
        passive_effect: None,
        description: "Agglomerates dust into pebbles.",
    },
    BuildingDef {
        id: BuildingId::ResonanceEngine,
        name: "Resonance Engine",
        paradigm: Paradigm::Planetesimals,
        base_cost: &[(ResourceId::Pebbles, 500.0)],
        production: Some((ResourceId::Planetesimals, 5.0)),
        cooldown: 2.0,
        passive_effect: None,
        description: "Forces orbital collisions that fuse pebbles.",
    },
    BuildingDef {
        id: BuildingId::MetalFoundry,
        name: "Metal Foundry",
        paradigm: Paradigm::Planets,
        base_cost: &[(ResourceId::Planetesimals, 200.0)],
        production: Some((ResourceId::Metals, 1.0)),
        cooldown: 2.0,
        passive_effect: None,
        description: "Extracts metals from planetesimals.",
    },
    BuildingDef {
        id: BuildingId::WaterCondenser,
        name: "Water Condenser",
        paradigm: Paradigm::Planets,
        base_cost: &[(ResourceId::Planetesimals, 300.0)],
        production: Some((ResourceId::Water, 1.0)),
        cooldown: 2.5,
        passive_effect: None,
        description: "Condenses water on nascent worlds.",
    },
    BuildingDef {
        id: BuildingId::OrbitalAnchor,
        name: "Orbital Anchor",
        paradigm: Paradigm::Planets,
        base_cost: &[(ResourceId::Planetesimals, 1000.0)],
        production: None,
        cooldown: 0.0,
        passive_effect: Some(PassiveEffect::MultiplyAll(1.20)),
        description: "Stabilizes orbits, boosting all production.",
    },
    BuildingDef {
        id: BuildingId::GasScoop,
        name: "Gas Scoop",
        paradigm: Paradigm::GasGiant,
        base_cost: &[(ResourceId::Metals, 500.0)],
        production: Some((ResourceId::Gas, 1.0)),
        cooldown: 2.0,
        passive_effect: None,
        description: "Captures hydrogen and helium from the giant.",
    },
    BuildingDef {
        id: BuildingId::MagnetosphereCoil,
        name: "Magnetosphere Coil",
        paradigm: Paradigm::GasGiant,
        base_cost: &[(ResourceId::Metals, 2000.0), (ResourceId::Water, 500.0)],
        production: None,
        cooldown: 0.0,
        passive_effect: Some(PassiveEffect::MultiplyResource(ResourceId::Gas, 1.30)),
        description: "Amplifies gas capture with a magnetic sheath.",
    },
    BuildingDef {
        id: BuildingId::FusionCatalyst,
        name: "Fusion Catalyst",
        paradigm: Paradigm::Star,
        base_cost: &[(ResourceId::Gas, 5000.0), (ResourceId::Metals, 2000.0)],
        production: Some((ResourceId::Light, 1.0)),
        cooldown: 3.0,
        passive_effect: None,
        description: "Ignites fusion, producing light.",
    },
    BuildingDef {
        id: BuildingId::JetInjector,
        name: "Jet Injector",
        paradigm: Paradigm::Star,
        base_cost: &[(ResourceId::Light, 500.0)],   // was 10,000
        production: None,
        cooldown: 0.0,
        passive_effect: Some(PassiveEffect::MultiplyResource(ResourceId::Light, 1.50)),
        description: "Focuses the stellar jet, multiplying light.",
    },
    BuildingDef {
        id: BuildingId::VoidWeaver,
        name: "Void Weaver",
        paradigm: Paradigm::Star,
        base_cost: &[(ResourceId::Light, 50_000.0)],
        production: Some((ResourceId::DarkMatter, 1.0)),
        cooldown: 5.0,
        passive_effect: None,
        description: "Weaves light into dark matter.",
    },
];

/// Look up a building's definition by id. Safe because `BUILDINGS` is
/// asserted (in tests) to be sorted by discriminant and 12 entries long.
pub fn id_to_def(id: BuildingId) -> &'static BuildingDef {
    &BUILDINGS[id as usize]
}

/// Cost of buying the `count`-th instance (0-indexed) of a building.
/// Uses Cookie Clicker scaling: `base * 1.15^count` per resource.
pub fn cost_of(def: &BuildingDef, count: u32) -> Vec<(ResourceId, f64)> {
    let scale = 1.15_f64.powi(count as i32);
    def.base_cost
        .iter()
        .map(|(r, v)| (*r, v * scale))
        .collect()
}

/// Whether the player can afford the next instance of a building.
pub fn can_afford(
    resources: &HashMap<ResourceId, f64>,
    def: &BuildingDef,
    count: u32,
) -> bool {
    cost_of(def, count)
        .iter()
        .all(|(r, c)| resources.get(r).copied().unwrap_or(0.0) >= *c)
}

/// A live building owned by the player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingInstance {
    pub id: BuildingId,
    pub count: u32,
    /// One shared cooldown, ticked once per `dt`. `count` multiplies
    /// the per-fire production. This is the modern incremental
    /// convention (Cookie Clicker, Adventure Capitalist).
    pub cooldown: Cooldown,
}

impl BuildingInstance {
    pub fn new(id: BuildingId) -> Self {
        let def = id_to_def(id);
        Self {
            id,
            count: 0,
            cooldown: if def.production.is_some() {
                Cooldown::new(def.cooldown)
            } else {
                Cooldown::default()
            },
        }
    }
}

/// Attempt to buy the next instance. Returns `true` on success.
///
/// On failure, the state is unchanged (no partial purchases).
pub fn buy(
    resources: &mut HashMap<ResourceId, f64>,
    instance: &mut BuildingInstance,
) -> bool {
    let def = id_to_def(instance.id);
    if !can_afford(resources, def, instance.count) {
        return false;
    }
    for (r, c) in cost_of(def, instance.count) {
        if let Some(v) = resources.get_mut(&r) {
            *v -= c;
        }
    }
    instance.count += 1;
    true
}

/// Compute per-resource production multipliers from passive buildings.
/// Seeds every resource with `g_multiplier` (prestige bonus), then
/// applies each passive effect per instance.
pub fn compute_multipliers(
    instances: &[BuildingInstance],
    g_multiplier: f64,
) -> HashMap<ResourceId, f64> {
    let mut mults: HashMap<ResourceId, f64> = ResourceId::ALL
        .iter()
        .map(|r| (*r, g_multiplier))
        .collect();

    for inst in instances {
        let def = id_to_def(inst.id);
        let Some(effect) = &def.passive_effect else { continue };
        let (value, target) = match effect {
            PassiveEffect::MultiplyResource(r, v) => (*v, Some(*r)),
            PassiveEffect::MultiplyAll(v) => (*v, None),
        };
        let scale = value.powi(inst.count as i32);
        match target {
            Some(r) => {
                if let Some(v) = mults.get_mut(&r) {
                    *v *= scale;
                }
            }
            None => {
                for v in mults.values_mut() {
                    *v *= scale;
                }
            }
        }
    }
    mults
}

pub fn tick_buildings(
    instances: &mut [BuildingInstance],
    resources: &mut HashMap<ResourceId, f64>,
    multipliers: &HashMap<ResourceId, f64>,
    dt: f32,
) -> Vec<(ResourceId, f64)> {
    const MAX_FIRINGS_PER_FRAME: u32 = 10_000;

    let mut produced = Vec::new();
    for inst in instances {
        let def = id_to_def(inst.id);
        let Some((resource, amount)) = &def.production else { continue };
        if inst.count == 0 {
            continue;
        }

        let mut remaining_dt = dt;
        let mut firings: u32 = 0;
        loop {
            if inst.cooldown.is_ready() {
                firings += 1;
                inst.cooldown.trigger(def.cooldown);
                if firings >= MAX_FIRINGS_PER_FRAME {
                    break;
                }
                if remaining_dt <= 0.0 {
                    break;
                }
                continue;
            }
            if remaining_dt <= 0.0 {
                break;
            }
            let to_next = inst.cooldown.remaining();
            if to_next <= 0.0 {
                break;
            }
            if remaining_dt >= to_next {
                inst.cooldown.tick(to_next);
                remaining_dt -= to_next;
            } else {
                inst.cooldown.tick(remaining_dt);
                remaining_dt = 0.0;
            }
        }

        if firings == 0 {
            continue;
        }
        let mult = multipliers.get(resource).copied().unwrap_or(1.0);
        let total = *amount * firings as f64 * inst.count as f64 * mult;
        *resources.entry(*resource).or_insert(0.0) += total;
        produced.push((*resource, total));
    }
    produced
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── BuildingDef table ────────────────────────────────────────────────

    #[test]
    fn buildings_table_has_twelve_entries() {
        assert_eq!(BUILDINGS.len(), 12);
    }

    #[test]
    fn buildings_table_sorted_by_discriminant() {
        for (i, def) in BUILDINGS.iter().enumerate() {
            assert_eq!(def.id as usize, i, "building at index {i} has mismatched id");
        }
    }

    #[test]
    fn building_all_matches_table_order() {
        assert_eq!(BuildingId::ALL.len(), BUILDINGS.len());
        for (i, id) in BuildingId::ALL.iter().enumerate() {
            assert_eq!(BUILDINGS[i].id, *id);
        }
    }

    #[test]
    fn active_buildings_have_cooldown_and_production() {
        for def in BUILDINGS {
            if def.production.is_some() {
                assert!(def.cooldown > 0.0, "{} has no cooldown", def.name);
                assert!(def.passive_effect.is_none(), "{} mixes active+passive", def.name);
            }
        }
    }

    #[test]
    fn passive_buildings_have_no_cooldown_or_production() {
        for def in BUILDINGS {
            if def.passive_effect.is_some() {
                assert!(def.cooldown == 0.0, "{} has a cooldown", def.name);
                assert!(def.production.is_none(), "{} produces directly", def.name);
            }
        }
    }

    #[test]
    fn dust_collector_is_priced_in_dust() {
        let def = id_to_def(BuildingId::DustCollector);
        assert_eq!(def.base_cost, &[(ResourceId::Dust, 10.0)]);
    }

    #[test]
    fn pebble_foundry_is_priced_in_dust() {
        // The DESIGN.md fix: pebble_foundry must be affordable at the
        // Pebbles paradigm, so it costs dust (not pebbles).
        let def = id_to_def(BuildingId::PebbleFoundry);
        assert_eq!(def.base_cost, &[(ResourceId::Dust, 100.0)]);
    }

    // ─── cost_of ─────────────────────────────────────────────────────────

    #[test]
    fn cost_of_count_zero_is_base() {
        let def = id_to_def(BuildingId::DustCollector);
        let cost = cost_of(def, 0);
        assert_eq!(cost, vec![(ResourceId::Dust, 10.0)]);
    }

    #[test]
    fn cost_of_applies_1_15_scaling() {
        let def = id_to_def(BuildingId::DustCollector);
        let cost = cost_of(def, 1);
        assert!((cost[0].1 - 11.5).abs() < 1e-9);
    }

    #[test]
    fn cost_of_after_ten_purchases() {
        let def = id_to_def(BuildingId::DustCollector);
        let cost = cost_of(def, 10);
        let expected = 10.0 * 1.15_f64.powi(10);
        assert!((cost[0].1 - expected).abs() < 1e-9);
    }

    #[test]
    fn cost_of_compound_scales_each_resource() {
        let def = id_to_def(BuildingId::MagnetosphereCoil);
        let cost = cost_of(def, 2);
        let scale = 1.15_f64.powi(2);
        assert_eq!(cost.len(), 2);
        assert!((cost[0].1 - 2000.0 * scale).abs() < 1e-9);
        assert!((cost[1].1 - 500.0 * scale).abs() < 1e-9);
    }

    // ─── can_afford ──────────────────────────────────────────────────────

    fn resources_with(pairs: &[(ResourceId, f64)]) -> HashMap<ResourceId, f64> {
        let mut m = HashMap::new();
        for (r, v) in pairs {
            m.insert(*r, *v);
        }
        m
    }

    #[test]
    fn can_afford_exactly() {
        let res = resources_with(&[(ResourceId::Dust, 10.0)]);
        assert!(can_afford(&res, id_to_def(BuildingId::DustCollector), 0));
    }

    #[test]
    fn can_afford_excess() {
        let res = resources_with(&[(ResourceId::Dust, 100.0)]);
        assert!(can_afford(&res, id_to_def(BuildingId::DustCollector), 0));
    }

    #[test]
    fn cannot_afford_insufficient() {
        let res = resources_with(&[(ResourceId::Dust, 9.0)]);
        assert!(!can_afford(&res, id_to_def(BuildingId::DustCollector), 0));
    }

    #[test]
    fn cannot_afford_missing_resource_type() {
        let res = resources_with(&[(ResourceId::Pebbles, 1000.0)]);
        assert!(!can_afford(&res, id_to_def(BuildingId::DustCollector), 0));
    }

    #[test]
    fn cannot_afford_compound_missing_second() {
        let res = resources_with(&[(ResourceId::Metals, 100_000.0)]);
        // MagnetosphereCoil needs Metals AND Water.
        assert!(!can_afford(&res, id_to_def(BuildingId::MagnetosphereCoil), 0));
    }

    // ─── buy ─────────────────────────────────────────────────────────────

    #[test]
    fn buy_decrements_resources() {
        let mut res = resources_with(&[(ResourceId::Dust, 100.0)]);
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        assert!(buy(&mut res, &mut inst));
        assert!((res[&ResourceId::Dust] - 90.0).abs() < 1e-9);
    }

    #[test]
    fn buy_increments_count() {
        let mut res = resources_with(&[(ResourceId::Dust, 100.0)]);
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        assert!(buy(&mut res, &mut inst));
        assert_eq!(inst.count, 1);
        assert!(buy(&mut res, &mut inst));
        assert_eq!(inst.count, 2);
    }

    #[test]
    fn buy_second_instance_costs_more() {
        let mut res = resources_with(&[(ResourceId::Dust, 100.0)]);
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        assert!(buy(&mut res, &mut inst)); // -10
        assert!(buy(&mut res, &mut inst)); // -11.5
        assert!((res[&ResourceId::Dust] - 78.5).abs() < 1e-9);
    }

    #[test]
    fn buy_fails_when_unaffordable() {
        let mut res = resources_with(&[(ResourceId::Dust, 5.0)]);
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        assert!(!buy(&mut res, &mut inst));
        assert_eq!(inst.count, 0);
        assert_eq!(res[&ResourceId::Dust], 5.0);
    }

    #[test]
    fn buy_compound_decrements_both() {
        let mut res = resources_with(&[
            (ResourceId::Metals, 10_000.0),
            (ResourceId::Water, 1_000.0),
        ]);
        let mut inst = BuildingInstance::new(BuildingId::MagnetosphereCoil);
        assert!(buy(&mut res, &mut inst));
        assert!((res[&ResourceId::Metals] - 8_000.0).abs() < 1e-9);
        assert!((res[&ResourceId::Water] - 500.0).abs() < 1e-9);
    }

    // ─── compute_multipliers ─────────────────────────────────────────────

    #[test]
    fn multipliers_identity_with_no_passives() {
        let m = compute_multipliers(&[], 1.0);
        for r in ResourceId::ALL {
            assert!((m[&r] - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn multipliers_include_g_bonus() {
        let m = compute_multipliers(&[], 2.5);
        for r in ResourceId::ALL {
            assert!((m[&r] - 2.5).abs() < 1e-9);
        }
    }

    #[test]
    fn accretion_ring_multiplies_dust_only() {
        let mut inst = BuildingInstance::new(BuildingId::AccretionRing);
        inst.count = 1;
        let m = compute_multipliers(&[inst], 1.0);
        assert!((m[&ResourceId::Dust] - 1.10).abs() < 1e-6);
        assert!((m[&ResourceId::Pebbles] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn accretion_ring_count_stacks_exponentially() {
        let mut inst = BuildingInstance::new(BuildingId::AccretionRing);
        inst.count = 3;
        let m = compute_multipliers(&[inst], 1.0);
        let expected = 1.10_f64.powi(3);
        assert!((m[&ResourceId::Dust] - expected).abs() < 1e-9);
    }

    #[test]
    fn orbital_anchor_multiplies_everything() {
        let mut inst = BuildingInstance::new(BuildingId::OrbitalAnchor);
        inst.count = 2;
        let m = compute_multipliers(&[inst], 1.0);
        let expected = 1.20_f64.powi(2);
        for r in ResourceId::ALL {
            assert!((m[&r] - expected).abs() < 1e-9);
        }
    }

    // ─── tick_buildings ──────────────────────────────────────────────────

    #[test]
    fn tick_with_no_buildings_produces_nothing() {
        let mut res = HashMap::new();
        let m = compute_multipliers(&[], 1.0);
        let produced = tick_buildings(&mut [], &mut res, &m, 1.0);
        assert!(produced.is_empty());
    }

    #[test]
    fn tick_below_cooldown_produces_nothing() {
        let mut res = HashMap::new();
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        inst.count = 1;
        let m = compute_multipliers(&[], 1.0);
        let produced = tick_buildings(
            std::slice::from_mut(&mut inst),
            &mut res,
            &m,
            0.5, // half the 1.0s cooldown
        );
        assert!(produced.is_empty());
        assert_eq!(res.get(&ResourceId::Dust).copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn tick_at_cooldown_produces_once() {
        let mut res = HashMap::new();
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        inst.count = 1;
        let m = compute_multipliers(&[], 1.0);
        tick_buildings(
            std::slice::from_mut(&mut inst),
            &mut res,
            &m,
            1.0,
        );
        assert_eq!(res[&ResourceId::Dust], 1.0);
    }

    #[test]
    fn tick_scales_with_count() {
        let mut res = HashMap::new();
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        inst.count = 5;
        let m = compute_multipliers(&[], 1.0);
        tick_buildings(
            std::slice::from_mut(&mut inst),
            &mut res,
            &m,
            1.0,
        );
        assert_eq!(res[&ResourceId::Dust], 5.0);
    }

    #[test]
    fn tick_applies_multipliers() {
        let mut res = HashMap::new();
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        inst.count = 1;
        let mut ring = BuildingInstance::new(BuildingId::AccretionRing);
        ring.count = 2; // ×1.10^2 = 1.21
        let m = compute_multipliers(&[ring], 1.0);
        tick_buildings(
            std::slice::from_mut(&mut inst),
            &mut res,
            &m,
            1.0,
        );
        assert!((res[&ResourceId::Dust] - 1.21).abs() < 1e-6);
    }

    #[test]
    fn tick_applies_g_bonus() {
        let mut res = HashMap::new();
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        inst.count = 1;
        let m = compute_multipliers(&[], 3.0);
        tick_buildings(
            std::slice::from_mut(&mut inst),
            &mut res,
            &m,
            1.0,
        );
        assert_eq!(res[&ResourceId::Dust], 3.0);
    }

    #[test]
    fn tick_accumulates_across_large_dt() {
        let mut res = HashMap::new();
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        inst.count = 1;
        let m = compute_multipliers(&[], 1.0);
        // 3.0s with a 1.0s cooldown: should produce 3 times.
        tick_buildings(
            std::slice::from_mut(&mut inst),
            &mut res,
            &m,
            3.0,
        );
        assert_eq!(res[&ResourceId::Dust], 3.0);
    }

    #[test]
    fn tick_returns_produced_amounts() {
        let mut res = HashMap::new();
        let mut inst = BuildingInstance::new(BuildingId::DustCollector);
        inst.count = 2;
        let m = compute_multipliers(&[], 1.0);
        let produced = tick_buildings(
            std::slice::from_mut(&mut inst),
            &mut res,
            &m,
            1.0,
        );
        assert_eq!(produced, vec![(ResourceId::Dust, 2.0)]);
    }

    #[test]
    fn passive_buildings_do_not_produce_on_tick() {
        let mut res = HashMap::new();
        let mut inst = BuildingInstance::new(BuildingId::AccretionRing);
        inst.count = 5;
        let m = compute_multipliers(&[], 1.0);
        let produced = tick_buildings(
            std::slice::from_mut(&mut inst),
            &mut res,
            &m,
            5.0,
        );
        assert!(produced.is_empty());
    }
}