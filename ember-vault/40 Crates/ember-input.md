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

ember-core, glam

## Implementation status

🟡 Snapshot + ADR-010 actions + typed text / mouse wheel / editing keys (ADR-032) done. **Action map** (`action_pressed`, `action_axis`, RON action-map format from 15-input) not implemented.

## Spec

- [[15-input]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
