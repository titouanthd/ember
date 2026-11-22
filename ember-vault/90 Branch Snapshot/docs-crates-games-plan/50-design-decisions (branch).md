---
title: "50 — Design Decisions (ADRs)"
aliases: ["50 — Design Decisions (ADRs)"]
tags: ["ember/spec", "ember/branch-snapshot", "ember/wave-4"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/50-design-decisions.md"
imported: "2026-11-22"
wave: "4 — Format, Process & Reference"
---

# 50 — Design Decisions (ADRs)

> **Status:** 🟡 partial — reviewed 2026-11-19; ADR-001..028 numbered consistently, ADR-016..028 match code except ADR-011 and ADR-020 loaders (not implemented, noted inline)
> **Audience:** all
> **Prerequisites:** [[01-architecture (branch)]], [[02-principles (branch)]]

## 1. Purpose

Record every non-obvious design choice with its context, alternatives,
and consequences. This is the file you read when you wonder "why did
they do it this way?" — and the file you edit (by adding a new ADR)
when you want to change a decision.

Each ADR follows the same structure:

- **Status** — proposed, accepted, superseded, deprecated
- **Context** — what problem was being solved
- **Decision** — what was chosen
- **Alternatives** — what was rejected and why
- **Consequences** — what the choice enables and what it costs

ADRs are **append-only**. When a decision is superseded, a new ADR
records the supersession; the old ADR stays.

## ADR-001: Scene tree over pure ECS

**Status:** accepted (v0.1)

### Context

The engine needs a way to compose entities, UI elements, and systems.
The two dominant models in the Rust game ecosystem are:

- **ECS** (Bevy, `hecs`): entities are integer ids, components are data,
  systems are functions over queries.
- **Scene tree** (Godot, Fyrox): entities are nodes, nodes own children,
  each node carries its own behavior.

The workspace's 17 games include:

- Turn-based (`freecell`, `minesweeper`, `simon`, `memory`)
- Physics-driven (`pong`, `air-hockey`, `asteroids`, `bullet-hell`)
- Grid-based (`bore-inc`, `jade-garden`)
- Complex combat (`ember-wars`)

A pure ECS is the wrong model for turn-based games — the components are
trivial and the systems are one function. A pure scene tree struggles
with hot-loop simulation (300+ bullets per frame).

### Decision

**Scene tree as the default; ECS as an optional crate.**

Games use `NodeTree` for structure and lifecycle. Games that need
high-performance simulation can add `ember_ecs` (v0.2) and store
data-oriented state in entities, referenced by nodes.

### Alternatives

- **Pure ECS.** Rejected because: (1) turn-based games have no
  performance need; (2) UI doesn't map to ECS; (3) the cognitive
  overhead of queries for a 3-card memory game is unjustifiable.
- **Pure scene tree.** Rejected because: (1) `bullet-hell` already
  needs 300+ bullets/frame; (2) without an escape hatch, the tree
  becomes the bottleneck.
- **Hybrid from day one.** Rejected because: (1) it doubles the design
  surface for v0.1; (2) the escape hatch can be added later without
  breaking the tree.

### Consequences

- Games are simple to write (tree is intuitive).
- `ember-wars` migration is the proof: if its hot combat loop is
  expressible in the tree, v0.2 can defer ECS entirely.
- ECS support is a v0.2 feature, not a v0.1 blocker.

## ADR-002: RON for scene format

**Status:** accepted (v0.1)

### Context

The scene format needs to be human-readable, VCS-friendly, and
parseable without a custom parser.

### Decision

**RON** (Rusty Object Notation), with a **TSCN-shaped structure**
(file header, external resources, internal resources, nodes,
connections).

### Alternatives

- **JSON.** Rejected: no tuple type (we need `Vec2(1.0, 2.0)`), no
  enums (we need `Variant` round-trips).
- **YAML.** Rejected: soft types cause silent coercion bugs; the
  2020 anchor CVEs are a warning.
- **TOML.** Rejected: config-shaped, not data-shaped. Tables of tables
  of arrays are not a scene tree.
- **TSCN exactly.** Rejected: custom parser required; the 5-section
  structure is preserved but the syntax is RON for `serde`.
- **A binary format.** Rejected: not VCS-diffable; scenes change more
  often than assets.

### Consequences

- `serde` does the parsing — no custom parser to maintain.
- Files are slightly more verbose than TSCN (`Float(5.0)` vs `5.0`) —
  acceptable cost for a parser we already have.
- Round-trip through Rust types is guaranteed.
- Forward-compatibility is enabled via unknown-property preservation.

## ADR-003: macroquad for v0.1, wgpu deferred

**Status:** accepted (v0.1)

### Context

The rendering backend choice affects everything: binary size, build
times, platform support, learning curve.

### Decision

**macroquad/miniquad for v0.1**, behind a `RenderingServer` trait.
`wgpu` is a v0.2 candidate.

### Alternatives

- **wgpu from day one.** Rejected: (1) the whole point of v0.1 is to
  ship; (2) `wgpu` abstraction setup is weeks of work before a single
  triangle draws; (3) the games already work on `macroquad`.
- **Custom Vulkan/Metal/D3D12.** Rejected: platform-specific, months
  of work, no benefit for a 2D POC.
- **Keep using macroquad directly.** Rejected: no seam for backend
  swap, hard to test, hard to profile. `RenderingServer` is the
  whole point of the migration.

### Consequences

- Fast time-to-first-frame.
- Web support is free (macroquad targets WebGL).
- The `RenderingServer` trait is the seam — wgpu adoption in v0.2 is a
  new impl crate, not a game rewrite.
- Performance ceiling is macroquad's; acceptable for 2D.

## ADR-004: Typed signals over string-keyed

**Status:** accepted (v0.1)

### Context

Godot uses string-keyed signals: `connect("body_entered", self, "_on_body_entered")`.
This is ergonomic in GDScript but has three failure modes:

1. Typo in signal name — runtime error.
2. Arity mismatch — runtime error.
3. Refactor breaks all `connect` calls.

### Decision

**Typed signals**: a signal is a struct, the struct's type is the
signal's identity, the receiver's closure signature is checked at
compile time.

```rust
#[signal]
pub struct BodyEntered { pub body: NodeId }

ctx.emit::<BodyEntered>(BodyEntered { body: id });
ctx.connect::<BodyEntered>(target, |ctx, ev| { /* checked */ });
```

### Alternatives

- **String signals** (Godot). Rejected: the failure modes above.
- **Const string signals** (`const BODY_ENTERED: &str = "..."`). Rejected:
  typo in const name is still a runtime error; doesn't fix arity.
- **Macro-generated method calls** (`ctx.on_body_entered(...)`). Rejected:
  requires codegen per signal; type inference is worse.

### Consequences

- Compile-time safety for the 90% case.
- `.escene` files still use string signal names (they have no type);
  `SceneLoader` validates that the name matches a registered signal
  type at load time.
- One extra struct definition per signal — the `#[signal]` macro is a
  no-op marker for readability.

## ADR-005: Rust-only, no scripting language

**Status:** accepted (v0.1)

### Context

Every major game engine supports a scripting language: Godot has
GDScript, Unity has C#, Unreal has Blueprints. Should Ember?

### Decision

**No embedded scripting language in v0.1.** Games are written in Rust.

### Alternatives

- **GDScript clone.** Rejected: months of language design, no benefit
  for a POC.
- **Embed Lua (`mlua`).** Rejected: (1) the borrow checker is the
  reason to use Rust; scripting reintroduces runtime errors; (2) Lua
  is dynamically typed — the exact opposite of Ember's philosophy.
- **Rhai (Rust-embedded scripting).** Rejected: closer, but still
  an extra runtime; loses the "one language" simplicity.
- **Hot-reloadable Rust (`dylib`).** Rejected: (1) `unsafe` at the
  boundary; (2) ABI incompatibilities; (3) platform-specific.
  Interesting v0.2+ direction.

### Consequences

- Compile times are the developer's iteration cost — this is Ember's
  main weakness compared to Godot.
- No runtime errors from dynamic typing — this is Ember's main
  strength.
- Games that need mod support must use Rust crates. v0.2 might revisit
  this if there's demand.

## ADR-006: Trait crates + impl crates

**Status:** accepted (v0.1)

### Context

The engine needs to support multiple backends (macroquad today, wgpu
tomorrow, a headless backend for tests). How should the abstraction be
structured?

### Decision

**Trait crate + impl crate** per backend area:

- `ember_servers` (traits only, no impl)
- `ember_render_macroquad` (impl of `RenderingServer`)
- `ember_audio_macroquad` (impl of `AudioServer`)
- `ember_render_wgpu` (future impl of `RenderingServer`)

Games depend on `ember` (facade) which picks a backend. Games do not
depend on a specific backend.

### Alternatives

- **Traits and impls in one crate** (e.g., `ember_servers::macroquad::*`).
  Rejected: dependencies leak. A game that wants `wgpu` would link
  `macroquad` anyway.
- **Traits in `ember_core`.** Rejected: `ember_core` should be pure
  math/time, not know about rendering.
- **No traits, feature flags** (`#[cfg(feature = "macroquad")]`).
  Rejected: feature flags multiply CI jobs; runtime panic on missing
  backend.

### Consequences

- Backend swap is a one-line `main.rs` change.
- Build times scale with backends enabled.
- The trait crate is small and easy to review.
- CI can verify no crate imports `macroquad` outside the impl crates.

## ADR-007: slotmap for NodeId

**Status:** accepted (v0.1)

### Context

The node tree needs stable ids that detect staleness. Options:

- `Rc<RefCell<Node>>` — classic reference-counted.
- `usize` index into a `Vec<Node>` — fast but no staleness detection.
- Generational arena (`slotmap`, `generation_arena`) — fast + safe.

### Decision

**`slotmap`** crate, wrapped in a `NodeId` newtype.

### Alternatives

- **`Rc<RefCell<Node>>`.** Rejected: (1) runtime borrow panics;
  (2) `!Send` without unsafe; (3) 2 words overhead per id.
- **`usize` index into Vec.** Rejected: removing a node shifts all
  subsequent indices; stale ids point to wrong nodes.
- **Custom generational arena.** Rejected: 300 lines of `unsafe` for
  no benefit over `slotmap`.

### Consequences

- `NodeId` is `Copy + Send + Sync`.
- Stale ids return `None` from `tree.get()`.
- Adding `slotmap` as a dependency (well-maintained, zero unsafe).

## ADR-008: Fixed-step physics, variable-step process

**Status:** accepted (v0.1)

### Context

The engine must run game logic consistently across frame rates. A
`Cooldown` that decrements by `dt` behaves differently at 30 Hz vs
144 Hz. The 31-test `ember-wars` bug is exactly this class of bug.

### Decision

**Fixed-step `physics_process` at 1/60 + variable-step `process`.**

Games put movement, collision, cooldowns in `physics_process`. Games
put UI, tweens, interpolation in `process`.

### Alternatives

- **All variable-step.** Rejected: the bug class above. Also: networked
  games need determinism, and variable-step breaks it.
- **All fixed-step at higher rate (240 Hz).** Rejected: wasted CPU;
  some behaviors (interpolation) *want* variable step.
- **Variable-step + accumulator in game code.** Rejected: games would
  reinvent it, and get it wrong.

### Consequences

- Games must respect the split (some learning curve).
- Deterministic physics enables future replay/network features.
- `ember-wars` migration must put combat resolution in
  `physics_process`, not `process`.

## ADR-009: Deferred spawn / structural changes

**Status:** accepted (v0.1)

### Context

Game code needs to spawn and despawn nodes during `process`. Immediate
spawn would invalidate iterators (`for child in children`). Immediate
despawn would leave dangling references.

### Decision

**Deferred spawn and despawn.** `ctx.spawn()` returns a `NodeId`
immediately; the tree applies the change at the end of the current
pass.

### Alternatives

- **Immediate spawn with iterator invalidation.** Rejected: it's the
  exact bug the deferral prevents.
- **`Option<Node>` + manual null checks.** Rejected: reintroduces the
  null problem Rust exists to prevent.
- **Queue spawns in game code.** Rejected: every game would reimplement
  it.

### Consequences

- `tree.get(id)` returns `None` for a freshly-spawned id until `flush`.
- `flush` runs at the end of `process` / `physics_process`.
- `draw` cannot spawn (it's `&self`).

## ADR-010: primary_action convenience method

**Status:** accepted (v0.1)

### Context

Every game implements the same menu-confirm pattern: Space OR Enter OR
Left Click OR gamepad South. That's three checks per game, times 17
games, times every menu.

### Decision

**`Input::primary_action()`** returns true if any of {Space, Enter,
Left Click, Gamepad South} pressed this frame.

Implementation (`ember-input`) also counts any touch started this frame
for `primary_action()`. `secondary_action()` covers Escape / Backspace /
Right Click / Gamepad East.

### Alternatives

- **Each game checks its own keys.** Rejected: inconsistent across
  games; new games miss a key.
- **Full action map for menus.** Rejected: menus shouldn't require a
  config file.
- **Configurable `primary_action` per game.** Rejected: it's a
  convention, not a customization point.

### Consequences

- Menu code is one line: `if ctx.input().primary_action()`.
- Games that need different keys override with raw key checks.
- Touch taps map to `primary_action` — mobile works for free.

## ADR-011: Cooldown::new → running / ready

**Status:** accepted (v0.1)

### Context

`Cooldown::new(0.5)` currently constructs a cooldown that **starts
running**. A programmer reading `new(0.5)` reasonably expects "a fresh
cooldown of 0.5s" which would be *ready*. This mismatch was the root
cause of the 31-test `ember-wars` bug.

### Decision

**Rename constructors:**

- `Cooldown::ready(d)` — ready now, re-arms after `trigger()`
- `Cooldown::running(d)` — starts active, ready after `d`

Deprecate `new` → `running`, `with_duration` → `ready` (removed in
v0.2).

> **Implemented:** `running(d)` / `ready(d)` exist; `new` and
> `with_duration` are `#[deprecated]` aliases and all workspace call
> sites are migrated.

### Alternatives

- **Keep `new`, add a doc comment.** Rejected: docs are unread.
- **Make `new` = `ready`.** Rejected: it's a breaking semantic change
  for all existing test code.
- **Remove `new` immediately.** Rejected: `#[deprecated]` is the
  principled path.

### Consequences

- Call sites are self-documenting: `Cooldown::ready(x)` and
  `Cooldown::running(x)` say what they mean.
- One version of deprecation noise (`warning: use Cooldown::running`)
  before removal in v0.2.
- The 31-test bug cannot recur with the same source code.

## ADR-012: Handle<T> for assets

**Status:** accepted (v0.1)

### Context

Assets (textures, audio, scenes) need to be shared across nodes and
unloaded when unused. Options:

- Global cache (Godot-style) with explicit `unload`.
- Reference-counted handles (Bevy, wgpu-style).
- Paths-as-keys, reload on demand.

### Decision

**Reference-counted `Handle<T>`.** Dropping the last handle unloads
the asset on the next `poll()`.

### Alternatives

- **Global cache + explicit `unload(path)`.** Rejected: leaks if a
  caller forgets; requires manual tracking of which assets are in use.
- **Path-based reload.** Rejected: slow (`load` re-reads file);
  no unloading.
- **`Arc<T>` directly.** Rejected: `Arc<Texture>` doesn't notify the
  server on drop; the texture would live forever.

### Consequences

- Game code is `if let Some(tex) = handle.get() { ... }`.
- `Handle` clone/compare is O(1).
- Hot-reload swaps the asset in place; all handles see the new data.

## ADR-013: No global mutable state

**Status:** accepted (v0.1)

### Context

Global mutable state (`static mut`, `lazy_static!`, `thread_local!` for
game state) is tempting — it removes `&mut self` threading. It is also
the source of the two hardest classes of bug:

1. **Hidden invariants** — no one knows what touches the state.
2. **Testing impossibility** — every test shares the same state.

### Decision

**No global mutable state in engine crates.** All mutable state lives
in a struct threaded through `&mut self`. `ClassDB` is created once and
passed by `&mut`. Server instances live in `MainLoop`.

### Alternatives

- **`lazy_static!` for `ClassDB`.** Rejected: makes tests non-isolated;
  test A affects test B.
- **`thread_local!` for RNG.** Rejected: same problem; also incompatible
  with future threading.
- **`once_cell` for `ClassDB`.** Rejected: same as above.

### Consequences

- Every context has explicit dependencies (in the signature).
- Tests are trivially isolated (each creates its own instances).
- Slightly more boilerplate at startup (plugin registration).

## ADR-014: Explicit plugin registration

**Status:** accepted (v0.1)

### Context

The `ClassDB` needs to know about every node type at startup. Options:

- `linkme` / `inventory` — auto-registration via linker sections.
- Explicit `db.register::<T>()` in `Plugin::build`.
- A `#[derive(Node)]` that generates a `register` function.

### Decision

**Explicit registration** in `Plugin::build`. The derive macro generates
the impl but not the registration call.

### Alternatives

- **`linkme` auto-registration.** Rejected: reintroduces global mutable
  state (ADR-013); doesn't work on WASM without linker flags.
- **Auto-registration via macro-generated `ctor`.** Rejected: same as
  `linkme` — implicit global side-effect.
- **Registration on first use.** Rejected: races in future threaded
  init; hard to debug missing registrations.

### Consequences

- 30 lines of registration per game (one per node type).
- Startup order is explicit — easy to debug.
- No hidden state.

## ADR-015: Hot-reload scope — assets only

**Status:** accepted (v0.1)

### Context

Hot-reload is the single biggest iteration speedup. Full hot-reload
(including Rust code) requires `dylib` loading, `unsafe`, ABI stability,
and platform-specific hacks.

### Decision

**Asset hot-reload in v0.1** (textures, audio, fonts, scenes). **No
code hot-reload.** Code changes require a restart.

### Alternatives

- **Full code hot-reload.** Rejected: (1) `dylib` incompatibilities;
  (2) unsafe at every function boundary; (3) requires ABI stability
  across crate versions.
- **Sub-second incremental builds.** Out of scope; `cargo` controls
  this.
- **Interpreted Rust (Miri in-game, etc.).** Not viable.

### Consequences

- Editing a PNG is visible in the running game.
- Editing a `.rs` file requires `cargo build` + relaunch.
- Games with a long startup cost (asset loading) benefit most.
- v0.2 might revisit code hot-reload with `subsecond` or
  `hot-lib-reloader`.

## ADR-016: RenderingServer / AudioServer extensions

**Status:** accepted (v0.1)

### Context

`12-servers.md §4.1` lists a floor of methods: `begin_frame`,
`end_frame`, `set_clear_color`, `set_camera`, `set_z`, `draw_rect`,
`draw_rect_outline`, `draw_circle`, `draw_line`, `draw_sprite`,
`draw_text`, `create_texture`, `destroy_texture`, `supports`. Three
inadequacies surfaced when implementing Phase 2 against Pagoda's actual
needs:

1. `create_texture` returned `TextureId` infallibly. GPU allocation can
   fail (out-of-memory, invalid image format, device lost). The signature
   provided no escape for the backend to report failure.
2. Pagoda's `CircleButton` tessellates manually into triangles because
   macroquad's `draw_circle` fill and `draw_circle_lines` border
   tessellate at different resolutions, producing a visible gap. No
   `draw_triangle` existed. Pagoda's `juice.rs` rotates falling slice
   pieces via `draw_rectangle_ex` — no `draw_rect_rotated` existed.
   Shockwave rings use `draw_circle_lines` — no `draw_circle_outline`
   existed (macroquad's is in a different module than what a trait
   would expose).
3. Pagoda bundles DejaVu Sans Mono as default and Noto Sans SC (9 KB
   CJK subset) for decorative glyphs. The spec's `draw_text` had no way
   to specify a non-default font; the CJK subset needs to be drawn
   explicitly through `draw_text_ex`.

`12-servers.md §5.1` similarly omitted `load_clip` / `destroy_clip` —
without them, `ember_assets` has no path to inject a loaded audio clip
into the backend.

### Decision

Extend `RenderingServer` with:

- `draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color)`
- `draw_circle_outline(&mut self, center: Vec2, radius: f32, thickness: f32, color: Color)`
- `draw_rect_rotated(&mut self, rect: Rect, rotation_radians: f32, color: Color)`
- `draw_text_with_font(&mut self, font: FontId, text: &str, pos: Vec2, size: f32, color: Color)`
- `measure_text_with_font(&self, font: FontId, text: &str, size: f32) -> TextMetrics`
- `set_default_font(&mut self, font: Option<FontId>)`
- `create_font(&mut self, bytes: &[u8]) -> Result<FontId, ServerError>`

Change:

- `create_texture(&mut self, image: &Image) -> Result<TextureId, ServerError>` (was infallible)

Extend `AudioServer` with:

- `load_clip(&mut self, bytes: &[u8]) -> Result<AudioClipId, ServerError>`
- `destroy_clip(&mut self, clip: AudioClipId)`

### Alternatives

- **Keep infallible `create_texture`, panic on backend failure.**
  Rejected: I4 forbids panics in engine crates on user inputs; texture
  creation is a user input (game loads a sprite).
- **Expose macroquad's `draw_circle_lines` under the same name.**
  Rejected: `draw_circle` and `draw_circle_lines` in macroquad use
  inconsistent tessellation, producing the visible seam Pagoda's
  `circle_button.rs` documents. A single `draw_circle_outline` method
  is a smaller surface than "fix macroquad's tessellation upstream."
- **Defer font methods to v0.2.** Rejected: Pagoda already ships the
  CJK subset today; dropping it in v0.1 would be a regression.
- **Make `create_texture` optionally fallible via a `try_create_texture`
  variant.** Rejected: two ways to do the same thing, `try_*` should
  mean "checks user input", not "may fail at OS layer."

### Consequences

- Backends are allowed to fail allocation. `create_texture().unwrap()`
  is a bug the compiler catches when the signature changes — good.
- `ember_render_macroquad` gets seven new method bodies in Phase 5;
  none of them complex (all are direct macroquad calls or one-liners).
- `draw_triangle` and `draw_circle_outline` can be added to future
  backends (wgpu) as primitive batch types without changing game code.
- The `12-servers.md §4.1` and `§5.1` code blocks need updating to
  match the actual trait. Tracked below.

## ADR-017: PhysicsServer uses opaque u64 handles, not NodeId

**Status:** accepted (v0.1)

### Context

`12-servers.md §6.1` sketches `PhysicsServer::register(&mut self, id: NodeId, shape: Shape, mask: CollisionMask)` — taking a `NodeId` as a parameter. `NodeId` lives in `ember_scene` (L3); `PhysicsServer` lives in `ember_servers` (L2). This would require `ember-servers` to import from `ember-scene`, violating I1 (strict downward dependency).

### Decision

`PhysicsServer` uses opaque `u64` handles:

~~~rust
pub trait PhysicsServer {
    fn register(&mut self, shape: Shape, mask: CollisionMask) -> u64;
    fn unregister(&mut self, handle: u64);
    fn query_overlaps(&self, shape: &Shape, mask: CollisionMask) -> Vec<u64>;
    fn raycast(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Option<RayHit>;
    fn rebuild(&mut self);
}
~~~

The mapping from `NodeId` to `u64` (if any) is `ember_scene`'s concern. The trait also gained `update(handle, shape)` (ADR-028). `LinearPhysics` (ADR-028) is the v0.1 implementation; games that have not migrated still use `ember_stdlib::collider`.

### Alternatives

- **Move `NodeId` into `ember-core`.** Rejected: `NodeId` is a slotmap key — its invariants (generational arena, parent tracking) live in `ember_scene`. Hoisting it means either duplicating slotmap in `ember-core` or exposing a bare `u64` newtype in `ember-core` that `ember_scene` wraps — cosmetic indirection.
- **Define `PhysicsServer` in `ember-scene`.** Rejected: it's a server trait (L2 concern). Putting it in `ember_scene` means `ember_servers` can't define it — inconsistent with the other two servers.
- **Generic `PhysicsServer<K>` with `K: Copy` for keys.** Rejected: overengineering for a trait with zero implementations in v0.1.

### Consequences

- I1 holds at the PhysicsServer boundary.
- When a v0.2 physics backend lands, the scene layer needs to keep a `HashMap<NodeId, u64>` (or equivalent). This is small, local, and easy to test.
- Games that want to use the raw trait directly (rare — most will use `NodeContext::physics()`) never deal with `NodeId`.

## ADR-018: `Input` is not `#[non_exhaustive]`

**Status:** accepted (v0.1)

### Context

`02-principles.md §2.6` (I6) requires all public types that are expected to grow to be `#[non_exhaustive]`. This is a good default — it prevents breaking changes when a variant is added to an enum or a field to a struct.

`Input` (`ember-input`) is a struct games need to construct as test fixtures:

~~~rust
let input = Input {
    mouse_pos: Vec2::new(100.0, 50.0),
    mouse_left_pressed: true,
    ..Default::default()
};
~~~

If `Input` were `#[non_exhaustive]`, that struct-update syntax would not compile outside `ember-input` — every game's `game_scenarios.rs` test file would break.

### Decision

`Input` is **not** `#[non_exhaustive]`. Adding a field is a breaking change; it happens once per minor version at most, and the migration is mechanical (`..Default::default()` still works with the struct-update syntax if the new field has a default).

The enums in `ember-input` (`Key`, `MouseButton`, `GamepadButton`, `TouchPhase`) **are** `#[non_exhaustive]` — matching on them is fine because games already use `_ => {}` in the rare case they match.

### Alternatives

- **`#[non_exhaustive]` + a `Input::builder()` API.** Rejected: three lines of builder boilerplate per field for a fixture struct. Tests would become `Input::builder().mouse_left_pressed(true).build()`.
- **`#[non_exhaustive]` + a `Input::with_*` chain.** Rejected: same boilerplate problem.
- **Make `Input` grow-only via interior optional fields.** Rejected: adds a field per future concept (`left_stick: Option<Vec2>`, `right_stick: Option<Vec2>`, …) even when most games never touch them.

### Consequences

- I6 has a documented exception: `Input`.
- Adding a field to `Input` in v0.2 is a minor-version breaking change. Acceptable — the fix at every game's test file is `..Default::default()` still working, or a mechanical `foo: false,` insertion.
- The `#[non_exhaustive]` policy in `02-principles.md §2.6` gets a one-line note: "Exception: fixture structs that games construct — see ADR-018."

## ADR-019: `#[derive(Node)]` scope in v0.1

**Status:** accepted (v0.1)

### Context

`10-nodes-and-scenes.md §4.1` sketches `#[derive(Node)]` as generating
an implicit `name: String` field, a `new()` constructor, and
serialization for `#[export]` fields. Three of those are not
implementable as a derive macro in Rust:

1. **Derive macros cannot add fields** to the input struct. An implicit
   `name` would require an attribute macro (`#[node]`) instead of
   `#[derive(Node)]`, changing the syntax users expect from the spec.
2. **A generated `new()` cannot know how to construct the struct.**
   Even for `Default`-able structs, generating `new()` collides with
   user-defined `new()` and hides construction semantics.
3. **Serialization depends on `.escene` format details** that
   `30-escene-format.md` has not frozen. Emitting a serialize body now
   would pin the format early or be discarded in Phase 11.

### Decision

For v0.1, `#[derive(Node)]`:

- **Requires** the struct to have a field `name: String`. The derive
  emits `name()` / `set_name()` against that field.
- **Requires** the `#[class_name = "..."]` attribute. Missing →
  compile error.
- **Does NOT generate** a `new()` constructor. Users write their own.
- **Does NOT generate** a `serialize()` body. The trait default (no-op)
  is used until Phase 11.

The derive emits exactly:

- `impl Object for T` — `class_name()`, `as_any()`, `as_any_mut()`
- `impl Node for T` — `name()`, `set_name()`

### Alternatives

- **Attribute macro `#[node]` that rewrites the struct.** Rejected:
  changes the user-facing syntax, and attribute macros are strictly
  harder to write and debug than derives.
- **Implicit `name` via a wrapper type** (`NodeName { inner: String }`).
  Rejected: adds a `.name` indirection to every field access,
  breaking `self.name == "..."`.
- **Generated `new()` via `Default`.** Rejected: `Default::default()`
  for a `Node` with a required `NodeId` child would be nonsense.
- **Emit `serialize()` with a fixed RON schema.** Rejected: schema is
  not frozen; churn in Phase 11.

### Consequences

- Every `#[derive(Node)]` struct has a visible `name: String` field.
  More verbose, but the field is honest — it exists, it is initialized
  by the user, it is accessible as `self.name`.
- `T::new()` is written by the user. The compile error on a missing
  `name` field points at the struct, not at a mysterious "no `new`".
- The derive is ~80 lines and can be extended in Phase 11 to serialize
  `#[export]` fields without a breaking change to existing deriving
  structs. The `#[export]` attribute is declared in the derive's
  `attributes(...)` list now, so adding the serialization behavior is
  purely additive.

## ADR-020: Wasm font/audio loading strategy

**Status:** accepted (v0.1)

### Context

`RenderingServer::create_font` and `AudioServer::load_clip` take
`&[u8]` and return a resource handle. On native, macroquad's
`load_ttf_font_from_bytes` and `load_sound_from_bytes` are `async` but
resolve immediately — the filesystem read is synchronous under the
hood, the `async` is for API uniformity with the web path. On wasm,
the same calls perform an HTTP fetch: genuinely async, and blocking
the executor on wasm deadlocks the JS event loop.

The trait signatures are synchronous because `&mut dyn RenderingServer`
cannot hold a borrowed future. Making the traits async (or returning
`Pin<Box<dyn Future>>`) would cascade through `NodeContext`,
`AssetServer`, `MainLoop`, and every game — the whole engine becomes
async.

### Decision

For v0.1:

- **Native**: `load_clip` blocks on the async macroquad call via
  `futures::executor::block_on`; `create_font` calls the synchronous
  `load_ttf_font_from_bytes`. The cost is negligible — these are called
  once at startup, not per frame.
- **Wasm**: `create_font` and `load_clip` return
  `Err(ServerError::NotSupported)`. Games on wasm load fonts and audio
  **before entering the loop** (directly via
  `macroquad::text::load_ttf_font` / `macroquad::audio::load_sound`),
  then pass the loaded resources into the engine through backend
  "adoption" methods:
  - `MacroquadRenderer::adopt_font(Font) -> FontId`
  - `MacroquadAudio::adopt_clip(Sound) -> AudioClipId`

Pagoda's current `main.rs` already follows this pattern (fonts and
`SoundBank` are awaited before `loop { ... }`). ADR-020 documents the
existing practice; it does not change it.

### Alternatives

- **Make the traits `async fn`.** Rejected: `async fn` in traits has
  object-safety limitations that would prevent `Box<dyn
  RenderingServer>`, which the whole design depends on.
- **Return `Pin<Box<dyn Future<Output = Result<...>>>>` from the sync
  methods.** Rejected: every caller must decide whether to `.await`,
  and the sync/async split leaks into game code.
- **Load all assets before the loop, everywhere (including native).**
  Rejected: `AssetServer` hot-reload needs to call `load_clip` at
  runtime on native. Forcing pre-load means no hot-reload.
- **Use `wasm_bindgen_futures::spawn_local`.** Rejected: introduces
  `wasm-bindgen` as a dependency, which the spec excludes (Pagoda's
  `DESIGN.md §7.1`: "No `wasm-bindgen`").

### Consequences

- Native games can call `create_font` / `load_clip` from anywhere. A
  `load_font` / `load_audio` asset path does not exist yet (font/audio
  loaders are deferred, ADR-026).
- Wasm games must load fonts and audio in `main.rs` before the loop.
  This is a constraint, not a bug — Pagoda works this way today.
- The two backend structs grow one small `adopt_*` method each (~10
  lines total).
- v0.2 may revisit if macroquad adds a sync load-from-memory path or
  if the engine moves to a fully async asset pipeline.

## ADR-021: Nested folder organization for backend families

**Status:** accepted (v0.1)

### Context

`01-architecture.md §3.1` sketches a flat crate tree. Every backend
crate lives at `crates/<feature>-<backend>/`. As v0.2 adds
`ember-render-wgpu`, `ember-audio-cpal`, and potentially
`ember-render-metal`, `crates/` accumulates one entry per backend,
with no structural grouping.

### Decision

Adopt `crates/<feature>/<backend>/` for backend families:

~~~
crates/
├── ember-render/
│   └── macroquad/      (package: ember-render-macroquad)
├── ember-audio/
│   └── macroquad/      (package: ember-audio-macroquad)
~~~

Package names remain `<feature>-<backend>` (unchanged). Folder layout
changes only. `ember-render/` and `ember-audio/` are grouping
directories — no `Cargo.toml`, no `src/`.

### Alternatives

- **Flat structure** (Bevy, tokio convention). Rejected: with 4+
  backend crates expected by v0.3, `ls crates/` becomes a scan rather
  than a categorization.
- **Feature flags on a single crate** (e.g., `ember-render` with
  `features = ["macroquad", "wgpu"]`). Rejected in ADR-006; feature
  flags multiply CI jobs and force the backend into the crate that
  games depend on.
- **Reorganize every crate into families** (e.g., `ember-core/` inside
  `crates/ember-core/ember-core/` → overkill). Rejected: only backend
  crates have multiple implementations; everything else is one crate
  per name.

### Consequences

- `ls crates/` remains scannable; `ls crates/ember-render/` reveals
  the backend inventory.
- `[workspace] members` lists nested paths; `[workspace.dependencies]`
  paths match.
- `xtask/check-macroquad` skips backend paths by prefix; that constant
  grows by one entry per backend crate.
- `check-layers` (name-based via `cargo metadata`) is unaffected.
- Games' `use ember_render_macroquad::...` statements are unaffected —
  package names do not change.

## ADR-022: Tree passes take `Servers`; nodes are checked out while they run

**Status:** accepted (v0.1)

### Context

`21-ember-scene.md §5.2` sketches `tree.process(&mut NodeContext, dt)`,
but `NodeContext` holds `&mut NodeTree` (§6). A tree method cannot
receive a context that already mutably borrows it, and a node cannot be
`&mut` borrowed from the tree while the context also borrows the tree.

### Decision

- `NodeTree::process`, `physics_process` and `flush` take
  `&mut Servers` (rendering, audio, input). The tree builds one
  `NodeContext` per callback.
- A node is taken out of its slot for the duration of its callback and
  put back afterwards. During that time `tree.get(current)` is `None`.
- `ember_scene` depends on `ember-input` (added to `check-layers`).
- `MainLoop` lives in `ember_core::main_loop`; `draw` takes `&mut self`
  because rendering backends are stateful. `ember_scene::SceneLoop`
  adapts a tree to it.
- A node with `visible() == false` hides its whole subtree from `draw`.

### Alternatives

- `Option<Box<dyn Node>>` with `RefCell` slots: runtime borrow panics,
  the failure mode ADR-007 avoids.
- Context holds only a command queue, no tree: no `ctx.tree().get_mut`,
  which `10-nodes-and-scenes.md §5.3` relies on.

### Consequences

- Looking up the running node by id from inside its own callback fails;
  `ctx.find("../Self")` does too.
- Direct `ctx.tree().attach/remove` is immediate and safe: passes walk
  an id snapshot and skip ids that are no longer live.
- `PhysicsServer` joins `Servers` with `LinearPhysics` (ADR-028).

## ADR-023: One tree-level `SignalTable`; handlers are `Arc`, dispatch lives on `NodeContext`

**Status:** accepted (v0.1)

### Context

`21-ember-scene.md §10` has `SignalTable::emit(payload, ctx)` and a
table per node slot, with `Box` handlers and a `ScopedConnection` that
disconnects on the next `flush`. Handlers take `&mut NodeContext`, which
borrows the tree, so the table cannot be borrowed while a handler runs.

### Decision

- A single `SignalTable` on `NodeTree` (handlers already carry their
  target `NodeId`; `11-signals.md §7.1`). `tree.remove` drops the removed
  nodes' handlers.
- Handlers are stored as `Arc`. `NodeContext::emit` snapshots the `T`
  handlers in connection order, releases the table, then calls each one
  with `current_id()` set to its target. Targets that are queued but not
  yet live are skipped; targets that no longer exist are disconnected.
- `emit_deferred` queues on the tree; `flush` runs the queue FIFO, one
  round at a time, alongside the ready cascade (same 1000-round cap).
- `ScopedConnection::drop` pushes its id on a shared queue. The table
  applies it at `flush` and at the start of every `emit` (earlier than
  the spec, never later). `forget()` keeps the handler connected.
- `connect`, `connect_scoped` and `disconnect` exist on both `NodeTree`
  (for setup code) and `NodeContext`.

### Consequences

- A handler targeting the node whose callback is running cannot reach
  that node through `ctx.tree()` (ADR-022: it is checked out).
- Scene-declared connections wait for `.escene` / `PackedScene`.
- `Node::signals` / `signals_mut` (spec §4.2) are not needed.

## ADR-024: `ClassDB` is generic over `Object`; node construction is an extension

**Status:** accepted (v0.1)

### Context

`20-ember-core.md §9.2` has `ClassDB::instantiate -> Box<dyn Object>`,
but `.escene` instantiation needs `Box<dyn Node>`, and `ember-core`
(L1) cannot name `Node` (L3). A `Box<dyn Object>` cannot be turned into
a `Box<dyn Node>`.

### Decision

- `ember_core::ClassDB` stores a boxed constructor per class name, plus
  a small per-class extension map keyed by `TypeId`
  (`set_extension` / `extension`).
- `ember_scene::ClassDbNodes` (extension trait on `ClassDB`) adds
  `register_node`, `register_node_with` and `instantiate_node`. It
  registers the class as an `Object` and stores the node constructor as
  an extension.
- `instantiate` and `names` take `&self`; there is no global registry.
  The first registration of a name wins.

### Alternatives

- `Object::into_node(self: Box<Self>)`: makes core depend on scene.
- A second registry in `ember-scene`: two places to register every class.

### Consequences

- Games register nodes with `db.register_node::<T>()`, not
  `db.register::<T>()`; the latter yields a class that `instantiate_node`
  refuses.
- `#[derive(Node)]` does not generate `Default`; classes need
  `#[derive(Default)]` or `register_node_with`.

## ADR-025: `.escene` loading takes a `ClassDB` and a tree; properties go through `Node::set_property`

**Status:** accepted (v0.1)

### Context

`21-ember-scene.md §8` has `instantiate(ctx)`, nodes with
`post_deserialize`, and connections dispatched through
`SignalReceiver::call_method`. `#[export]` is not implemented, `ctx`
spawns are deferred (ADR-022), and ADR-023 dropped per-node signal
tables. `AssetServer` (needed for `instance:`) does not exist yet.

### Decision

- `PackedScene::instantiate(&ClassDB, &mut NodeTree) -> NodeId` builds
  the nodes immediately and returns the root detached;
  `instantiate_into(.., parent)` attaches it. Classes are all checked
  before anything is built, so `ClassNotRegistered` leaves the tree
  untouched.
- `Node::set_property(&mut self, key, &Variant) -> bool` (default
  `false`) applies each property. Unknown or mistyped keys warn and are
  skipped but stay in the template, so `to_ron` round-trips them.
  `visible: false` and `z != 0` are applied as `"visible"` / `"z"`.
- The file model is private serde structs; `PackedScene` keeps it, so
  `to_ron` is lossless. `parent` and `instance` are plain strings
  (empty = absent), which keeps the spec's `parent: "."` syntax.
- `Variant` lives in `ember-core` with RON tuple syntax
  (`Vec2(1.0, 2.0)`).

### Deferred

- `instance:` nodes error until `AssetServer` (5c); no cycle check yet
  (both resolved by ADR-027).
- Connections are parsed, validated and kept, but not wired: no
  method-name dispatch exists. They are applied when scenes can name
  handlers.

### Consequences

- Every scene-loadable class implements `set_property` by hand until
  `#[export]` lands.

## ADR-026: `AssetServer` is type-driven and synchronous; GPU loads take a renderer

**Status:** accepted (v0.1)

### Context

`25-ember-assets.md` specifies `AssetLoader` objects in a
`LoaderRegistry`, async loading, a `notify` hot-reload watcher, and
`SceneLoader`/`PackedScene` inside `ember-assets`. `ember-scene` depends
on `ember-assets` (L3 -> L3), so assets cannot name `PackedScene`.
`load(&self, path)` also cannot create a texture without a
`RenderingServer`.

### Decision

- The `Asset` trait carries `extensions()`, `load(bytes, ctx)` and
  `reload`. There is no loader registry: the extension must be in
  `T::extensions()` (else `WrongType`), and a custom asset is a type
  implementing `Asset`. `ember-scene` implements `Asset` for
  `PackedScene` and adds `SceneAssets::scene`.
- `load` passes no renderer; `load_with` / `texture` pass
  `&mut dyn RenderingServer` in `AssetLoadContext::rendering`.
  `Texture` without one fails with `NeedsRenderer`.
- `Handle<T>` is `Arc<RwLock<T>>` plus `AssetId` and `Uid`; the server
  caches a `Weak` per path. `poll()` prunes dead entries; there is no
  drop hook.
- `uid_registry.ron` lives in the asset root and is saved on the first
  load of a path. UIDs are `uid://<stem>_<5 hex of content hash>` and
  never change.
- Built-ins: `Texture` (PNG via the `png` crate) and `RonAsset<T>`.

### Deferred

- Hot-reload watching (`reload` exists and is manual), async loads,
  audio/font loaders, JPEG.
- Unloading a `Texture` does not call `destroy_texture`; only `reload`
  does.
- Nested `instance:` in `.escene` (ADR-025) can now be built on
  `SceneAssets`.

### Consequences

- Loading a GPU asset requires passing the renderer at the call site.

## ADR-027: Nested `instance:` is resolved by `instantiate_with`

**Status:** accepted

### Decision

- An entry has exactly one of `class` or `instance` (`class` is now
  optional in the file; both or neither is a `Parse` error).
- `PackedScene::instantiate_with(&ClassDB, &AssetServer, &mut NodeTree)`
  loads each `instance:` path through `SceneAssets` and builds it
  recursively. The nested root takes the entry's `name`, `properties`,
  `visible` and `z` as overrides.
- `instantiate` (no `AssetServer`) fails on `instance:` with `Asset`.
- Cycles are detected by scene `uid` along the current chain and fail
  with `InstanceCycle("a -> b -> a")`.
- Any failure removes everything built so far; the tree is untouched.

### Consequences

- Overrides reach only the nested root, not its descendants.
- Connection wiring and hot-reload remain open.

## ADR-028: `LinearPhysics` lives in `ember-servers` and joins `Servers`

**Status:** accepted

### Decision

- `LinearPhysics` is the v0.1 `PhysicsServer` backend, in
  `ember-servers/src/physics/linear.rs` (the layer rules forbid
  `ember-servers` from depending on `ember-stdlib`). The old
  `ember-stdlib::collider` is not migrated; the backend is written
  against `ember_core::math::{Aabb, Circle}`.
- Handles are monotonically increasing `u64` from 1 (ADR-017).
- A shape's mask is the layers it belongs to; a query matches when
  the masks share a bit (`CollisionMask::overlaps`).
- `PhysicsServer::update(handle, shape)` was added so moving bodies
  need no unregister/register churn.
- `raycast` returns the nearest hit (`t` in `[0, 1]` along the
  segment). A ray starting inside a shape hits at distance 0 with the
  normal opposite the ray.
- `Servers` and `NodeContext` expose `physics`. `SceneLoop` owns a
  `LinearPhysics` (`physics_mut()`), so its generics are unchanged.

### Consequences

- Queries are O(n); fine below ~300 shapes. Spatial indexing is v0.2.
- Collision response and swept tests stay in game code for now.

## ADR-029: Shared `ember_stdlib::juice`; game-specific effects wrap it

**Status:** accepted

### Context

Three games still shipped diverged copies of `juice.rs` (particles,
floating text, shockwaves, flashes, screen shake). Their APIs differed
(`spawn_hit_spark` vs `spark`, per-game gravity/drag, extra effects such as
Pagoda's falling slices and Jade Garden's radial rays).

### Decision

- `ember_stdlib::juice` owns the common effects. Per-game tuning goes
  through `JuiceConfig` (gravity, drag, particle life, text rise/life);
  draw functions take an explicit offset (`draw_particles(juice, offset)`)
  or size (`draw_flashes_sized`) so they work with cameras and virtual
  canvases.
- Games with extra effects keep a local `effects.rs` with an `Effects`
  struct that holds a `Juice` and `Deref`s to it. Inherent methods
  (e.g. `Effects::tick`) shadow the stdlib ones and tick both layers.

### Consequences

- `find games -name juice.rs` is empty; ~600 duplicated lines are gone.
- `Deref` means `self.juice.spawn_*(self.method())` can hit E0502;
  hoist arguments into locals.

## How to add an ADR

1. Copy the template:

```markdown
## ADR-NNN: <short title>

**Status:** proposed | accepted | superseded by ADR-MMM

### Context
<the problem>

### Decision
<what was chosen>

### Alternatives
<what was rejected and why>

### Consequences
<what it enables and what it costs>
```

2. Number sequentially (`ADR-022`, etc.).
3. Update `docs/engine/v0.1/README.md` if the ADR is referenced by a
   doc.
4. Do not delete old ADRs — supersede them.

## Cross-references

- Architecture → [[01-architecture (branch)]]
- Invariants → [[02-principles (branch)]]
- Server trait contracts → [[12-servers (branch)]]
- Node/scene concepts → [[10-nodes-and-scenes (branch)]]
- Roadmap phases → [[40-roadmap (branch)]]
- Glossary → [[51-glossary (branch)]]
