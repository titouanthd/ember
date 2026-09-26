//! Arbre de progression — nœuds, validation, agrégation d'effets.
//!
//! `UpgradeTree` est **sérialisable** : il contient le solde d'or et
//! l'ensemble des nœuds achetés. Les définitions (`UpgradeTreeDefs`)
//! sont un singleton statique, chargé une fois.

use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use ember_core::io::load_from_file;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub enum UpgradeEffect {
    None,
    TowerHpMult(f32),
    TowerRegen(f32),
    Fortress,
    UnitHpMult(f32),
    UnitDamageMult(f32),
    UnlockUnit(String),
    TurretDamageMult(f32),
    TurretFireRateMult(f32),
    MultiShot(u32),
    ManaRegenMult(f32),
    ManaCapMult(f32),
    GoldPerKillAdd(f32),
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpgradeNode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: f32,
    pub requires: Vec<String>,
    pub col: i32,
    pub row: i32,
    pub effect: UpgradeEffect,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpgradeTreeDefs {
    pub nodes: Vec<UpgradeNode>,
}

impl UpgradeTreeDefs {
    pub fn node(&self, id: &str) -> Option<&UpgradeNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn root(&self) -> Option<&UpgradeNode> {
        self.nodes.iter().find(|n| n.requires.is_empty())
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        for node in &self.nodes {
            if !ids.insert(node.id.as_str()) {
                return Err(format!("duplicate node id: {}", node.id));
            }
        }

        for node in &self.nodes {
            for req in &node.requires {
                if !ids.contains(req.as_str()) {
                    return Err(format!(
                        "node '{}' requires unknown node '{}'",
                        node.id, req
                    ));
                }
            }
        }

        let roots: Vec<_> = self.nodes.iter().filter(|n| n.requires.is_empty()).collect();
        if roots.len() != 1 {
            return Err(format!("expected exactly 1 root, found {}", roots.len()));
        }

        // Détection de cycles (Kahn).
        use std::collections::HashMap;
        let mut in_degree: HashMap<&str, usize> = self
            .nodes
            .iter()
            .map(|n| (n.id.as_str(), n.requires.len()))
            .collect();
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut visited = 0;
        while let Some(id) = queue.pop() {
            visited += 1;
            for node in &self.nodes {
                if node.requires.iter().any(|r| r == id)
                    && let Some(deg) = in_degree.get_mut(node.id.as_str())
                {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(node.id.as_str());
                    }
                }
            }
        }
        if visited != self.nodes.len() {
            return Err(format!(
                "cycle detected: {} nodes visited out of {}",
                visited,
                self.nodes.len()
            ));
        }

        Ok(())
    }
}

static DEFS: OnceLock<UpgradeTreeDefs> = OnceLock::new();

pub fn load_defs() -> &'static UpgradeTreeDefs {
    DEFS.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("upgrade_tree.ron");
        let defs: UpgradeTreeDefs =
            load_from_file(&path).expect("assets/upgrade_tree.ron should load and parse");
        defs.validate().expect("upgrade tree should be valid");
        defs
    })
}

/// État persistant de l'arbre : or disponible + nœuds achetés.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeTree {
    pub gold: f32,
    pub purchased: HashSet<String>,
}

impl Default for UpgradeTree {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl UpgradeTree {
    /// Crée un arbre vierge avec la racine achetée gratuitement.
    pub fn new(starting_gold: f32) -> Self {
        let mut purchased = HashSet::new();
        if let Some(root) = load_defs().root() {
            purchased.insert(root.id.clone());
        }
        Self {
            gold: starting_gold,
            purchased,
        }
    }

    /// Assure que la racine est bien présente (post-load).
    pub fn ensure_root(&mut self) {
        if let Some(root) = load_defs().root() {
            self.purchased.insert(root.id.clone());
        }
    }

    pub fn is_purchased(&self, id: &str) -> bool {
        self.purchased.contains(id)
    }

    pub fn is_accessible(&self, id: &str) -> bool {
        match load_defs().node(id) {
            Some(node) => node.requires.iter().all(|r| self.purchased.contains(r)),
            None => false,
        }
    }

    pub fn can_buy(&self, id: &str) -> bool {
        if self.is_purchased(id) || !self.is_accessible(id) {
            return false;
        }
        match load_defs().node(id) {
            Some(node) => self.gold >= node.cost,
            None => false,
        }
    }

    pub fn buy(&mut self, id: &str) -> bool {
        if !self.can_buy(id) {
            return false;
        }
        let node = match load_defs().node(id) {
            Some(n) => n,
            None => return false,
        };
        self.gold -= node.cost;
        self.purchased.insert(id.to_string());
        true
    }

    /// Force l'ajout d'un nœud sans vérifier le coût ni les prérequis.
    /// Utile pour les tests et les futures mécaniques de triche.
    pub fn grant(&mut self, id: &str) {
        self.purchased.insert(id.to_string());
    }

    pub fn purchased_count(&self) -> usize {
        self.purchased.len()
    }

    // ---------- Agrégation ----------

    pub fn tower_hp_mult(&self) -> f32 {
        self.aggregate_mult(|e| match e {
            UpgradeEffect::TowerHpMult(m) => Some(*m),
            _ => None,
        })
    }

    pub fn tower_regen(&self) -> f32 {
        self.aggregate_add(|e| match e {
            UpgradeEffect::TowerRegen(v) => Some(*v),
            _ => None,
        })
    }

    pub fn has_fortress(&self) -> bool {
        self.purchased.iter().any(|id| {
            load_defs()
                .node(id)
                .map(|n| matches!(n.effect, UpgradeEffect::Fortress))
                .unwrap_or(false)
        })
    }

    pub fn unit_hp_mult(&self) -> f32 {
        self.aggregate_mult(|e| match e {
            UpgradeEffect::UnitHpMult(m) => Some(*m),
            _ => None,
        })
    }

    pub fn unit_damage_mult(&self) -> f32 {
        self.aggregate_mult(|e| match e {
            UpgradeEffect::UnitDamageMult(m) => Some(*m),
            _ => None,
        })
    }

    pub fn unlocked_units(&self, defaults: &[&str]) -> HashSet<String> {
        let mut set: HashSet<String> = defaults.iter().map(|s| s.to_string()).collect();
        for id in &self.purchased {
            if let Some(node) = load_defs().node(id)
                && let UpgradeEffect::UnlockUnit(kind) = &node.effect
            {
                set.insert(kind.clone());
            }
        }
        set
    }

    pub fn turret_damage_mult(&self) -> f32 {
        self.aggregate_mult(|e| match e {
            UpgradeEffect::TurretDamageMult(m) => Some(*m),
            _ => None,
        })
    }

    pub fn turret_fire_rate_mult(&self) -> f32 {
        self.aggregate_mult(|e| match e {
            UpgradeEffect::TurretFireRateMult(m) => Some(*m),
            _ => None,
        })
    }

    pub fn multi_shot_count(&self) -> u32 {
        self.purchased
            .iter()
            .filter_map(|id| load_defs().node(id))
            .filter_map(|n| match n.effect {
                UpgradeEffect::MultiShot(n) => Some(n),
                _ => None,
            })
            .max()
            .unwrap_or(1)
    }

    pub fn mana_regen_mult(&self) -> f32 {
        self.aggregate_mult(|e| match e {
            UpgradeEffect::ManaRegenMult(m) => Some(*m),
            _ => None,
        })
    }

    pub fn mana_cap_mult(&self) -> f32 {
        self.aggregate_mult(|e| match e {
            UpgradeEffect::ManaCapMult(m) => Some(*m),
            _ => None,
        })
    }

    pub fn gold_per_kill_add(&self) -> f32 {
        self.aggregate_add(|e| match e {
            UpgradeEffect::GoldPerKillAdd(v) => Some(*v),
            _ => None,
        })
    }

    fn aggregate_mult<F: Fn(&UpgradeEffect) -> Option<f32>>(&self, f: F) -> f32 {
        let mut result = 1.0;
        for id in &self.purchased {
            if let Some(node) = load_defs().node(id)
                && let Some(v) = f(&node.effect)
            {
                result *= v;
            }
        }
        result
    }

    fn aggregate_add<F: Fn(&UpgradeEffect) -> Option<f32>>(&self, f: F) -> f32 {
        let mut result = 0.0;
        for id in &self.purchased {
            if let Some(node) = load_defs().node(id)
                && let Some(v) = f(&node.effect)
            {
                result += v;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs() -> &'static UpgradeTreeDefs {
        load_defs()
    }

    fn fresh(gold: f32) -> UpgradeTree {
        UpgradeTree::new(gold)
    }

    #[test]
    fn tree_ron_loads_and_has_18_nodes() {
        assert_eq!(defs().nodes.len(), 18);
    }

    #[test]
    fn tree_validation_passes() {
        defs().validate().expect("valid");
    }

    #[test]
    fn tree_has_exactly_one_root() {
        let roots: Vec<_> = defs().nodes.iter().filter(|n| n.requires.is_empty()).collect();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, "root");
    }

    #[test]
    fn validate_rejects_duplicate_ids() {
        let mut bad = defs().clone();
        let dup = bad.nodes[1].clone();
        bad.nodes.push(dup);
        assert!(bad.validate().is_err());
    }

    #[test]
    fn validate_rejects_unknown_requires() {
        let mut bad = defs().clone();
        bad.nodes[1].requires.push("nope".to_string());
        assert!(bad.validate().is_err());
    }

    #[test]
    fn validate_rejects_multiple_roots() {
        let mut bad = defs().clone();
        let mut extra_root = bad.nodes[1].clone();
        extra_root.requires.clear();
        extra_root.id = "extra_root".to_string();
        bad.nodes.push(extra_root);
        assert!(bad.validate().is_err());
    }

    #[test]
    fn validate_rejects_cycles() {
        let mut bad = defs().clone();
        if let Some(root) = bad.nodes.iter_mut().find(|n| n.id == "root") {
            root.requires.push("unit_hp_1".to_string());
        }
        assert!(bad.validate().is_err());
    }

    #[test]
    fn new_buys_root_for_free() {
        let t = fresh(0.0);
        assert!(t.is_purchased("root"));
    }

    #[test]
    fn new_preserves_starting_gold() {
        let t = fresh(500.0);
        assert!((t.gold - 500.0).abs() < 1e-6);
    }

    #[test]
    fn purchased_count_is_one_at_start() {
        let t = fresh(0.0);
        assert_eq!(t.purchased_count(), 1);
    }

    #[test]
    fn tier_1_nodes_are_accessible_at_start() {
        let t = fresh(0.0);
        assert!(t.is_accessible("tower_hp_1"));
        assert!(t.is_accessible("unit_hp_1"));
        assert!(t.is_accessible("turret_dmg_1"));
        assert!(t.is_accessible("mana_regen_1"));
        assert!(t.is_accessible("gold_per_kill"));
    }

    #[test]
    fn tier_2_nodes_are_locked_until_parent_bought() {
        let t = fresh(0.0);
        assert!(!t.is_accessible("tower_hp_2"));
        assert!(!t.is_accessible("unlock_archer"));
    }

    #[test]
    fn can_buy_true_when_affordable_and_accessible() {
        let t = fresh(200.0);
        assert!(t.can_buy("tower_hp_1"));
    }

    #[test]
    fn can_buy_false_without_gold() {
        let t = fresh(50.0);
        assert!(!t.can_buy("tower_hp_1"));
    }

    #[test]
    fn buy_deducts_gold_and_marks_purchased() {
        let mut t = fresh(500.0);
        assert!(t.buy("tower_hp_1"));
        assert!((t.gold - 400.0).abs() < 1e-6);
        assert!(t.is_purchased("tower_hp_1"));
    }

    #[test]
    fn buy_unlocks_children() {
        let mut t = fresh(1000.0);
        assert!(!t.is_accessible("tower_hp_2"));
        t.buy("tower_hp_1");
        assert!(t.is_accessible("tower_hp_2"));
    }

    #[test]
    fn tower_hp_mult_default_is_one() {
        let t = fresh(0.0);
        assert!((t.tower_hp_mult() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn tower_hp_mult_multiplies_across_nodes() {
        let mut t = fresh(0.0);
        t.grant("tower_hp_1");
        t.grant("tower_hp_2");
        assert!((t.tower_hp_mult() - 1.20 * 1.25).abs() < 1e-6);
    }

    #[test]
    fn has_fortress_true_after_purchase() {
        let mut t = fresh(0.0);
        t.grant("fortress");
        assert!(t.has_fortress());
    }

    #[test]
    fn unlocked_units_default_set() {
        let t = fresh(0.0);
        let u = t.unlocked_units(&["grunt", "brute"]);
        assert_eq!(u.len(), 2);
    }

    #[test]
    fn unlocked_units_after_archer_unlock() {
        let mut t = fresh(0.0);
        t.grant("unlock_archer");
        let u = t.unlocked_units(&["grunt", "brute"]);
        assert!(u.contains("archer"));
    }

    #[test]
    fn multi_shot_default_is_one() {
        let t = fresh(0.0);
        assert_eq!(t.multi_shot_count(), 1);
    }

    #[test]
    fn multi_shot_after_purchase() {
        let mut t = fresh(0.0);
        t.grant("multi_shot");
        assert_eq!(t.multi_shot_count(), 2);
    }

    #[test]
    fn ensure_root_adds_root_if_missing() {
        let mut t = UpgradeTree {
            gold: 0.0,
            purchased: HashSet::new(),
        };
        t.ensure_root();
        assert!(t.is_purchased("root"));
    }
}