//! FreeCell persistence — best times per seed.

use std::collections::HashMap;
use std::path::PathBuf;

use ember_stdlib::persistence::Persistence;

/// Typed handle to FreeCell's best-times file.
pub type BestTimes = Persistence<HashMap<String, f32>>;

/// Build the default handle: `best_times.ron` next to FreeCell's manifest.
pub fn default() -> BestTimes {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    BestTimes::in_manifest_dir(&manifest_dir, "best_times.ron")
}