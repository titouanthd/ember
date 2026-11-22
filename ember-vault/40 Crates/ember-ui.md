---
title: "ember-ui"
tags: ["ember/crate"]
layer: "L3 · Scene"
path: "crates/ember-ui"
loc: ~900
tests: 0
generated: "2026-11-22"
---

# ember-ui

**Layer:** L3 · Scene · **Path:** `crates/ember-ui` · **Files:** 1 · **Lines:** 4 · **`#[test]`:** 0

## Purpose

Retained-mode `Control` nodes, layout, focus, theme (planned)

## Dependencies (Cargo.toml)

ember-scene, ember-servers

## Implementation status

🟡 Session 5.1 (`cdbc078`): `Control` + `ControlBase`, `Anchors`, `Theme`, `ControlRegistry`, `UiRoot` (layout once per frame), `control_node!`, `UiDrawContext` (ADR-034). Widgets and containers next (5.2).

## Spec

- [[24-ember-ui]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
