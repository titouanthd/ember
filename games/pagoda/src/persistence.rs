//! Pagoda persistence — typed handle to `best_score.ron`.
//!
//! The heavy lifting lives in [`ember_stdlib::persistence`]. Native builds
//! write next to the game's manifest; on WASM the save call fails silently,
//! which leaves the best score in memory for the session.

use ember_stdlib::persistence::Persistence;
use std::path::PathBuf;

/// Typed handle to Pagoda's best-score file.
pub type BestScore = Persistence<u32>;

/// Build the default handle: `best_score.ron` next to Pagoda's manifest.
pub fn default() -> BestScore {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    BestScore::in_manifest_dir(&manifest_dir, "best_score.ron")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn default_points_at_best_score_in_the_game_folder() {
        let path = default();
        assert_eq!(
            path.path(),
            Path::new(env!("CARGO_MANIFEST_DIR")).join("best_score.ron")
        );
    }
}
