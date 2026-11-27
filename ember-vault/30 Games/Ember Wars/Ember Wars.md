---
title: "Ember Wars"
tags: ["ember/game", "ember/hub", "game/ember-wars"]
crate: "ember-wars"
platform: "Native"
generated: "2026-11-22"
---

# Ember Wars

| | |
|---|---|
| **Crate** | `games/ember-wars` (`cargo run -p ember-wars`) |
| **Platform** | Native |
| **Engine bricks exercised** | Catapult, auto-combat, 28-node upgrade tree, procedural textures, juice |
| **Engine migration (Phase 7)** | 🟢 node tree `0ad070c` (port `f16fe0c`) — a level is a `World` node; catapults, units and projectiles are nodes; no macroquad. **Acceptance test passed** (all 20 items; play-throughs by the user, 2026-11-27) — see [[41-migration-guide#5.17 ember-wars (acceptance test) — **node tree** (`f16fe0c` port, `0ad070c` nodes; 2026-11-27)|§5.17]] and [[42-acceptance-tests]] |

## Documents

- [[Ember Wars — Design]]

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
- Acceptance definition: [[42-acceptance-tests]]
