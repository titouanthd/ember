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

🟡 Sessions 5.1 (`cdbc078`), 5.2 (`064e44c`), 5.3 (`45e44da`) and 5.4: `Control` + `ControlBase`, `Anchors`, `Theme`, `ControlRegistry`, `UiRoot` (layout once per frame, bottom-up measure), `control_node!`, `UiDrawContext`, `VBox`/`HBox`/`Stretch`, `Label`/`Panel`/`Image` (ADR-034); `Button`, `FocusManager`, input routing and the `ControlBase` signal outbox (`ButtonClicked`, `FocusChanged`, ADR-036). 5.4 (`bc8191d`): `ScrollContainer`, `TextInput`, children clip and overlay hooks, `TextChanged` / `TextSubmitted` (ADR-035); demo: `make ui-demo`. Phase 5 closed 2026-11-24. `Control::hit_test` (`3f456aa`) for round controls; first game user: [[Simon]].

## Spec

- [[24-ember-ui]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Handover 2026-11-26]]
