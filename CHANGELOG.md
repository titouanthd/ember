# Changelog

All notable changes to the Ember workspace. The design record is the
vault (`ember-vault/`): the engine spec in `10 Engine Spec v0.1/`, the
decisions (ADR-001…048) in `50 Decisions/`, progress in `60 Progress/`.

## [0.2.0] — unreleased

Plan: `ember-vault/60 Progress/Next Phase Plan — Engine v0.2 & Game 18.md`.
Each v0.2 round adds its line here.

### Engine

- **Frame snapshot (V1.1, ADR-050):** `NodeTree::time()` (seconds of
  `process` time, `f64`) and, on `NodeContext`, `time()`, `frame()`,
  `viewport()`, `camera()`, `pointer()` (first touch, else the mouse) and
  `pointer_world()`; `DrawContext::time()`.

### Removed

- `Cooldown::new` and `Cooldown::with_duration`, deprecated in 0.1.0:
  use `Cooldown::running` and `Cooldown::ready` (ADR-011).

## [0.1.0] — 2026-11-27

The first engine release: all 17 games run on the Ember node tree, and
macroquad is only a backend. The tag follows the spec 42 §8 gate
(`make pre-push`).

### Engine

- **Foundations (Phase 0):** `MainLoop`, `FrameStepper` and
  `run_headless` in `ember-core`; `Cooldown::ready()`.
- **Scene core (Phase 1):** `Node`, `NodeId`, `NodeTree`, `NodeContext`
  and the `SceneLoop` adapter in `ember-scene`; `#[derive(Node)]`
  generates `NodeName` (ADR-019, ADR-030).
- **Serialization (Phase 2):** `.escene` files, `PackedScene`,
  `ClassDB`, nested `instance:`, `SceneWriter` from live trees
  (ADR-024…027, ADR-037).
- **Signals (Phase 3):** one tree-level `SignalTable`, dispatched
  through `NodeContext` (ADR-023).
- **Servers (Phase 4):** `RenderingServer`, `AudioServer` and
  `PhysicsServer` traits in `ember-servers`, with `MockRenderer` /
  `MockAudio` for headless tests and `LinearPhysics` (ADR-017, ADR-028);
  the macroquad backends `ember-render-macroquad` and
  `ember-audio-macroquad`; `AssetServer` (ADR-026); clip rects, camera
  offset + zoom, virtual resolution with letterbox, polygons, arcs and
  ellipses, texture filtering (ADR-031, 040, 041, 042, 048); font and
  audio loaders with hot reload, looped clips and MP3 (ADR-038, 043);
  window options (`run_window_with`, ADR-047).
- **UI (Phase 5):** `ember-ui` controls in the scene tree — `Button`,
  `Label`, `Panel`, `Image`, `VBox`, `HBox`, `ScrollContainer`,
  `TextInput`, themes, `UiRoot` routing input and emitting control
  signals (ADR-034…036); typed text, wheel and editing keys in `Input`
  (ADR-032).
- **Editor (Phase 6, Phase 9 in progress):** `ember-editor` opens,
  edits and saves `.escene` files; it is a library a game plugs its
  `ClassDB` into (`make edit GAME=pong`, ADR-039).
- **Architecture checks:** `xtask check-layers` and
  `xtask check-macroquad` (`make layers macroquad`).

### Games (Phase 7)

All 17 games are ported to the node tree and import no macroquad:
accretion, air-hockey, asteroids, bore-inc (abandoned as a game, kept
as a reference), breakout, bullet-hell, ember-wars (the acceptance test,
spec 42), freecell, jade-garden, memory, minesweeper, pagoda, pong,
simon, snake, taiji, zhuo-ji.

### Cleanup (Phase 8)

- `ember-stdlib` no longer depends on macroquad. `config::env_color`
  and `Sprite` use `ember_core::Color`; `layout::centered_rect` returns
  `ember_core::math::Rect`; new `clock::format_time`; `music` keeps its
  playlist logic on any `MusicBackend`; `fx` is the engine-typed juice
  (ADR-046).
- Games use these stdlib versions instead of local twins.
- `xtask/macroquad-allowlist.txt` holds comments only.

### Removed

- `ember-stdlib` modules `audio`, `fonts`, `graphics`, `input`, `juice`,
  `ui` and `music::MacroquadMusic` (macroquad-typed, unused by any
  game).
- The games' `juice.rs` copies.

### Deprecated

- `Cooldown::new` and `Cooldown::with_duration` (use `Cooldown::running`
  and `Cooldown::ready`). No call site is left; they are removed in
  0.2.0.
