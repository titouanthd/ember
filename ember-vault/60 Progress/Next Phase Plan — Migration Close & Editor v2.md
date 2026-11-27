---
title: "Next Phase Plan — Migration Close & Editor v2"
aliases: ["Current plan", "Phase 7-9 plan", "Editor v2 plan"]
tags: ["ember/progress", "ember/plan", "ember/phase-7", "ember/phase-8", "ember/phase-9"]
created: "2026-11-26"
phase: "7 (close) · 8 (cleanup) · 9 (Editor v2, new)"
estimate: "≈ 26 sessions"
---

# Next phases — finish the migration, then Editor v2

> [!summary] Goal
> 1. **Every game runs on the latest engine.** All 17 games are node trees
>    on `SceneLoop`, and none imports macroquad, a macroquad-backed
>    `ember-stdlib` module or a local "twin" of an engine type.
> 2. **A big refactor of the editor (Editor v2).** It becomes a general
>    tool that a game plugs its node classes into, not a demo that knows
>    two hard-coded classes.
> 3. **Proof:** `make edit GAME=pong` opens Pong in the editor. You can
>    edit it and save it, and `cargo run -p pong` then plays the saved
>    scene.
>
> The proof comes **first** (milestone M1), because it shapes how every
> later game is written: as data the editor can open, not only as code.

## Where we start (2026-11-26)

| Area | State |
|---|---|
| Phases 0, 2, 4, 5, 6 | ✅ closed |
| Phase 1 / 3 | one open item each (`hello_scene` headless run; scene-declared connections) |
| **Phase 7** | **9 games fully migrated** (snake, simon, memory, minesweeper, freecell, breakout, asteroids, air-hockey, bullet-hell) + **pong partly** → 10 / 17 counted, 8 to go incl. pong |
| Phase 8 | `juice.rs` gone from games; 10 `ember-stdlib` modules still import macroquad |
| Editor | POC: opens `player.escene`, edits properties as text, previews, saves. Knows only its own `Player` / `Sprite` classes; drops connections and ext resources on save; no add/remove nodes, no undo |

Live checklist: [[Roadmap Tracker]] · state and push steps: [[Handover 2026-11-26]].

## Definition of done

**Per game ("latest engine"):**
- `src/scene.rs` holds the node tree, and `main.rs` is ≤ 10 lines.
- No macroquad anywhere in `src/` or `tests/`, and the game is off `xtask/macroquad-allowlist.txt`.
- It uses no macroquad-backed `ember-stdlib` module (`audio`, `config::env_color`, `fonts`, `graphics`, `input`, `juice`, `layout`, `music`, `sprite`, `ui`).
- It has no local twin of an engine type (`env_color`, `Sprite`), once Phase 8 gives `ember-stdlib` engine-typed versions.
- Existing tests are green with only mechanical edits (imports, `KeyCode` → `Key`), and a `tests/scene.rs` drives `SceneLoop` on mocks.
- You have played the game before pushing, as per the round rule below.

**Editor proof ("opens Pong"):** see [[Next Phase Plan — Migration Close & Editor v2#Proof checklist — the editor opens Pong|the proof checklist]].

## Milestones

| # | Milestone | Sessions | Unblocks |
|---|---|---:|---|
| **M1** | Pong finished **and** described as an `.escene`; the editor opens it | 4 | the proof; the pattern for "games as data" |
| **M2** | Engine gaps, each as a small renderer/audio feature with its own ADR, landed just before the first game that needs it | 4 | M3 |
| **M3** | The 7 remaining games, one per round | 13 | Phase 7 exit, acceptance test |
| **M4** | Editor v2: lossless documents, node editing, undo, typed inspector, panel split | 3 | Phase 9 exit |
| **M5** | Phase 8 cleanup: retire the macroquad parts of `ember-stdlib`, drop the twins, `CHANGELOG.md` | 2 | v0.1 POC done |

Rounds keep the current rhythm: **one game or one engine feature per round**. I commit locally; you run `make pre-push`, play the game and push ([[Commit & Push]]).

---

## M1 — Pong, and the editor opens it (proof first) — ✅ proof checked 2026-11-27

### M1.1 Finish Pong (0.5 session) — ✅ `09dd9b8`
- `macroquad::Color` → `ember_core::Color` in `config.rs`, `components.rs` and `scene.rs` (the `color()` bridge goes away).
- `main.rs` (20 lines) moves into `scene::run()`, leaving 5 lines.
- Drop `ember_stdlib::app::GameState` for `ember_core::GameState`.
- Off the allowlist. The tracker line becomes ✅.

### M1.2 Pong as data (1 session) — ✅ `f866d2e`
> [!note] As built
> - `Paddle` exports `position` instead of `side` (the editor preview needs a position; `ai` already picks the side). Only `x` matters in play: `y` is centred at each serve, on each paddle's own height.
> - `Ball` and `Scoreboard` also export `position`, and `Pong` exports `court` (the window size), so the preview can draw the scene before `ready`.
> - `Pong` reads its children in `ready`, and copies the entities' positions back to them each frame. The children draw themselves.
> - `.env` is now all comments. Its values moved into `pong.escene`, so play is unchanged, and a variable you set there overrides the scene. Without that change, `.env`'s `PADDLE_HEIGHT` would have hidden any size edited in the editor.
> - Engine: an `f32` export now saves as its shortest decimal (`0.08`, not `0.07999999821186066`).
- The nodes become editable classes with `#[derive(Node)]` and `#[export]`:
  - `Pong` (root): match rules such as the score to win and the ball speed.
  - `Paddle` ×2: `side`, `size`, `speed`, `color`, `ai`.
  - `Ball`: `size` and `color`.
  - `Scoreboard`: `font_size` and `color`.
- Their behaviour stays in hand-written `impl Node`, as allowed since ADR-030.
- New `games/pong/scenes/pong.escene`, with `pong::scene::classes() -> ClassDB` registering the four classes.
- `build_tree` **instantiates the scene file** instead of building the tree in code. `.env` values still apply as overrides, so nothing changes in play.
- Tests:
  - The loaded tree matches the old hand-built one (names, classes, positions).
  - A save → load → save round trip is identical.
  - The existing pong tests stay green.
- Risk: `.escene` cannot express the `PointScored` connection yet (Phase 3 open item). Keep connecting in `ready` for now; it gets solved in M4.1.

### M1.3 Editor as a library + game plug-in (1 session) · **ADR-039 "How a game plugs into the editor"** — ✅ `6aaf9a0`
> [!note] As built
> `make edit` edits `games/<game>/scenes/<game>.escene` **in place**, not on a scratch copy. The proof checklist needs Save to reach `cargo run -p pong`, and the file is tracked by git. The headless `games/pong/tests/editor.rs` landed here too.
- Layers forbid `ember-editor` from depending on games, so the dependency points the other way:
  - `ember_editor::run(EditorConfig { classes, scene, viewport })` becomes a library entry point.
  - Each game that wants editing gets an `examples/edit.rs` (3 lines) calling it with its own `classes()`.
  - `make edit GAME=pong` runs `cargo run -p pong --example edit -- scenes/pong.escene`, on a scratch copy as `make editor` does today.
- The built-in `Player` / `Sprite` classes stay as the editor's own demo set (`make editor` unchanged).

### M1.4 Preview v2 (1 session) · **ADR-040 "Camera transform (offset + zoom)"** — ✅ `dfa8b04` (Play / Stop not done)
> [!note] As built
> `set_camera(Camera2d { offset, zoom })` replaces `set_camera(offset)`. Pong previews at zoom 0.625, centred. The Play / Stop stretch goal is **not done**; it fits M4.2, next to undo.
- The editor's preview draws the scene at its **real resolution** (Pong is 800 × 600), scaled to fit the preview panel.
- That needs a zoom in `RenderingServer::set_camera`, which today takes an offset only. Pagoda, taiji and ember-wars need the same feature, so it is built once here.
- The preview calls `ready` on the opened scene, so nodes that set themselves up at that point (fonts, children) look right.
- Stretch goal: a **Play / Stop** toggle that runs `process` in the preview and restores the document on Stop.

### Proof checklist — the editor opens Pong
> [!success] Checked in the window, 2026-11-27
> The user ran the five checks with `make edit GAME=pong`. The edit used was **`Ball.size` 20 → 50**, not `PlayerPaddle.size` → `16, 160`: same path (select → inspector → Enter → preview → Save → `cargo run -p pong`), and the user accepted it as the proof. Save wrote `pong.escene` in place; play showed the bigger ball. The scene was then restored to its committed sizes. No code change was needed. **M1 is closed.**
- [x] `make edit GAME=pong` opens a window showing the outline `Pong / PlayerPaddle / AiPaddle / Ball / Scoreboard`
- [x] the preview shows the Pong court at 800 × 600, scaled into the panel
- [x] selecting `PlayerPaddle` shows `size`, `speed`, `color`; changing `size` to `16, 160` updates the preview *(proved on `Ball.size` 20 → 50)*
- [x] Save writes `pong.escene`; reopening it shows the change
- [x] `cargo run -p pong` with the saved scene plays with the taller paddle *(the bigger ball)*
- [x] headless test: `games/pong/tests/editor.rs` opens `pong.escene` in `ember_editor::Document`, edits, saves and reloads *(`6aaf9a0`)*

---

## M2 — Engine gaps (with an ADR each)

The gaps below come from a grep of what the remaining games use. Each one lands as its own round, just before the first game that needs it.

| Feature | Needed by | Plan | ADR |
|---|---|---|---|
| Camera offset + zoom ✅ `dfa8b04` | pagoda, taiji, editor preview (ember-wars already scrolls with its own camera and `set_camera(offset)`) | `set_camera(Camera2d { offset, zoom })` + `DrawContext::world_to_screen` | ADR-040 (M1.4) |
| **Virtual resolution / letterbox** ✅ `ef68ce5` | pagoda, taiji (both render to a `render_target` of a fixed size and scale it to the window) | `SceneLoop` / backend option `VirtualResolution { w, h, filter }`: draw at a fixed size, scaled and letterboxed into the window; mouse mapped back. As built: `with_virtual_resolution(w, h)` with vector scaling through `Camera2d`, so no render target and no `filter` | ADR-041 |
| **Shapes** ✅ `20a1c21` | zhuo-ji, jade-garden (`draw_poly`, `draw_poly_lines`, `draw_arc`, `draw_ellipse`), pagoda (`draw_arc`) | `draw_polygon(points)`, `draw_regular_polygon(center, sides, radius, rotation)`, `draw_arc`, `draw_ellipse` (+ outline variants) on `RenderingServer`; mock records them | ADR-042 |
| **Looping audio / music** ✅ `d5ba842` | bore-inc, jade-garden (`ember_stdlib::music`), pagoda | `AudioServer::play_looped` / `set_volume(SoundId)`; music becomes a looped clip. As built: `play_looped` + `is_clip_ready`, MP3 decoded natively; `set_volume` deferred | ADR-043 |
| **Window size at runtime** ✅ `bebe16d` | accretion, jade-garden, bore-inc, ember-wars (`screen_width()` / `screen_height()` 20+ times each) | `NodeContext::viewport()` / `DrawContext::viewport()` (the size `SceneLoop` already knows) + a `ViewportResized` signal for resizable windows. As built: `SceneLoop::with_viewport_following_window` and `RenderingServer::viewport`, no signal (games re-read it each frame) | ADR-041 amendment (ADR-044 not needed) |
| Sprite drawing options ✅ | bore-inc, pagoda, taiji (`draw_texture_ex`: source rect, size, rotation, flip) | check `draw_sprite(texture, dst, src, tint)` covers it; add rotation / flip only if a game needs them. As built: `draw_sprite` covered bore-inc; it needed nearest filtering instead | ADR-048 |
| Text options | zhuo-ji, pagoda, jade-garden (`draw_text_ex`) | check each use: scale = size multiplier (as air-hockey); rotation only if used | — |
| Clipping ✅ | jade-garden, accretion (`get_internal_gl().quad_gl.scissor`) | **already exists** (`push_clip_rect`) — no new API, just replace | — |
| Elapsed time | pagoda, taiji, zhuo-ji, jade-garden, accretion, ember-wars (`get_time`) | keep a time counter in the root node (as asteroids does); add `NodeContext::time()` only if three games need it | — |

Web builds (pagoda, jade-garden, accretion, bore-inc) must keep building ([[Browser Build (WASM)]]). Accretion's `ember_storage_*` web storage functions stay game-local.

---

## M3 — Remaining games (one per round)

Suggested order: small games first, each one landing the engine feature the next one needs. This is a change from [[41-migration-guide#3. Migration order|§3]]: taiji now goes before pagoda.

| Round | Game | Size | Needs | Est. | Notes |
|---|---|---:|---|---:|---|
| 1 | **taiji** ✅ `60ba747` | 0.7 k lines, 0 tests | ADR-040, ADR-041 | 1 | smallest user of the render-target pattern → proves the letterbox; add a first scenario test |
| 2 | **pagoda** ✅ `980dc1e` | 3.4 k, 0 tests | ADR-041, shapes (arc), juice, persistence, sounds | 1.5 | Native + WASM; replace `ember_stdlib::juice` with an engine-typed version (Phase 8 work, pulled forward) |
| 3 | **zhuo-ji** ✅ `614cf30` | 9.2 k, 1 test file | ADR-042 shapes, fonts, `layout::centered_rect`, `ui::hit` | 2 | biggest UI-style game; `ui::hit` → `Rect::contains` |
| 4 | **jade-garden** ✅ `4ef6142` | 9.7 k, 3 test files | ADR-042, ADR-043, ADR-044, clip, mouse wheel, animation, juice | 2.5 | shipped on itch.io (web) — playtest native **and** web |
| 5 | **accretion** ✅ `ddf58a6` | 3.7 k, 2 test files | ADR-044, clip, mouse wheel | 1.5 | web storage stays as is |
| 6 | **bore-inc** ✅ `1b08851` | 8.3 k, 1 test file | textures, sounds, music (ADR-043), ADR-044, touches | 2 | marked **abandoned** — see [[Next Phase Plan — Migration Close & Editor v2#Open questions\|open questions]] |
| 7 | **ember-wars** ✅ ported `f16fe0c`, node tree `0ad070c` | 9.5 k, 1 test file | ADR-044, own camera (`set_camera` offset), juice, fonts | 2.5 | **acceptance test** — full checklist in [[42-acceptance-tests]]; migrates last |

Each round follows the migration recipe in [[41-migration-guide#5. Per-game notes|§5]]:
- existing tests unchanged, or changed only mechanically (and said so);
- a new `tests/scene.rs`;
- off the allowlist;
- a `41-migration-guide` §5 entry, a game hub note, the tracker, and the Home count.

---

## M4 — Editor v2 (the big refactor)

The proof (M1) already makes the editor general. M4 makes it a real tool. Each session is one round, with tests on `Document` (headless) plus `SceneLoop` tests on the app.

### M4.1 Lossless documents (1 session) — ✅ `8b20132` (2026-11-27)
- `Document` keeps `ext_resources`, `connections` and unknown properties, and writes them back on save. Today they are dropped.
- Scene-declared connections get wired at instantiate time. **This closes the Phase 3 open item**, and Pong's `PointScored` connection moves into `pong.escene`.
- Test: opening and saving every `.escene` in the repo changes no byte.
- **Done:** `PackedScene::instantiate_tracked` keeps a `NodeOrigin` per node (properties as written, values as loaded); `PackedScene::resave` writes the tree with the file's uid, ext / sub resources and connections. A property is written if the file had it (in the file's form while unchanged, so `Int(5)` stays `Int(5)`) or if it differs from the class default; unknown keys go back as written; a node with no origin (M4.2's added nodes) writes every export. Connections are wired after all nodes exist, in file order, by `ClassDbNodes::register_receiver::<N, T>(signal, method, fn(&mut N, &T))`; one with no receiver warns and is skipped. `player.escene` was rewritten once in canonical form (comment and entry order only). `ember-editor/tests/lossless.rs` fails when a new `.escene` has no round-trip test.
- **Left for M4.2:** a renamed or removed node must update or drop the connections that name it (`resave` keeps the file's connection list as is); nested `instance:` nodes still save inline (no repo scene uses them); a receiver gets `(&mut N, &T)`, no `NodeContext`.

### M4.2 Node editing + undo (1 session) · **ADR-045 "Editor commands and undo"** — ✅ `203c0a4` (2026-11-27)
- A command stack: `SetProperty`, `AddNode(class, parent)`, `RemoveNode`, `RenameNode`, `Reparent`, each one reversible.
- Undo / redo with Ctrl+Z / Ctrl+Shift+Z, and Ctrl+S to save. These need `Key::LeftControl` / `RightControl` / `Super` (a small input addition, like Semicolon and Shift).
- The outline gets **Add** (a class picker over `ClassDB`), **Delete** and **Rename**, and the dirty flag asks before closing.
- **Done:** see [[50-design-decisions#ADR-045: Editor commands and undo|ADR-045]]. Commands live on `Document` (one linear stack; removed nodes stay detached, so ids never die); connections are kept by id and saved with their current paths, so rename / reparent follow them and M4.1's left-over is closed. Toolbar: Undo, Redo, Add (class picker), Delete, name field + Rename; Ctrl / Cmd + Z, + Shift + Z, + Y, + S, Delete. The close button asks through `WindowConfig::confirm_close` → `Input::close_requested` (Save and quit / Quit without saving / Cancel). Tests: `ember-editor/tests/commands.rs` (9, `Document`), `tests/app_commands.rs` (9, `SceneLoop`), `ember-scene/tests/attach_at.rs`.
- **Not done:** Reparent has no outline UI yet (drag and drop → M4.3); Play / Stop (optional) not started; added nodes do not run `ready` in the editor.

### M4.3 Inspector v2 + panel split (split into M4.3a + M4.3b, below)
- `app.rs` (469 lines) is split into `toolbar.rs`, `outline.rs`, `inspector.rs` and `preview.rs`, each a `Control` subtree talking through signals (ADR-036).
- Typed editors replace "everything is a text box":
  - `bool` → a checkbox;
  - `Vec2` → two number fields;
  - `Color` → a swatch plus four fields;
  - an enum → cycling through its values;
  - text stays as text for everything else.
- The 16-property cap goes, because the inspector scrolls (`ScrollContainer`).
- The preview gets selection highlight, click-to-select in the preview, and drawn textures.

**Split agreed 2026-11-27** (the user's choices; [[Token Budget]]: the M4 session was past ~100 steps). M4.3 runs as two fresh sessions, **M4.3a then M4.3b**. Start each from this section, ADR-045 and the current `crates/ember-editor` code.

#### M4.3a Editor: panels, typed inspector, preview select (1 session) · **ADR-049 "Enum exports (`#[export(enum)]`)"**
- **Enum exports first (engine, own ADR).** `#[export(enum = "Easy,Normal,Hard")]` in `ember-macros` (today only `#[export]` / `#[export(skip)]`, `crates/ember-macros/src/node.rs`), plus `NodeName::property_hint(key) -> PropertyHint` with a default of `PropertyHint::None` and `PropertyHint::Enum(&'static [&'static str])`. The value travels as `Variant::String` (the field's type implements `VariantValue` through its name; a helper derive or a hand impl is the ADR's call). `Document::property_hint` passes it to the inspector.
- **Panel split.** `app.rs` (~750 lines after M4.2) becomes `toolbar.rs`, `outline.rs`, `inspector.rs`, `preview.rs`, each building its own `Control` subtree and emitting editor signals (`NodeSelected`, `PropertyEdited`, `CommandRequested`…, ADR-036 style: signals carry their sender); `EditorApp` stays the one owner of the `Document` and applies them. Keep the M4.2 behaviour and its tests (`tests/app.rs`, `tests/app_commands.rs`): only mechanical changes, said in the commit.
- **Typed editors** (ember-ui has Button, Label, TextInput, Panel, ScrollContainer, HBox / VBox, no checkbox or dropdown): bool → a toggle `Button`; number → `TextInput`; `Vec2` → two `TextInput`s; `Color` → a swatch (a small drawn rect) and four fields; enum → a cycling `Button` (or a popup list like the Add class picker); anything else stays text. Each edit is one `SetProperty` (undoable).
- **No 16-property cap:** the inspector rows live in a `ScrollContainer` and are built per selection (`spawn_child` / `despawn`, as the outline rebuild does), so `MAX_PROPERTIES` goes.
- **Preview:** click-to-select (screen → scene through `preview_camera`, topmost node whose rect holds the point; today `selection_rect` guesses a rect from `position` / `size`), selection highlight kept. Drawn textures only if cheap.
- **Reparent UI** (left from M4.2): in the outline, e.g. "Move into…" / indent-outdent buttons, or drag and drop; `Document::reparent` exists.
- Done: tests (`Document` + `SceneLoop`), `feat(editor)` commit, then plan / tracker / ADR-049.

#### M4.3b Breakout as data (1 session)
- **Bricks as nodes** (the user's choice): `games/breakout/scenes/level1..3.escene`, each a level root with one `Brick` node per brick (`#[export]` position, size, color, health), replacing `levels/levelN.ron` (`systems::load_level`, `levels.rs` `BrickData`). `BreakoutGame` reads the bricks from its scene the way Pong's root reads its children (M1.2 pattern, `games/pong/src/scene.rs` `read_scene`).
- `games/breakout/examples/edit.rs` (copy of Pong's) and `make edit GAME=breakout`: the Makefile opens `games/$(GAME)/scenes/$(GAME).escene`, so either name the first level `breakout.escene` or pass `GAME_SCENE=…`.
- Add the new scenes to `COVERED` in `crates/ember-editor/tests/lossless.rs` with a byte-for-byte test in `games/breakout/tests/`.
- **Avoid `games/breakout/src/components.rs`:** M5.2 switches its `Sprite` twin to `ember_stdlib::Sprite`. Check M5's branch first and coordinate at merge.
- Done: the last Phase 9 exit box; `feat(breakout)` commit, then plan / tracker.

### Phase 9 exit criteria (Editor v2)
- [ ] the M1 proof checklist passes
- [x] every `.escene` in the repo round-trips byte-for-byte (connections, ext resources included) *(M4.1)*
- [x] add / remove / rename / reparent a node, each undoable *(M4.2; reparent from code / tests only, UI in M4.3)*
- [ ] typed editors for bool, number, `Vec2`, `Color`, enum
- [ ] at least one other migrated game (suggested: breakout, bricks as data) also opens with `make edit GAME=…`

---

## M5 — Phase 8 cleanup (2 sessions)

- `ember-stdlib` loses its macroquad modules: `audio`, `config::env_color`, `fonts`, `graphics`, `input`, `juice`, `layout`, `music`, `sprite`, `ui`.
  - Each one is replaced by an engine-typed version (for example `env_color → ember_core::Color`, `Sprite` with `ember_core::Color`, `juice` on `RenderingServer`), or deleted when nothing uses it.
- The games' local twins (`env_color` in 5 games, `Sprite` in breakout and asteroids) switch to the stdlib versions.
- `xtask/macroquad-allowlist.txt` is empty except for comments, so `check-macroquad` holds for the whole workspace.
- `CHANGELOG.md`, and the tracker's Phase 8 boxes.
- `40-roadmap` gains Phase 9 (Editor v2). [[Doc–Code Discrepancies]] items #4 (phase numbering) and #18 (Makefile `CRATES`) get closed.

---

## After M3 — order agreed (2026-11-27)

M3 is done (17 / 17, ember-wars acceptance passed). Next, in this order:

1. **The remaining milestones, each in its own session** ([[Token Budget]]):
   - **M1 proof** — the editor-opens-Pong checklist in the window (the user checks; the session fixes what fails).
   - **M4 Editor v2** — M4.1 → M4.2 → M4.3, one round each.
   - **M5 Phase 8 cleanup** — `ember-stdlib` without macroquad, twins replaced, `CHANGELOG.md`.

   M1 and M4 both touch `crates/ember-editor` and `games/pong`: M4 starts once the M1 proof is merged. M5 is independent (stdlib and games), except pong's twins if any — coordinate at merge.
2. **Plan engine v0.2**, once v0.1 is tagged (spec 42 §8 gate). Candidates seen during M3: a per-frame screen/pointer/time snapshot in the engine (accretion and ember-wars each wrote one), a typed "children of class `T`" helper, shared font helpers, sprite animation, tilemaps, more `ember_ui` in games.
3. **A new game, designed for the engine**: it uses the v0.2 features, ships a web build, and is built with the editor (levels and entities as `.escene` data).

---

## Open questions

1. ~~**bore-inc** is marked *abandoned* (WASM reference only). Should we migrate it, or freeze it?~~ Migrated (`1b08851`) on the user's go-ahead ("Start bore-inc round"); it stays marked abandoned as a game.
2. **Which games are editable?** The definition of done above does not require every game to open in the editor. Pong is the proof, and at least one more game is required by Phase 9's exit criteria. Converting a game to data costs ≈ 0.5 session each. Should more of them become editable?
3. ~~**High DPI:** air-hockey used `high_dpi: true`, and `run_window` has no such option.~~ Answered by ADR-047 (`run_window_with`, `45392e9`): zhuo-ji uses it; air-hockey can opt back in.

## Estimate

| Milestone | Sessions |
|---|---:|
| M1 Pong + editor proof | 4 |
| M2 engine gaps (ADR-041…044) | 4 |
| M3 seven games | 13 |
| M4 Editor v2 | 3 |
| M5 cleanup | 2 |
| **Total** | **≈ 26** |

At the current pace (≈ 6 rounds a day), that is about **4–5 working days**.
