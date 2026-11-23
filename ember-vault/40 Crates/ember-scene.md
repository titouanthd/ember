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

🟢 The most complete crate: tree, deferred spawn/despawn, signals with deterministic order / stale-target / cycle / scoped tests, `.escene` load + round-trip + nested `instance:` with cycle detection. `Node: Object + NodeName` (ADR-030); `Node::draw_after_children` hook (ADR-035). **Gaps:** `SceneWriter` is a stub, scene-declared connections parsed but not wired, no `find_by_class`/`find_all_by_class` (used by [[42-acceptance-tests]] §5.4).

## Spec

- [[21-ember-scene]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
