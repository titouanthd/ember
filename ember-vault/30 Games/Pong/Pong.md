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
| **Engine migration (Phase 7)** | ✅ `09dd9b8` — on `NodeTree` + `SceneLoop` + `PointScored` signal; no macroquad, `main.rs` 5 lines. Next: describe it as `scenes/pong.escene` — the **editor proof** (M1 in [[Next Phase Plan — Migration Close & Editor v2]]) |

## Documents

- *No markdown docs in this game folder* (version.txt lists a retroactive DESIGN.md as still owed).

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
