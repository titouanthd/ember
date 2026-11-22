---
title: "ember-render-macroquad"
tags: ["ember/crate"]
layer: "L2 · Servers (backend)"
path: "crates/ember-render/macroquad"
loc: 695
tests: 9
generated: "2026-11-22"
---

# ember-render-macroquad

**Layer:** L2 · Servers (backend) · **Path:** `crates/ember-render/macroquad` · **Files:** 6 · **Lines:** 695 · **`#[test]`:** 9

## Purpose

`MacroquadRenderer`, input poller, `run` / `run_window` runner

## Dependencies (Cargo.toml)

ember-core, ember-servers, ember-input, macroquad, futures, glam

## Implementation status

🟢 All trait methods implemented (clip via GL scissor, ADR-031; camera subtracts the offset since `e190c21`); runner does fixed-step accumulator. Runner order is *physics → process → draw* (architecture diagram shows *process → physics*). Re-exports `macroquad` for games.

## Spec

- [[23-ember-render-macroquad]]

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Progress Evaluation 2026-11-22]]
