---
title: "Next Phase Plan — Engine v0.2 & Game 18"
aliases: ["v0.2 plan", "Engine v0.2 plan", "Game 18 plan"]
tags: ["ember/progress", "ember/plan", "ember/v0.2"]
created: "2026-11-27"
phase: "v0.2 (V0–V3 engine · G1–G5 game)"
estimate: "≈ 20 sessions"
---

# Engine v0.2 and Game 18 — a grid puzzle built in the editor

> [!summary] Goal
> 1. **Ember v0.2** turns the engine from "games run on it" into "games are
>    *made* with it": tiles, animation, input actions, scene changes, and an
>    editor you can paint levels in and press Play.
> 2. **Game 18** is the first game designed for the engine, not ported to it.
>    A grid puzzle with handcrafted levels, an ink-wash look like
>    [[Pagoda]], native + web (mobile swipe).
> 3. **Proof:** every level of Game 18 is an `.escene` made in the editor
>    (`make edit GAME=<slug>`), and Play in the editor runs it. This also
>    closes Phase 9's last box ("a second game opens with `make edit`").
>
> Direction chosen by the user on 2026-11-27: **grid puzzle** (over an
> action platformer and a tower defense). It keeps the game's code small, so
> the engine and editor carry the weight, and its content grows through the
> editor.

## Where we start (2026-11-27)

| Area | State |
|---|---|
| v0.1 | Phases 0–8 ✅; Phase 9 (Editor v2) has one open box; `CHANGELOG.md` on `main` (`6f3a1fd`, merged in PR #7), `0.1.0 — unreleased`, no git tag yet |
| Engine | node tree, signals, `.escene` + `PackedScene`, servers + mocks, `ember-ui`, editor library with undo and a typed inspector (ADR-001…049) |
| Games | 17 / 17 on the engine, no macroquad outside the backends; only Pong is editable |

Live checklist: [[Roadmap Tracker]] · previous plan: [[Next Phase Plan — Migration Close & Editor v2]] · how we work: [[Token Budget]], [[Commit & Push]].

---

## V0 — Close v0.1 (0.5 session)

- **You** tag `v0.1.0` on `main` once `make pre-push` is green (spec [[42-acceptance-tests]] §8), and push the tag. `CHANGELOG.md`: `[0.1.0] — 2026-11-27`.
- Phase 9's last box (a second editable game) **moves to v0.2**: Game 18 is that second game (see G2). The tracker says so.
- Fix the one CHANGELOG mismatch found while planning: it lists `Cooldown::new` / `with_duration` under *Removed*, but they are still in `ember-core` as `#[deprecated]` aliases (`crates/ember-core/src/time/cooldown.rs:94`). Reword to *Deprecated* in 0.1.0; **delete them in 0.2.0**, as [[02-principles]] asks for v0.1 deprecations.
- Open a `## [0.2.0] — unreleased` section in `CHANGELOG.md`; every v0.2 round adds its line.

## What v0.2 is — and is not

**In** (each item has evidence from v0.1 or a need in Game 18):

| # | Feature | Why | Round |
|---|---|---|---|
| 1 | **Frame snapshot**: `NodeContext::time()` (elapsed), pointer in world space, viewport | accretion and ember-wars each wrote one; 6 games keep their own `get_time` counter | V1.1 |
| 2 | **Typed tree queries**: `find_by_class`, `children_of::<T>()`, optional groups | [[Doc–Code Discrepancies]] #14; the "children of class `T`" helper seen in M3 | V1.2 |
| 3 | **Input action map**: named actions from RON (keys, gamepad, swipe), `action_pressed` / `action_axis` | spec [[15-input]], discrepancy #13; Game 18 needs keys + swipe for one "move" | V1.3 |
| 4 | **Text helpers** in `ember-stdlib` on `RenderingServer` (centred, shadowed, bold) | deleted with the macroquad `fonts` / `graphics` in M5.1; many games re-wrote them | V1.4 |
| 5 | **`TileSet` + `TileMap` node**: atlas texture, tile ids on a `Grid<T>`, cell ↔ world, compact rows in `.escene` | Game 18's board; also the base for any later level-based game | V2.1 |
| 6 | **Scene change at runtime**: `ctx.change_scene(path)` / load a `PackedScene` from `AssetServer` and swap the root | level select → level → next level; today each game builds one root | V2.2 |
| 7 | **`AnimatedSprite` + `SpriteFrames`** (sheet, named clips, fps, loop) and **`Tween`** in stdlib (from jade-garden's `animation.rs`) | Game 18's player and tile moves; jade-garden is the first user, Game 18 the second | V2.3 |
| 8 | **Editor: tile brush** — palette panel, paint / erase / pick on the preview, undoable (ADR-045 commands) | levels are painted, not typed | V3.1 |
| 9 | **Editor: Play / Stop** in the preview, restoring the document on Stop | stretch goal deferred since M1.4; testing a level without leaving the editor | V3.2 |
| 10 | **Editor: open any scene of a game + place instances** (`instance:` of a sub-scene, grid snap) | Game 18 has ~30 level files and reusable pieces (stone, lantern) | V3.3 |

**Out** (stay v0.3+ candidates, per [[00-overview]] §5.2): wgpu backend, ECS, scripting, networking, 3D, scene/code hot reload, profiler UI, Windows CI, visual regression tests. Physics against tiles is out too: the puzzle moves on the grid, no bodies needed.

**Debt to pay in passing** (only where a round touches the file): `unwrap` in engine code (discrepancy #12), `trybuild` failure tests for `ember-macros`, the WASM `fetch-mq-bundle.sh` helper (Rule of Three met 4×, [[Browser Build (WASM)]]) in G5.

---

## Game 18 — first design draft

> Working title: **to be chosen** (open question 1). Slug and `30 Games/<Name>/` hub + design note get created in G1, from [[Game Design Template]].

### Pitch
A small paper-lantern spirit crosses ink-wash courtyards at night. **Every move slides** the spirit until something stops it. Light every lantern in the courtyard to open the gate. Short handcrafted levels, solved in a few moves, with undo and restart always one key away.

### Core loop
1. Pick a level (chapter map, stars earned so far).
2. Swipe / press a direction: the spirit slides to the next obstacle.
3. Passing next to an unlit lantern lights it; the gate opens when all are lit.
4. Solve in **par** moves for a star; undo / restart freely.
5. Next level; a chapter adds one new piece.

### Pieces (one new per chapter, ≈ 10 levels each)
| Chapter | New piece | Rule |
|---|---|---|
| 1 — Courtyard | wall, lantern, gate | slide until blocked; adjacent lantern lights |
| 2 — Garden | stone | a slide into a stone pushes it one cell if free, and stops you |
| 3 — Pond | ink pool | entering one sends you out of its twin, keeping direction |
| (stretch) 4 — Bridge | paper bridge | can be crossed once, then tears into a hole |

### Engine bricks used
`TileMap` (floor, walls), instanced piece scenes (lantern, stone, pool), `AnimatedSprite` (spirit, flame), `Tween` (slides), action map (keys + swipe), `change_scene` (menu ↔ level), `Persistence<T>` (stars), `AudioServer` + `music`, `fx` (light-up burst), `ember-ui` (menu, level select), virtual resolution (ADR-041) for the web/mobile shell.

### Data
- `scenes/levels/<chapter>-<nn>.escene` — one per level, painted in the editor; par moves as an export on the root.
- `scenes/pieces/*.escene` — reusable pieces, placed as instances.
- `assets/` — one atlas for tiles + sprites (procedural ink-wash at first, as Pagoda did; drawn art later).

### Tests
- Pure rules (`rules.rs`): slide, push, pool, lantern, win, undo — unit tests, no engine.
- **Every level is solvable in par**: a test runs a BFS solver over each `.escene` and checks the par. Adding a level that can't be solved fails `make pre-push`.
- `tests/scene.rs`: `SceneLoop` on mocks, menu → level → win → next.
- `tests/editor.rs`: open a level in `ember_editor::Document`, paint, save, reload.

---

## Milestones and rounds

One round per session ([[Token Budget]] §2.1); each engine feature is its own round with its own ADR (next free: **ADR-050**), landed just before the game round that needs it (the M2 pattern).

| # | Round | Kind | Sessions | ADR | Unblocks |
|---|---|---|---:|---|---|
| **V0** | Close v0.1 (tag, CHANGELOG) | chore | 0.5 | — | v0.2 |
| **G1** | Game 18 rules: crate, `rules.rs`, solver, design note | game, pure | 1 | — | all G rounds |
| **V1.1** | Frame snapshot / `time()` | engine | 1 | 050 | G2 |
| **V1.2** | Typed tree queries | engine | 0.5 | 051 | G2 |
| **V1.3** | Input action map | engine | 1.5 | 052 | G2 |
| **V1.4** | stdlib text helpers | engine | 0.5 | — | G4 |
| **V2.1** | `TileSet` + `TileMap` node | engine | 2 | 053 | G2, V3.1 |
| **V2.2** | Runtime scene change | engine | 1 | 054 | G4 |
| **G2** | Game 18 on nodes: board as `TileMap`, pieces as instances, 3 hand-written levels; `make edit GAME=<slug>` opens them (**closes Phase 9**) | game | 1.5 | — | V3 |
| **V3.1** | Editor tile brush | editor | 1.5 | 055 | G3 |
| **V3.2** | Editor Play / Stop | editor | 1 | 056 | G3 |
| **V3.3** | Editor: scene picker + instance placement | editor | 1 | — | G3 |
| **V2.3** | `AnimatedSprite`, `SpriteFrames`, stdlib `Tween` | engine | 1.5 | 057 | G3 |
| **G3** | Chapters 1–2 made in the editor; art, animation, sounds | game + content | 2 | — | G4 |
| **G4** | Menu, level select, stars, persistence; chapter 3 | game | 1.5 | — | G5 |
| **G5** | Web build + mobile shell, itch.io page ([[itch.io Release Media]]); tag `v0.2.0` | release | 1 | — | — |
| | **Total** | | **≈ 20** | | |

G1 needs no engine change, so it can run in parallel with V1.x (different files: a new `games/<slug>` vs `crates/`).

## Definition of done (v0.2.0)

- Every feature in the "In" table has an ADR, tests (headless on mocks), a `CHANGELOG.md` line, and a spec note (a new `10 Engine Spec v0.2/` folder holds the deltas against v0.1; v0.1's files stay frozen).
- Game 18: ≥ 25 levels, all made with the editor, all proven solvable in par by the test; native + web; played by you before each push.
- No game regressed: `make pre-push` green, the deprecated `Cooldown` aliases gone.
- Tag `v0.2.0`.

## Open questions

1. **Game 18's name** (and slug). Fits the Chinese-themed line (Zhuo Ji, Jade Garden, Pagoda, Taiji)?
2. **Art**: procedural ink-wash (as Pagoda, no asset work) for the whole game, or drawn sprite sheets from G3 on? Drawn art tests `AnimatedSprite` harder.
3. **Groups** in V1.2: add them now (spec deferred them to v0.2), or wait until a third user appears (Rule of Three)?
4. **Mobile**: swipe only, or also on-screen arrows (as Pagoda's shell has a tap button)?
