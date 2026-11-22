---
title: "51 — Glossary"
aliases: ["51 — Glossary"]
tags: ["ember/spec", "ember/wave-1"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/51-glossary.md"
imported: "2026-11-22"
wave: "1 — Foundation"
---

# 51 — Glossary

> **Status:** 🟡 partial — reviewed 2026-11-19; terms are defined, but Progress, Plugin, Control and hot-reload are not implemented (flagged inline)
> **Audience:** all
> **Prerequisites:** none

Terms are listed alphabetically. Each entry has one sentence explaining
what it is and a cross-reference to the file that documents it fully.

## A

**ADR** — Architecture Decision Record; a short document capturing one
non-obvious design choice, its alternatives, and consequences. See
[[50-design-decisions]].

**Asset** — any file loaded at runtime (texture, font, audio clip, scene,
level data). Assets are referenced by `Handle<T>` and loaded through
`AssetServer`. See [[13-assets]].

**AssetServer** — the engine subsystem that loads, caches, and reloads
assets. Games never call `image::open` directly; they ask the
`AssetServer`. Hot-reload is manual (`reload`); no file watcher yet. See [[25-ember-assets]].

**AudioServer** — the trait that abstracts audio playback. The reference
implementation is `ember_audio_macroquad`. See [[12-servers]].

## C

**ClassDB** — a registry mapping type names (strings) to constructors.
Allows `.escene` files to instantiate nodes by name without an explicit
type map in every game. Godot-parity concept; documented in
[[20-ember-core]].

**Control** — the base trait for retained-mode UI nodes (`Button`,
`Label`, `Panel`, etc.). Planned for `ember_ui` (currently an empty stub). See [[24-ember-ui]].

**Cooldown** — a timer utility in `ember_core::time`. Two constructors:
`ready(d)` (ready now, re-arms to `d` after `trigger`) and `running(d)`
(already running, ready after `d`). The old `new` / `with_duration` constructors are
`#[deprecated]` aliases (`new` was the root of the `ember-wars` 31-test bug). See
[[20-ember-core]].

## D

**DrawContext** — the state passed to `Node::draw`. Provides access to
`RenderingServer`, viewport size, and camera. See [[10-nodes-and-scenes]].

## E

**.escene** — Ember's scene file format. A RON-encoded file with five
sections (format header, external resources, internal resources, nodes,
connections), inspired by Godot's `.tscn`. See [[30-escene-format]].

## F

**Fixed-step physics** — `physics_process` is called a whole number of
times per frame at a constant `1/60` dt. Prevents lifecycle bugs that
depend on frame rate. See [[14-main-loop]].

## G

**GameState** — the coarse enum (`Start`, `Playing`, `LevelCleared`,
`GameOver`, `Win`; `ember_core::app`) that the host uses to decide
whether to save. Games with richer internal states map to these five. See [[14-main-loop]].

## H

**Handle<T>** — an owned reference to a loaded asset. Keeping a `Handle`
alive keeps the asset in memory; the server caches only a `Weak` per
path and prunes dead entries on `poll()`. See [[13-assets]].

**Hot-reload** — the ability to modify an asset file on disk and have the
running game reflect the change without a restart. Enabled in dev builds
only. Not implemented: `Asset::reload` exists but there is no watcher.
See [[13-assets]].

## L

**Layer** — one of Ember's five architecture tiers (Main, Scene, Servers,
Core, Drivers). A crate belongs to exactly one layer. See
[[01-architecture#2. The five-layer model|01-architecture.md §2]].

**L0, L1, L2, L3, L4** — numeric shortcuts for Drivers, Core, Servers,
Scene, Main respectively. Used in ADRs and code review.

## M

**MainLoop** — the trait that every game implements to define its entry
point. Methods: `init`, `process`, `physics_process`, `draw`,
`finalize`. See [[14-main-loop]].

**miniquad** — the low-level graphics abstraction underneath macroquad.
Ember does not use it directly.

**macroquad** — the immediate-mode 2D graphics library Ember uses for
v0.1. Migrated games never depend on it directly; only
`ember_render_macroquad` and `ember_audio_macroquad` should. Until
migration finishes, `ember-stdlib` and unmigrated games still import it
(`xtask/macroquad-allowlist.txt`).

## N

**Node** — the smallest unit of composition in Ember. Every entity, UI
element, and system hook is a `Node`. See [[10-nodes-and-scenes]].

**NodeContext** — the state passed to `Node::ready`, `process`, and
`physics_process`. Provides access to the tree, servers, and input. See
[[10-nodes-and-scenes]].

**NodeId** — an opaque generational index into a `NodeTree`. Cheap to
copy; can be compared; safe to use across frames if the node is still
alive. See [[10-nodes-and-scenes]].

**NodeTree** — the ownership container for all nodes at runtime. Owns a
slotmap of node data; provides `insert`, `remove`, `attach`, `detach`.
See [[10-nodes-and-scenes]].

## O

**Object** — the trait (analogue to Godot's `Object`) that all nodes
implement to be stored in a `NodeTree`. Provides `as_any` /
`as_any_mut` for downcasting and a `class_name` for `.escene`
serialization. See [[20-ember-core]].

## P

**PackedScene** — a serialized subtree that can be instantiated any
number of times. Analogous to a Godot scene file. See
[[10-nodes-and-scenes]].

**Plugin** — an optional trait a game implements to hook into engine
startup (registering assets, initialising game state). Planned only: no
`Plugin` trait exists in code (its home is an open question in
[[01-architecture#7. Open questions|01-architecture.md §7]]).

**POC** — Proof of Concept. The v0.1 milestone: engine structure works,
all 17 games migrate, `ember-wars` passes its conformance suite. See
[[00-overview#4. What "POC" means here|00-overview.md §4]].

**prelude** — a curated set of re-exports (`ember::prelude::*`) that
covers the common 80% of any game's usage. See
[[02-principles#4.3 `prelude.rs` rules|02-principles.md §4.3]].

**Progress** — the serializable struct a game persists between runs
(best score, unlocked levels, upgrades). Stored by the `MainLoop` via
`MainLoop::progress()` and `MainLoop::take_dirty()` (specified, not
implemented: the `MainLoop` trait has no such methods). See
[[14-main-loop]].

## R

**RenderingServer** — the trait that abstracts 2D rendering. The
reference implementation is `ember_render_macroquad`. Games never call
`draw_rectangle` directly; they call `ctx.render().draw_rect(...)`.
See [[12-servers]].

**Root node** — the single top-level node of a `NodeTree`. Games create
one root, attach children to it, and never replace it during a run.

## S

**Scene** — a tree of nodes. Either live (a subtree in a `NodeTree`) or
serialized (a `PackedScene` / `.escene` file). Scenes nest: a scene can
contain other scenes, instanced. See [[10-nodes-and-scenes]].

**SceneTree** — Godot's name for what Ember calls `NodeTree`. Ember keeps
Godot's vocabulary elsewhere but prefers the explicit name.

**Server** — a trait defining a backend subsystem (`RenderingServer`,
`AudioServer`, `PhysicsServer`). Servers are passed around as
`&mut dyn Server` inside `NodeContext`. See [[12-servers]].

**Signal** — a typed event emitter. Nodes `emit::<T>(payload)` and other
nodes `connect::<T>(target, handler)`. The runtime matches by type, not
by string. See [[11-signals]].

**slotmap** — the generational arena data structure underlying
`NodeTree`. Provides `O(1)` insert/remove/lookup with stale-reference
detection.

## U

**UID** — a stable identifier for a resource (asset or scene). Used in
`.escene` files to reference assets by stable string rather than fragile
path. Format: `uid://<file-stem>_<5 hex digits of content hash>`. See
[[30-escene-format]].

## V

**Variant** — a tagged union of primitives (`Nil`, `Bool`, `Int`,
`Float`, `String`, `Vec2`, `Color`, `Array(Vec<Variant>)`, `Map<String, Variant>`,
`ObjectRef`). Used at the `Object`/`ClassDB` boundary for serialization
and dynamic property editing. Type-safe Rust structs are still preferred
in game code. See [[20-ember-core]].

## W

**wgpu** — the modern cross-platform GPU abstraction. Not used in v0.1;
the target backend for `ember_render_wgpu` in v0.2. Ember's layer
discipline ensures wgpu adoption is a self-contained change.

## Z

**Zero-cost abstraction** — a design principle from Rust that Ember
follows where possible: server traits compile down to direct calls in
monomorphised code (though dynamic dispatch is used at node boundaries).
Not to be confused with the *absence* of overhead in macroquad itself,
which is immediate-mode and cheap but not zero-cost.
