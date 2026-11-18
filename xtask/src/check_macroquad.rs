//! Backend isolation: only `ember-render-macroquad` and
//! `ember-audio-macroquad` may import `macroquad` or `miniquad`.
//! Transitional usage (ember-stdlib modules, games pre-migration) is
//! listed in `xtask/macroquad-allowlist.txt` and must shrink each phase.

use std::fs;
use std::path::Path;

const ALLOWLIST_PATH: &str = "xtask/macroquad-allowlist.txt";

const PERMANENT_BACKENDS: &[&str] = &[
    "crates/ember-render/macroquad",
    "crates/ember-audio/macroquad",
];

pub fn run() -> Result<(), String> {
    let allowlist = load_allowlist(ALLOWLIST_PATH)?;
    let mut violations = Vec::new();
    for root in ["crates", "games"] {
        if Path::new(root).is_dir() {
            walk(Path::new(root), &allowlist, &mut violations)?;
        }
    }
    if violations.is_empty() {
        eprintln!("check-macroquad: OK");
        Ok(())
    } else {
        Err(format!(
            "macroquad/miniquad used outside allowed crates:\n  {}\n\
             Fix: route through a RenderingServer/AudioServer call, or add to \
             {ALLOWLIST_PATH} with a TODO naming the phase that removes it.",
            violations.join("\n  ")
        ))
    }
}

fn load_allowlist(path: &str) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect())
}

fn walk(dir: &Path, allowlist: &[String], violations: &mut Vec<String>) -> Result<(), String> {
    let dir_str = dir.to_string_lossy();
    if PERMANENT_BACKENDS
        .iter()
        .any(|p| dir_str == *p || dir_str.starts_with(&format!("{p}/")))
    {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| format!("read {dir:?}: {e}"))? {
        let path = entry.map_err(|e| format!("dir entry: {e}"))?.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == ".git")
            {
                continue;
            }
            walk(&path, allowlist, violations)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let p = path.to_string_lossy();
            let allowed = allowlist
                .iter()
                .any(|prefix| p.starts_with(prefix) || p.starts_with(&format!("{prefix}/")));
            if allowed {
                continue;
            }
            let content = fs::read_to_string(&path).map_err(|e| format!("read {path:?}: {e}"))?;
            if content.contains("use macroquad") || content.contains("use miniquad") {
                violations.push(p.into_owned());
            }
        }
    }
    Ok(())
}
