//! Persistence of player progress (`progress.ron`).
//!
//! The file is gitignored. It stores best scores, stars per level, and
//! poem progress.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths;
use crate::poem;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    pub best_scores: HashMap<String, u32>,
    pub stars: HashMap<String, u8>,
    /// Number of poem characters revealed (0-20).
    pub poem_fragments: u8,
    /// True once the player has watched the epilogue cinematic.
    #[serde(default)]
    pub epilogue_seen: bool,
    /// True once the player has watched the Halloween teaser.
    #[serde(default)]
    pub halloween_seen: bool,
   /// Best times per level. `#[serde(default)]` so old files load.
    #[serde(default)]
    pub best_times: HashMap<String, LevelTimes>,
}

/// Best times achieved on a level, in seconds.
///
/// `None` means "not set yet" (level never reached that threshold).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelTimes {
    #[serde(default)]
    pub time_1_star: Option<f32>,
    #[serde(default)]
    pub time_2_star: Option<f32>,
    #[serde(default)]
    pub time_3_star: Option<f32>,
    #[serde(default)]
    pub best_score_time: Option<f32>,
}

/// Times recorded during a single run, passed to `record_win`.
#[derive(Debug, Clone, Default)]
pub struct RunTimes {
    /// Time at which the run first crossed each star threshold.
    pub to_1_star: Option<f32>,
    pub to_2_star: Option<f32>,
    pub to_3_star: Option<f32>,
    /// Time at which the run achieved its peak score.
    pub peak_time: f32,
    /// The peak score the run achieved.
    pub peak_score: u32,
    /// Total elapsed time of the run.
    pub total: f32,
}

impl Progress {
    /// Records a win. Kept for backward compatibility — use
    /// [`record_win_with_times`] for full timing info.
    pub fn record_win(
        &mut self,
        level_id: &str,
        score: u32,
        stars: u8,
        poem_reveal: u8,
    ) {
        self.record_win_with_times(
            level_id,
            score,
            stars,
            poem_reveal,
            RunTimes::default(),
        );
    }

    /// Records a win with timing info.
    pub fn record_win_with_times(
        &mut self,
        level_id: &str,
        score: u32,
        stars: u8,
        poem_reveal: u8,
        times: RunTimes,
    ) {
        let was_new = !self.stars.contains_key(level_id);

        // ── Best score ──
        let prev_best = self.best_scores.get(level_id).copied().unwrap_or(0);
        let beat_best = score > prev_best;
        if beat_best {
            self.best_scores.insert(level_id.to_string(), score);
        }

        // ── Stars ──
        let s = self.stars.entry(level_id.to_string()).or_insert(0);
        if stars > *s {
            *s = stars;
        }

        // ── Times ──
        let entry = self
            .best_times
            .entry(level_id.to_string())
            .or_default();

        fn update_min(field: &mut Option<f32>, new: Option<f32>) {
            if let Some(t) = new {
                *field = Some(field.map_or(t, |prev| prev.min(t)));
            }
        }
        update_min(&mut entry.time_1_star, times.to_1_star);
        update_min(&mut entry.time_2_star, times.to_2_star);
        update_min(&mut entry.time_3_star, times.to_3_star);

        if beat_best && times.peak_score > 0 {
            entry.best_score_time = Some(times.peak_time);
        }

        // ── Poem fragments (first win only) ──
        if was_new {
            self.poem_fragments = self
                .poem_fragments
                .saturating_add(poem_reveal)
                .min(poem::TOTAL_CHARS);
        }
    }

    /// Best time (seconds) at which the best score was achieved.
    pub fn best_time_for_score(&self, id: &str) -> Option<f32> {
        self.best_times.get(id).and_then(|t| t.best_score_time)
    }

    /// Full `LevelTimes` for a level, if any.
    pub fn level_times(&self, id: &str) -> Option<&LevelTimes> {
        self.best_times.get(id)
    }

    pub fn level_stars(&self, id: &str) -> u8 {
        self.stars.get(id).copied().unwrap_or(0)
    }

    pub fn level_best_score(&self, id: &str) -> u32 {
        self.best_scores.get(id).copied().unwrap_or(0)
    }

    pub fn level_completed(&self, id: &str) -> bool {
        self.level_stars(id) > 0
    }

    pub fn total_stars(&self) -> u32 {
        self.stars.values().map(|&s| s as u32).sum()
    }

    pub fn reset(&mut self) {
        self.best_scores.clear();
        self.stars.clear();
        self.poem_fragments = 0;
        self.epilogue_seen = false;
        self.halloween_seen = false;
    }
}

pub fn progress_path() -> PathBuf {
    paths::progress_path()
}

pub fn load_progress() -> Progress {
    let path = progress_path();
    if !path.exists() {
        return Progress::default();
    }
    match ember_core::io::load_from_file(&path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "⚠️  Could not read {} ({e}). Starting with empty progress.",
                path.display()
            );
            Progress::default()
        }
    }
}

pub fn save_progress(progress: &Progress) -> Result<(), String> {
    let path = progress_path();
    paths::ensure_parent_dir(&path)?;
    ember_core::io::save_to_file(&path, progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_progress_is_empty() {
        let p = Progress::default();
        assert_eq!(p.poem_fragments, 0);
        assert!(p.stars.is_empty());
    }

    #[test]
    fn record_win_sets_best_score() {
        let mut p = Progress::default();
        p.record_win("l01", 1000, 1, 2);
        assert_eq!(p.level_best_score("l01"), 1000);
    }

    #[test]
    fn record_win_keeps_higher_score() {
        let mut p = Progress::default();
        p.record_win("l01", 1000, 1, 2);
        p.record_win("l01", 800, 1, 2);
        assert_eq!(p.level_best_score("l01"), 1000);
    }

    #[test]
    fn record_win_updates_higher_score() {
        let mut p = Progress::default();
        p.record_win("l01", 1000, 1, 2);
        p.record_win("l01", 1500, 2, 2);
        assert_eq!(p.level_best_score("l01"), 1500);
    }

    #[test]
    fn record_win_reveals_poem_only_once() {
        let mut p = Progress::default();
        p.record_win("l01", 1000, 1, 2);
        assert_eq!(p.poem_fragments, 2);
        p.record_win("l01", 2000, 3, 2);
        assert_eq!(p.poem_fragments, 2, "replay must not add fragments");
    }

    #[test]
    fn poem_fragments_capped_at_total() {
        let mut p = Progress::default();
        for i in 0..30 {
            p.record_win(&format!("l{i}"), 1000, 1, 5);
        }
        assert_eq!(p.poem_fragments, poem::TOTAL_CHARS);
    }

    #[test]
    fn total_stars_sums_all_levels() {
        let mut p = Progress::default();
        p.record_win("l01", 1000, 3, 0);
        p.record_win("l02", 1000, 2, 0);
        p.record_win("l03", 1000, 1, 0);
        assert_eq!(p.total_stars(), 6);
    }

    #[test]
    fn level_completed_true_after_win() {
        let mut p = Progress::default();
        assert!(!p.level_completed("l01"));
        p.record_win("l01", 1000, 1, 2);
        assert!(p.level_completed("l01"));
    }

    #[test]
    fn reset_clears_everything() {
        let mut p = Progress::default();
        p.record_win("l01", 1000, 3, 5);
        p.reset();
        assert_eq!(p.total_stars(), 0);
        assert_eq!(p.poem_fragments, 0);
        assert!(p.best_scores.is_empty());
    }

    #[test]
    fn progress_roundtrips_through_ron() {
        let mut p = Progress::default();
        p.record_win("l01", 1234, 2, 2);
        p.record_win("l02", 5678, 3, 2);

        let text = ron::ser::to_string(&p).unwrap();
        let back: Progress = ron::from_str(&text).unwrap();
        assert_eq!(back.poem_fragments, p.poem_fragments);
        assert_eq!(back.level_best_score("l01"), 1234);
        assert_eq!(back.level_stars("l02"), 3);
    }

    #[test]
    fn progress_parses_without_epilogue_seen_field() {
        // Old progress files don't have `epilogue_seen`.
        let ron = r#"(
            best_scores: {},
            stars: {},
            poem_fragments: 0,
        )"#;
        let p: Progress = ron::from_str(ron).unwrap();
        assert!(!p.epilogue_seen);
    }

    #[test]
    fn progress_parses_without_new_flags() {
        // Old progress files lack epilogue_seen / halloween_seen.
        let ron = r#"(
            best_scores: {},
            stars: {},
            poem_fragments: 0,
        )"#;
        let p: Progress = ron::from_str(ron).unwrap();
        assert!(!p.epilogue_seen);
        assert!(!p.halloween_seen);
    }

    #[test]
    fn record_win_with_times_stores_star_times() {
        let mut p = Progress::default();
        let times = RunTimes {
            to_1_star: Some(45.0),
            to_2_star: Some(72.0),
            to_3_star: Some(110.0),
            peak_time: 95.0,
            peak_score: 3500,
            total: 120.0,
        };
        p.record_win_with_times("l01", 3500, 3, 2, times);
        let t = p.level_times("l01").unwrap();
        assert_eq!(t.time_1_star, Some(45.0));
        assert_eq!(t.time_2_star, Some(72.0));
        assert_eq!(t.time_3_star, Some(110.0));
        assert_eq!(t.best_score_time, Some(95.0));
    }

    #[test]
    fn time_records_keep_the_fastest() {
        let mut p = Progress::default();
        // First run: slow.
        p.record_win_with_times(
            "l01", 3500, 3, 2,
            RunTimes {
                to_1_star: Some(80.0),
                to_2_star: Some(110.0),
                to_3_star: Some(150.0),
                peak_time: 130.0,
                peak_score: 3500,
                total: 170.0,
            },
        );
        // Second run: same stars, faster 1-star, but slower score
        // (so best score time stays the first run's).
        p.record_win_with_times(
            "l01", 3400, 3, 2,
            RunTimes {
                to_1_star: Some(30.0),
                to_2_star: Some(90.0),
                to_3_star: Some(140.0),
                peak_time: 100.0,
                peak_score: 3400,
                total: 160.0,
            },
        );
        let t = p.level_times("l01").unwrap();
        assert_eq!(t.time_1_star, Some(30.0), "kept the faster 1-star");
        assert_eq!(t.time_2_star, Some(90.0), "kept the faster 2-star");
        assert_eq!(
            t.best_score_time,
            Some(130.0),
            "kept the score time from the run that actually scored higher"
        );
    }

    #[test]
    fn progress_roundtrips_with_every_field_populated() {
        let mut p = Progress::default();
        p.record_win_with_times(
            "l01",
            3000,
            3,
            2,
            RunTimes {
                to_1_star: Some(30.0),
                to_2_star: Some(60.0),
                to_3_star: Some(90.0),
                peak_time: 85.0,
                peak_score: 3000,
                total: 120.0,
            },
        );
        p.record_win_with_times(
            "l02",
            2500,
            2,
            2,
            RunTimes {
                to_1_star: Some(40.0),
                to_2_star: Some(80.0),
                to_3_star: None,
                peak_time: 75.0,
                peak_score: 2500,
                total: 100.0,
            },
        );
        p.epilogue_seen = true;
        p.halloween_seen = true;

        let text = ron::ser::to_string_pretty(&p, ron::ser::PrettyConfig::default()).unwrap();
        let back: Progress = ron::from_str(&text).unwrap();

        assert_eq!(back.poem_fragments, p.poem_fragments);
        assert_eq!(back.epilogue_seen, p.epilogue_seen);
        assert_eq!(back.halloween_seen, p.halloween_seen);
        assert_eq!(back.total_stars(), p.total_stars());

        for id in ["l01", "l02"] {
            assert_eq!(back.level_best_score(id), p.level_best_score(id));
            assert_eq!(back.level_stars(id), p.level_stars(id));
            let a = back.level_times(id).expect("times should roundtrip");
            let b = p.level_times(id).unwrap();
            assert_eq!(a.time_1_star, b.time_1_star);
            assert_eq!(a.time_2_star, b.time_2_star);
            assert_eq!(a.time_3_star, b.time_3_star);
            assert_eq!(a.best_score_time, b.best_score_time);
        }
    }
}