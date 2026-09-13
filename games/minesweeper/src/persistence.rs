//! Best times persistence. Testable: all functions take a `Path`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Default path next to the crate manifest.
pub fn default_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("best_times.ron")
}

/// Load best times keyed by difficulty name. Missing file or parse error
/// returns an empty map — never panics.
pub fn load_best_times(path: &Path) -> HashMap<String, f32> {
    ember_core::io::load_from_file::<HashMap<String, f32>>(path).unwrap_or_default()
}

/// Save best times. Errors are swallowed (best-effort persistence).
pub fn save_best_times(path: &Path, times: &HashMap<String, f32>) {
    let _ = ember_core::io::save_to_file(path, times);
}

/// Update the map with a new time if it beats the existing one.
/// Returns true if the map was updated.
pub fn update_if_better(times: &mut HashMap<String, f32>, name: &str, secs: f32) -> bool {
    match times.get(name) {
        Some(&current) if current <= secs => false,
        _ => {
            times.insert(name.to_string(), secs);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("minesweeper_test_{}.ron", name))
    }

    #[test]
    fn test_load_missing_returns_empty() {
        let p = temp_path("does_not_exist_xyz");
        let _ = std::fs::remove_file(&p);
        let m = load_best_times(&p);
        assert!(m.is_empty());
    }

    #[test]
    fn test_save_then_load() {
        let p = temp_path("save_load");
        let _ = std::fs::remove_file(&p);

        let mut m = HashMap::new();
        m.insert("Beginner".to_string(), 42.5);
        save_best_times(&p, &m);

        let loaded = load_best_times(&p);
        assert_eq!(loaded.get("Beginner"), Some(&42.5));

        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn test_update_if_better_new() {
        let mut m = HashMap::new();
        assert!(update_if_better(&mut m, "Beginner", 10.0));
        assert_eq!(m.get("Beginner"), Some(&10.0));
    }

    #[test]
    fn test_update_if_better_slower_is_rejected() {
        let mut m = HashMap::new();
        m.insert("Beginner".to_string(), 10.0);
        assert!(!update_if_better(&mut m, "Beginner", 15.0));
        assert_eq!(m.get("Beginner"), Some(&10.0));
    }

    #[test]
    fn test_update_if_better_faster_replaces() {
        let mut m = HashMap::new();
        m.insert("Beginner".to_string(), 10.0);
        assert!(update_if_better(&mut m, "Beginner", 8.0));
        assert_eq!(m.get("Beginner"), Some(&8.0));
    }
}