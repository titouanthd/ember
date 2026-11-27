---
title: "Moon Gate"
tags: ["ember/game", "ember/hub", "game/moon-gate"]
crate: "moon-gate"
platform: "Rules only (native + web planned)"
created: "2026-11-27"
---

# Moon Gate

| | |
|---|---|
| **Crate** | `games/moon-gate` (`cargo test -p moon-gate`; no binary until G2) |
| **Platform** | Rules only for now; native + web (mobile swipe) planned for G5 |
| **Engine bricks exercised** | None yet (pure rules). Planned: `TileMap`, instanced piece scenes, `AnimatedSprite`, `Tween`, input actions, `change_scene`, the editor's tile brush |
| **Status** | G1 ✅ (2026-11-27): rules, undo, BFS solver, 8 draft levels, 35 tests |

Game 18: the first game designed for Ember, not ported to it. Name chosen by the user on 2026-11-27 (月门, the round gateway in a Chinese garden wall), in the line of [[Zhuo Ji]], [[Jade Garden]], [[Pagoda]] and [[Taiji]].

## Documents

- README (in the repo, outside the vault): `games/moon-gate/README.md`.
- [[Moon Gate — Design]]

## Links

- [[Games MOC]] · [[Next Phase Plan — Engine v0.2 & Game 18]] · [[Roadmap Tracker#v0.2 — Engine v0.2 & Game 18|v0.2 tracker]]
