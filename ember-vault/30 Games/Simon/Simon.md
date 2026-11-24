---
title: "Simon"
tags: ["ember/game", "ember/hub", "game/simon"]
crate: "simon"
platform: "Native"
generated: "2026-11-22"
---

# Simon

| | |
|---|---|
| **Crate** | `games/simon` (`cargo run -p simon`) |
| **Platform** | Native |
| **Engine bricks exercised** | `SceneLoop`, signals, `ember_ui` (`UiRoot`, `Panel`, `Label`, custom `Control` with `hit_test`), `AudioServer::load_clip`, sequence timers, best score |
| **Engine migration (Phase 7)** | ✅ migrated `1d0dc87` — `src/scene.rs`: `Simon` root node, `ember_ui` screens, custom `ColorPad` control, `PadClicked` signal, sounds via `AudioServer` |

## Documents

- *No markdown docs in this game folder* (version.txt lists a retroactive DESIGN.md as still owed).

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
