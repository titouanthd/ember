# jade-garden — Release status

**Status:** **released** — v0.1.0.
Two builds published on itch.io:
- **Native macOS** (downloadable): `jade-garden-osx.zip`
- **HTML5 / WASM** (plays in browser): `jade-garden-web.zip`

All tests green. Clippy clean.

## What shipped

### Native (macOS)

- `cargo build --release -p jade-garden` → single binary next to `assets/`.
- Progress saved to `~/Library/Application Support/jade-garden/progress.ron`.
- Music via soloud, fallback to `afplay` / `ffplay` / `mpv`.

### Web (WASM)

- Build with `./tools/build-web.sh`.
- Bundle in `dist/web/`: `index.html`, `jade-garden.wasm`,
  `mq_js_bundle.js`, `assets/`.
- Music via `macroquad::audio` (WebAudio) — tracks preloaded
  asynchronously before the main loop.
- Progress is in-memory only (no localStorage bridge yet — see
  limitations).
- Halloween teaser disabled (no `SystemTime` on `wasm32-unknown-unknown`).

## Done

- [x] Runtime path resolution (`paths.rs`): native uses a
      filesystem cascade, wasm uses a relative `assets/` URL.
- [x] `tests/objectives.rs` — duplicated `println!` removed.
- [x] `audio.rs` — a missing track no longer disables the whole
      playlist; it tries every other track once.
- [x] `audio.rs` — web backend: no `wasm-bindgen`, preloaded tracks,
      manually accumulated elapsed time (no `Instant::now` on wasm).
- [x] `systems.rs` — `tile_center` returns `Option<Vec2>`, no more
      `unwrap()` on grid lookups during resolution.
- [x] `main.rs` — `handle_won` bails to menu instead of `expect`.
- [x] `main.rs` — `WebStart` screen blocks the audio autoplay policy
      on web.
- [x] `progress.rs` — `load_progress` logs load failures instead of
      silently resetting; web uses an in-memory `thread_local`
      fallback.
- [x] `config.rs` — `from_env` cfg'd; on wasm uses the compiled-in
      palette directly.
- [x] Test coverage: hint on all three objectives, RON roundtrip with
      every field populated, `next_reveal_step` / `is_spooky_month` /
      `month_from_unix` (exact Gregorian calendar), help meanings,
      audio spawn fallback, balance invariant.
- [x] `tools/itch-media/{cover,banner}.svg` — rice-paper cover,
      moon-gate banner.
- [x] `tools/build-web.sh` — one-command WASM build.
- [x] `web/index.html` — minimal HTML5 shell.
- [x] itch.io page for the native download.
- [x] itch.io page for the HTML5 build.

## Smoke test (native)

- [x] Run the binary from `/tmp` — music plays, levels load, no
      missing asset warnings.
- [x] Complete level 1 → `progress.ron` appears in the user-data
      directory.
- [x] Delete the save file, relaunch → fresh game starts.
- [x] Double-click from Finder — game launches (with the Gatekeeper
      prompt, documented on the itch.io page).

## Smoke test (web)

- [x] `./tools/build-web.sh` produces `dist/web/`.
- [x] `python3 -m http.server` in `dist/web/` → game loads.
- [x] Music preloads (three tracks in the console) and plays after
      the first click.
- [x] Complete level 1, score recorded in memory.
- [x] Uploaded zip plays on itch.io (`Click to launch in fullscreen`).

## Known limitations

### Native

- `.env` colour overrides only take effect in dev; the release
  binary uses the compiled-in palette.
- On Linux, music needs `ffplay` or `mpv` installed. The game runs
  silently without them.
- The Halloween teaser triggers on an approximate month
  calculation — exact Gregorian, but still based on the system
  clock.

### Web

- **Progress is in-memory.** Best scores, stars, and poem fragments
  reset when the browser tab is closed. Persistence needs a
  `localStorage` bridge via a miniquad plugin (wasm-bindgen is
  incompatible with miniquad's import system).
- **No Halloween teaser.** `SystemTime::now()` is unavailable on
  `wasm32-unknown-unknown`; the teaser is disabled on web.
- **No dedicated SFX.** `play_scare` / `play_scream` are silent on
  web.

## Post-release (bonus, defer freely)

- [ ] `localStorage` bridge for web progression (miniquad plugin,
      ~30 lines of JS + a small Rust wrapper).
- [ ] Web audio for `play_scare` / `play_scream`.
- [ ] Stele screen in the menu once the poem is complete: replay the
      full poem with pinyin and prose.
- [ ] Volume slider in the menu.
- [ ] Screenshot-on-victory for sharing the score.
- [ ] Achievements ("first five-match", "cascade ×5", "no-hint
      clear", …).
- [ ] Extract `ember_stdlib::juice` (particles + floating text +
      shake + flash + vignette). Three occurrences now confirmed
      (Bullet Hell, ember-wars, jade-garden) — the Rule of Three is
      satisfied.
- [ ] Extract `Tween` / `Easing` into `ember_stdlib::animation` when
      a second game needs it.
- [ ] Extract the Unicode-font + CJK-subset pattern into
      `ember_stdlib` (FreeCell, ember-wars, jade-garden).
- [ ] `find_best_swap` fallback: if no swap advances the current
      objective, fall back to *any* matching swap so a hint is
      never silently empty.