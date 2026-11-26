---
title: "41 — Migration Guide"
aliases: ["41 — Migration Guide"]
tags: ["ember/spec", "ember/wave-4"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/41-migration-guide.md"
imported: "2026-11-22"
wave: "4 — Format, Process & Reference"
---

# 41 — Migration Guide

> **Status:** 🟡 partial — reviewed 2026-11-19; 10 of 17 games (pong, simon, snake, memory, minesweeper, freecell, breakout, asteroids, air-hockey, bullet-hell) migrated to `NodeTree`/`SceneLoop`; the recipe is a plan and diverges from what pong needed (see §5.2)
> **Audience:** implementers, game authors
> **Prerequisites:** [[40-roadmap]], [[21-ember-scene]]

## 1. Purpose

Describe how to move a single game from the current code structure
(per-game `main.rs`, direct `macroquad` calls, hand-written state) to
the engine structure (`MainLoop`, `NodeTree`, `NodeContext`,
`RenderingServer`). One game at a time, with a rollback plan.

This is the file you read before you start a migration PR.

> **Progress:** only `games/pong` is migrated (commit "migrate to NodeTree
> with SceneLoop and PointScored signals"). The other 16 games still use
> their own `main.rs` and macroquad directly. The recipe in §4 is the
> original plan; where the real engine API differs, notes below say so.
> In particular, the engine entry point is
> `ember::prelude::{SceneLoop, run_window}` (`run_window(title, w, h, loop)`),
> `MainLoop` (`ember-core`) has `init`/`process`/`physics_process`/
> `draw(&mut self)`/`finalize` and **no** `progress()`/`progress_mut()`, and
> there is no `ember::run(game).await`.

## 2. Principles

### 2.1 Behavior first, then structure

**Do not touch game logic.** `systems::update`, `resolve_attacks`,
`try_fire`, etc. are unchanged by migration. What changes is the
*wrapping* — how the game is invoked, how it draws, how it persists.

If a migration requires changing game logic to make the tree work, that
is a signal the engine is missing something. **Stop and file an ADR
instead of hacking the game.**

### 2.2 One game per PR

Never migrate two games in one PR. The review burden is O(n²), and a bug
introduced in the shared PR is harder to bisect.

### 2.3 Tests green before, green after

Every game already has unit tests and most have `tests/game_scenarios.rs`. All of them
must pass **at every commit** of the migration branch. Not "at the end
of the branch" — every commit.

This is what makes the migration safe: if a test breaks, the last commit
was wrong.

### 2.4 Rollback is a single revert

A migration is one PR. If it breaks something subtle, `git revert` on
the merge commit returns to the pre-migration state. No half-migrated
games.

## 3. Migration order

The order below minimizes risk:

| Order | Game | Complexity | Notes |
|---|---|---|---|
| 1 | `snake` | Trivial | Single state machine, no UI, no audio |
| 2 | `pong` | Trivial | Two paddles, one ball, score |
| 3 | `simon` | Trivial | Sequence game, button clicks |
| 4 | `memory` | Simple | Card grid, no physics |
| 5 | `minesweeper` | Simple | Grid, no rendering complexity |
| 6 | `freecell` | Simple | Card game, undo/redo |
| 7 | `breakout` | Medium | Bricks, ball, paddle, levels |
| 8 | `asteroids` | Medium | Wrap-around, bullets, waves |
| 9 | `air-hockey` | Medium | Physics-heavy, AI |
| 10 | `bullet-hell` | Medium | Wave patterns, bullets, collisions |
| 11 | `pagoda` | Medium | Slicing, combo, camera |
| 12 | `taiji` | Medium | Match state, simple |
| 13 | `zhuo-ji` | Medium | Custom rendering |
| 14 | `bore-inc` | Large | Grid, mining, tool upgrades, save/load |
| 15 | `jade-garden` | Large | Match-3, cascades, poem reveals, chapters |
| 16 | `accretion` | Large | Prestige, economy, save state, story |
| 17 | `ember-wars` | Largest | Waves, AI, catapults, combat, upgrades |

**`ember-wars` is last on purpose.** It is the acceptance test — if
migration of `ember-wars` requires changes to engine primitives, we
have time to iterate on the primitives before they are frozen.

## 4. The migration recipe (per game)

Each migration PR follows the same 8-step pattern. The steps are
sequential and each can be a separate commit.

### Step 1 — Add the trait impl skeleton

Create `games/<game>/src/game.rs` (or reuse `lib.rs`) with a `Game`
struct that implements `MainLoop`.

*Planned recipe; superseded by pong's approach (§5.2): instead of a custom
`MainLoop`, build a `NodeTree` and hand it to `SceneLoop`, which already
implements `MainLoop`.*

```rust
pub struct Game {
    world: World,
    state: AppState,
    // ...
}

impl MainLoop for Game {
    fn process(&mut self, dt: f32) -> bool {
        // (stub for now)
        true
    }
    fn draw(&mut self) {
        // (stub for now)
    }
}
```

Add `impl Game` to `main.rs`:

```rust
fn main() {
    let game = Game::new();
    run_window("Game", 800, 600, game);
}
```

**Commit: "migration skeleton"** — the game runs, but the `process` and
`draw` bodies are stubs (blank screen).

### Step 2 — Move state from main.rs to Game

Move the existing game state (from `main.rs`'s loop body) into the
`Game` struct. `main.rs`'s loop body becomes `Game::process`.

```rust
impl MainLoop for Game {
    fn process(&mut self, dt: f32) -> bool {
        // moved from main.rs
        let input = Input::capture();
        self.world.update(dt, &input);
        true
    }

    fn draw(&mut self) {
        self.world.draw();
    }
}
```

**Commit: "move game loop into Game::process"** — the game runs and
looks identical to pre-migration.

### Step 3 — Replace direct rendering with RenderingServer

If the game calls `macroquad::draw_rectangle`, `macroquad::draw_text`,
etc. directly, replace with `ctx.render().draw_rect(...)`. This requires
moving drawing behind `DrawContext` (`Node::draw(&self, ctx: &mut
DrawContext)`, `ctx.render()` returns the `RenderingServer`). Solution: create a single root `Node` that owns the world's
draw logic, and let `NodeTree::draw` drive it.

```rust
#[derive(Node)]
#[class_name = "Root"]
pub struct Root {
    name: String,
    world: World,
}

impl Node for Root {
    fn draw(&self, ctx: &mut DrawContext) {
        self.world.draw_via(ctx);
    }
}
```

And `World::draw_via` calls `ctx.render().draw_rect(...)` instead of
`macroquad::draw_rectangle(...)`.

**Commit: "route draws through RenderingServer"** — the game looks
identical but `macroquad` is gone from `World`.

### Step 4 — Move input capture to Input

`Input::capture()` becomes `ctx.input()` (inside `Node::process`). The `World::update` signature
stays `fn update(&mut self, dt: f32, input: &Input)` — no change needed.

```rust
fn process(&mut self, dt: f32) -> bool {
    self.world.update(dt, ctx_input());
    true
}
```

**Commit: "unify input through NodeContext"** — no visual change.

### Step 5 — Add entities as nodes

The core of the migration. For a game with a `Vec<Unit>` or similar:

- Create a `UnitNode` struct implementing `Node`.
- Replace `Vec<Unit>` with `NodeId` references in `World`.
- `World::update` becomes `World::system_tick` — still operates on
  `Vec<UnitData>` conceptually, but through the tree.

This is the step where the migration slows down. Different games need
different granularity:

- **Snake, pong, asteroids:** migrate `Unit` → `Node`.
- **Minesweeper, freecell:** the "units" are tiles/cards; migrating them
  to nodes may be unnecessary — keep them as a data structure managed
  by a single node.

**Commit: "entities are nodes"** — behavior identical, but entity state
now lives in `NodeTree`.

### Step 6 — Add audio through AudioServer

Replace `macroquad::audio::play_sound_once` with
`ctx.audio().play(clip_id, volume, pitch)`.

**Commit: "route audio through AudioServer"** — audio still works.

### Step 7 — Add progress persistence through MainLoop

*Not yet implemented in the engine:* `MainLoop` has no `progress()` /
`progress_mut()`. Keep the game's own `save::load` / `save::save` until a
persistence API lands.

**Commit: "progress through MainLoop"** — deferred.

### Step 8 — Delete old code

- Delete the old `main.rs` loop (already replaced in step 2).
- Delete `juice.rs` (already replaced by `ember_stdlib::juice` in
  Phase 0).
- Delete `audio.rs` (already replaced in step 6).
- Delete any `use macroquad` imports.
- Delete `use ember_stdlib::input::Input` if no longer needed.

**Commit: "delete dead code"** — the game is fully migrated.

## 5. Per-game notes

### 5.1 snake — **migrated** (`77d7694`, 2026-11-24)

- 20 unit + 12 scenario (unchanged) + 7 headless `SceneLoop` tests
  (`games/snake/tests/scene.rs`).
- Result: `src/scene.rs`, one `SnakeGame` root node owning the
  `SnakeWorld` plus the session state the old loop kept in locals (best,
  level index, level-cleared timer, now counted in `dt` instead of
  `Instant`). HUD and overlays are drawn text (`measure_text` to centre),
  not `ember_ui` — a single fixed screen does not need widgets.
  `main.rs` is 5 lines; no macroquad.
- Data-only prep commit (`fe976b2`): `ember_core::Color` for colours and
  an `ember_core::Rng` behind a static for food placement (macroquad's
  global `rand` was unseeded too; the scene seeds it from the clock).
  Food positions differ from before; no test depended on them.
- Lesson: a game with no widgets migrates in one node — a second node
  only pays off when another one listens to its signals (pong).

### 5.2 pong — **migrated** (`09dd9b8`, 2026-11-26)

- 20 unit + 13 scenario + 3 headless `SceneLoop` tests
  (`games/pong/tests/scene.rs`).
- Result: `src/scene.rs` has two nodes — `Pong` (root; owns the match,
  reads input in `process`, draws via `DrawContext`) and `Scoreboard`
  (child; listens to `PointScored` / `MatchReset` signals emitted via
  `ctx.emit`). `main.rs` (5 lines) calls `scene::run()`, which wraps
  `build_tree` in `SceneLoop::new(tree, MacroquadRenderer, MacroquadAudio,
  viewport)` and `run_window`.
- Lessons: pure logic (`systems`, `components`) was untouched; the
  `Node` impls are written by hand (`Object` + `Node`) rather than with
  derive; entities stay plain structs inside one node (steps 5's "one
  node per entity" was not needed); handlers find their node via
  `ctx.current_id()` + `tree().get_mut` + downcast; `dt` is capped
  (`MAX_FRAME_DT`) inside the node; tests drive `SceneLoop` with
  `MockRenderer`/`MockAudio`.
- Closed in `09dd9b8` (M1.1): `ember_core::Color` everywhere (the
  `color()` bridge is gone), local `env_color` and `Sprite` twins until
  Phase 8, `ember_core::GameState`, `main.rs` → `scene::run()`; off the
  macroquad allowlist. Tests unchanged.
- Scene file in `f866d2e` (M1.2): `scenes/pong.escene` with four
  `#[derive(Node)]` classes (`Pong`, `Paddle`, `Ball`, `Scoreboard`).
  `Pong::ready` reads its children into `GameContext` and the entities,
  and `process` copies positions back so the children draw themselves.
  `.env` only overrides. `build_tree(cfg, seed)` loads the file, then writes
  every `cfg` value into it (the tests' hermetic path). The only change to
  game logic: `reset_match` centres each paddle on its own height.
- Order in §3 has pong 2nd but it was migrated first.

### 5.3 simon — **migrated** (`1d0dc87`, 2026-11-24)

- 21 unit + 8 scenario (unchanged) + 8 headless `SceneLoop` tests
  (`games/simon/tests/scene.rs`).
- Result: `src/scene.rs`. `Simon` (root) owns the `GameWorld`, reads
  Space / R, ticks playback and pushes state into the UI every frame.
  Under a `UiRoot`: three `Panel` groups (start, playfield, game-over
  box) of `Label`s, and four `ColorPad`s — a game-defined `Control` with
  a round `Control::hit_test` that emits `PadClicked` through `UiRoot`
  (ADR-036). `main.rs` is 5 lines (`simon::scene::run()`); the crate
  depends on `ember` and `ember-servers`, not macroquad.
- Lessons: a custom control is ~60 lines (`Object` + `NodeName` +
  `Control` + `control_node!`). Round buttons needed `Control::hit_test`
  (added in `ember-ui`, `3f456aa`). A handler that needs the root's
  state downcasts it from `ctx.tree()` and returns what to do (here the
  clip to play), then acts on `ctx`. Sounds are `include_bytes!` +
  `AudioServer::load_clip` in `ready` (no file paths at runtime).
  `ember_stdlib::config::env_color` returns a macroquad colour, so
  `config.rs` has a local twin returning `ember_core::Color`.
- Kept as-is: the click sound plays on click *and* again when the
  flash lights the pad next frame (same as before); the best score
  still goes through `ember_stdlib::persistence` (step 7 deferred).

### 5.4 memory — **migrated** (`8b08c5d`, 2026-11-26)

- 20 unit + 4 scenario (unchanged) + 8 headless `SceneLoop` tests
  (`games/memory/tests/scene.rs`).
- Result: `src/scene.rs`. `MemoryGame` (root) owns the `Game`, handles
  keys and card / menu clicks, and draws menu, board, header and banner.
  START / Restart / Menu are `ember_ui` `Button`s under a themed
  `UiRoot`; a click becomes `ButtonClicked`, applied on the next process.
- Logic touched: `Game` lost its three `ember_stdlib` `Button` fields and
  `update_menu_layout` (UI state, now in the scene); `Rect` / `Color`
  come from `ember_core`.
- Lessons: the card grid stayed drawn by hand — a 64-card board is data
  in one node, not 64 controls (same call as minesweeper, §5.5). A
  clicked button keeps focus and would then react to Space / Enter, so
  the game drops focus after each click (`UiRoot::request_focus(None)`).
  `draw` has no input: hover effects read the pointer position saved in
  `process`.

### 5.5 minesweeper — **migrated** (`237243e`, 2026-11-26)

- 28 unit + 8 scenario (unchanged) + 7 headless `SceneLoop` tests
  (`games/minesweeper/tests/scene.rs`).
- Result: `src/scene.rs`, same shape as memory (§5.4): one
  `MinesweeperGame` root node and three `ember_ui` buttons. The 16×16 /
  30×16 board stays a `Grid` drawn by one node, as planned. Left, right
  and middle clicks come straight from `Input`.
- Restart / Menu were drawn but inert on the end screens; they are now
  shown **disabled** there (`Button::set_disabled`), which makes that
  visible.
- Logic touched: the three `Button` fields and `update_menu_layout` left
  `Game`; `Color` comes from `ember_core`.


### 5.6 freecell — **migrated** (`9371b95`, 2026-11-26)

- 78 unit + 8 scenario (unchanged) + 7 headless `SceneLoop` tests
  (`games/freecell/tests/scene.rs`).
- Result: `src/scene.rs`, same shape as memory (§5.4): one
  `FreeCellGame` root node and four `ember_ui` header buttons (Hint,
  Restart, New, Menu). Cards are drawn by the root node, not as nodes.
- The plan said click-to-select; not needed: the game's own drag-and-drop
  state machine (`drag.rs`) runs unchanged on `Input` (press / down /
  release), and the drag overlay reads the pointer saved in `process`.
- Font: DejaVu Sans Mono (for ♠♥♦♣, `·`, `→`) is read from `assets/` and
  installed with `RenderingServer::create_font` + `set_default_font` in
  `ready` — no `load_ttf_font`.
- Engine: `Key::Semicolon` added (`18513a1`) for the win screen's
  M / Semicolon "menu" shortcut.
- Logic touched: the five `Button` fields and `update_menu_layout` left
  `Game` (`start_btn` was never drawn); `Rect` / `Color` come from
  `ember_core`; `hit::contains` became `Rect::contains`.

### 5.7 breakout — **migrated** (`ee84048`, 2026-11-26)

- 17 unit + 1 scenario (body unchanged) + 6 headless `SceneLoop` tests
  (`games/breakout/tests/scene.rs`).
- Result: `src/scene.rs`, one `BreakoutGame` root node, same shape as
  snake (keyboard only, no UI subtree). Bricks stay a `Vec<Brick>` drawn
  by the root node — no node per brick.
- The per-run `GameContext` lives in the node: the ball speeds up 10 %
  per level and R reloads the config, as before.
- `ember_stdlib::Sprite` holds a macroquad `Color`, so breakout has a
  local `Sprite` twin (same fields and `hide` / `show`). The scenario
  test's `use macroquad::prelude::Color` became `use ember_core::Color`
  (one import line).

### 5.8 asteroids — **migrated** (`f03a117`, 2026-11-26)

- 19 unit + 10 scenario (bodies unchanged) + 7 headless `SceneLoop`
  tests (`games/asteroids/tests/scene.rs`).
- Result: `src/scene.rs`, one `AsteroidsGame` root node (keyboard only,
  like breakout). Ship, asteroids, bullets and stars use the existing
  `draw_triangle` / `draw_line` / `draw_circle`.
- Wraparound: **no engine change needed.** The macroquad version only
  wrapped *positions* (`wrap_position`) and never drew an object twice
  across an edge, so `set_wrap_mode` stays unplanned until a game needs
  it.
- Randomness: `macroquad::rand::gen_range` became a seeded
  `ember_core::Rng` behind `systems::seed_rng` (clock seed in `run`),
  the snake pattern (§5.3).
- `Sprite` is a local twin (as breakout, §5.7); the scenario test's
  `macroquad::prelude::Color` import became `ember_core::Color`.
- Gotcha found here: macroquad's named `RED` / `GREEN` / `YELLOW` are not
  pure RGB. Copy the macroquad values (`8a1d7d9` fixed breakout).

### 5.9 air-hockey — **migrated** (`66987a2`, 2026-11-26)

- 58 unit + 10 scenario + 6 headless `SceneLoop` tests
  (`games/air-hockey/tests/scene.rs`).
- Result: `src/scene.rs`, one `AirHockeyGame` root node (keyboard only).
  The game's own physics (`physics.rs`) is kept as is; it does not use
  `PhysicsServer` — that stress test is left for later.
- Input: `Game::update` took the stdlib `Input` (macroquad `KeyCode`);
  it now takes the engine `Input` / `Key`. Tests changed mechanically
  only: `KeyCode` → `Key`, and `..Input::default()` in the hand-built
  `empty_input` (the engine `Input` has more fields).
- Audio: `SoundBank` holds `AudioClipId`s loaded through
  `AudioServer::load_clip` in `ready`; `on_event` returns the clip and
  volume, and the node plays it. No `.wav` ships in git, so the game is
  silent from a fresh checkout, as before.
- Font: DejaVu Sans Mono via `create_font` / `set_default_font` (as
  freecell, §5.6). The countdown's `draw_text_ex` scale is a size
  multiplier (`120 × scale`).
- Dropped: the frame-time text — it was drawn before `clear_background`,
  so it never showed. `high_dpi` is not configurable through
  `run_window`.

### 5.10 bullet-hell — **migrated** (`8b27a11`, 2026-11-26)

- 34 unit + 11 scenario (unchanged — they never touched macroquad) + 6
  headless `SceneLoop` tests (`games/bullet-hell/tests/scene.rs`).
- Result: `src/scene.rs`, one `BulletHellGame` root node (keyboard only).
  Bullets are circles drawn by the root node, not nodes; 300+ bullets is
  300+ `draw_circle` + `draw_circle_outline` calls per frame, no sprite
  batching needed yet.
- Engine: `Key::LeftShift` / `Key::RightShift` added (`c6d3055`) for
  focus mode.
- As before, there is no quit key; `draw_centered_shadowed` became a
  local `shadowed` helper (black, 2 px).

### 5.11 pagoda

- 112 unit.
- Custom rendering (blocks with gradients, slicing effects). Good test
  of `RenderingServer` primitives beyond rectangles.

### 5.12 taiji

- Various unit tests.
- Match state machine, good test of `GameState`.

### 5.13 zhuo-ji

- Custom rendering. The migration reveals any gaps in
  `RenderingServer`'s primitive coverage.

### 5.14 bore-inc

- 202 unit + 21 scenario.
- Large. Grid + upgrades + save/load + sprites. The largest single
  migration after `ember-wars` and `jade-garden`.

### 5.15 jade-garden

- 293 unit + 10+22 scenario.
- Match-3 with cascades, poem reveals, chapters. The most complex
  *content* after `ember-wars`.

### 5.16 accretion

- 125 unit + 2 balance.
- Prestige economy. The prestige system is a good test of cross-run
  persistence (`progress()` / `progress_mut()`).

### 5.17 ember-wars (acceptance test)

- 316 unit + 20 scenario.
- The most complex game. If migration requires engine changes, we learn
  them here. See [[42-acceptance-tests]] for the
  detailed checklist.

## 6. Rollback

### 6.1 Reverting a single migration

Every migration PR is a single squash-merge to `main`. Reverting is
`git revert <merge-commit-hash>`. The game returns to its pre-migration
state, and any subsequent migrations to other games are unaffected
(since each game's migration is independent).

### 6.2 Reverting a mid-migration break

If a migration is in-progress and something goes wrong (e.g., step 5
reveals a design gap), the branch is abandoned:
`git branch -D migration/<game>`. The game on `main` is untouched.

The branch should be small enough (≤ 8 commits) that re-doing it from
scratch after the design gap is fixed is cheaper than trying to rebase.

## 7. When to stop a migration and file an ADR

A migration should be paused if:

1. **The trait blocks a game.** If migrating a game requires
   downcasting past `Node` in a way that no other game needs, the trait
   is missing something. File an ADR.
2. **Behavior changes.** If a migration changes a test's expected value,
   stop. The test was checking behavior; the migration must preserve it.
3. **New unsafe is needed.** If a migration requires `unsafe` that
   wasn't there before, stop. Something is wrong with the engine's
   abstraction.
4. **The game needs a new server method that only it uses.** If
   `RenderingServer` needs a new method for one game, wait until two
   games need it. Otherwise the trait grows to fit a corner case.

In all four cases, the answer is "fix the engine, then resume
migration", not "hack the game".

## 8. Success criteria per migration

A migration is done when:

| Criterion | How verified |
|---|---|
| `main.rs` is ≤ 10 lines | `wc -l games/<game>/src/main.rs` |
| Game runs a `SceneLoop` (or `impl MainLoop`) | `grep -rn "SceneLoop\|impl MainLoop" games/<game>/src/` |
| No `use macroquad` in game code | `grep -rn "macroquad" games/<game>/src/` returns 0 hits |
| No `juice.rs` | `ls games/<game>/src/juice.rs` fails |
| No `audio.rs` | `ls games/<game>/src/audio.rs` fails |
| All tests pass | `cargo test -p <game>` |
| Visual behavior matches | Manual play-test against pre-migration video |

The first four are grep-able and can be added to CI (`check-macroquad` already
enforces the third for every game off the allowlist). The last one
requires human verification.

## 9. Cross-references

- The phases that this guide depends on → [[40-roadmap]]
- The acceptance test (`ember-wars`) → [[42-acceptance-tests]]
- The trait being implemented → [[21-ember-scene]]
- The MainLoop being implemented → [[14-main-loop]]
