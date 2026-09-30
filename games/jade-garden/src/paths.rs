//! Runtime path resolution.
//!
//! Assets and data files used to be resolved via
//! `env!("CARGO_MANIFEST_DIR")` at compile time. That points at the
//! source tree: it works under `cargo run`, but breaks the moment the
//! release binary is moved away from the repository.
//!
//! ## Assets (read-only)
//!
//! 1. `$JADE_GARDEN_ASSETS` — explicit override.
//! 2. `<exe_dir>/assets` — the release layout (binary next to its
//!    `assets/` folder).
//! 3. `CARGO_MANIFEST_DIR/assets` — dev fallback.
//!
//! ## Progress file (read/write)
//!
//! 1. `$JADE_GARDEN_PROGRESS` — explicit override.
//! 2. Platform user-data directory:
//!    - macOS: `~/Library/Application Support/jade-garden/`
//!    - Linux: `$XDG_DATA_HOME/jade-garden/` or
//!      `~/.local/share/jade-garden/`
//!    - Windows: `%APPDATA%\jade-garden\`
//! 3. `CARGO_MANIFEST_DIR/progress.ron` — dev fallback.
//!
//! The parent directory of the progress file is created on demand the
//! first time we save.

use std::path::{Path, PathBuf};

const APP_DIR_NAME: &str = "jade-garden";

/// Read-only assets directory (levels, chapters, fonts, music).
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

/// Convenience: a file inside the assets directory.
pub fn asset(relative: &str) -> PathBuf {
    assets_dir().join(relative)
}

/// Writable progress file location.
pub fn progress_path() -> PathBuf {
    if let Some(p) = std::env::var_os("JADE_GARDEN_PROGRESS") {
        return PathBuf::from(p);
    }

    if let Some(dir) = user_data_dir() {
        return dir.join("progress.ron");
    }

    // Last resort: the source tree (dev fallback).
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("progress.ron")
}

/// Ensures the parent directory of `path` exists. Returns an error
/// string suitable for logging if it can't be created.
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

/// Directory containing the running executable, if discoverable.
fn current_exe_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent().map(PathBuf::from)
}

/// Platform user-data directory for jade-garden.
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

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: we deliberately don't test the `$JADE_GARDEN_*` env
    // overrides here. `std::env::set_var` is process-global and tests
    // run on parallel threads, so mutating env would race with the
    // assertions in this module.

    #[test]
    fn assets_dir_falls_back_to_manifest_when_no_exe_assets() {
        // Under `cargo test`, the runner lives in `target/debug/deps`,
        // so `<exe_dir>/assets` doesn't exist → we fall back to the
        // source tree.
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
    fn progress_path_ends_with_progress_ron() {
        if std::env::var_os("JADE_GARDEN_PROGRESS").is_some() {
            return;
        }
        let p = progress_path();
        assert!(p.ends_with("progress.ron"), "got {}", p.display());
    }

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

    #[test]
    fn ensure_parent_dir_ok_for_bare_filename() {
        assert!(ensure_parent_dir(Path::new("progress.ron")).is_ok());
    }
}