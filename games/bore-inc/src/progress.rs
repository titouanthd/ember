//! Persistent progression: dollars, purchased upgrades, unlocked
//! robots, level state (seed + tunnel delta), truck position on the
//! map. DESIGN §19.
//!
//! **Native**: `progress.ron` in the user-data directory.
//! **Web**: in-memory `thread_local`, doesn't survive a reload.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::world::DeltaEntry;

// ---------- Data ----------

/// Saved state for a single level: the generation seed plus the list
/// of cells the player has changed since generation. `delta` is
/// replayed on load to reconstruct the tunnel without storing the
/// whole grid.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LevelSave {
    pub seed: u32,
    pub completed: bool,
    #[serde(default)]
    pub delta: Vec<DeltaEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Currency, in dollars. DESIGN §15.
    pub dollars: u64,

    /// Purchased upgrade blocks. Key = block id, value = current
    /// level. DESIGN §11.
    #[serde(default)]
    pub upgrades: HashMap<String, u8>,

    /// Ids of unlocked robot types ("digger", "drill", "seeker").
    /// DESIGN §8.
    #[serde(default)]
    pub unlocked_robots: Vec<String>,

    /// Per-level save state. Key = level id.
    #[serde(default)]
    pub levels: HashMap<String, LevelSave>,

    /// Truck position on the top-down map. DESIGN §13.
    #[serde(default)]
    pub map_truck_pos: (f32, f32),
}

impl Progress {
    pub fn add_dollars(&mut self, amount: u64) {
        self.dollars = self.dollars.saturating_add(amount);
    }

    pub fn try_spend(&mut self, cost: u64) -> bool {
        if self.dollars >= cost {
            self.dollars -= cost;
            true
        } else {
            false
        }
    }

    pub fn upgrade_level(&self, id: &str) -> u8 {
        self.upgrades.get(id).copied().unwrap_or(0)
    }

    pub fn buy_upgrade(&mut self, id: &str) -> u8 {
        let entry = self.upgrades.entry(id.to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
        *entry
    }

    pub fn has_robot(&self, id: &str) -> bool {
        self.unlocked_robots.iter().any(|r| r == id)
    }

    pub fn unlock_robot(&mut self, id: &str) {
        if !self.has_robot(id) {
            self.unlocked_robots.push(id.to_string());
        }
    }

    pub fn level_save(&self, id: &str) -> Option<&LevelSave> {
        self.levels.get(id)
    }

    /// Get or create the seed for a level. Idempotent — calling twice
    /// with different fallbacks returns the first seed both times.
    pub fn level_seed(&mut self, id: &str, fallback: u32) -> u32 {
        self.levels
            .entry(id.to_string())
            .or_insert_with(|| LevelSave {
                seed: fallback,
                completed: false,
                delta: Vec::new(),
            })
            .seed
    }

    /// Persist a new seed for a level. If the seed actually changed,
    /// clears `completed` **and** `delta` — a different world means
    /// the old tunnel is stale. No-op if the seed is unchanged.
    pub fn set_level_seed(&mut self, id: &str, seed: u32) {
        let entry = self
            .levels
            .entry(id.to_string())
            .or_insert_with(|| LevelSave {
                seed,
                completed: false,
                delta: Vec::new(),
            });
        if entry.seed != seed {
            entry.seed = seed;
            entry.completed = false;
            entry.delta.clear();
        }
    }

    /// Read a level's saved tunnel. Empty if never saved.
    pub fn level_delta(&self, id: &str) -> Vec<DeltaEntry> {
        self.levels
            .get(id)
            .map(|l| l.delta.clone())
            .unwrap_or_default()
    }

    /// Write a level's tunnel. No-op if the level entry doesn't exist
    /// (create it first via `set_level_seed`).
    pub fn set_level_delta(&mut self, id: &str, delta: Vec<DeltaEntry>) {
        if let Some(entry) = self.levels.get_mut(id) {
            entry.delta = delta;
        }
    }

    pub fn mark_level_completed(&mut self, id: &str, seed: u32) {
        self.levels
            .entry(id.to_string())
            .or_insert_with(|| LevelSave {
                seed,
                completed: false,
                delta: Vec::new(),
            })
            .completed = true;
    }

    /// True if the level has been marked completed.
    pub fn is_level_completed(&self, id: &str) -> bool {
        self.levels.get(id).map(|l| l.completed).unwrap_or(false)
    }
}

// ---------- Load / save ----------

pub fn load_progress() -> Progress {
    match load_raw() {
        Ok(Some(text)) => match ron::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("⚠️  corrupted progress ({e}); starting fresh.");
                Progress::default()
            }
        },
        Ok(None) => Progress::default(),
        Err(e) => {
            eprintln!("⚠️  could not read progress ({e}); starting fresh.");
            Progress::default()
        }
    }
}

pub fn save_progress(progress: &Progress) -> Result<(), String> {
    let text = ron::ser::to_string_pretty(progress, ron::ser::PrettyConfig::default())
        .map_err(|e| format!("serialize progress: {e}"))?;
    save_raw(&text)
}

// ---------- Native backend ----------

#[cfg(not(target_arch = "wasm32"))]
fn load_raw() -> Result<Option<String>, String> {
    use crate::paths;
    let path = paths::progress_path();
    if !path.exists() {
        return Ok(None);
    }
    std::fs::read_to_string(&path).map(Some).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn save_raw(text: &str) -> Result<(), String> {
    use crate::paths;
    let path = paths::progress_path();
    paths::ensure_parent_dir(&path)?;
    std::fs::write(&path, text).map_err(|e| format!("write {path:?}: {e}"))
}

// ---------- Web backend ----------

#[cfg(target_arch = "wasm32")]
thread_local! {
    static IN_MEMORY_PROGRESS: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
fn load_raw() -> Result<Option<String>, String> {
    IN_MEMORY_PROGRESS.with(|c| Ok(c.borrow().clone()))
}

#[cfg(target_arch = "wasm32")]
fn save_raw(text: &str) -> Result<(), String> {
    IN_MEMORY_PROGRESS.with(|c| {
        *c.borrow_mut() = Some(text.to_string());
    });
    Ok(())
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use crate::components::TileKind;

    use super::*;

    #[test]
    fn default_progress_is_empty() {
        let p = Progress::default();
        assert_eq!(p.dollars, 0);
        assert!(p.upgrades.is_empty());
        assert!(p.unlocked_robots.is_empty());
        assert!(p.levels.is_empty());
    }

    #[test]
    fn add_dollars_accumulates() {
        let mut p = Progress::default();
        p.add_dollars(50);
        p.add_dollars(30);
        assert_eq!(p.dollars, 80);
    }

    #[test]
    fn try_spend_succeeds_when_affordable() {
        let mut p = Progress::default();
        p.add_dollars(100);
        assert!(p.try_spend(25));
        assert_eq!(p.dollars, 75);
    }

    #[test]
    fn try_spend_fails_when_broke() {
        let mut p = Progress::default();
        p.add_dollars(10);
        assert!(!p.try_spend(25));
        assert_eq!(p.dollars, 10);
    }

    #[test]
    fn buy_upgrade_increments_level() {
        let mut p = Progress::default();
        assert_eq!(p.buy_upgrade("pickaxe"), 1);
        assert_eq!(p.buy_upgrade("pickaxe"), 2);
    }

    #[test]
    fn unlock_robot_is_idempotent() {
        let mut p = Progress::default();
        p.unlock_robot("digger");
        p.unlock_robot("digger");
        assert_eq!(p.unlocked_robots.len(), 1);
    }

    #[test]
    fn mark_level_completed_stores_seed() {
        let mut p = Progress::default();
        p.mark_level_completed("level-one", 42);
        assert!(p.is_level_completed("level-one"));
    }

    #[test]
    fn progress_roundtrips_through_ron() {
        let mut p = Progress::default();
        p.add_dollars(1234);
        p.buy_upgrade("pickaxe");
        p.unlock_robot("digger");
        p.mark_level_completed("level-one", 42);
        p.map_truck_pos = (100.0, 200.0);

        let text = ron::ser::to_string(&p).unwrap();
        let back: Progress = ron::from_str(&text).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn progress_parses_minimal_ron() {
        let p: Progress = ron::from_str("(dollars: 0)").unwrap();
        assert_eq!(p.dollars, 0);
    }

    #[test]
    fn level_seed_creates_entry_on_first_call() {
        let mut p = Progress::default();
        assert_eq!(p.level_seed("level-one", 42), 42);
    }

    #[test]
    fn level_seed_returns_existing_seed() {
        let mut p = Progress::default();
        p.level_seed("level-one", 42);
        assert_eq!(p.level_seed("level-one", 999), 42);
    }

    #[test]
    fn set_level_seed_clears_completion_and_delta_on_change() {
        let mut p = Progress::default();
        p.mark_level_completed("level-one", 42);
        p.set_level_delta("level-one", vec![(5, 13, None)]);

        // Same seed: nothing changes.
        p.set_level_seed("level-one", 42);
        assert!(p.is_level_completed("level-one"));
        assert_eq!(p.level_delta("level-one").len(), 1);

        // Different seed: completed and delta cleared.
        p.set_level_seed("level-one", 99);
        assert!(!p.is_level_completed("level-one"));
        assert!(p.level_delta("level-one").is_empty());
    }

    #[test]
    fn level_delta_defaults_empty() {
        let p = Progress::default();
        assert!(p.level_delta("nonexistent").is_empty());
    }

    #[test]
    fn set_level_delta_noop_without_entry() {
        let mut p = Progress::default();
        p.set_level_delta("nonexistent", vec![(5, 13, None)]);
        // No entry created — the caller must create one first.
        assert!(!p.levels.contains_key("nonexistent"));
    }

    #[test]
    fn level_delta_roundtrips_through_ron() {
        let mut p = Progress::default();
        p.set_level_seed("level-one", 42);
        let delta = vec![
            (5u16, 13u16, None),
            (6u16, 13u16, Some(TileKind::Stone)),
        ];
        p.set_level_delta("level-one", delta.clone());

        let text = ron::ser::to_string(&p).unwrap();
        let back: Progress = ron::from_str(&text).unwrap();
        assert_eq!(back.level_delta("level-one"), delta);
    }

    #[test]
    fn level_save_accepts_missing_delta_field() {
        // Forward compatibility: old saves without a delta parse fine.
        let s: LevelSave = ron::from_str("(seed: 42, completed: false)").unwrap();
        assert!(s.delta.is_empty());
    }
}