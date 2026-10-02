//! Persistence of player progress.
//!
//! **Native**: reads/writes `progress.ron` in the user-data directory.
//!
//! **Web**: uses an in-memory `thread_local` for now. Persistence
//! doesn't survive a page reload. A proper `localStorage` bridge will
//! come later via a miniquad plugin (wasm-bindgen is incompatible
//! with miniquad's own import system).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::poem;

// ---------- Data types ----------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    pub best_scores: HashMap<String, u32>,
    pub stars: HashMap<String, u8>,
    pub poem_fragments: u8,
    #[serde(default)]
    pub epilogue_seen: bool,
    #[serde(default)]
    pub halloween_seen: bool,
    #[serde(default)]
    pub best_times: HashMap<String, LevelTimes>,
}

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

#[derive(Debug, Clone, Default)]
pub struct RunTimes {
    pub to_1_star: Option<f32>,
    pub to_2_star: Option<f32>,
    pub to_3_star: Option<f32>,
    pub peak_time: f32,
    pub peak_score: u32,
    pub total: f32,
}

impl Progress {
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

    pub fn record_win_with_times(
        &mut self,
        level_id: &str,
        score: u32,
        stars: u8,
        poem_reveal: u8,
        times: RunTimes,
    ) {
        let was_new = !self.stars.contains_key(level_id);

        let prev_best = self.best_scores.get(level_id).copied().unwrap_or(0);
        let beat_best = score > prev_best;
        if beat_best {
            self.best_scores.insert(level_id.to_string(), score);
        }

        let s = self.stars.entry(level_id.to_string()).or_insert(0);
        if stars > *s {
            *s = stars;
        }

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

        if was_new {
            self.poem_fragments = self
                .poem_fragments
                .saturating_add(poem_reveal)
                .min(poem::TOTAL_CHARS);
        }
    }

    pub fn best_time_for_score(&self, id: &str) -> Option<f32> {
        self.best_times.get(id).and_then(|t| t.best_score_time)
    }

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

// ---------- Load / Save ----------

pub fn load_progress() -> Progress {
    match load_raw() {
        Ok(Some(text)) => match ron::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("⚠️  Corrupted progress file ({e}); starting fresh.");
                Progress::default()
            }
        },
        Ok(None) => Progress::default(),
        Err(e) => {
            eprintln!("⚠️  Could not read progress ({e}); starting fresh.");
            Progress::default()
        }
    }
}

pub fn save_progress(progress: &Progress) -> Result<(), String> {
    let text = ron::ser::to_string(progress)
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
    std::fs::read_to_string(&path).map(Some).map_err(|e| format!("{e}"))
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
    /// In-memory fallback for wasm. Persistence won't survive a page
    /// reload until we add a proper localStorage bridge via a
    /// miniquad plugin.
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
        assert_eq!(p.poem_fragments, 2);
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
    fn progress_parses_without_new_flags() {
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
        assert_eq!(t.time_1_star, Some(30.0));
        assert_eq!(t.time_2_star, Some(90.0));
        assert_eq!(t.best_score_time, Some(130.0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn progress_path_ends_with_progress_ron() {
        if std::env::var_os("JADE_GARDEN_PROGRESS").is_some() {
            return;
        }
        assert!(crate::paths::progress_path().ends_with("progress.ron"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn save_and_load_roundtrip_on_disk() {
        let tmp = std::env::temp_dir()
            .join(format!("jade-garden-progress-test-{}.ron", std::process::id()));

        // SAFETY: this test is the only writer to this env var.
        // We set it and immediately use it — no concurrent reader.
        unsafe {
            std::env::set_var("JADE_GARDEN_PROGRESS", &tmp);
        }

        let mut p = Progress::default();
        p.record_win("l01", 1234, 2, 2);
        save_progress(&p).expect("save should succeed");

        let back = load_progress();
        assert_eq!(back.level_best_score("l01"), 1234);

        unsafe {
            std::env::remove_var("JADE_GARDEN_PROGRESS");
        }
        let _ = std::fs::remove_file(&tmp);
    }
}