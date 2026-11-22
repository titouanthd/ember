---
title: "ember-ui"
tags: ["ember/crate"]
layer: "L3 · Scene"
path: "crates/ember-ui"
loc: ~2500
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

🟡 Sessions 5.1 (`cdbc078`) and 5.2 (`064e44c`): `Control` + `ControlBase`, `Anchors`, `Theme`, `ControlRegistry`, `UiRoot` (layout once per frame, bottom-up measure), `control_node!`, `UiDrawContext`, `VBox`/`HBox`/`Stretch`, `Label`/`Panel`/`Image` (ADR-034). Next: focus + `Button` (5.3), scroll + `TextInput` (5.4).

## Spec

- [[24-ember-ui]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
