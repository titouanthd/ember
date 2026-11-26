---
title: "Asteroids"
tags: ["ember/game", "ember/hub", "game/asteroids"]
crate: "asteroids"
platform: "Native"
generated: "2026-11-22"
---

# Asteroids

| | |
|---|---|
| **Crate** | `games/asteroids` (`cargo run -p asteroids`) |
| **Platform** | Native |
| **Engine bricks exercised** | Rotation, inertia, wrap-around, high score persistence |
| **Engine migration (Phase 7)** | ✅ migrated `f03a117` — `src/scene.rs`: `AsteroidsGame` root node drawing ship, rocks, bullets and stars through the `RenderingServer`; seeded `Rng`; no macroquad |

## Documents

- [[Asteroids — Design]]

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
