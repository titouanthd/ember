---
title: "Roadmap Tracker"
tags: ["ember/progress", "ember/tracker"]
generated: "2026-11-22"
---

# Roadmap tracker

Checklist of every **exit criterion** in [[40-roadmap]], ticked against the `main` branch on 2026-11-22. Tick boxes as work lands. ⚑ = landed with the merge of `titouanthd-docs-crates-games-plan` (2026-11-22).

```mermaid
gantt
  dateFormat YYYY-MM-DD
  title Ember v0.1 — estimated completion per phase
  section Engine core
  P0 Foundations (done)  :done, p0, 2026-11-18, 1d
  P1 Scene core (90%)    :active, p1, 2026-11-16, 3d
  P2 Serialization (65%) :active, p2, 2026-11-18, 5d
  P3 Signals (85%)       :active, p3, 2026-11-18, 2d
  P4 Servers (done)      :done, p4, 2026-11-16, 3d
  section Tools
  P5 UI (done)           :done, p5, 2026-11-22, 3d
  P6 Editor (0%)         :p6, after p5, 4d
  section Application
  P7 Migration (2/17)    :active, p7, 2026-11-18, 13d
  P8 Cleanup (0%)        :p8, after p7, 1d
```

## Phase 0 — Foundations ✅ closed 2026-11-22
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes *(baseline 2026-11-22)*
- [x] `cargo test --workspace --no-fail-fast` passes *(2,065 / 2,065)*
- [x] New APIs are additive
- [x] `ember_stdlib::juice` has unit tests ⚑
- [x] `MainLoop` headless integration test — `run_headless` + `crates/ember-core/tests/main_loop.rs`
- [x] `Cooldown::running` / `Cooldown::ready(d)` + `#[deprecated]` aliases ⚑
- [x] `primary_action()` *(in `ember-input`)*

## Phase 1 — Scene core
- [x] `NodeTree` + `Sprite`-like node rendered via `ember_render_macroquad` (`crates/ember/examples/hello_scene.rs`, build-verified only)
- [x] process order test — `tests/tree_ordering.rs`
- [x] ready order (bottom-up) test — `tests/ready_ordering.rs`
- [x] stale `NodeId` test — `tests/stale_id.rs`

## Phase 2 — Serialization
- [ ] `#[derive(Node)]` struct instantiable via `ClassDB::instantiate` *(needs `Default` + `register_node`; works, but derive can't carry behaviour)*
- [x] `PackedScene` round-trips through `to_ron()` — `tests/packed_scene.rs`
- [x] hand-written `player.escene` instantiates into a `NodeTree`
- [x] `AssetServer` loads PNG → `TextureId` (with renderer, ADR-026)
- [x] UID registry persisted (`uid_registry.ron`)
- [ ] `#[export]` serialization · `SceneWriter` · font/audio loaders · hot-reload watcher

## Phase 3 — Signals
- [x] Pong paddle→score via signal (`PointScored`)
- [x] deterministic dispatch order — `tests/signal_dispatch_order.rs`
- [x] stale targets skipped — `tests/signal_stale_target.rs`
- [x] cyclic signals don't overflow — `tests/signal_cycle.rs`
- [x] `ScopedConnection` drop disconnects — `tests/scoped_connection.rs`
- [ ] scene-declared connections wired from `.escene`

## Phase 4 — Servers ✅ closed 2026-11-22 (ADR-033)
- [x] engine crates (except legacy `ember-stdlib`) import no macroquad — games → Phase 7 exit, stdlib → Phase 8 (ADR-033)
- [x] `xtask check-layers` exists *(pass/fail unverified)*
- [x] `MockRenderer` drives a `NodeTree` and records calls
- [x] `LinearPhysics` backend; `ctx.physics()`
- [x] ~~`ember_stdlib::collider` migrated into `LinearPhysics`~~ — dropped: `LinearPhysics` written fresh in `ember-servers` (ADR-028)

## Phase 5 — UI ✅ closed 2026-11-24 (5.1 `cdbc078`, 5.2 `064e44c`, 5.3 `45e44da`, 5.4 `bc8191d`; `make ui-demo` checked by hand)
- [x] `Button`, `FocusManager`, mouse routing, `ButtonClicked` / `FocusChanged` emitted by `UiRoot` (5.3, ADR-036)
- [x] `VBox`, `HBox`, `Stretch`, `Label`, `Panel`, `Image`; bottom-up measuring; built-ins registered (5.2)
- [x] `Control` trait, `ControlBase`, `Anchors`, `Theme::dark()/light()`, `UiRoot` layout (5.1)
- [x] `VBox` with 3 `Button`s, Tab/Enter navigation *(`tests/input.rs` + `make ui-demo`)*
- [x] `ScrollContainer` clips *(`tests/scroll_text.rs` + `make ui-demo`)*
- [x] `TextInput` accepts text *(`tests/scroll_text.rs` + `make ui-demo`)*
- [x] `Theme::dark()`

## Phase 6 — Editor POC
- [ ] opens `player.escene`, shows `Player / Sprite / Hitbox`
- [ ] property panel shows `texture`, `size`
- [ ] edit `size` → preview updates
- [ ] save round-trips

## Phase 7 — Migration
Per game: `main.rs` ≤ 10 lines · logic unchanged · scenarios green · no `macroquad`.
- [ ] snake
- [ ] pong *(on NodeTree + signal; `main.rs` 20 lines; still imports macroquad Color)*
- [x] simon *(`1d0dc87`: root node + `ember_ui` screens + custom `ColorPad`; `main.rs` 5 lines; no macroquad; 8 `SceneLoop` tests)*
- [ ] memory
- [ ] minesweeper
- [ ] freecell
- [ ] breakout
- [ ] asteroids
- [ ] air-hockey
- [ ] bullet-hell
- [ ] pagoda
- [ ] taiji
- [ ] zhuo-ji
- [ ] bore-inc
- [ ] jade-garden
- [ ] accretion
- [ ] ember-wars → full checklist in [[42-acceptance-tests]]

## Phase 8 — Cleanup
- [x] no `Cooldown::new` / `with_duration` call sites ⚑
- [x] no `juice.rs` in games ⚑
- [ ] no `audio.rs` in games
- [ ] workspace tests + clippy green
- [ ] `CHANGELOG.md`
