---
title: "Next Phase Plan — Phase 5 UI"
aliases: ["Phase 5 plan", "Next phase"]
tags: ["ember/progress", "ember/plan", "ember/phase-5"]
created: "2026-11-22"
phase: "5 — UI (+ entry gate)"
estimate: "5–6 sessions (gate 1–2, UI 4)"
---

# Next phase — Phase 5 · UI

> [!summary]
> Per [[40-roadmap]], Phases 0–3 are essentially done and Phase 4 is open only on its *macroquad isolation* criterion, which can't close before the games migrate (Phase 7). The next phase to **start** is **Phase 5 — UI** ([[24-ember-ui]]).
> The UI spec can't be built on today's engine: every widget is a `#[derive(Node)]` with behaviour (the derive doesn't allow that), `ScrollContainer` needs clipping (no clip in `RenderingServer`), and `TextInput` needs typed characters (not in `Input`). So the phase opens with a short **entry gate (5.0)** that fixes those three things, then four UI sessions.

## Where we start (2026-11-22, after the Phase 0 merge)

| Phase | State | What's left |
|---|---|---|
| P0 Foundations | ✅ closed 2026-11-22 | — |
| P1 Scene core | ✅ ~90 % | run `hello_scene` once headless |
| P2 Serialization | 🟡 65 % | `#[export]`, `SceneWriter`, scene connections, font/audio loaders, hot-reload |
| P3 Signals | ✅ ~85 % | scene-declared connections (needs P2) |
| P4 Servers | 🟡 70 % | macroquad isolation (blocked by P7), `collider` → `LinearPhysics` |
| **P5 UI** | ⬜ 0 % | **this plan** |

Tracker: [[Roadmap Tracker]] · evaluation: [[Progress Evaluation 2026-11-22]].

## 5.0 — Entry gate (1–2 sessions)

Roadmap entry criteria for P5 are "Phase 4 complete" and "[[24-ember-ui]] ✅". Neither holds, so the gate makes them hold honestly.

| # | Task | Why | Deliverable | Commit |
|---|---|---|---|---|
| G1 ✅ | Commit the pending merge and push; run `make hooks` | Phase 0 is staged but not committed | merge on `origin/main` | `chore(git): Merge Phase 0 branch into main` |
| G2 ✅ | Baseline: `make test-count`, `make check`, `make gates` → [[Baseline 2026-11-22]] | No test/clippy run since the engine work started | numbers in a new dated evaluation note | `doc(vault-progress): Record post-merge baseline` |
| G3 ✅ `8cc3112` | **ADR-030 — derive + behaviour.** Split naming into `trait NodeName { name, set_name }`; `Node: Object + NodeName`. `#[derive(Node)]` emits `Object` + `NodeName` only; the user writes `impl Node for T { … }` with just the lifecycle they need. Add `#[node(no_behavior)]` (or `#[derive(Node, DataNode)]`) to emit the empty `impl Node`. Migrate `hello_scene`, pong, tests | Every UI widget in the spec is a derived node with behaviour; acceptance item 8 needs it too | `ember-macros`, `ember-scene`, compile-fail tests (`trybuild`) | `feat(macros-derive): Derive Object and NodeName, leave Node lifecycle to the user` |
| G4 | **ADR-031 — clip stack.** `RenderingServer::push_clip(Rect)` / `pop_clip()` (intersecting stack, scissor in screen space after camera); macroquad impl via the GL scissor; `MockRenderer` records it | `ScrollContainer`, text fields, panels | `ember-servers`, `ember-render-macroquad`, contract test | `feat(servers-render): Add push_clip and pop_clip scissor stack` |
| G5 | **ADR-032 — text & wheel input.** `Input` gains `chars_typed: Vec<char>` and `mouse_wheel: Vec2`; poller fills them from macroquad. `Key` gains `Home`, `End`, `Delete` if missing. ADR-018 already allows adding fields | `TextInput`, `ScrollContainer` | `ember-input`, `render-mq/input.rs`, contract tests | `feat(input): Add chars_typed and mouse_wheel to Input` |
| G6 | **ADR-033 — re-scope Phase 4's exit.** Engine crates (except legacy `ember-stdlib`, retired in P8) must not import macroquad; the *games* half of the criterion moves to the Phase 7 exit. Narrow `macroquad-allowlist.txt` from `games/` to the 16 unmigrated games | Makes "Phase 4 complete" true without waiting for 16 migrations | ADR + allowlist + roadmap edit | `chore(xtask): Allow-list unmigrated games individually` |
| G7 | Review [[24-ember-ui]] against G3–G5 (remove the implicit `name` field from examples, add `push_clip`, `chars_typed`), resolve its open questions, mark it **✅ frozen** | The spec-first rule ([[Engine Spec Index]] §6) | updated spec | `doc(vault-spec): Freeze 24-ember-ui for Phase 5` |

**Gate exit:** `make pre-push` green; ADR-030…033 accepted in [[50-design-decisions]]; [[24-ember-ui]] ✅.

## 5.1–5.4 — UI sessions (~4)

Branch: `feat/ui` (or one branch per session), pushed after the first commit.

| Session | Build | Tests (MockRenderer + synthetic `Input`) |
|---|---|---|
| **5.1 Foundations** | `ember-ui` crate layout per spec §2. `Control` trait (rect, min_size, focusable, input hooks), `UiDrawContext` (wraps `DrawContext` + `Theme` + clip stack), `Anchors` + `compute_rect`, `Theme::dark()` / `light()`, `UiRoot` node that runs layout once per frame | `layout_anchors.rs`: anchors resolve for 4 corners + stretch; theme defaults |
| **5.2 Layout & static widgets** | `VBox`, `HBox`, `Stretch` (one-pass layout, spec §11.1), `Label`, `Panel`, `Image` (`Handle<Texture>`) | `layout_vbox.rs` (3 children, spacing, stretch weights), `layout_hbox.rs`; draw-call snapshots |
| **5.3 Interaction** | `FocusManager` (Tab / Shift+Tab, wrap-around, `set_focus`), `Button` emitting `ButtonPressed { id }` via signals, mouse routing top-down with consumption (spec §9), `primary_action` activates the focused button | `focus_navigation.rs` (Tab ×3 cycles, Enter emits), `mouse_routing.rs` (overlapping controls, consumption) |
| **5.4 Scroll, text, demo** | `ScrollContainer` (wheel + clip + scrollbar), `TextInput` (caret, Backspace/Delete/Home/End, max length, `TextSubmitted` signal), `crates/ember/examples/ui_demo.rs` showing every widget | `scroll_clip.rs` (clip calls bracket children, offset), `text_input.rs` (typing, backspace, submit) |

Every commit uses the format in [[Commit & Push]], e.g. `feat(ui-layout): Add VBox and HBox one-pass layout`.

## Exit criteria (from [[40-roadmap]] §8.3, made testable)

- [ ] `ui_demo` shows a `VBox` with 3 `Button`s; Tab moves focus, Enter activates (manual + `focus_navigation.rs`)
- [ ] `ScrollContainer` clips its content (`scroll_clip.rs` + visual check)
- [ ] `TextInput` accepts and displays text (`text_input.rs` + visual check)
- [ ] `Theme::dark()` renders a readable UI (visual check, screenshot in the vault)
- [ ] `make pre-push` green; `xtask check-layers` still passes (`ember-ui` → `ember-scene`, `ember-servers` only)
- [ ] [[ember-ui]] crate note and [[Roadmap Tracker]] updated

## Validation on a real game

Right after 5.4, migrate **simon** (3rd in [[41-migration-guide#3. Migration order|migration order]]; menu + circle buttons + audio). It checks the UI against real needs and moves Phase 7 forward. In parallel (Phase 7 may run after P4), **snake** needs no UI and can migrate any time.

## Risks

| Risk | Mitigation |
|---|---|
| G3 touches every hand-written node (pong, tests, `hello_scene`) | Mechanical change; do it first, alone, behind green tests |
| Scissor + camera offset mismatch in macroquad | Contract test in `MockRenderer`, one visual check in `ui_demo` |
| Per-frame layout cost | One pass, only `UiRoot` subtrees; measure with 200 controls before optimising |
| Scope creep (drag-and-drop, rich text) | Out of scope per spec §11.4; park in "Open questions" of [[24-ember-ui]] |

## After Phase 5

Phase 6 (editor POC) needs P5 widgets plus P2's `SceneWriter` and `#[export]`. Plan P2 close-out next, alongside the next Phase 7 migrations.
