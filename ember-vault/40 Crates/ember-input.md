---
title: "ember-input"
tags: ["ember/crate"]
layer: "L2 · Servers"
path: "crates/ember-input"
loc: 758
tests: 33
generated: "2026-11-22"
---

# ember-input

**Layer:** L2 · Servers · **Path:** `crates/ember-input` · **Files:** 8 · **Lines:** 758 · **`#[test]`:** 33

## Purpose

Backend-agnostic `Input` snapshot, `Key`, mouse, gamepad, touch, `primary_action`/`secondary_action`, `InputSink`

## Dependencies (Cargo.toml)

ember-core, glam, serde, ron

## Implementation status

🟡 Snapshot + ADR-010 actions + typed text / mouse wheel / editing keys (ADR-032) done. Keys added during Phase 7: `Semicolon` (`18513a1`, freecell), `LeftShift` / `RightShift` (`c6d3055`, bullet-hell); Control / Super planned for editor shortcuts (Editor v2). **Action map** (ADR-052, v0.2 V1.3): `ActionMap` from RON, `ActionResolver` (edges, swipes), `Input::action` / `action_pressed` / `action_released` / `action_axis` / `action_vector` / `swipe`; run by `SceneLoop::with_action_map`. Delta: [[v0.2-15-input]]. **Gaps:** no gamepad sticks or dead zones, no asset hot reload, no chords.

## Spec

- [[15-input]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Handover 2026-11-26]]
