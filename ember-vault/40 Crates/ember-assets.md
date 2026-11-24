---
title: "ember-assets"
tags: ["ember/crate"]
layer: "L3 · Scene"
path: "crates/ember-assets"
loc: 815
tests: 7
generated: "2026-11-22"
---

# ember-assets

**Layer:** L3 · Scene · **Path:** `crates/ember-assets` · **Files:** 8 · **Lines:** 815 · **`#[test]`:** 7

## Purpose

`AssetServer`, `Handle<T>`, `WeakHandle`, `Asset` trait, `UidRegistry`, PNG `Texture`, `RonAsset<T>`

## Dependencies (Cargo.toml)

ember-core, ember-servers, serde, ron, png

## Implementation status

🟢 Per ADR-026/038: synchronous, type-driven; `Texture`, `Font`, `AudioClip`, `RonAsset`, `PackedScene`; hot reload via `reload_changed` (polling). **Deferred:** async loads, watcher thread, loader registry. 14 tests (`assets.rs`, `hot_reload.rs`).

## Spec

- [[25-ember-assets]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
