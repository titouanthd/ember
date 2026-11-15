//! Runtime asset path resolution.
//!
//! **Native**: look for `assets/` next to the executable first (release
//! builds), then fall back to the source tree via `CARGO_MANIFEST_DIR`.
//!
//! **Web**: assets sit next to `index.html` and are fetched via relative
//! URLs, so the native filesystem logic is bypassed.

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
pub fn assets_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PAGODA_ASSETS") {
        return PathBuf::from(dir);
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let candidate = exe_dir.join("assets");
        if candidate.is_dir() {
            return candidate;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// On WASM, assets are fetched relative to the page.
#[cfg(target_arch = "wasm32")]
pub fn assets_dir() -> PathBuf {
    PathBuf::from("assets")
}

pub fn asset(relative: &str) -> PathBuf {
    assets_dir().join(relative)
}

/// Resolve an asset to a string ready for macroquad's loaders
/// (`load_ttf_font`, `load_string`, …).
pub fn asset_str(relative: &str) -> String {
    asset(relative).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_joins_relative_path() {
        let p = asset("DejaVuSansMono.ttf");
        assert!(p.ends_with("DejaVuSansMono.ttf"));
        assert!(p.parent().unwrap().ends_with("assets"));
    }

    #[test]
    fn asset_str_is_usable() {
        let s = asset_str("DejaVuSansMono.ttf");
        assert!(s.ends_with("DejaVuSansMono.ttf"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn bundled_font_is_present_in_the_source_tree() {
        assert!(asset("DejaVuSansMono.ttf").is_file());
    }
}
