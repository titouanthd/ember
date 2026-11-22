---
title: "ember-macros"
tags: ["ember/crate"]
layer: "L1 · Core (proc-macro)"
path: "crates/ember-macros"
loc: 137
tests: 0
generated: "2026-11-22"
---

# ember-macros

**Layer:** L1 · Core (proc-macro) · **Path:** `crates/ember-macros` · **Files:** 2 · **Lines:** 137 · **`#[test]`:** 0

## Purpose

`#[derive(Node)]`, `#[signal]` (no-op), `#[export]` (inert field attr)

## Dependencies (Cargo.toml)

syn, quote, proc-macro2

## Implementation status

🟡 Minimal. Derive emits a **complete** `impl Node`, so a node with lifecycle behaviour cannot use it (see `hello_scene.rs`: *“the derive generates an empty Node impl, so a node with behavior implements Object and Node itself”*). `#[export]` has no effect; no tests in-crate (covered by `ember-scene/tests/derive_works.rs`).

## Spec

- [[26-ember-macros]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
