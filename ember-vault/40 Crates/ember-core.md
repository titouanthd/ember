---
title: "ember-core"
tags: ["ember/crate"]
layer: "L1 · Core"
path: "crates/ember-core"
loc: 2514
tests: 126
generated: "2026-11-22"
---

# ember-core

**Layer:** L1 · Core · **Path:** `crates/ember-core` · **Files:** 22 · **Lines:** 2514 · **`#[test]`:** 126

## Purpose

Math, time (Cooldown, TickTimer), hash, rng, Color, Image, Variant, Object, ClassDB, MainLoop trait

## Dependencies (Cargo.toml)

glam, serde, ron

## Implementation status

🟢 Substantial. `MainLoop` + `FrameStepper` + `run_headless` (headless lifecycle tests), `Variant` + `VariantValue` conversions, `ClassDB` (with type → class lookup, ADR-037), `Object` present. **Gaps:** no `Plugin` trait, no `Stopwatch`; several spec'd Rng/hash helpers use different names.

## Spec

- [[20-ember-core]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
