---
title: "Bullet Hell"
tags: ["ember/game", "ember/hub", "game/bullet-hell"]
crate: "bullet-hell"
platform: "Native"
generated: "2026-11-22"
---

# Bullet Hell

| | |
|---|---|
| **Crate** | `games/bullet-hell` (`cargo run -p bullet-hell`) |
| **Platform** | Native |
| **Engine bricks exercised** | Data-driven waves, 4 emitters, graze multiplier, screen shake, hitstop |
| **Engine migration (Phase 7)** | ✅ migrated `8b27a11` — `src/scene.rs`: `BulletHellGame` root node drawing bullets, enemies and ship through the `RenderingServer`; Shift focus via the new `Key::LeftShift` / `RightShift`; no macroquad |

## Documents

- [[Bullet Hell — Design]]

## Links

- [[Games MOC]] · [[41-migration-guide#3. Migration order|Migration order]] · [[Roadmap Tracker#Phase 7 — Migration|Phase 7 tracker]]
