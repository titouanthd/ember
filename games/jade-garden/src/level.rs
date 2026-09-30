//! Level and chapter loading from `assets/*.ron`.

use std::path::PathBuf;

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

pub fn assets_dir() -> PathBuf {
    paths::assets_dir()
}

pub fn load_levels() -> Result<Vec<LevelConfig>, String> {
    let path = paths::asset("levels.ron");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path:?}: {e}"))?;
    ron::from_str(&text).map_err(|e| format!("parse {path:?}: {e}"))
}

pub fn load_chapters() -> Result<Vec<ChapterConfig>, String> {
    let path = paths::asset("chapitres.ron");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path:?}: {e}"))?;
    ron::from_str(&text).map_err(|e| format!("parse {path:?}: {e}"))
}

/// Display label for an objective.
pub fn objective_label(obj: Objective) -> String {
    match obj {
        Objective::Score(n) => format!("Score {n}"),
        Objective::ClearJade(j, n) => format!("Clear {n} {}", j.latin_name()),
                Objective::FillPoem(n) => format!("Trigger {n} cascades"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_level_is_valid() {
        let l = LevelConfig::default();
        assert_eq!(l.moves, 20);
        assert_eq!(l.seed, 1);
    }

    #[test]
    fn load_levels_returns_twelve_entries() {
        let levels = load_levels().expect("levels.ron should parse");
        assert_eq!(levels.len(), 12);
    }

    #[test]
    fn load_chapters_returns_four_entries() {
        let chapters = load_chapters().expect("chapitres.ron should parse");
        assert_eq!(chapters.len(), 4);
    }

    #[test]
    fn every_chapter_has_three_levels() {
        let levels = load_levels().unwrap();
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
        let levels = load_levels().unwrap();
        let total: u32 = levels.iter().map(|l| l.poem_reveal as u32).sum();
        assert_eq!(total, 20);
    }

    #[test]
    fn level_ids_are_unique() {
        let levels = load_levels().unwrap();
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
}