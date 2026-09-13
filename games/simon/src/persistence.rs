//! Simon persistence — typed handle to `best_score.ron`.
//!
//! The heavy lifting lives in [`ember_stdlib::persistence`]. This module
//! just names the type and the default file for Simon.

use ember_stdlib::persistence::Persistence;

/// Typed handle to Simon's best-score file.
pub type BestScore = Persistence<u32>;

/// Build the default handle: `best_score.ron` next to the manifest.
pub fn default() -> BestScore {
    BestScore::in_manifest_dir("best_score.ron")
}