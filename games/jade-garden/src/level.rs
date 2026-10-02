//! Level and chapter loading from `assets/*.ron`.
//!
//! Loading is async because on the web target the assets are fetched
//! via HTTP (`macroquad::file::load_string`). On native the same API
//! resolves to a fast synchronous read under the hood, so the only
//! visible change is an `.await` at the call site.

use serde::Deserialize;

use crate::components::Jade;
use crate::paths;

/// Objective for a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Objective {
    /// Reach `target` points.
    Score(u32),
    /// Clear `count` jades of type `jade`.
    ClearJade(Jade, u32),
    /// Complete `count` matches (each match fills 1 "verse").
    FillPoem(u8),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LevelConfig {
    pub id: String,
    pub name: String,
    pub chapter: u32,
    pub moves: u32,
    pub objective: Objective,
    pub star_target: u32,
    pub poem_reveal: u8,
    pub seed: u32,
}

impl Default for LevelConfig {
    fn default() -> Self {
        Self {
            id: "test".into(),
            name: "Test".into(),
            chapter: 0,
            moves: 20,
            objective: Objective::Score(1000),
            star_target: 1000,
            poem_reveal: 0,
            seed: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ChapterConfig {
    pub id: u32,
    pub name: String,
    pub subtitle: String,
    pub intro: String,
}

// ---------- Asset reading ----------
//
// Two implementations to keep `level.rs` testable in isolation:
//
// - Native: `std::fs::read_to_string`, no macroquad context needed.
//   Tests can call `load_levels().await` without a window.
// - Web: `macroquad::file::load_string`, which fetches the file
//   through the browser. Requires a macroquad context, but on web
//   the tests are not run anyway.

#[cfg(not(target_arch = "wasm32"))]
async fn read_asset(relative: &str) -> Result<String, String> {
    let path = paths::asset(relative);
    std::fs::read_to_string(&path).map_err(|e| format!("load {path:?}: {e}"))
}

#[cfg(target_arch = "wasm32")]
async fn read_asset(relative: &str) -> Result<String, String> {
    let path = paths::asset_str(relative);
    macroquad::file::load_string(&path)
        .await
        .map_err(|e| format!("load {path}: {e}"))
}

pub async fn load_levels() -> Result<Vec<LevelConfig>, String> {
    let text = read_asset("levels.ron").await?;
    ron::from_str(&text).map_err(|e| format!("parse levels.ron: {e}"))
}

pub async fn load_chapters() -> Result<Vec<ChapterConfig>, String> {
    let text = read_asset("chapitres.ron").await?;
    ron::from_str(&text).map_err(|e| format!("parse chapitres.ron: {e}"))
}

/// Display label for an objective.
pub fn objective_label(obj: Objective) -> String {
    match obj {
        Objective::Score(n) => format!("Score {n}"),
        Objective::ClearJade(j, n) => format!("Clear {n} {}", j.latin_name()),
        Objective::FillPoem(n) => format!("Trigger {n} cascades"),
    }
}

/// Human-readable requirement for a given star tier (1, 2, or 3).
///
/// For Score levels, all three tiers are score thresholds. For
/// ClearJade and FillPoem levels, tier 1 is objective-based and
/// tiers 2 and 3 remain score thresholds on `star_target`.
pub fn star_tier_label(level: &LevelConfig, tier: u8) -> String {
    match (tier, level.objective) {
        (1, Objective::Score(_)) => level.star_target.to_string(),
        (1, _) => "obj".to_string(),
        (2, _) => ((level.star_target as f32) * 1.5).round().to_string(),
        (3, _) => level.star_target.saturating_mul(2).to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block<F: std::future::Future>(fut: F) -> F::Output {
        pollster::block_on(fut)
    }

    #[test]
    fn default_level_is_valid() {
        let l = LevelConfig::default();
        assert_eq!(l.moves, 20);
        assert_eq!(l.seed, 1);
    }

    #[test]
    fn load_levels_returns_twelve_entries() {
        let levels = block(load_levels()).expect("levels.ron should parse");
        assert_eq!(levels.len(), 12);
    }

    #[test]
    fn load_chapters_returns_four_entries() {
        let chapters = block(load_chapters()).expect("chapitres.ron should parse");
        assert_eq!(chapters.len(), 4);
    }

    #[test]
    fn every_chapter_has_three_levels() {
        let levels = block(load_levels()).unwrap();
        let mut counts = [0u32; 4];
        for l in &levels {
            counts[l.chapter as usize] += 1;
        }
        for (i, c) in counts.iter().enumerate() {
            assert_eq!(*c, 3, "chapter {i} has {c} levels");
        }
    }

    #[test]
    fn poem_reveal_sums_to_twenty() {
        let levels = block(load_levels()).unwrap();
        let total: u32 = levels.iter().map(|l| l.poem_reveal as u32).sum();
        assert_eq!(total, 20);
    }

    #[test]
    fn level_ids_are_unique() {
        let levels = block(load_levels()).unwrap();
        let mut ids: Vec<&str> = levels.iter().map(|l| l.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), levels.len());
    }

    #[test]
    fn objective_label_score() {
        assert_eq!(objective_label(Objective::Score(1000)), "Score 1000");
    }

    #[test]
    fn objective_label_clear_jade() {
        let s = objective_label(Objective::ClearJade(Jade::Bi, 15));
        assert!(s.contains("Bi"));
        assert!(s.contains("15"));
    }

    #[test]
    fn objective_label_fill_poem() {
        assert_eq!(
            objective_label(Objective::FillPoem(4)),
            "Trigger 4 cascades"
        );
    }

    #[test]
    fn star_tier_label_score_level() {
        let lvl = LevelConfig {
            objective: Objective::Score(1000),
            star_target: 1000,
            ..Default::default()
        };
        assert_eq!(star_tier_label(&lvl, 1), "1000");
        assert_eq!(star_tier_label(&lvl, 2), "1500");
        assert_eq!(star_tier_label(&lvl, 3), "2000");
    }

    #[test]
    fn star_tier_label_clear_jade_level() {
        let lvl = LevelConfig {
            objective: Objective::ClearJade(Jade::Bi, 15),
            star_target: 2400,
            ..Default::default()
        };
        assert_eq!(star_tier_label(&lvl, 1), "obj");
        assert_eq!(star_tier_label(&lvl, 2), "3600");
        assert_eq!(star_tier_label(&lvl, 3), "4800");
    }
}