//! Best score persistence. Testable: all functions take a `Path`.

use std::path::{Path, PathBuf};

/// Default path next to the crate manifest.
pub fn default_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("best_score.ron")
}

/// Load the best score. Missing file or parse error returns 0.
pub fn load_best(path: &Path) -> u32 {
    ember_core::io::load_from_file::<u32>(path).unwrap_or(0)
}

/// Save the best score. Errors are swallowed (best-effort persistence).
pub fn save_best(path: &Path, best: u32) {
    let _ = ember_core::io::save_to_file(path, &best);
}

/// If `score` beats `current`, return `score`. Otherwise `current`.
pub fn max_best(current: u32, score: u32) -> u32 {
    current.max(score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("simon_test_{}.ron", name))
    }

    #[test]
    fn test_load_missing_returns_zero() {
        let p = temp_path("does_not_exist_xyz");
        let _ = std::fs::remove_file(&p);
        assert_eq!(load_best(&p), 0);
    }

    #[test]
    fn test_save_then_load() {
        let p = temp_path("save_load");
        let _ = std::fs::remove_file(&p);
        save_best(&p, 42);
        assert_eq!(load_best(&p), 42);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn test_max_best_new_higher() {
        assert_eq!(max_best(3, 7), 7);
    }

    #[test]
    fn test_max_best_keeps_higher() {
        assert_eq!(max_best(10, 4), 10);
    }

    #[test]
    fn test_max_best_equal() {
        assert_eq!(max_best(5, 5), 5);
    }
}