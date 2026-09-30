# jade-garden — Release checklist

**Status:** pre-release. 299 tests green
(265 unit + 10 main-bin + 1 balance + 22 integration + 1 objectives),
clippy clean.

## Done

- [x] Runtime path resolution (`paths.rs`): assets resolve from
      `<exe_dir>/assets`, then `$JADE_GARDEN_ASSETS`, then the source
      tree. Progress file lives in the platform user-data directory.
- [x] `tests/objectives.rs` — duplicated `println!` removed.
- [x] `audio.rs` — a missing track no longer disables the whole
      playlist; it tries every other track once.
- [x] `systems.rs` — `tile_center` returns `Option<Vec2>`, no more
      `unwrap()` on grid lookups during resolution.
- [x] `main.rs` — `handle_won` bails to menu instead of `expect`.
- [x] `progress.rs` — `load_progress` logs load failures instead of
      silently resetting.
- [x] Test coverage: hint on all three objectives, RON roundtrip with
      every field populated, `next_reveal_step` / `is_spooky_month` /
      `month_from_unix` (exact Gregorian calendar), help meanings,
      audio spawn fallback, balance invariant.

## Before tagging the release

### Bundle

- [ ] `cargo build --release -p jade-garden`.
- [ ] Create a clean bundle directory:
      `jade-garden` + `assets/` side-by-side.
- [ ] Copy the binary from `target/release/jade-garden`.
- [ ] Copy `games/jade-garden/assets/` verbatim.
- [ ] Confirm `progress.ron` is **not** in the bundle.
- [ ] Confirm `.env` is **not** in the bundle.

### Smoke test

- [ ] Run the binary from `/tmp` (or any unrelated cwd).
      Verify: music plays, levels load, no warning about missing
      assets.
- [ ] Complete level 1, confirm `progress.ron` appears in the
      user-data directory.
- [ ] Delete the save file, relaunch, confirm a fresh game starts.
- [ ] Double-click from Finder / Nautilus — the game launches.
- [ ] Resize the window — the layout scales (already supported).

### Metadata

- [ ] `Cargo.toml` workspace `version` field — currently `0.1.0`,
      appropriate for a first public release.
- [ ] Window icon (optional): add a `.png` under `assets/` and wire
      it through `window_conf()` via `Icon::from_bytes`.
- [ ] Binary size: `ls -lh target/release/jade-garden`. Expect
      ~15–25 MB in release; check for obvious bloat.

### Documentation

- [x] `README.md`.
- [x] `CREDITS.md`.
- [x] `DESIGN.md` status line updated (see below).
- [ ] `.gitignore` includes `progress.ron` and `.env`.

## After release (bonus, defer freely)

- [ ] Stele screen in the menu once the poem is complete: replay the
      full poem with pinyin and prose.
- [ ] Volume slider in the menu.
- [ ] Screenshot-on-victory for sharing the score.
- [ ] Achievements ("first five-match", "cascade ×5", "no-hint
      clear", …).
- [ ] Extract `ember_stdlib::juice` (particles + floating text +
      shake + flash + vignette). Three occurrences now confirmed
      (Bullet Hell, ember-war, jade-garden) — the Rule of Three is
      satisfied.
- [ ] Extract `Tween` / `Easing` into `ember_stdlib::animation` when
      a second game needs it.
- [ ] `find_best_swap` fallback: if no swap advances the current
      objective, fall back to *any* matching swap so a hint is
      never silently empty. Tested seeds (9, 10, 12) all pass, so
      this is theoretical for now.
- [ ] Kill orphan `afplay` / `ffplay` processes spawned by
      `play_scare` / `play_scream`. The child is dropped immediately
      after spawn, so the OS reaps it — but on slow tracks a manual
      `kill` would be cleaner.

## Known limitations

- `.env` colour overrides only take effect in dev; the release
  binary uses the compiled-in palette.
- On Linux, music needs `ffplay` or `mpv` installed. The game runs
  silently without them.
- The Halloween teaser triggers on an approximate month calculation
  (now exact thanks to the Gregorian implementation, but still
  based on `SystemTime`, so a wildly wrong system clock will shift
  the window).