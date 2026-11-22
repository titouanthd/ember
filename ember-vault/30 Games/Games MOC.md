---
title: "Games MOC"
tags: ["ember/moc"]
generated: "2026-11-22"
---

# Games — Map of Content

17 game crates under `games/` (16 playable, 1 abandoned). The engine docs say **18** in many places — see [[Doc–Code Discrepancies]].

| # | Game | Platform | Docs | Phase 7 status |
|---|---|---|---|---|
| 1 | [[Snake]] | Native | 0 | ⬜ not migrated (first in migration order) |
| 2 | [[Pong]] | Native | 0 | ✅ migrated to NodeTree + SceneLoop + PointScored signal (only migrated game) |
| 3 | [[Simon]] | Native | 0 | ⬜ not migrated |
| 4 | [[Memory]] | Native | 1 | ⬜ not migrated |
| 5 | [[Minesweeper]] | Native | 0 | ⬜ not migrated |
| 6 | [[FreeCell]] | Native | 1 | ⬜ not migrated |
| 7 | [[Breakout]] | Native | 0 | ⬜ not migrated |
| 8 | [[Asteroids]] | Native | 1 | ⬜ not migrated |
| 9 | [[Air Hockey]] | Native | 0 | ⬜ not migrated · has audio.rs |
| 10 | [[Bullet Hell]] | Native | 1 | ⬜ not migrated |
| 11 | [[Pagoda]] | Native + WASM | 3 | ⬜ not migrated · has juice.rs, audio.rs |
| 12 | [[Taiji]] | Native (prototype) | 2 | ⬜ not migrated |
| 13 | [[Zhuo Ji]] | Native | 1 | ⬜ not migrated |
| 14 | [[Bore Inc]] | Abandoned (WASM ref) | 4 | ⬜ not migrated · has audio.rs |
| 15 | [[Jade Garden]] | Native + WASM | 4 | ⬜ not migrated · has juice.rs, audio.rs · shipped on web |
| 16 | [[Accretion]] | Native + WASM | 2 | ⬜ not migrated |
| 17 | [[Ember Wars]] | Native | 1 | ⬜ not migrated · has juice.rs · **acceptance test** (migrates last) |

Rows are in the planned migration order from [[41-migration-guide#3. Migration order|41-migration-guide §3]].
