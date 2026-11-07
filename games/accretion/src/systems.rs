//! The `Game` state struct: owns all mutable state.
//!
//! `main.rs` holds one instance of `Game`. All tick / input handling
//! is pure methods on `Game`, so the whole game is testable headless.

use std::collections::HashMap;

use crate::components::{BuildingId, Ending, Paradigm, ResourceId};
use crate::economy::{
    self, BuildingInstance, buy, compute_multipliers, id_to_def, tick_buildings,
};
use crate::paradigm;
use serde::{Deserialize, Serialize};

/// Maximum number of log entries (narrative fragments + status).
const LOG_CAPACITY: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub paradigm: Paradigm,
    pub resources: HashMap<ResourceId, f64>,
    pub buildings: Vec<BuildingInstance>,
    pub g_multiplier: f64,
    pub g_total: u64,
    /// Endings seen (across all runs). `#[serde(default)]` so V1.2
    /// saves load with an empty Vec.
    #[serde(default)]
    pub endings_seen: Vec<Ending>,
    pub elapsed: f64,
    pub log: Vec<String>,
    pub dust_collected_total: f64,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// Fresh run: Dust paradigm, all resources at zero, no buildings.
    pub fn new() -> Self {
        let resources: HashMap<ResourceId, f64> =
            ResourceId::ALL.iter().map(|r| (*r, 0.0)).collect();
        let buildings: Vec<BuildingInstance> = BuildingId::ALL
            .iter()
            .map(|&id| BuildingInstance::new(id))
            .collect();
        let mut log = Vec::new();
        log.push("I am.".to_string());
        log.push("I do not know what I am.".to_string());
        Self {
            paradigm: Paradigm::Dust,
            resources,
            buildings,
            g_multiplier: 1.0,
            elapsed: 0.0,
            log,
            dust_collected_total: 0.0,
            g_total: 0,
            endings_seen: Vec::new(),
        }
    }

    /// Reset the current run, preserving meta-progress (`g_total`,
    /// `g_multiplier`, `endings_seen`). Used by `prestige::prestige` and
    /// "Abandon run" from the Title screen.
    pub fn reset_run(&mut self) {
        let g_total = self.g_total;
        let g_multiplier = self.g_multiplier;
        let endings_seen = std::mem::take(&mut self.endings_seen);
        *self = Game::new();
        self.g_total = g_total;
        self.g_multiplier = g_multiplier;
        self.endings_seen = endings_seen;
    }

    pub(crate) fn push_log(&mut self, line: impl Into<String>) {   // was `fn`
        if self.log.len() >= LOG_CAPACITY {
            self.log.remove(0);
        }
        self.log.push(line.into());
    }

    /// One frame's worth of simulation.
    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.elapsed += dt as f64;
        let mults = compute_multipliers(&self.buildings, self.g_multiplier);
        let _produced = tick_buildings(
            &mut self.buildings,
            &mut self.resources,
            &mults,
            dt,
        );
        // Floating text for produced resources arrives in Session 3
        // (with the juice module). For now, `_produced` is discarded.
    }

    /// Manual click: adds 1 Dust. Always available (just inefficient
    /// after the Pebbles paradigm — that's the DESIGN intent).
    pub fn click(&mut self) -> f64 {
        let amount = 1.0;
        *self.resources.get_mut(&ResourceId::Dust).unwrap() += amount;
        self.dust_collected_total += amount;
        amount
    }

    /// Try to buy the next instance of `id`. Returns `true` on success.
    ///
    /// Fails if: the building's paradigm is higher than the current
    /// one, or the player can't afford the next cost.
    pub fn buy_building(&mut self, id: BuildingId) -> bool {
        let def = id_to_def(id);
        if def.paradigm > self.paradigm {
            return false;
        }
        let Some(inst) = self.buildings.iter_mut().find(|b| b.id == id) else {
            return false;
        };
        buy(&mut self.resources, inst)
    }

    /// The paradigm that can be reached *right now*, or `None`.
    pub fn can_advance(&self) -> Option<Paradigm> {
        paradigm::check_advance(self.paradigm, &self.buildings, &self.resources)
    }

    /// Advance to the next paradigm if the condition is met.
    pub fn try_advance(&mut self) -> Option<Paradigm> {
        let next = self.can_advance()?;
        self.paradigm = next;
        self.push_log(format!("→ {}", next.name()));
        Some(next)
    }

    /// Current cost of the next instance of `id`.
    pub fn cost_of_next(&self, id: BuildingId) -> Vec<(ResourceId, f64)> {
        let def = id_to_def(id);
        let count = self
            .buildings
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.count)
            .unwrap_or(0);
        economy::cost_of(def, count)
    }

    /// Whether `id` is currently purchasable (paradigm gate + afford).
    pub fn can_buy(&self, id: BuildingId) -> bool {
        let def = id_to_def(id);
        if def.paradigm > self.paradigm {
            return false;
        }
        economy::can_afford(&self.resources, def, self.building_count(id))
    }

    /// How many of `id` the player owns.
    pub fn building_count(&self, id: BuildingId) -> u32 {
        self.buildings
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.count)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_starts_at_dust() {
        assert_eq!(Game::new().paradigm, Paradigm::Dust);
    }

    #[test]
    fn new_game_has_zero_dust() {
        let g = Game::new();
        assert_eq!(g.resources[&ResourceId::Dust], 0.0);
    }

    #[test]
    fn new_game_has_all_buildings_at_zero() {
        let g = Game::new();
        for id in BuildingId::ALL {
            assert_eq!(g.building_count(id), 0);
        }
    }

    #[test]
    fn new_game_has_g_multiplier_one() {
        assert_eq!(Game::new().g_multiplier, 1.0);
    }

    #[test]
    fn new_game_has_starter_log() {
        let g = Game::new();
        assert!(!g.log.is_empty());
    }

    #[test]
    fn click_adds_one_dust() {
        let mut g = Game::new();
        assert_eq!(g.click(), 1.0);
        assert_eq!(g.resources[&ResourceId::Dust], 1.0);
    }

    #[test]
    fn click_accumulates() {
        let mut g = Game::new();
        for _ in 0..10 {
            g.click();
        }
        assert_eq!(g.resources[&ResourceId::Dust], 10.0);
        assert_eq!(g.dust_collected_total, 10.0);
    }

    #[test]
    fn tick_without_buildings_produces_nothing() {
        let mut g = Game::new();
        g.tick(1.0);
        assert_eq!(g.resources[&ResourceId::Dust], 0.0);
    }

    #[test]
    fn tick_advances_elapsed() {
        let mut g = Game::new();
        g.tick(0.5);
        g.tick(0.5);
        assert!((g.elapsed - 1.0).abs() < 1e-9);
    }

    #[test]
    fn tick_with_zero_dt_does_nothing() {
        let mut g = Game::new();
        let before = g.elapsed;
        g.tick(0.0);
        g.tick(-1.0);
        assert_eq!(g.elapsed, before);
    }

    #[test]
    fn cannot_buy_building_from_future_paradigm() {
        let mut g = Game::new();
        assert!(!g.buy_building(BuildingId::DustCollector));
    }

    #[test]
    fn cannot_buy_without_resources() {
        let mut g = Game::new();
        g.paradigm = Paradigm::Pebbles;
        assert!(!g.buy_building(BuildingId::DustCollector));
    }

    #[test]
    fn can_buy_when_affordable() {
        let mut g = Game::new();
        g.paradigm = Paradigm::Pebbles;
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 100.0;
        assert!(g.buy_building(BuildingId::DustCollector));
        assert_eq!(g.building_count(BuildingId::DustCollector), 1);
    }

    #[test]
    fn buy_decrements_resources() {
        let mut g = Game::new();
        g.paradigm = Paradigm::Pebbles;
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 100.0;
        g.buy_building(BuildingId::DustCollector);
        assert!((g.resources[&ResourceId::Dust] - 90.0).abs() < 1e-9);
    }

    #[test]
    fn can_advance_false_at_start() {
        let g = Game::new();
        assert_eq!(g.can_advance(), None);
    }

    #[test]
    fn can_advance_true_with_enough_dust() {
        let mut g = Game::new();
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 100.0;
        assert_eq!(g.can_advance(), Some(Paradigm::Pebbles));
    }

    #[test]
    fn try_advance_changes_paradigm() {
        let mut g = Game::new();
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 100.0;
        assert_eq!(g.try_advance(), Some(Paradigm::Pebbles));
        assert_eq!(g.paradigm, Paradigm::Pebbles);
    }

    #[test]
    fn try_advance_fails_when_not_ready() {
        let mut g = Game::new();
        assert_eq!(g.try_advance(), None);
        assert_eq!(g.paradigm, Paradigm::Dust);
    }

    #[test]
    fn try_advance_pushes_log() {
        let mut g = Game::new();
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 100.0;
        let before = g.log.len();
        g.try_advance();
        assert_eq!(g.log.len(), before + 1);
        assert!(g.log.last().unwrap().contains("Pebbles"));
    }

    #[test]
    fn can_buy_false_for_future_paradigm() {
        let g = Game::new();
        assert!(!g.can_buy(BuildingId::DustCollector));
    }

    #[test]
    fn can_buy_true_when_affordable() {
        let mut g = Game::new();
        g.paradigm = Paradigm::Pebbles;
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 100.0;
        assert!(g.can_buy(BuildingId::DustCollector));
    }

    #[test]
    fn cost_of_next_matches_economy() {
        let g = Game::new();
        let cost = g.cost_of_next(BuildingId::DustCollector);
        assert_eq!(cost, vec![(ResourceId::Dust, 10.0)]);
    }

    #[test]
    fn log_caps_at_capacity() {
        let mut g = Game::new();
        for i in 0..(LOG_CAPACITY + 10) {
            g.push_log(format!("line {i}"));
        }
        assert_eq!(g.log.len(), LOG_CAPACITY);
    }

    #[test]
    fn after_advancing_dust_collectors_are_purchasable() {
        let mut g = Game::new();
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 200.0;
        g.try_advance();
        assert!(g.buy_building(BuildingId::DustCollector));
    }

    #[test]
    fn new_game_has_zero_g_total() {
        assert_eq!(Game::new().g_total, 0);
    }

    #[test]
    fn new_game_has_no_endings() {
        assert!(Game::new().endings_seen.is_empty());
    }

    #[test]
    fn reset_run_preserves_meta() {
        let mut g = Game::new();
        g.paradigm = Paradigm::GasGiant;
        g.g_total = 7;
        g.g_multiplier = 1.35;
        g.endings_seen.push(Ending::NeutronStar);
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 999.0;

        g.reset_run();

        assert_eq!(g.paradigm, Paradigm::Dust);
        assert_eq!(g.g_total, 7);
        assert_eq!(g.g_multiplier, 1.35);
        assert_eq!(g.endings_seen.len(), 1);
        assert_eq!(g.resources[&ResourceId::Dust], 0.0);
    }
}