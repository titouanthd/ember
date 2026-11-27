---
title: "ember-scene"
tags: ["ember/crate"]
layer: "L3 · Scene"
path: "crates/ember-scene"
loc: 2910
tests: 53
generated: "2026-11-22"
---

# ember-scene

**Layer:** L3 · Scene · **Path:** `crates/ember-scene` · **Files:** 25 · **Lines:** 2910 · **`#[test]`:** 53

## Purpose

`Node`, `NodeId` (slotmap), `NodeTree`, `NodeContext`, `DrawContext`, `Servers`, `SceneLoop`, `SignalTable`, `PackedScene`, `ClassDbNodes`

## Dependencies (Cargo.toml)

ember-core, ember-servers, ember-assets, ember-macros, ember-input, slotmap, glam, serde, ron

## Implementation status

🟢 The most complete crate: tree, deferred spawn/despawn, signals with deterministic order / stale-target / cycle / scoped tests, `.escene` load + round-trip + nested `instance:` with cycle detection. `Node: Object + NodeName` (ADR-030); `Node::draw_after_children` hook (ADR-035). `#[export]` properties on `NodeName`, `SceneWriter` + `PackedScene::from_tree` (ADR-037). Scene-declared connections wired through `ClassDbNodes::register_receiver`; `instantiate_tracked` + `resave` for lossless saves (M4.1). Frame snapshot on `NodeContext` (`time`, `frame`, `viewport`, `pointer_world`; ADR-050, v0.2 V1.1). **Gaps:** no `find_by_class`/`find_all_by_class` (used by [[42-acceptance-tests]] §5.4).

## Spec

- [[21-ember-scene]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Handover 2026-11-26]]
