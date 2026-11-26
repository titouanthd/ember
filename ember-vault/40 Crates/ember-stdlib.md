---
title: "ember-stdlib"
tags: ["ember/crate"]
layer: "L1 · Core (legacy utilities)"
path: "crates/ember-stdlib"
loc: 2709
tests: 112
generated: "2026-11-22"
---

# ember-stdlib

**Layer:** L1 · Core (legacy utilities) · **Path:** `crates/ember-stdlib` · **Files:** 20 · **Lines:** 2709 · **`#[test]`:** 112

## Purpose

transform, sprite, collider, config, grid, input, ui widgets, audio, fonts, layout, persistence, graphics/text

## Dependencies (Cargo.toml)

ember-core, glam, macroquad, serde, ron, dotenvy

## Implementation status

🟡 Legacy layer still used by 16/17 games and still depends on macroquad (allow-listed). Shared `juice` (ADR-029) and `music` (`MusicPlayer`) modules merged 2026-11-22.

## Spec

- No dedicated spec file (covered by [[01-architecture]]).

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Handover 2026-11-26]]
