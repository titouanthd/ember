//! Typed persistence for game save data.
//!
//! Wraps [`ember_core::io`] (RON format) with a `Path` and a default value.
//! Same pattern across Asteroids (high score), Minesweeper (best times),
//! and Simon (best score) — extracted after 3 occurrences (Rule of Three).
//!
//! # Example
//!
//! ```ignore
//! use ember_stdlib::persistence::Persistence;
//!
//! let p: Persistence<u32> = Persistence::in_manifest_dir("best_score.ron");
//! let best = p.load_or(0);
//! p.save(&(best + 1));
//! ```

use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// A typed handle to a persisted value on disk.
///
/// `T` is the type of the value. `Persistence<u32>` and `Persistence<i32>`
/// are distinct types, so the compiler checks you load/save the right thing.
pub struct Persistence<T> {
    path: PathBuf,
    _marker: PhantomData<T>,
}

impl<T> Clone for Persistence<T> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> Persistence<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Build a handle from an explicit path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            _marker: PhantomData,
        }
    }

    /// Build a handle for a file next to the crate manifest.
    ///
    /// Must be called from a game crate: `env!("CARGO_MANIFEST_DIR")` is
    /// resolved at the call site (same gotcha as `config::load_dotenv_once`).
    pub fn in_manifest_dir(filename: &str) -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(filename);
        Self::new(path)
    }

    /// Load the value from disk. Returns `None` if the file is missing or
    /// fails to parse.
    pub fn load(&self) -> Option<T> {
        ember_core::io::load_from_file::<T>(&self.path).ok()
    }

    /// Load the value, or return `default` if missing or invalid.
    pub fn load_or(&self, default: T) -> T {
        self.load().unwrap_or(default)
    }

    /// Save the value to disk. Best-effort: returns `true` on success,
    /// `false` if the write failed. Errors are not surfaced (matches the
    /// previous per-game implementations).
    pub fn save(&self, value: &T) -> bool {
        ember_core::io::save_to_file(&self.path, value).is_ok()
    }

    /// Path to the backing file. Useful for tests and diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

// ---------------------------------------------------------------------------
// Record helpers
// ---------------------------------------------------------------------------

/// If `candidate > *current`, update `*current` and return `true`.
/// Used for "higher is better" records (score, waves cleared).
pub fn update_if_higher<T: PartialOrd + Copy>(current: &mut T, candidate: T) -> bool {
    if candidate > *current {
        *current = candidate;
        true
    } else {
        false
    }
}

/// If `candidate < *current`, update `*current` and return `true`.
/// Used for "lower is better" records (best time).
pub fn update_if_lower<T: PartialOrd + Copy>(current: &mut T, candidate: T) -> bool {
    if candidate < *current {
        *current = candidate;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("ember_persistence_test_{name}.ron"))
    }

    // --- Persistence<u32> round-trip ---

    #[test]
    fn test_load_missing_returns_none() {
        let p: Persistence<u32> = Persistence::new(temp_path("missing"));
        let _ = std::fs::remove_file(p.path());
        assert_eq!(p.load(), None);
    }

    #[test]
    fn test_load_or_missing_returns_default() {
        let p: Persistence<u32> = Persistence::new(temp_path("load_or_missing"));
        let _ = std::fs::remove_file(p.path());
        assert_eq!(p.load_or(42), 42);
    }

    #[test]
    fn test_save_then_load_u32() {
        let p: Persistence<u32> = Persistence::new(temp_path("save_load_u32"));
        let _ = std::fs::remove_file(p.path());

        assert!(p.save(&1234));
        assert_eq!(p.load(), Some(1234));

        let _ = std::fs::remove_file(p.path());
    }

    #[test]
    fn test_save_then_load_i32() {
        let p: Persistence<i32> = Persistence::new(temp_path("save_load_i32"));
        let _ = std::fs::remove_file(p.path());

        assert!(p.save(&-42));
        assert_eq!(p.load(), Some(-42));

        let _ = std::fs::remove_file(p.path());
    }

    #[test]
    fn test_save_then_load_map() {
        use std::collections::HashMap;
        let p: Persistence<HashMap<String, f32>> = Persistence::new(temp_path("save_load_map"));
        let _ = std::fs::remove_file(p.path());

        let mut m = HashMap::new();
        m.insert("Beginner".to_string(), 42.5);
        m.insert("Expert".to_string(), 99.9);

        assert!(p.save(&m));
        let loaded = p.load().expect("should load");
        assert_eq!(loaded.get("Beginner"), Some(&42.5));
        assert_eq!(loaded.get("Expert"), Some(&99.9));

        let _ = std::fs::remove_file(p.path());
    }

    #[test]
    fn test_load_invalid_file_returns_none() {
        let p: Persistence<u32> = Persistence::new(temp_path("invalid"));
        // Write garbage that isn't valid RON for a u32.
        std::fs::write(p.path(), "not a number").expect("write garbage");
        assert_eq!(p.load(), None);
        let _ = std::fs::remove_file(p.path());
    }

    #[test]
    fn test_path_exposed() {
        let p: Persistence<u32> = Persistence::new("/tmp/foo.ron");
        assert_eq!(p.path(), Path::new("/tmp/foo.ron"));
    }

    // --- update_if_higher ---

    #[test]
    fn test_update_if_higher_new_record() {
        let mut current = 10;
        assert!(update_if_higher(&mut current, 20));
        assert_eq!(current, 20);
    }

    #[test]
    fn test_update_if_higher_lower_is_rejected() {
        let mut current = 10;
        assert!(!update_if_higher(&mut current, 5));
        assert_eq!(current, 10);
    }

    #[test]
    fn test_update_if_higher_equal_is_rejected() {
        let mut current = 10;
        assert!(!update_if_higher(&mut current, 10));
        assert_eq!(current, 10);
    }

    // --- update_if_lower ---

    #[test]
    fn test_update_if_lower_new_record() {
        let mut current = 100.0f32;
        assert!(update_if_lower(&mut current, 42.5));
        assert_eq!(current, 42.5);
    }

    #[test]
    fn test_update_if_lower_higher_is_rejected() {
        let mut current = 10.0f32;
        assert!(!update_if_lower(&mut current, 15.0));
        assert_eq!(current, 10.0);
    }

    #[test]
    fn test_update_if_lower_equal_is_rejected() {
        let mut current = 10.0f32;
        assert!(!update_if_lower(&mut current, 10.0));
        assert_eq!(current, 10.0);
    }

    #[test]
    fn test_clone_preserves_path() {
        let p: Persistence<u32> = Persistence::new("/tmp/foo.ron");
        let q = p.clone();
        assert_eq!(p.path(), q.path());
    }
}