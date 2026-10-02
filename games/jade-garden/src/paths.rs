//! Runtime path resolution.
//!
//! **Native**: assets and progress are resolved via a cascade of
//! filesystem lookups (env var → exe dir → source tree). The
//! progress file is written to disk.
//!
//! **Web**: assets are fetched via relative URLs from the page.
//! There is no filesystem, so all the native resolution logic is
//! bypassed. Progress uses an in-memory fallback (see `progress.rs`).

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
const APP_DIR_NAME: &str = "jade-garden";

// ---------- Assets ----------

#[cfg(not(target_arch = "wasm32"))]
pub fn assets_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("JADE_GARDEN_ASSETS") {
        return PathBuf::from(dir);
    }

    if let Some(exe_dir) = current_exe_dir() {
        let candidate = exe_dir.join("assets");
        if candidate.is_dir() {
            return candidate;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// On wasm, assets are fetched via relative URLs from the page. The
/// `dist/web/` bundle places them at `assets/` next to `index.html`.
#[cfg(target_arch = "wasm32")]
pub fn assets_dir() -> PathBuf {
    PathBuf::from("assets")
}

pub fn asset(relative: &str) -> PathBuf {
    assets_dir().join(relative)
}

/// Returns the asset path as a plain string, ready to be passed to
/// `macroquad::file::load_string`, `load_ttf_font`, etc. On native
/// this is a filesystem path; on web it's a URL relative to the page.
pub fn asset_str(relative: &str) -> String {
    asset(relative).to_string_lossy().into_owned()
}

// ---------- Progress file (native only) ----------

#[cfg(not(target_arch = "wasm32"))]
pub fn progress_path() -> PathBuf {
    if let Some(p) = std::env::var_os("JADE_GARDEN_PROGRESS") {
        return PathBuf::from(p);
    }

    if let Some(dir) = user_data_dir() {
        return dir.join("progress.ron");
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("progress.ron")
}

/// Ensures the parent directory of `path` exists. Returns an error
/// string suitable for logging if it can't be created.
#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() || parent.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("could not create {}: {e}", parent.display()))
}

// ---------- Native helpers ----------

#[cfg(not(target_arch = "wasm32"))]
fn current_exe_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent().map(PathBuf::from)
}

#[cfg(not(target_arch = "wasm32"))]
fn user_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library/Application Support")
                .join(APP_DIR_NAME)
        })
    }

    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
            })
            .map(|base| base.join(APP_DIR_NAME))
    }

    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(APP_DIR_NAME))
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    )))]
    {
        None
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_dir_falls_back_to_manifest_when_no_exe_assets() {
        let dir = assets_dir();
        assert!(dir.ends_with("assets"), "got {}", dir.display());
        assert!(dir.is_dir(), "assets dir should exist: {}", dir.display());
        assert!(
            dir.join("levels.ron").is_file(),
            "levels.ron should be present in {}",
            dir.display()
        );
    }

    #[test]
    fn asset_helper_joins_relative() {
        let p = asset("levels.ron");
        assert!(p.ends_with("levels.ron"));
        assert!(p.parent().unwrap().ends_with("assets"));
    }

    #[test]
    fn asset_str_returns_a_usable_string() {
        let s = asset_str("levels.ron");
        assert!(s.ends_with("levels.ron"), "got {s}");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn progress_path_ends_with_progress_ron() {
        if std::env::var_os("JADE_GARDEN_PROGRESS").is_some() {
            return;
        }
        let p = progress_path();
        assert!(p.ends_with("progress.ron"), "got {}", p.display());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn ensure_parent_dir_creates_missing_dirs() {
        let tmp = std::env::temp_dir()
            .join(format!("jade-garden-test-{}", std::process::id()));
        let target = tmp.join("nested/deep/progress.ron");
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(ensure_parent_dir(&target).is_ok());
        assert!(tmp.join("nested/deep").is_dir());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn ensure_parent_dir_ok_for_bare_filename() {
        assert!(ensure_parent_dir(Path::new("progress.ron")).is_ok());
    }
}