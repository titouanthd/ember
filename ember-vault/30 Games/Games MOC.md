---
title: "Games MOC"
tags: ["ember/moc"]
generated: "2026-11-22"
---

# Games — Map of Content

18 game crates under `games/` (16 playable, 1 abandoned, and [[Moon Gate]], Game 18, in progress for v0.2). The engine docs say **18** in many places — see [[Doc–Code Discrepancies]].

| # | Game | Platform | Phase 7 status (2026-11-26) |
|---|---|---|---|
| 1 | [[Snake]] | Native | ✅ `77d7694` |
| 2 | [[Pong]] | Native | 🟡 partly — finish + `.escene` = editor proof (M1) |
| 3 | [[Simon]] | Native | ✅ `1d0dc87` |
| 4 | [[Memory]] | Native | ✅ `8b08c5d` |
| 5 | [[Minesweeper]] | Native | ✅ `237243e` |
| 6 | [[FreeCell]] | Native | ✅ `9371b95` |
| 7 | [[Breakout]] | Native | ✅ `ee84048` |
| 8 | [[Asteroids]] | Native | ✅ `f03a117` |
| 9 | [[Air Hockey]] | Native | ✅ `66987a2` |
| 10 | [[Bullet Hell]] | Native | ✅ `8b27a11` |
| 11 | [[Pagoda]] | Native + WASM | ⬜ needs ADR-041 (virtual resolution) · has juice |
| 12 | [[Taiji]] | Native (prototype) | ⬜ needs ADR-040 / 041 · **next game** (moved before pagoda) |
| 13 | [[Zhuo Ji]] | Native | ⬜ needs ADR-042 (shapes) |
| 14 | [[Bore Inc]] | Abandoned (WASM ref) | ⬜ needs ADR-043 / 044 · migrate or freeze? (open question) |
| 15 | [[Jade Garden]] | Native + WASM | ⬜ needs ADR-042 / 043 / 044 · shipped on web |
| 16 | [[Accretion]] | Native + WASM | ⬜ needs ADR-044 |
| 17 | [[Ember Wars]] | Native | ⬜ needs ADR-044 · **acceptance test** (migrates last) |
| 18 | [[Moon Gate]] | Rules only (native + web planned) | — new for v0.2: G1 ✅ rules + solver (2026-11-27); see [[Next Phase Plan — Engine v0.2 & Game 18]] |

Rows are in the original migration order from [[41-migration-guide#3. Migration order|41-migration-guide §3]]; the order for the remaining games and the ADRs they need are in [[Next Phase Plan — Migration Close & Editor v2]]. **10 / 17** on the engine (pong counted as partly).
