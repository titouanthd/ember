---
title: "Progress Evaluation 2026-11-22"
tags: ["ember/progress", "ember/evaluation"]
branch: "main @ cd4f289 (+ unmerged local branch titouanthd-docs-crates-games-plan)"
generated: "2026-11-22"
---

# Ember v0.1 — Progress evaluation (2026-11-22)

> [!info] Method
> Static read of every engine spec in [[Engine Spec Index]] and every source file in `crates/`, `xtask/`, `games/pong`, plus game docs, git reflogs and the two `.worktrees`. **Tests and clippy were not run** — the analysis environment has no crates.io access — so every pass/fail number below that comes from the repo's own docs is unverified.

## TL;DR

- **The engine core is real and well-built.** Phases 1–4 (scene tree, `.escene`, signals, servers) landed in **~3 days (16–18 Nov)** with ~9,400 lines of engine code and ~250 engine `#[test]`s. Code quality is high: generational ids, deferred structural changes, deterministic signal dispatch, cycle detection, layer-check tooling.
- **Overall POC completion ≈ 30 %** (effort-weighted against [[40-roadmap]]). Engine core (P0–P4) ≈ **70 %**; tools (P5 UI, P6 Editor) **0 %**; migration (P7) **1 / 17 games**.
- **None of the 10 success criteria** in [[00-overview#6. Success criteria|00-overview §6]] is fully met yet; one (scene round-trip) is essentially met.
- **Biggest process risk:** Phase 0 (shared `juice`, `music`, `Cooldown::running/ready`) and a full doc-vs-code review exist **only on a local, unpushed branch** (`titouanthd-docs-crates-games-plan`). Merge or push it.
- **Biggest design risk:** `#[derive(Node)]` emits a complete `impl Node`, so nodes with behaviour can't use it. That blocks acceptance item 8 (“`Catapult` and `Unit` are `#[derive(Node)]`”) and will make every migration verbose.
- **The docs drift:** the spec index still shows 📋/⬜ for files whose code is done, the spec-first “✅ frozen before PR” rule was never applied, and two different phase numbering schemes coexist. Details: [[Doc–Code Discrepancies]].

## Timeline (from git reflog)

| Date | Milestone |
|---|---|
| 2026-09-12 → 11-15 | 17 games built on `ember-core` + `ember-stdlib` (“Sessions”, game Phases 14–23) |
| 2026-11-16 | `docs(engine): create engine doc v0.1` — the 24-file spec |
| 2026-11-16 | *impl phase 1*: scaffold 11 crates, `Color`/`Rect`, xtask, CI gates |
| 2026-11-16 | *impl phase 2*: server traits, mock backends, `Image` |
| 2026-11-17 | *impl phase 3*: backend-agnostic `Input`, `primary_action` |
| 2026-11-18 | *impl phase 4–5*: `Node`/`Object`/`Signal`, `#[derive(Node)]`, macroquad backends, ADR-019…021 |
| 2026-11-18 (branch `scene-core-verify-build`, merged) | `NodeTree`, runner, signals, `ClassDB`, `PackedScene`, `AssetServer`, nested `instance:`, `LinearPhysics`, **pong migrated** (ADR-022…028) |
| 2026-11-18/19 (branch `docs-crates-games-plan`, **not merged, not pushed**) | docs synced with code; **Phase 0**: shared `juice`, `Cooldown::running/ready`, shared `music` |

## Phase-by-phase

| Phase | Roadmap says | Code says (main) | Done |
|---|---|---|---:|
| **P0 Foundations** | no progress note | `MainLoop` trait ✅, `primary_action` ✅ (in `ember-input`, not `ember_stdlib`), `run` exists in `ember-render-macroquad` ✅. `juice` extraction ❌, `Cooldown` rename ❌ — **both done on the unmerged branch** | 50 % (90 % with branch) |
| **P1 Scene core** | 🟡 | `NodeTree`, slotmap `NodeId`, contexts, `SceneLoop`, ordering/ready/stale-id tests, `hello_scene` example. Only “not run headlessly”. | 90 % |
| **P2 Serialization** | 🟡 | `ClassDB` ✅ · `Variant` ✅ · `PackedScene` load/round-trip/instantiate ✅ · nested `instance:` + cycle detection ✅ · `AssetServer`+UID registry+PNG ✅ · `#[export]` inert · `SceneWriter` stub · font loaders, hot-reload, scene connections ❌ | 65 % |
| **P3 Signals** | 🟡 | All five exit tests present; pong uses `PointScored` signal ✅ (merged to main, not a side branch). Scene-declared connections ❌ | 85 % |
| **P4 Servers** | **✅** (overstated) | Traits ✅ · mocks ✅ · backends ✅ · `LinearPhysics` ✅ · `xtask check-layers`/`check-macroquad` ✅. Exit criterion `grep "use macroquad"` ❌ (allow-list covers `ember-stdlib` + **all of `games/`**). `ember_stdlib::collider` not migrated. | 70 % |
| **P5 UI** | — | `ember-ui` is a 4-line stub | 0 % |
| **P6 Editor POC** | — | `ember-editor` is the cargo `add()` template + “Coming soon” | 0 % |
| **P7 Migration** | — | 1 / 17 (pong). Pong's `main.rs` is 20 lines (target ≤ 10) and still imports `macroquad::Color` in `config.rs`, `components.rs`, `scene.rs` | 5 % |
| **P8 Cleanup** | — | not started | 0 % |

Effort-weighted with the roadmap's own session estimates (P0 1, P1 2.5, P2 4.5, P3 2, P4 3, P5 2.5, P6 3.5, P7 12.5, P8 1 = 32.5 sessions): **≈ 10.1 / 32.5 ≈ 31 %** on main, ≈ 32 % counting the branch. Remaining ≈ 22 sessions, most of it migration.

> [!note] Phases ran out of order
> The roadmap says phases are *strictly sequential* and each needs ✅-frozen specs as entry criteria. In practice P1–P4 were built in parallel slices, P0 came last (on a branch), and no spec is frozen. That's a fine way to work, but [[40-roadmap]] and [[Engine Spec Index]] should say so.

## Success criteria scorecard ([[00-overview#6. Success criteria|00-overview §6]])

| # | Criterion | State | Evidence |
|---|---|---|---|
| 1 | 18 games still runnable | ❓ | Workspace has **17**; not run here |
| 2 | `ember-wars` lib + scenarios green | ❓ | README says 31 failing; commit `fix(ember-wars): spawn entities with ready cooldowns` (11-15) likely fixed them — re-run `cargo test -p ember-wars` |
| 3 | `ember-wars` `main.rs` ≤ 10 lines | ❌ | `main.rs` is ~8 KB |
| 4 | No game imports `macroquad` | ❌ | All 17 games, including pong |
| 5 | `juice.rs` deleted from games | ❌ main / ✅ branch | main: `ember-wars`, `jade-garden`, `pagoda` |
| 6 | `audio.rs` deleted from games | ❌ | `air-hockey`, `bore-inc`, `jade-garden`, `pagoda` |
| 7 | `.escene` round-trips a player | ✅ (≈) | `ember-scene/tests/packed_scene.rs::fixture_round_trips` on `assets/test_fixtures/player.escene` |
| 8 | Inspector loads/edits/saves | ❌ | editor is a placeholder |
| 9 | Clippy `-D warnings` green | ❓/❌ | README: not green (2 lints accretion, 1 taiji) |
| 10 | All Wave 3 docs ✅ | ❌ | none frozen |

## Quality observations (code)

**Strengths**
- `ember-scene` design is careful: nodes are *checked out* during callbacks (ADR-022) instead of `RefCell` — no runtime borrow panics; structural changes are deferred and flushed with a 1000-round cap; signals snapshot handlers so emit-during-emit is safe.
- `.escene` loading validates all classes before building, so a failure leaves the tree untouched; nested instances detect cycles by scene uid.
- Clean trait seam: `Servers` bundle of `&mut dyn` backends; `MockRenderer` lets tests inspect draw calls.
- Layer discipline is tooling-enforced (`xtask`), not just documented.
- ADRs are honest about deviations (022–028 each say what was deferred).

**Issues**
1. **`#[derive(Node)]` can't be combined with behaviour** (it writes the whole `impl Node`). Fix: have the derive implement only `Object` + a `NodeBase` (name) trait, or generate `impl Node` with an opt-out attribute and delegate lifecycle to an inherent/trait method.
2. **Invariant I4** (“no `unwrap()`/`expect()` in engine crates”): 10 `expect`/`unwrap` calls in non-test engine code (`ember-assets` server/handle/uid_registry, `ember-scene` packed/signal, `ember-core` variant). Most are poison/invariant cases — document them as allowed or switch to `?`/`unwrap_or_else(PoisonError::into_inner)`.
3. **`ember-assets` has only 7 tests** for 815 lines (lowest ratio among implemented crates).
4. **`ember-macros` has no compile-fail tests** (e.g. missing `name`, missing `class_name`) — consider `trybuild`.
5. Runner order is *physics → process → draw*; [[01-architecture#4.1 Single-frame sequence|01-architecture §4.1]] shows *process → physics*. Pick one and document it.
6. `check-macroquad` allow-lists `games/` wholesale, so criterion 4 can't regress — or progress — visibly. Narrow it per game as each migrates (pong first).
7. `Makefile` `CRATES` still lists only 3 crates; `make test-crate` only tests `ember-stdlib`.

## Recommended next steps (in order)

1. **Push and merge `titouanthd-docs-crates-games-plan`.** It holds Phase 0 and the doc/code sync; right now it only exists in `.worktrees/` on this Mac.
2. **Run the gates and refresh the numbers:** `make ci`, `cargo run -p xtask -- check-layers`, `cargo run -p xtask -- check-macroquad`. Update `README.md` and `version.txt` (both pre-date the engine).
3. **Finish pong as the reference migration:** swap `macroquad::Color` for `ember_core::Color`, drop the `macroquad` dependency, get `main.rs` ≤ 10 lines (a `ember::run_scene(title, tree, viewport)` helper would do it), and remove `games/pong` from the allow-list.
4. **Fix the derive** before migrating more games — every migration will otherwise hand-write `Object` + `Node`.
5. **Migrate `snake`, `simon`, `memory`** (next in [[41-migration-guide#3. Migration order|migration order]]) — these exercise grid, audio and click input through `NodeContext` and will shake out missing APIs (`find_by_class`, audio via signals).
6. **Then Phase 5 (`ember-ui`)** — `minesweeper`/`freecell` migrations need it.
7. **Freeze what's implemented** (21, 22, 25, 30 at least) so the spec-first rule means something again, and unify phase numbering.

Related: [[Roadmap Tracker]] · [[Doc–Code Discrepancies]] · [[Crates MOC]] · [[ADR Index]]
