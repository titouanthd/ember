---
title: "Air Hockey"
tags: ["ember/game", "ember/hub", "game/air-hockey"]
crate: "air-hockey"
platform: "Native"
generated: "2026-11-22"
---

# Air Hockey

| | |
|---|---|
| **Crate** | `games/air-hockey` (`cargo run -p air-hockey`) |
| **Platform** | Native |
| **Engine bricks exercised** | PvP/PvAI, goalkeeper AI, hitstop guard, countdown |
| **Engine migration (Phase 7)** | ✅ migrated `66987a2` — `src/scene.rs`: `AirHockeyGame` root node; engine `Input` / `Key`, sounds through the `AudioServer`, font via `create_font`; no macroquad |

## Documents

- *No markdown docs in this game folder* (version.txt lists a retroactive DESIGN.md as still owed).

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
