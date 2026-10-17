# SESSION2.md — Handoff to the next agent

**Date**: October 2026
**Status**: Starter kit complete. Archie sprite iterated to "good enough".
**Next**: gameplay systems (truck, robots, HQ, map, procedural gen).

---

## 1. Context

`bore-inc` is the 14th game in the Ember workspace. It's an incremental
mining game: Archie (a cat) drills a tunnel through a mountain so the
Bore Inc. truck can cross. Movement-only input (no menus, no clicking).
Target: browser via WASM, native optional.

The game's design doc is `DESIGN.md` in this folder. It's complete and
authoritative — read it before touching anything. `SESSION1.md` is the
design-phase report (no code yet).

**The user speaks French.** The codebase and docs are in English
(workspace convention). Reply in whichever language the user writes in,
but keep code and comments in English.

**Read `version.txt` at the workspace root before starting.** It lists
~120 pitfalls learned across 13 previous games. Most of them apply here.

---

## 2. What was accomplished this session

### Starter kit (Session 2 in conversation)

Created `games/bore-inc/` from scratch, modeled on `jade-garden`:

- `Cargo.toml` — standard workspace deps, no `soloud`, no `image`.
- `src/main.rs` — entry point, preload order, main loop.
- `src/lib.rs` — module list + reexports.
- `src/paths.rs` — asset/progress paths, cfg'd native/wasm.
- `src/config.rs` — `GameContext`, `Colors::defaults()`.
- `src/components.rs` — `TileKind`, `ResourceKind`, `Tile`.
- `src/systems.rs` — `AppState`, `AppScreen`, `Game`.
- `src/audio.rs` — `MusicPlayer` (empty playlist for now).
- `src/progress.rs` — RON native, in-memory wasm.
- `src/fonts.rs` — DejaVu Sans Mono loader (graceful fallback).
- `web/index.html`, `tools/build-web.sh` — wasm bundle pipeline.
- `tests/game_scenarios.rs` — integration tests.

**Both native and wasm builds work.** `./tools/build-web.sh` produces
`dist/web/` ready for `python3 -m http.server`. `mq_js_bundle.js` is
copied from the local cargo registry (per version.txt pitfall #118).

### Session 2 — mining loop

- `src/input.rs` — `GameInput` struct, `from_keyboard()`.
- `src/world.rs` — `World` wrapper around `ember_stdlib::grid::Grid`,
  pixel↔tile conversion, `is_solid`, `is_empty`.
- `src/archie.rs` — `Archie` struct: position, size, mining, bag.

### Session 3 — gravity, jump, drops

- `src/archie.rs` rewritten: `vy`, `on_ground`, `coyote` (Cooldown),
  `jump_buffer` (Cooldown), `BreakResult`.
- `src/drop.rs` — `Drop` struct: falls under gravity, lands, pickup.

### Session 4 — truck (PARTIAL, then paused)

- `src/truck.rs` — `Truck` struct: 32×32, soft-blocks on wall/hole/low
  ceiling, `can_move_to`, `resting_on`, `near`.
- **User paused this** to focus on Archie first. `pub mod truck;` was
  removed from `lib.rs`. The file still exists in `src/` but is not
  compiled. **Re-add when ready.**

### Session 5A — procedural sprites

- `src/pixel.rs` — `Canvas` (RGBA buffer + drawing primitives),
  `Palette`, `from_ascii`.
- `src/atlas.rs` — `AtlasBuilder`, `BakedAtlas`, `Atlas`. One texture,
  fixed grid layout, 32×32 cells, 16 columns.
- `src/sprites/archie.rs` — `draw_side`, `Pose`, `bake_into`.
- `main.rs` — `SpriteSource` enum. Currently `Procedural`. Future:
  `Craftpix { sheet: "archie.png" }`.

### Camera (pixel-perfect viewport)

`Game::draw` uses `VIRTUAL_W = 320`, `VIRTUAL_H = 180`, integer scale
4× on a 1280×720 window, camera follows Archie with overscroll
(`CAMERA_OVERSCROLL_X/Y`).

---

## 3. Current state

**Tests**: ~90 total, all green.
**Clippy**: clean with `-D warnings`.
**Native**: builds and runs. Archie walks, jumps, mines, collects drops.
**Wasm**: builds and runs. `dist/web/` is functional.

**Command to verify:**
```sh
cd ~/Desktop/ember-workspace
cargo test -p bore-inc
cargo clippy -p bore-inc --all-targets -- -D warnings
cargo run -p bore-inc
```

---

## 4. Key decisions locked in

**Archie's visual:**
- **Side view only.** No front-facing sprite. Right-facing canonical;
  left is a `mirrored_x()` at bake time.
- **No outline.** Flat shapes, palette carries the silhouette.
- **No legs, no arms.** Feet and hand are cream blobs attached directly
  to the body blob.
- **Chibi proportions.** Head ≈ body size. 24×32 sprite canvas.
- **Hard hat stays.** Backpack added.
- **2-px wide tail** at the back-bottom, curled up with a cream tip.
- **Backpack 5×6** at back-top, distinct from the tail region.

**Rendering:**
- Pixel-perfect at 320×180 virtual, integer scale.
- `next_frame().await` called **exactly once** per iteration.

**Input:**
- Movement only. `←`/`→`/`A`/`D` move, `Space` jumps.
- `Esc` returns to Title (debug convenience).
- `R` rebuilds the world (debug).

**Animation names** (contract — must stay stable):
```
archie_idle_{0..1}_{R|L}
archie_walk_{0..1}_{R|L}
archie_jump_0_{R|L}
archie_mine_0_{R|L}
```

When swapping to CraftPix, the loader **must** produce these names.

---

## 5. What's pending

Priority order:

1. **Ask the user what to tackle next.** Do not start something without
   asking — the last session stopped mid-truck to focus on Archie.

2. **Truck** (partially written). Re-add `pub mod truck;` to `lib.rs`,
   wire `Game` to own a `Truck`, implement entry/exit, driving, soft-block.
   The file is already written; just needs integration.

3. **Robots** (not started). Mining AI, personalities, capsules.

4. **HQ** (not started). Selling, upgrades (headbutt blocks), 3 machines.

5. **Top-down map** (not started). Truck drives freely to select levels.

6. **Procedural generation** (not started). Currently `World::test_world`
   is hand-authored.

7. **Juice** (not started). Break particles, floating text, hit-stop on
   crystal, screen shake.

8. **Pickaxe sprite** (not started). Layered over Archie's hand during
   mining.

9. **CraftPix swap** (blocked on user). User bought/will buy CraftPix
   membership. `SpriteSource::Craftpix` in `main.rs` is a stub. When
   ready: write a Python script to slice the sprite sheet, adapt
   `atlas.rs` to load external PNG, produce the same sprite names.

10. **Music** (not started). `audio.rs` has an empty `DEFAULT_TRACKS`.

---

## 6. What to read before starting

**Mandatory:**
1. `version.txt` (workspace root) — 120 pitfalls, many apply here.
2. `DESIGN.md` (this folder) — the game's design.
3. `SESSION1.md` (this folder) — design phase report.
4. This file (`SESSION2.md`).
5. All of `games/bore-inc/src/` — it's small, ~1500 lines total.
6. `games/bore-inc/tests/game_scenarios.rs`.

**Helpful:**
7. `games/jade-garden/src/main.rs` and `systems.rs` — the reference
   pattern for the wasm/WebStart/audio-preload dance.
8. `crates/ember-stdlib/src/` — what's available.

---

## 7. Known issues / quirks

- **Truck file is dead code.** `src/truck.rs` exists but `lib.rs` doesn't
  include it. Clippy won't complain. Re-add `pub mod truck;` when ready.

- **Wasm progress is in-memory.** No `localStorage` bridge yet. Reload
  loses `progress.ron` data. Same limitation as jade-garden.

- **No outline on Archie.** If he blends into dark backgrounds later,
  consider a directional outline (right/bottom only), not a full ring.

- **Archie's camera overscroll.** Allows 80 px past world edges. Small
  void visible at world boundaries. May want a backdrop later.

- **Sprite anchoring.** `SPRITE_OFFSET_X/Y` in `sprites/archie.rs`
  centers the 24×32 canvas on the 12×14 physics bbox. If sprite
  dimensions change, these must change too.

- **`.env` warning on boot.** `⚠️ Aucun .env trouvé` is expected — no
  `.env` file. Colors come from `Colors::defaults()`.

---

## 8. Open questions for the user

Not yet answered — ask before assuming:

1. **Truck**: continue where we left off? The file exists, just needs
   integration.

2. **CraftPix swap**: when do you want to replace the procedural
   sprites? The `SpriteSource` enum is ready; the loader isn't.

3. **Archie's idle breathing**: current implementation moves only the
   head. Was that the right call, or should the whole body bob?

4. **Bag visualization**: the bag is not drawn on Archie yet. Add it as
   a visual element, or rely on the HUD only?

5. **The user is a solo dev on a jam**. Everything is iterative. Do
   not batch too much — one system at a time, with a screenshot
   after each change.

---

## 9. Architecture notes

**State structs** (workspace convention): `AppState` and `Game` live in
`systems.rs`. `GameContext` in `config.rs`. `lib.rs` reexports.

**Purity**: `Game::tick` is pure — takes `dt` + `GameInput`, no
macroquad. `Archie::tick` takes `&mut World` for mining. Testable
headless.

**Rendering**: `AppState::draw` → `Game::draw` → builds sprite list →
`Atlas::draw_scaled` with integer scale. HUD text at native resolution.

**Wasm**: no `wasm-bindgen`, no `Instant::now()`, no `SystemTime`.
See version.txt pitfalls #108–#111.

**Assets**: fonts + music only. Everything visual is procedural pixel
code. If that changes (CraftPix), it's the first external art asset.

---

## 10. The user's working style (for the next agent)

- **Very iterative.** Expect 5–10 rounds of "the sprite looks wrong,
  fix X". Show a screenshot after every change.
- **Prefers minimal explanations over long docs.** Code first, prose
  second.
- **Rejects ornate solutions.** "Simpler" is almost always the answer.
- **Good at spotting problems.** Trusts their eye. If they say
  something looks wrong, it does.
- **Speaks French.** Don't be formal. Be direct.

Good luck.