---
title: "FreeCell"
tags: ["ember/game", "ember/hub", "game/freecell"]
crate: "freecell"
platform: "Native"
generated: "2026-11-22"
---

# FreeCell

| | |
|---|---|
| **Crate** | `games/freecell` (`cargo run -p freecell`) |
| **Platform** | Native |
| **Engine bricks exercised** | Drag-and-drop state machine, super-moves, undo/redo |
| **Engine migration (Phase 7)** | ✅ migrated `9371b95` — `src/scene.rs`: `FreeCellGame` root node drawing cards through the `RenderingServer`, Hint / Restart / New / Menu as `ember_ui` Buttons, font via `create_font`; no macroquad |

## Documents

- [[FreeCell — Design]]

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
