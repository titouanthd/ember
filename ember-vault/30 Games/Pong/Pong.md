---
title: "Pong"
tags: ["ember/game", "ember/hub", "game/pong"]
crate: "pong"
platform: "Native"
generated: "2026-11-22"
---

# Pong

| | |
|---|---|
| **Crate** | `games/pong` (`cargo run -p pong`) |
| **Platform** | Native |
| **Engine bricks exercised** | AABB collisions, simple AI, match state |
| **Engine migration (Phase 7)** | ✅ `09dd9b8` — on `NodeTree` + `SceneLoop` + `PointScored` signal; no macroquad, `main.rs` 5 lines. Described as `scenes/pong.escene` (`f866d2e`; `.env` only overrides). Opens in the editor with `make edit GAME=pong` (`6aaf9a0`, ADR-039). Previews at 800 × 600, scaled to the panel (`dfa8b04`, ADR-040). Next: check the proof checklist by hand — the **editor proof** (M1 in [[Next Phase Plan — Migration Close & Editor v2]]) |

## Documents

- *No markdown docs in this game folder* (version.txt lists a retroactive DESIGN.md as still owed).

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
