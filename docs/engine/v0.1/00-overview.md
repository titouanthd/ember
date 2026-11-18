# 00 — Overview

> **Status:** 🟡 partial — reviewed 2026-11-19; goals and scope are accurate, but only pong is migrated and several in-scope items are unimplemented (flagged inline)
> **Audience:** all
> **Prerequisites:** none

## 1. Purpose

Explain what Ember is, what the v0.1 POC promises, and what it explicitly
does not. This is the first file every reader opens. It should answer
"why does this project exist?" before the reader sees a single diagram.

## 2. The 30-second pitch

Ember is a 2D game engine in Rust that lets you build games by composing
**nodes** into **scenes**, wiring them together with **typed signals**, and
running them through a single **MainLoop**. It targets desktop and web,
renders through macroquad/miniquad today, and leaves the door open for a
wgpu backend tomorrow — because games only ever see a `RenderingServer`
trait, not a specific GPU library.

Ember is not trying to replace Godot. It is trying to be **the smallest
engine that lets a Rust programmer build a 2D game without reimplementing
the loop, the UI, and the asset pipeline every single time**.

## 3. Why this engine exists (evidence from the current codebase)

The workspace already contains 17 working 2D games. Reading through them
surfaces three concrete pains that a small engine will solve:

### 3.1 The `main.rs` duplication

Every game's `main.rs` reimplements the same pattern:

- Create a window
- Initialise a `Juice` instance
- Load a `Progress` from disk
- Loop: read input → update state → draw → maybe save
- Handle restart

That is 40–80 lines per game, × 17 games, × every bug that ever gets
fixed in that pattern. A `MainLoop` trait collapses it to under 10 lines
per game, all sharing the same code path.

### 3.2 The `juice.rs` duplication

Six games (`accretion`, `ember-wars`, `jade-garden`, `pagoda`, `air-hockey`,
`asteroids`) were audited as shipping a `juice.rs` (3 copies remain today:
`ember-wars`, `pagoda`, `jade-garden`), implementing floating text, particles,
screen shake, shockwave, and hit flash. They are near-identical copies.
Extracting `ember_stdlib::juice` deletes ~600 lines of duplicated code
today and prevents the six copies from silently diverging tomorrow.

### 3.3 The `Cooldown::new` bug class

`Cooldown::new(d)` builds a cooldown that *starts running*; the intended
"ready now, re-arms after `d`" constructor is `Cooldown::with_duration(d)`.
Four call sites used the wrong one, and 31 tests failed in `ember-wars` as
a direct consequence (catapults spawned dead, units spawned on cooldown,
bombers never detonated). Renaming the constructors to
`Cooldown::running(d)` / `Cooldown::ready(d)` and deprecating `new` removes
the footgun structurally (planned; not yet in code — only a no-arg
`Cooldown::ready()` exists). This is the kind of API sharpening a real engine
does on purpose.

**Every design decision in this document set traces back to one of these
three pains, or to a Godot/Fyrox/Astrelis precedent.**

## 4. What "POC" means here

The v0.1 deliverable is a **proof of concept**, not a production engine.
Concretely, POC means:

| POC is | POC is **not** |
|---|---|
| 2D only | 3D, VR, AR |
| Rust-only scripting | GDScript, Lua, or any embed language |
| One rendering backend (macroquad) | Abstraction over wgpu, Vulkan, D3D12 |
| Statically linked plugins | Dynamic loading (GDExtension equivalent) |
| Scene format round-trips | Full editor |
| Scene-tree architecture | Pure ECS (ECS is an optional extra crate) |
| All 17 existing games still pass | Every game on the old code path deleted |

The **acceptance test** is that **`ember-wars` migrates end-to-end** —
catapults, units, projectiles, waves, upgrades, AI, combat resolution —
onto the new engine primitives without special-casing the game. If
`ember-wars` migrates cleanly, the POC has succeeded. If it needs a
special escape hatch, the engine is not ready.

See [42-acceptance-tests.md](42-acceptance-tests.md) for the detailed
migration checklist.

## 5. Scope

### 5.1 In scope (v0.1)

- `ember_core` — math, time, hash, rng, `Variant`, `Object`/`ClassDB`
- `ember_scene` — `Node`, `NodeId`, `NodeTree`, `NodeContext`, `PackedScene`
- `ember_servers` — trait definitions for `RenderingServer`, `AudioServer`,
  `PhysicsServer`
- `ember_render_macroquad` — reference `RenderingServer` impl
- `ember_audio_macroquad` — reference `AudioServer` impl
- `ember_assets` — `AssetServer`, `Handle<T>`, UID registry, hot-reload
  (hot-reload watching not implemented)
- `ember_input` — input mapping, `primary_action` (action-map RON format
  not implemented)
- `ember_ui` — retained-mode `Control` nodes (stub)
- `ember_macros` — `#[derive(Node)]`, `#[signal]`, `#[export]` (`#[signal]`
  is a no-op marker; `#[export]` is a field attribute with no effect)
- `ember_editor` — POC inspector (scene tree + property panel, no WYSIWYG)
  (stub)
- `ember` — top-level facade crate that re-exports the prelude
- `.escene` v1 format spec
- Migration of all 17 games onto the new `MainLoop + SceneTree`

### 5.2 Out of scope (v0.1)

- 3D rendering, 3D physics
- GDScript / Lua / any embedded scripting language
- Dynamic plugin loading (`dlopen`-style `GDExtension`)
- Visual shader editor
- Animation tree / blend tree UI
- Networking, multiplayer primitives
- Mobile export pipeline
- Full editor (only the inspector ships)
- Hot-reloading code (only *assets* hot-reload)
- Profiler UI (profiling macros only, no display)
- Save-format migration tooling (each game owns its save logic)
- Automated CI artifact publishing

Every one of these is a v0.2+ candidate. Do not merge v0.2 features into
v0.1 branches even if they are small.

## 6. Success criteria

The POC is done when **all** of the following are true:

| # | Criterion | How measured |
|---|---|---|
| 1 | 17 games still runnable | Manual launch + smoke test of each |
| 2 | `ember-wars` lib + scenarios green | `cargo test -p ember-wars` |
| 3 | `ember-wars` `main.rs` ≤ 10 lines (currently 210; only `pong` is migrated, at 20) | `wc -l games/ember-wars/src/main.rs` |
| 4 | No game imports `macroquad` outside `ember_render_macroquad` | `grep -r "macroquad" games/*/src/` returns only engine crates |
| 5 | `juice.rs` deleted from all games | `find games -name juice.rs` → empty |
| 6 | `audio.rs` deleted from all games | `find games -name audio.rs` → empty |
| 7 | `.escene` round-trips a player entity | Integration test in `ember_scene` |
| 8 | Scene inspector loads, edits a property, saves | Manual walkthrough |
| 9 | Workspace green under clippy `-D warnings` | CI |
| 10 | All Wave 3 files marked ✅ | Review log |

Status today: criteria 4–6 and 8 are unmet (3 `juice.rs`, 4 `audio.rs` remain);
7 is met by `PackedScene` round-trip tests.

Criterion 4 is the crucial one for future-proofing: as long as exactly one
crate depends on `macroquad`, swapping to `wgpu` later is a contained change.

## 7. Relation to existing engines

We are not the first to build a 2D engine in Rust, or a scene-tree engine
at all. The comparison below is not marketing — it is a design audit that
explains *why* each of our non-obvious choices is non-obvious.

### 7.1 Godot 4

- **What we borrow:**
  - The 5-layer architecture (Scene → Servers → Drivers → Core → Main)
  - The Node / Scene / SceneTree / Signal mental model
  - The `.tscn` file format structure (5 sections, UID-based referencing)
  - Fixed-step physics vs variable-step process split in `MainLoop`
- **What we reject:**
  - GDScript (Rust-first, like Fyrox)
  - Full editor (POC ships an inspector only)
  - String-keyed signals (we use typed signals; see [11-signals.md](11-signals.md))
  - `Variant`-only property storage (Rust types with `serde` derives)
- **What we can never match:** Godot's C++ performance, its ecosystem,
  its 3D pipeline. We are not competing on those axes.

### 7.2 Fyrox

- **What we borrow:**
  - Rust-first scripting (no embed language)
  - `Plugin` trait as the engine's extension point
  - `Scene`-centric workflow (scenes are the unit of composition)
- **What we reject:** Fyrox's full-ECS-plus-scene-graph hybrid (too
  complicated for our POC); we choose pure tree with optional ECS.
- **What we can match:** Most 2D-game workflows are expressible in Fyrox's
  model — that's our viability proof.

### 7.3 Astrelis

- **What we borrow:**
  - The **trait crate + impl crate** pattern (`ember_servers` defines
    traits; `ember_render_macroquad` implements them). This is the single
    most important structural decision for future backend swaps.
  - Strict layer discipline (each crate depends only on strictly lower
    layers, enforced in CI).
- **What we reject:** Astrelis is 3D-first; our scope is 2D only.

### 7.4 Bevy

- **What we borrow:** Nothing structurally. Bevy is ECS-first; we are
  tree-first with optional ECS in `ember_ecs`.
- **Why not use Bevy directly:** Bevy's design assumes ECS as the only
  paradigm. Fitting a `NodeTree` into an ECS becomes a `#[derive(Component)]`
  party. For turn-based games like `freecell` or grid games like
  `minesweeper`, an ECS is overhead. We want the tree by default.

### 7.5 macroquad / miniquad

- **What we borrow:** The entire rendering stack for v0.1. Ember is a
  thin layer above macroquad today.
- **Why we do not fork macroquad:** Our `RenderingServer` is a
  higher-level façade over macroquad's drawing API, not a fork (the
  backend does not touch `miniquad` directly).
- **Why the seam matters:** if v0.2 wants wgpu, we write
  `ember_render_wgpu` and don't touch `ember_scene`, `ember_ui`, or any
  game. That is the whole point of the trait crate + impl crate split.

## 8. Open questions

Recorded here so they get resolved in `50-design-decisions.md` rather
than being lost:

- **Q1:** Should `ember_ui` be a separate crate or a module inside
  `ember_scene`? *Provisional answer:* separate crate, since games can
  opt out of UI entirely.
- **Q2:** Should `ember_macros` re-export `serde` derives, or should
  users depend on `serde` directly? *Provisional answer:* direct
  dependency — no wrapper crates unless proven necessary.
- **Q3:** Hot-reload scope — assets only (v0.1) vs also scenes?
  *Answer:* assets only; scene hot-reload is a v0.2 topic (ADR-015).
- **Q4:** Should the editor use `egui` for the inspector panel, or
  reimplement everything in `ember_ui`? *Provisional answer:* `egui`
  for the editor only; `ember_ui` is for runtime UIs, not developer tools.

## 9. Cross-references

- Layer details → [01-architecture.md](01-architecture.md)
- Enforced rules → [02-principles.md](02-principles.md)
- Phases and gates → [40-roadmap.md](40-roadmap.md)
- Acceptance definition → [42-acceptance-tests.md](42-acceptance-tests.md)
- Decision log → [50-design-decisions.md](50-design-decisions.md)