---
title: "Memory"
tags: ["ember/game", "ember/hub", "game/memory"]
crate: "memory"
platform: "Native"
generated: "2026-11-22"
---

# Memory

| | |
|---|---|
| **Crate** | `games/memory` (`cargo run -p memory`) |
| **Platform** | Native |
| **Engine bricks exercised** | Grid<T> clicking, 2-step selection, best time persistence |
| **Engine migration (Phase 7)** | ✅ migrated `8b08c5d` — `src/scene.rs`: `MemoryGame` root node drawing through the `RenderingServer`, START / Restart / Menu as `ember_ui` Buttons; no macroquad |

## Documents

- [[Memory — Design]]

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
