//! Minesweeper persistence — best times per difficulty.

use std::collections::HashMap;
use std::path::PathBuf;

use ember_stdlib::persistence::Persistence;

/// Typed handle to Minesweeper's best-times file.
pub type BestTimes = Persistence<HashMap<String, f32>>;

/// Build the default handle: `best_times.ron` next to Minesweeper's manifest.
pub fn default() -> BestTimes {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    BestTimes::in_manifest_dir(&manifest_dir, "best_times.ron")
}

/// Update the map with a new time for `name` if it beats the existing one.
/// Returns `true` if the map was updated.
pub fn update_if_better(times: &mut HashMap<String, f32>, name: &str, secs: f32) -> bool {
    match times.get_mut(name) {
        Some(current) => ember_stdlib::persistence::update_if_lower(current, secs),
        None => {
            times.insert(name.to_string(), secs);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_if_better_new_key_inserts() {
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
