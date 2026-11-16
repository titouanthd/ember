# 51 — Glossary

> **Status:** ⬜ draft
> **Audience:** all
> **Prerequisites:** none

Terms are listed alphabetically. Each entry has one sentence explaining
what it is and a cross-reference to the file that documents it fully.

## A

**ADR** — Architecture Decision Record; a short document capturing one
non-obvious design choice, its alternatives, and consequences. See
[50-design-decisions.md](50-design-decisions.md).

**Asset** — any file loaded at runtime (texture, font, audio clip, scene,
level data). Assets are referenced by `Handle<T>` and loaded through
`AssetServer`. See [13-assets.md](13-assets.md).

**AssetServer** — the engine subsystem that loads, caches, and hot-reloads
assets. Games never call `image::open` directly; they ask the
`AssetServer`. See [25-ember-assets.md](25-ember-assets.md).

**AudioServer** — the trait that abstracts audio playback. The reference
implementation is `ember_audio_macroquad`. See [12-servers.md](12-servers.md).

## C

**ClassDB** — a registry mapping type names (strings) to constructors.
Allows `.escene` files to instantiate nodes by name without an explicit
type map in every game. Godot-parity concept; documented in
[20-ember-core.md](20-ember-core.md).

**Control** — the base trait for retained-mode UI nodes (`Button`,
`Label`, `Panel`, etc.). Lives in `ember_ui`. See [24-ember-ui.md](24-ember-ui.md).

**Cooldown** — a timer utility in `ember_core::time`. Two constructors:
`ready(d)` (ready now, re-arms to `d` after `trigger`) and `running(d)`
(already running, ready after `d`). The `new` constructor is deprecated
because it is the root of the `ember-wars` 31-test bug. See
[20-ember-core.md](20-ember-core.md).

## D

**DrawContext** — the state passed to `Node::draw`. Provides access to
`RenderingServer`, viewport size, and camera. See [10-nodes-and-scenes.md](10-nodes-and-scenes.md).

## E

**.escene** — Ember's scene file format. A RON-encoded file with five
sections (format header, external resources, internal resources, nodes,
connections), inspired by Godot's `.tscn`. See [30-escene-format.md](30-escene-format.md).

## F

**Fixed-step physics** — `physics_process` is called a whole number of
times per frame at a constant `1/60` dt. Prevents lifecycle bugs that
depend on frame rate. See [14-main-loop.md](14-main-loop.md).

## G

**GameState** — the coarse enum (`Menu`, `Playing`, `Paused`, `Won`,
`Lost`) that the host uses to decide whether to save. Games with richer
internal states map to these five. See [14-main-loop.md](14-main-loop.md).

## H

**Handle<T>** — an owned reference to a loaded asset. Keeping a `Handle`
alive keeps the asset in memory; dropping it allows the `AssetServer` to
unload on demand. See [13-assets.md](13-assets.md).

**Hot-reload** — the ability to modify an asset file on disk and have the
running game reflect the change without a restart. Enabled in dev builds
only. See [13-assets.md](13-assets.md).

## L

**Layer** — one of Ember's five architecture tiers (Main, Scene, Servers,
Core, Drivers). A crate belongs to exactly one layer. See
[01-architecture.md §2](01-architecture.md#2-the-five-layer-model).

**L0, L1, L2, L3, L4** — numeric shortcuts for Drivers, Core, Servers,
Scene, Main respectively. Used in ADRs and code review.

## M

**MainLoop** — the trait that every game implements to define its entry
point. Methods: `init`, `process`, `physics_process`, `draw`,
`finalize`. See [14-main-loop.md](14-main-loop.md).

**miniquad** — the low-level graphics abstraction underneath macroquad.
`ember_render_macroquad` sits on `miniquad::RenderingBackend`; a future
`ember_render_wgpu` would not.

**macroquad** — the immediate-mode 2D graphics library Ember uses for
v0.1. Games never depend on it directly; only `ember_render_macroquad`
and `ember_audio_macroquad` do.

## N

**Node** — the smallest unit of composition in Ember. Every entity, UI
element, and system hook is a `Node`. See [10-nodes-and-scenes.md](10-nodes-and-scenes.md).

**NodeContext** — the state passed to `Node::ready`, `process`, and
`physics_process`. Provides access to the tree, servers, and input. See
[10-nodes-and-scenes.md](10-nodes-and-scenes.md).

**NodeId** — an opaque generational index into a `NodeTree`. Cheap to
copy; can be compared; safe to use across frames if the node is still
alive. See [10-nodes-and-scenes.md](10-nodes-and-scenes.md).

**NodeTree** — the ownership container for all nodes at runtime. Owns a
slotmap of node data; provides `insert`, `remove`, `attach`, `detach`.
See [10-nodes-and-scenes.md](10-nodes-and-scenes.md).

## O

**Object** — the trait (analogue to Godot's `Object`) that all nodes
implement to be stored in a `NodeTree`. Provides `as_any` /
`as_any_mut` for downcasting and a `class_name` for `.escene`
serialization. See [20-ember-core.md](20-ember-core.md).

## P

**PackedScene** — a serialized subtree that can be instantiated any
number of times. Analogous to a Godot scene file. See
[10-nodes-and-scenes.md](10-nodes-and-scenes.md).

**Plugin** — an optional trait a game implements to hook into engine
startup (registering assets, initialising game state). Games work
without it; it's the extension point. See [40-roadmap.md](40-roadmap.md).

**POC** — Proof of Concept. The v0.1 milestone: engine structure works,
all 18 games migrate, `ember-wars` passes its conformance suite. See
[00-overview.md §4](00-overview.md#4-what-poc-means-here).

**prelude** — a curated set of re-exports (`ember::prelude::*`) that
covers the common 80% of any game's usage. See
[02-principles.md §4.3](02-principles.md#43-preluders-rules).

**Progress** — the serializable struct a game persists between runs
(best score, unlocked levels, upgrades). Stored by the `MainLoop` via
`MainLoop::progress()` and `MainLoop::take_dirty()`. See
[14-main-loop.md](14-main-loop.md).

## R

**RenderingServer** — the trait that abstracts 2D rendering. The
reference implementation is `ember_render_macroquad`. Games never call
`draw_rectangle` directly; they call `ctx.render().draw_rect(...)`.
See [12-servers.md](12-servers.md).

**Root node** — the single top-level node of a `NodeTree`. Games create
one root, attach children to it, and never replace it during a run.

## S

**Scene** — a tree of nodes. Either live (a subtree in a `NodeTree`) or
serialized (a `PackedScene` / `.escene` file). Scenes nest: a scene can
contain other scenes, instanced. See [10-nodes-and-scenes.md](10-nodes-and-scenes.md).

**SceneTree** — Godot's name for what Ember calls `NodeTree`. Ember keeps
Godot's vocabulary elsewhere but prefers the explicit name.

**Server** — a trait defining a backend subsystem (`RenderingServer`,
`AudioServer`, `PhysicsServer`). Servers are passed around as
`&mut dyn Server` inside `NodeContext`. See [12-servers.md](12-servers.md).

**Signal** — a typed event emitter. Nodes `emit::<T>(payload)` and other
nodes `connect::<T>(target, handler)`. The runtime matches by type, not
by string. See [11-signals.md](11-signals.md).

**slotmap** — the generational arena data structure underlying
`NodeTree`. Provides `O(1)` insert/remove/lookup with stale-reference
detection.

## U

**UID** — a stable identifier for a resource (asset or scene). Used in
`.escene` files to reference assets by stable string rather than fragile
path. Format: `uid://<base36-encoded-hash>`. See
[30-escene-format.md](30-escene-format.md).

## V

**Variant** — a tagged union of primitives (`Nil`, `Bool`, `Int`,
`Float`, `String`, `Vec2`, `Color`, `Vec<Variant>`, `Map<String, Variant>`,
`ObjectRef`). Used at the `Object`/`ClassDB` boundary for serialization
and dynamic property editing. Type-safe Rust structs are still preferred
in game code. See [20-ember-core.md](20-ember-core.md).

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