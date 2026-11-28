---
title: "Doc–Code Discrepancies"
tags: ["ember/progress", "ember/docs-debt"]
aliases: ["Doc-Code Discrepancies"]
generated: "2026-11-22"
---

# Doc ↔ code discrepancies

Found while reading the specs against `main`. Most are fixed on the unmerged branch `titouanthd-docs-crates-games-plan` (see [[Engine Spec Index]]) — marked **⚑ fixed on branch**.

| # | Where | Docs say | Code / repo says | |
|---|---|---|---|---|
| 1 | [[00-overview]], [[01-architecture]], [[40-roadmap]], [[Engine Spec Index]] | **18** games | 17 game crates (16 playable + bore-inc abandoned) | ⚑ fixed on branch |
| 2 | [[Engine Spec Index]] §4 | Wave 2–4 files 📋 *planned* | Many are written and implemented; 4 files carry 🟡 headers that the index doesn't reflect | ⚑ fixed on branch |
| 3 | [[Engine Spec Index]] §6 | Spec must be ✅ frozen before a crate PR lands | No file is frozen; P1–P4 crates merged | open |
| 4 | Phase numbering | [[40-roadmap]]: 9 phases (0–8) | Commits say *phase 1…5*; code comments reference **Phase 5, 6, 11, 13** (`xtask/macroquad-allowlist.txt`, `ember-scene/src/lib.rs` “Phase 6 status”, `scene.rs` “Stub until Phase 11”, ADR-019) | ✅ closed 2026-11-27 (M5.2): the roadmap has 10 phases (0–9, Phase 9 = Editor v2); no “Phase 11/13” left in code (the last, in `ember-render-macroquad`, fixed in `6f3a1fd`); “Phase 6 status” in `ember-scene` names a real phase |
| 5 | [[40-roadmap#7. Phase 4 — Servers\|Phase 4]] | ✅ | Exit criteria (macroquad isolation, collider migration) unmet | ⚑ downgraded to 🟡 on branch |
| 6 | [[01-architecture]] §7 Q4 | No mock servers | `ember-servers` ships public `MockRenderer`/`MockAudio`; roadmap wants them in `tests/common/` | ⚑ noted on branch |
| 7 | [[01-architecture]] §3.2 | allowed edges | `xtask` also allows `ember-scene → ember-input`, `ember-render-macroquad → ember-input`; doc has `ember-macros → ember-core`, code has none | partly ADR-022 |
| 8 | [[00-overview]] §3.2 | 6 games ship `juice.rs` (accretion, ember-wars, jade-garden, pagoda, air-hockey, asteroids) | On main only **3** (`ember-wars`, `jade-garden`, `pagoda`); air-hockey has `effects.rs` | ⚑ |
| 9 | [[01-architecture#4.1 Single-frame sequence\|§4.1 frame sequence]] | process → physics → draw | runner: physics → process → draw | ✅ resolved 2026-11-22 (one runner, `FrameStepper`; specs aligned) |
| 10 | [[40-roadmap]] P0 deliverables | `Cooldown::running`/`ready(d)` | main: `new`, `with_duration`, `ready()` (no arg) — ADR-011 accepted but not on main | ✅ merged |
| 11 | [[26-ember-macros]], [[10-nodes-and-scenes]] | derive + lifecycle overrides | derive writes whole `impl Node`; behavioural nodes must hand-write it | ✅ resolved by ADR-030 (`8cc3112`) |
| 12 | [[02-principles]] I4 | no `unwrap/expect` in engine crates | 10 in non-test engine code | open |
| 13 | [[15-input]] | action map (`action_pressed`, `action_axis`, RON) | not implemented | ✅ closed 2026-11-28 (V1.3, ADR-052): `ActionMap` (RON) + `ActionResolver`, `Input::action*`, swipes, `SceneLoop::with_action_map`; axes are two actions, no dead zones until gamepad axes exist ([[v0.2-15-input]]) |
| 14 | [[21-ember-scene]], [[42-acceptance-tests]] §5.4 | `find_by_class`, `find_all_by_class` | missing | ✅ closed 2026-11-28 (V1.2, ADR-051): typed `NodeTree::find_by_class::<T>` / `find_all_by_class::<T>` / `children_of::<T>` / `get_as::<T>`, the `NodeContext` forms, and node groups |
| 15 | `README.md`, [[Project Recap (version.txt)]] | 3-layer architecture; 1909/1941 tests | pre-engine snapshot (Session 17.9); no mention of the 10 new crates | ✅ README rewritten; `version.txt` moved to the vault |
| 16 | [[Engine Spec Index]] §7 | — | stray `---S` before §8 (typo) | open |
| 17 | [[41-migration-guide]] | per-PR migrations + `MIGRATION_LOG.md` | pong migrated in the scene-core branch; no `MIGRATION_LOG.md` | open |
| 18 | `Makefile` | `CRATES := ember-core ember-stdlib ember-editor` | 12 crates exist | ✅ closed 2026-11-27 (M5.2): `Makefile` `ENGINE` lists all 12 crates (`test-engine`, `doc`); no `CRATES` left |
| 19 | `RenderingServer::set_camera` | `DrawContext::world_to_screen`: `world - offset` | macroquad renderer drew at `world + offset` | ✅ fixed `e190c21` |
