//! Layer discipline: every ember-* crate may depend only on strictly
//! lower layers, per `ember-vault/10 Engine Spec v0.1/01-architecture.md §3.2`.
//! Uses package names (hyphenated), matching `cargo metadata`.

use std::process::Command;

const ALLOWED: &[(&str, &[&str])] = &[
    (
        "ember",
        &[
            "ember-scene",
            "ember-ui",
            "ember-assets",
            "ember-servers",
            "ember-input",
            "ember-core",
            "ember-render-macroquad",
            "ember-audio-macroquad",
            "ember-macros",
        ],
    ),
    (
        "ember-editor",
        &["ember-scene", "ember-assets", "ember-render-macroquad"],
    ),
    (
        "ember-scene",
        &[
            "ember-servers",
            "ember-assets",
            "ember-core",
            "ember-input",
            "ember-macros",
        ],
    ),
    ("ember-ui", &["ember-scene", "ember-servers", "ember-input", "ember-core"]),
    ("ember-assets", &["ember-core", "ember-servers"]),
    ("ember-input", &["ember-core"]),
    (
        "ember-render-macroquad",
        &["ember-servers", "ember-core", "ember-input"],
    ),
    ("ember-audio-macroquad", &["ember-servers", "ember-core"]),
    ("ember-servers", &["ember-core"]),
    ("ember-macros", &[]), // ADR-016 (Phase 4): proc-macro hygiene
    ("ember-stdlib", &["ember-core"]),
    ("ember-core", &[]),
];

pub fn run() -> Result<(), String> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("parse metadata: {e}"))?;
    let packages = json["packages"]
        .as_array()
        .ok_or("metadata has no packages")?;

    // Only crates explicitly listed in ALLOWED are subject to layer
    // discipline. Everything else (games, xtask, future tools) is a
    // downstream consumer and may depend on any engine crate.
    let engine_names: std::collections::HashSet<&str> = ALLOWED.iter().map(|(n, _)| *n).collect();

    let mut errors = Vec::new();
    for pkg in packages {
        let name = pkg["name"].as_str().unwrap_or("");
        let Some(allowed) = ALLOWED.iter().find(|(n, _)| *n == name).map(|(_, a)| *a) else {
            // Not a recognized engine crate — skip.
            continue;
        };
        for dep in pkg["dependencies"].as_array().into_iter().flatten() {
            let dn = dep["name"].as_str().unwrap_or("");
            if engine_names.contains(dn) && !allowed.contains(&dn) {
                errors.push(format!("{name} -> {dn}"));
            }
        }
    }

    if errors.is_empty() {
        eprintln!("check-layers: OK");
        Ok(())
    } else {
        Err(format!(
            "layer violations (see 01-architecture.md §3.2):\n  {}",
            errors.join("\n  ")
        ))
    }
}
