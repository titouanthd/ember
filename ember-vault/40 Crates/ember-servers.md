---
title: "ember-servers"
tags: ["ember/crate"]
layer: "L2 · Servers (traits)"
path: "crates/ember-servers"
loc: 1303
tests: 19
generated: "2026-11-22"
---

# ember-servers

**Layer:** L2 · Servers (traits) · **Path:** `crates/ember-servers` · **Files:** 9 · **Lines:** 1303 · **`#[test]`:** 19

## Purpose

`RenderingServer`, `AudioServer`, `PhysicsServer` traits, `LinearPhysics`, `MockRenderer`/`MockAudio`, opaque ids

## Dependencies (Cargo.toml)

ember-core, glam

## Implementation status

🟢 Traits match ADR-016/017/028; clip stack (`push_clip_rect`, `ClipStack`, ADR-031). Mocks are public in `src/mock.rs` (contradicts [[01-architecture]] §7 Q4 “no mocks”, and roadmap’s `tests/common/` location).

## Spec

- [[22-ember-servers]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
