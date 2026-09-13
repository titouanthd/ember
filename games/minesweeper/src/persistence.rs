//! Minesweeper persistence — best times per difficulty.
//!
//! The heavy lifting lives in [`ember_stdlib::persistence`]. This module
//! names the type, the default file, and the "lower is better" record
//! logic for the `HashMap<String, f32>` shape.

use std::collections::HashMap;

use ember_stdlib::persistence::Persistence;

/// Typed handle to Minesweeper's best-times file.
pub type BestTimes = Persistence<HashMap<String, f32>>;

/// Build the default handle: `best_times.ron` next to the manifest.
pub fn default() -> BestTimes {
    BestTimes::in_manifest_dir("best_times.ron")
}

/// Update the map with a new time for `name` if it beats the existing one.
/// Returns `true` if the map was updated.
///
/// A faster time is "better" (lower is better), so this uses
/// [`ember_stdlib::persistence::update_if_lower`] once the entry exists.
/// Inserts the entry if missing.
pub fn update_if_better(
    times: &mut HashMap<String, f32>,
    name: &str,
    secs: f32,
) -> bool {
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