//! Runtime asset path resolution.
//!
//! Native: paths are anchored to `CARGO_MANIFEST_DIR`, so
//! `cargo run -p accretion` works from anywhere in the workspace.
//! Web: paths are relative to `index.html` (macroquad does a fetch).

use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
pub fn asset(path: &str) -> PathBuf {
    PathBuf::from("assets").join(path)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn asset(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(path)
}

/// Convenience for APIs that take `&str` (e.g. `load_ttf_font`).
pub fn asset_str(path: &str) -> String {
    asset(path).to_string_lossy().into_owned()
}