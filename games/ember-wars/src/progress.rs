//! Progression du joueur — étoiles par niveau + arbre d'upgrades.

use std::collections::HashMap;
use std::path::Path;

use ember_stdlib::persistence::Persistence;
use serde::{Deserialize, Serialize};

use crate::config::{all_chapters, LevelConfig};
use crate::upgrades::UpgradeTree;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    /// Étoiles par niveau (0 à 3). Absent = niveau jamais complété.
    pub stars: HashMap<String, u8>,
    /// Arbre d'upgrades persisté (bank gold + nœuds achetés).
    pub tree: UpgradeTree,
}

impl Progress {
    pub fn stars_for(&self, level_id: &str) -> u8 {
        self.stars.get(level_id).copied().unwrap_or(0)
    }

    pub fn is_completed(&self, level_id: &str) -> bool {
        self.stars_for(level_id) >= 1
    }

    /// Enregistre une victoire : garde le meilleur score, crédite la
    /// bank d'or.
    pub fn record_win(&mut self, level_id: &str, stars: u8, gold_earned: f32) {
        let prev = self.stars_for(level_id);
        if stars > prev {
            self.stars.insert(level_id.to_string(), stars);
        }
        self.tree.gold += gold_earned;
    }

    /// Liste à plat des niveaux dans l'ordre linéaire.
    pub fn flat_levels() -> Vec<&'static LevelConfig> {
        all_chapters()
            .iter()
            .flat_map(|c| c.levels.iter())
            .collect()
    }

    /// Un niveau est débloqué si c'est le premier, ou si le précédent
    /// (dans l'ordre linéaire) a été complété.
    pub fn is_level_unlocked(&self, level_id: &str) -> bool {
        let flat = Self::flat_levels();
        let pos = match flat.iter().position(|l| l.id == level_id) {
            Some(p) => p,
            None => return false,
        };
        if pos == 0 {
            return true;
        }
        self.is_completed(&flat[pos - 1].id)
    }

    /// Un chapitre est débloqué si le précédent a tous ses niveaux
    /// complétés (≥1 étoile).
    pub fn is_chapter_unlocked(&self, chapter_idx: usize) -> bool {
        if chapter_idx == 0 {
            return true;
        }
        let chapters = all_chapters();
        let prev = match chapters.get(chapter_idx - 1) {
            Some(c) => c,
            None => return false,
        };
        prev.levels.iter().all(|l| self.is_completed(&l.id))
    }
}

/// Calcule le nombre d'étoiles (1, 2 ou 3) pour une partie gagnée.
///
/// - 1⭐ : victoire
/// - 2⭐ : + tower HP ≥ 50 %
/// - 3⭐ : + tower HP ≥ 90 %
pub fn stars_for_victory(tower_hp_fraction: f32) -> u8 {
    if tower_hp_fraction >= 0.90 {
        3
    } else if tower_hp_fraction >= 0.50 {
        2
    } else {
        1
    }
}

/// Charge la progression. `UpgradeTree` est fixé pour avoir la racine
/// si le fichier était vide ou ancien.
pub fn load(manifest_dir: &Path) -> (Progress, Persistence<Progress>) {
    let store = Persistence::in_manifest_dir(manifest_dir, "progress.ron");
    let mut progress: Progress = store.load().unwrap_or_default();
    progress.tree.ensure_root();
    (progress, store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_progress_has_no_stars() {
        let p = Progress::default();
        assert_eq!(p.stars_for("sh_01"), 0);
        assert!(!p.is_completed("sh_01"));
    }

    #[test]
    fn record_win_credits_gold() {
        let mut p = Progress::default();
        p.record_win("sh_01", 2, 150.0);
        assert!((p.tree.gold - 150.0).abs() < 1e-6);
    }

    #[test]
    fn record_win_keeps_best_stars() {
        let mut p = Progress::default();
        p.record_win("sh_01", 3, 100.0);
        p.record_win("sh_01", 1, 100.0);
        assert_eq!(p.stars_for("sh_01"), 3);
    }

    #[test]
    fn record_win_upgrades_stars() {
        let mut p = Progress::default();
        p.record_win("sh_01", 1, 0.0);
        p.record_win("sh_01", 2, 0.0);
        assert_eq!(p.stars_for("sh_01"), 2);
    }

    #[test]
    fn first_level_unlocked() {
        let p = Progress::default();
        assert!(p.is_level_unlocked("sh_01"));
    }

    #[test]
    fn second_level_locked_without_first_completed() {
        let p = Progress::default();
        assert!(!p.is_level_unlocked("sh_02"));
    }

    #[test]
    fn second_level_unlocked_after_first() {
        let mut p = Progress::default();
        p.record_win("sh_01", 1, 0.0);
        assert!(p.is_level_unlocked("sh_02"));
    }

    #[test]
    fn chapter_2_locked_without_chapter_1_completed() {
        let p = Progress::default();
        assert!(!p.is_chapter_unlocked(1));
    }

    #[test]
    fn chapter_2_unlocked_after_chapter_1_all_completed() {
        let mut p = Progress::default();
        p.record_win("sh_01", 1, 0.0);
        p.record_win("sh_02", 1, 0.0);
        p.record_win("sh_03", 1, 0.0);
        assert!(p.is_chapter_unlocked(1));
    }

    #[test]
    fn chapter_2_locked_if_one_level_missing() {
        let mut p = Progress::default();
        p.record_win("sh_01", 1, 0.0);
        p.record_win("sh_02", 1, 0.0);
        // sh_03 pas fini.
        assert!(!p.is_chapter_unlocked(1));
    }

    #[test]
    fn chapter_1_always_unlocked() {
        let p = Progress::default();
        assert!(p.is_chapter_unlocked(0));
    }

    #[test]
    fn stars_calculation_3() {
        assert_eq!(stars_for_victory(0.95), 3);
    }

    #[test]
    fn stars_calculation_2() {
        assert_eq!(stars_for_victory(0.65), 2);
    }

    #[test]
    fn stars_calculation_1() {
        assert_eq!(stars_for_victory(0.30), 1);
    }

    #[test]
    fn flat_levels_has_six_entries() {
        assert_eq!(Progress::flat_levels().len(), 6);
    }
}