//! Asteroids persistence — high score.
//!
//! The heavy lifting lives in [`ember_stdlib::persistence`]. This module
//! names the type and the default file for Asteroids.

use std::path::PathBuf;
use ember_stdlib::persistence::Persistence;

/// Typed handle to Asteroids' high-score file.
pub type HighScore = Persistence<i32>;

/// Build the default handle: `highscore.ron` next to Asteroids' manifest.
pub fn default() -> HighScore {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    HighScore::in_manifest_dir(&manifest_dir, "highscore.ron")
}