//! Upgrade catalog. Central place for costs, max levels, labels,
//! and the stat application on Archie and Truck.
//!
//! Design §11: upgrades are bought by headbutting blocks at the HQ.
//! Each block has a fixed id, a max level, and a cost table.

use crate::player::Archie;
use crate::truck::Truck;

/// One upgrade's static definition.
#[derive(Debug, Clone, Copy)]
pub struct UpgradeDef {
    /// Stable id used as the key in `Progress.upgrades`.
    pub id: &'static str,
    /// Short label shown on the block.
    pub label: &'static str,
    /// Maximum purchasable level. Level == max means "MAX".
    pub max_level: u8,
    /// Cost to go from level N to N+1, indexed by N.
    /// Length must be `max_level`.
    pub costs: &'static [u64],
}

/// All V1 upgrades, in the order blocks appear at the HQ.
pub const UPGRADES: &[UpgradeDef] = &[
    UpgradeDef {
        id: "pickaxe",
        label: "Pickaxe",
        max_level: 3,
        costs: &[25, 75, 200],
    },
    UpgradeDef {
        id: "mining_speed",
        label: "Mining",
        max_level: 3,
        costs: &[30, 90, 250],
    },
    UpgradeDef {
        id: "move_speed",
        label: "Move",
        max_level: 3,
        costs: &[20, 60, 150],
    },
    UpgradeDef {
        id: "bag",
        label: "Bag",
        max_level: 3,
        costs: &[20, 60, 150],
    },
    UpgradeDef {
        id: "truck_storage",
        label: "Truck",
        max_level: 3,
        costs: &[30, 90, 250],
    },
];

/// Base values, matching `Archie::new` and `Truck::new`. Kept as
/// constants so the application formulas are easy to sanity-check.
pub const BASE_PICKAXE_DAMAGE: f32 = 1.0;
pub const BASE_MINING_SPEED: f32 = 1.0;
pub const BASE_MOVE_SPEED: f32 = 100.0;
pub const BASE_BAG_CAPACITY: u32 = 10;
pub const BASE_TRUCK_STORAGE: u32 = 100;

/// Apply one upgrade at `level` to Archie. Unknown ids are ignored.
pub fn apply_to_archie(id: &str, level: u8, archie: &mut Archie) {
    let l = level as i32;
    match id {
        "pickaxe" => {
            archie.pickaxe_damage = BASE_PICKAXE_DAMAGE * 1.25_f32.powi(l);
        }
        "mining_speed" => {
            archie.mining_speed = BASE_MINING_SPEED * 1.20_f32.powi(l);
        }
        "move_speed" => {
            archie.move_speed = BASE_MOVE_SPEED * 1.10_f32.powi(l);
        }
        "bag" => {
            archie.bag.set_capacity(BASE_BAG_CAPACITY + 10 * level as u32);
        }
        _ => {}
    }
}

/// Apply one upgrade at `level` to the Truck. Unknown ids are ignored.
pub fn apply_to_truck(id: &str, level: u8, truck: &mut Truck) {
    if id == "truck_storage" {
        truck.storage_capacity = BASE_TRUCK_STORAGE + 100 * level as u32;
    }
}

/// Convenience: apply one upgrade to both. Used by `Game::apply_upgrades`.
pub fn apply(id: &str, level: u8, archie: &mut Archie, truck: &mut Truck) {
    apply_to_archie(id, level, archie);
    apply_to_truck(id, level, truck);
}

/// Cost to go from `current_level` to `current_level + 1`, or `None`
/// if already at max.
pub fn cost_for(def: &UpgradeDef, current_level: u8) -> Option<u64> {
    if current_level >= def.max_level {
        return None;
    }
    def.costs.get(current_level as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn costs_length_matches_max_level() {
        for def in UPGRADES {
            assert_eq!(
                def.costs.len(),
                def.max_level as usize,
                "{}: costs len != max_level",
                def.id,
            );
        }
    }

    #[test]
    fn ids_are_unique() {
        for (i, a) in UPGRADES.iter().enumerate() {
            for b in UPGRADES.iter().skip(i + 1) {
                assert_ne!(a.id, b.id, "duplicate upgrade id");
            }
        }
    }

    #[test]
    fn costs_are_increasing() {
        for def in UPGRADES {
            for i in 1..def.costs.len() {
                assert!(
                    def.costs[i] > def.costs[i - 1],
                    "{} cost not increasing at {i}",
                    def.id,
                );
            }
        }
    }

    #[test]
    fn cost_for_returns_none_at_max() {
        let def = &UPGRADES[0];
        assert_eq!(cost_for(def, def.max_level), None);
        assert!(cost_for(def, def.max_level - 1).is_some());
    }

    #[test]
    fn pickaxe_compounds() {
        let mut a = Archie::new(Vec2::ZERO);
        apply_to_archie("pickaxe", 0, &mut a);
        assert!((a.pickaxe_damage - 1.0).abs() < 1e-6);
        apply_to_archie("pickaxe", 1, &mut a);
        assert!((a.pickaxe_damage - 1.25).abs() < 1e-6);
        apply_to_archie("pickaxe", 3, &mut a);
        assert!((a.pickaxe_damage - 1.25_f32.powi(3)).abs() < 1e-6);
    }

    #[test]
    fn bag_upgrade_sets_capacity() {
        let mut a = Archie::new(Vec2::ZERO);
        apply_to_archie("bag", 0, &mut a);
        assert_eq!(a.bag.capacity(), 10);
        apply_to_archie("bag", 2, &mut a);
        assert_eq!(a.bag.capacity(), 30);
    }

    #[test]
    fn truck_storage_upgrade_sets_capacity() {
        let mut t = Truck::new(Vec2::ZERO);
        apply_to_truck("truck_storage", 0, &mut t);
        assert_eq!(t.storage_capacity, 100);
        apply_to_truck("truck_storage", 2, &mut t);
        assert_eq!(t.storage_capacity, 300);
    }

    #[test]
    fn unknown_id_is_noop() {
        let mut a = Archie::new(Vec2::ZERO);
        let before = a.pickaxe_damage;
        apply_to_archie("bogus", 3, &mut a);
        assert_eq!(a.pickaxe_damage, before);
    }
}