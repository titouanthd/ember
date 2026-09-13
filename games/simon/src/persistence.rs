//! Simon persistence — typed handle to `best_score.ron`.

use std::path::PathBuf;
use ember_stdlib::persistence::Persistence;

/// Typed handle to Simon's best-score file.
pub type BestScore = Persistence<u32>;

/// Build the default handle: `best_score.ron` next to Simon's manifest.
pub fn default() -> BestScore {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    BestScore::in_manifest_dir(&manifest_dir, "best_score.ron")
}