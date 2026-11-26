---
title: "50 — Design Decisions (ADRs)"
aliases: ["50 — Design Decisions (ADRs)"]
tags: ["ember/spec", "ember/wave-4"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/50-design-decisions.md"
imported: "2026-11-22"
wave: "4 — Format, Process & Reference"
---

# 50 — Design Decisions (ADRs)

> **Status:** 🟡 partial — reviewed 2026-11-19; ADR-001..028 numbered consistently, ADR-016..028 match code except ADR-011 and ADR-020 loaders (not implemented, noted inline)
> **Audience:** all
> **Prerequisites:** [[01-architecture]], [[02-principles]]

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

> **Amended by ADR-030:** the derive now emits `impl NodeName` (name,
> set_name) instead of `impl Node`; users write `impl Node for T { … }`.

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

## ADR-030: `#[derive(Node)]` generates `NodeName`; users own `impl Node`

**Status:** accepted (2026-11-22) — amends ADR-019

### Context

ADR-019's derive emitted `impl Object` **and** a complete `impl Node`
(`name`, `set_name`). Rust allows one `impl Node for T`, so a derived
struct could not add `ready`/`process`/`draw`: `hello_scene.rs` had to
hand-write `Player`, and every derived node in [[10-nodes-and-scenes]],
[[24-ember-ui]] and [[26-ember-macros]] (`#[derive(Node)]` followed by
`impl Node for T { fn process … }`) did not compile. Phase 5's widgets and
acceptance item 8 ([[42-acceptance-tests]]) depend on that form.

### Decision

- Split identity out of `Node`: `pub trait NodeName { fn name(&self) -> &str;
  fn set_name(&mut self, name: String); }` and `pub trait Node: Object +
  NodeName { /* z, visible, lifecycle, set_property, serialize — all
  defaulted */ }`.
- `#[derive(Node)]` emits `impl Object` + `impl NodeName` (still from the
  required `name: String` field and `#[class_name]`). It emits **no**
  `impl Node`.
- The user always writes `impl Node for T { … }` with only the methods
  the node needs; a node without behaviour writes `impl Node for T {}`.
- `NodeName` is exported at the `ember_scene` root and in both preludes.

### Alternatives

- **Derive delegates to a second trait** (`#[node(behavior)]` +
  `impl NodeBehavior for T`): non-breaking, but two traits with the same
  method names, and an attribute to remember. Rejected.
- **Attribute macro on the `impl` block** (`#[node] impl Player { … }`):
  hides the trait the spec documents; harder to read and debug. Rejected.
- **Derive also emits an empty `impl Node` unless `#[node(custom)]`**:
  saves one line for data nodes but makes the common case (behaviour)
  need an opt-out. Rejected; may return as `#[node(data)]` if data nodes
  become common.

### Consequences

- The spec's `#[derive(Node)]` + `impl Node for T { … }` form now compiles;
  Phase 5 widgets can use it.
- Breaking for hand-written nodes: `name`/`set_name` move into
  `impl NodeName for T`. Migrated in the same commit: pong (2 nodes),
  `hello_scene`, ember-scene tests (6 impls).
- Calls through `dyn Node` (`tree.get(id)?.name()`) are unchanged:
  supertrait methods are callable on the trait object.
- Forgetting `impl Node` on a derived struct is a compile error at first
  use as a node (`register_node`, `NodeTree::insert`), not at the derive.
- Compile-fail tests for the derive (`trybuild`) are still open.

## ADR-031: Clip-rect stack on `RenderingServer`

**Status:** accepted (2026-11-22)

### Context

[[24-ember-ui]] needs real clipping (`ScrollContainer`, text fields,
panels). `RenderingServer` had none; [[22-ember-servers]] listed
`push_clip_rect` / `pop_clip_rect` as not implemented. jade-garden's help
modal already clipped by calling macroquad's GL scissor directly.

### Decision

- `push_clip_rect(rect)`, `pop_clip_rect()`, `clip_rect() -> Option<Rect>`
  on `RenderingServer` (required, no default: a backend that ignores
  clipping would silently draw outside scroll areas).
- Rects are **screen pixels**, top-left origin, **not** moved by the
  camera (UI is screen-space; GPU scissors are screen-space).
- `ember_servers::ClipStack` owns the rules for every backend: nested
  pushes intersect; a push outside the current clip clips everything;
  an extra pop is ignored; `begin_frame` clears the stack.
- Macroquad: `quad_gl.scissor(...)`, reset on `begin_frame` and on an
  unbalanced `end_frame`. `MockRenderer` records
  `PushClipRect { rect, effective }` / `PopClipRect`.

### Alternatives

- **Render to texture per scroll area:** correct but costly, and wasm
  render targets are fragile. Rejected.
- **Clip in `ember_ui` by skipping/cropping draws:** cannot crop text or
  sprites. Rejected.
- **World-space clip rects:** would mix UI with the camera. Rejected.

### Consequences

- The scissor is integer pixels (rect rounded outward). High-DPI scaling
  needs a visual check in the Phase 5 `ui_demo`.
- Same commit series fixed the camera sign: draws land at
  `world - camera_offset`, matching `DrawContext::world_to_screen`.

## ADR-032: Typed text, mouse wheel and editing keys in `Input`

**Status:** accepted (2026-11-22)

### Context

`TextInput` needs the characters the user typed, already resolved by the
keyboard layout (AZERTY, accents, shift); key codes cannot give that.
`ScrollContainer` needs the wheel. `Input` had neither, and `Key` had no
Delete/Home/End/PageUp/PageDown.

### Decision

- `Input.chars_typed: Vec<char>` — typed characters this frame, in
  order, control characters excluded (Backspace, Enter, Tab… stay in
  `keys_pressed`). Helper `typed_text()`.
- `Input.mouse_wheel: Vec2` — wheel movement this frame, `y > 0` = wheel
  up / away from the user. Helper `wheel_moved()`.
- `Key::{Delete, Home, End, PageUp, PageDown}`.
- The macroquad poller drains `get_char_pressed()` and reads
  `mouse_wheel()`.

### Alternatives

- **A separate `TextEvent` stream on `NodeContext`:** a second input
  path to learn. Rejected; one snapshot per frame stays the rule.
- **Raw IME composition support:** v0.2.

### Consequences

- Additive: fixtures use `..Default::default()` (ADR-018), so no game or
  test changes.

## ADR-033: Phase 4's macroquad-isolation exit moves to Phase 7

**Status:** accepted (2026-11-22)

### Context

[[40-roadmap]] Phase 4's exit criterion “`grep -rn "use macroquad"
crates/` returns only the backends” cannot hold until the games stop
using `ember-stdlib`'s macroquad modules, which is Phase 7–8 work. That
blocks Phase 5 (UI), whose entry criterion is “Phase 4 complete”.
`xtask/macroquad-allowlist.txt` allowed `games/` wholesale, so the check
showed no progress either.

### Decision

- Phase 4 is complete when **engine crates other than the legacy
  `ember-stdlib`** import no macroquad (true today) and the gates pass.
- The games half of the criterion is part of the **Phase 7** exit; the
  `ember-stdlib` half is part of **Phase 8**.
- The allowlist names the 17 games and 10 `ember-stdlib` modules
  individually; lines are only ever removed (one per migrated game).

### Alternatives

- **Block Phase 5 until all games migrate:** serialises the whole plan.
  Rejected.
- **Drop the criterion:** loses the wgpu-readiness guarantee. Rejected.

### Consequences

- Phase 4 closes; Phase 5 can start.
- The allowlist is the migration progress bar for Phase 7.

## ADR-034: How `ember_ui` controls live in the scene tree

**Status:** accepted (2026-11-22) — refines [[24-ember-ui]] §3

### Context

Implementing session 5.1 hit three constraints the frozen UI spec glosses
over: (1) the tree stores `Box<dyn Node>` and Rust cannot cast `dyn Node`
to `dyn Control`, yet a parent's layout must call `Control` methods on
its children; (2) `DrawContext` has no tree access by design (ADR-022),
so `draw` cannot reach a theme stored on the tree (spec §7.2's
`ctx.tree().set_ui_theme` / `ctx.theme()` do not exist); (3) the orphan
rule forbids `impl<T: Control> Node for T` in `ember-ui`.

### Decision

- **`ControlBase`** is embedded in every control (rect, anchors, offset,
  min size, visible, focused, hovered, `Arc<Theme>`). `Control` requires
  only `base`, `base_mut`, `draw_control`; `rect`, `set_rect`, `anchors`,
  `min_size`, `measure`, `content_rect` default from it.
- **`ControlRegistry`** maps a concrete `TypeId` to cast functions built
  on `Object::as_any`. It lives in `UiRoot`; built-in widgets are
  registered by `UiRoot::new`, custom ones with `register::<T>()`.
- **Layout and theme hand-down run in `UiRoot::process`** (one pass,
  `layout::layout_children`): parents arrange children through
  `Control::arrange_children` (containers) or the child's anchors.
- **`control_node!(T)`** writes the control's `impl Node`: `z = UI_Z`
  (100), `visible` from the base, `draw` → `draw_control` with a
  `UiDrawContext` built from the base's theme and focus/hover flags.
- `UiDrawContext::is_focused()` replaces `has_focus(id)` (the drawing
  control is the one asking). Clips pushed through it are popped on drop.
- `Theme::from_ron` is deferred: `Color` has no serde support yet.
- `ember-ui` may depend on `ember-core` and `ember-input` (lower layers).

### Alternatives

- **Add `as_control()` to `Node`:** puts a UI concept into `ember-scene`.
  Rejected.
- **UiRoot owns a private control tree (not scene nodes):** simplest, but
  drops signals, `NodeId`s and `.escene` support for UI. Rejected.
- **Theme lookup at draw time through the tree:** impossible without
  giving `DrawContext` tree access (ADR-022). Rejected.

### Consequences

- An unregistered control is invisible to layout (and its subtree is
  skipped); `UiRoot::new` registering built-ins keeps that rare.
- Controls draw through the normal tree pass, so z, visibility and
  draw order behave like any node.
- Open for 5.3/5.4: event handlers have no `NodeContext`, so signals are
  emitted by `UiRoot` after routing (ADR-036); a scroll clip
  must close after the children draw, which needs a post-children draw
  hook in `ember-scene` (ADR-035).

### Addendum — session 5.2 (2026-11-22)

- **Bottom-up measuring.** `Control::measure(available, &Measure)` sizes
  the control itself; the new `Control::measure_children(own, sizes)`
  lets containers add their children (measured first). Spec §4.3's
  "`measure` sums the children" is not possible from `&self`.
- **`Measure`** gives text metrics from the renderer; `UiRoot` gets tree
  and renderer together through `NodeContext::tree_and_render` (new,
  generic split borrow).
- **Padding is `Margins`** (per side), not a `Rect` as in the spec.
- **`Image` takes a `TextureId` + pixel size**, not `Handle<Texture>`:
  `ember-ui` does not depend on `ember-assets` (same layer).
- `UiRoot::new` registers every built-in control; the `ember` facade
  re-exports `ember_ui` and its prelude.

## ADR-035: Post-children draw hook and children clip for controls

**Status:** accepted (2026-11-23) — refines [[24-ember-ui]] §5.3, §5.4

### Context

`ScrollContainer` must clip its children's drawing to its rect. The
clip stack (ADR-031) is push / pop, but the tree draws a node, then its
children, with no call after the children (ADR-022): a node can open a
clip in `draw` and never close it. `TextInput` also needs typed
characters, and Space must type a space in a text field while it still
presses a focused button (ADR-036).

### Decision

- **`Node::draw_after_children(&self, &mut DrawContext)`**, default
  empty, in `ember-scene`. The tree calls it after a visible node's
  subtree has drawn, with the server's z set back to the node's `z()`.
  Hidden nodes skip it with their subtree.
- **On `Control`:** `clip_children() -> Option<Rect>` (default `None`)
  and `draw_overlay(&self, ui)` (default empty; named so it does not
  clash with the `Node` method). `control_node!` now writes both draw
  methods: `draw` = `draw_control`, then push the children clip on the
  raw server (`UiDrawContext` pops its own clips on drop);
  `draw_after_children` = pop that clip, then `draw_overlay`, which is
  unclipped (the scrollbar).
- **Routing honours clips:** the pointer reaches a control only inside
  the intersection of its ancestors' `clip_children`, so rows scrolled
  out of view cannot be hovered or clicked.
- **`Control::children_arranged(&mut self, rects)`** runs after layout
  placed a control's children. `ScrollContainer` reads its content
  height from it (layout callbacks are otherwise `&self`).
- **Text entry:** `Control::on_text_input(char)` receives
  `Input::chars_typed` without control characters, before the frame's
  keys (so "type then Backspace" erases the typed character).
  `Control::captures_text()` makes Space type instead of activate;
  Enter still activates (`TextSubmitted`).
- `ScrollContainer` is vertical only; wheel up (positive y) scrolls
  towards the top by 40 px per notch; content is laid out at the
  container width and each child's desired height.

### Alternatives

- **Clip inside `draw_control` around a manual child draw:** controls
  cannot draw their children (`DrawContext` has no tree, ADR-022).
  Rejected.
- **A `Clip` marker node closing the clip as last child:** fragile with
  dynamic children and z. Rejected.
- **Route `chars_typed` through `on_key_press`:** `Key` has no
  character values. Rejected.

### Consequences

- The hook is additive: existing nodes draw exactly as before, plus one
  extra `set_z` after a node with children.
- Clip rects are screen pixels; a scroll area under a camera transform
  is not supported (UI is screen space anyway).
- To check visually: Retina scissor scale and the wheel direction on
  macOS (natural scrolling) — `make ui-demo`.

## ADR-036: `UiRoot` routes input and emits control signals

**Status:** accepted (2026-11-22) — refines [[24-ember-ui]] §5.2, §6, §9

### Context

Session 5.3 adds `Button`, focus and mouse routing. The spec has a
button call `self.emit_clicked()` from `on_mouse_up`, but control input
handlers (`on_mouse_up(&mut self, …)`) receive no `NodeContext`, and a
control does not know its own `NodeId`. Spec §6.1 also gives
`FocusManager::tick(&NodeTree, &Input)`, which cannot call `on_focus` /
`on_blur` (they need `&mut`).

### Decision

- **Signal outbox on `ControlBase`.** `base.emit(|id| ButtonClicked {
  button: id })` queues a closure; `UiRoot` drains every control's
  outbox after routing and calls each with `(ctx, control_id)`, i.e.
  a normal `ctx.emit`. Handlers therefore run in the same frame, in tree
  order.
- **Signals carry the sender.** Signals are broadcast by type, so
  `ButtonClicked { button: NodeId }` and `FocusChanged { from, to }`
  name the control; handlers compare ids. (Spec §5.2 shows a unit
  `ButtonClicked`.)
- **`UiRoot` owns a `FocusManager`** and routes in `process`, after
  layout, in this order: hover (enter/leave) → mouse buttons → wheel →
  focus (`request_focus`, then left click, then Tab / Down / D-pad down
  = next, Up / D-pad up = previous, wrapping) → activation (Enter,
  Space or gamepad South call `Control::on_activate`) → other pressed
  keys to the focused control's `on_key_press`. Mouse comes before
  focus so a click can focus (spec §9.1 lists focus first).
- **No consumption in v0.1:** every visible control under the pointer
  gets mouse events (spec §9.1 item 4 already defers consumption).
- **Press tracking.** `ControlBase::pressed` is set by a left press on
  the control and cleared after the left release, which is delivered to
  the pressed control even if the pointer left it. `Button` clicks only
  on release inside *and* `pressed`.
- **Left / Right stay free** for the `TextInput` caret (5.4).
  `primary_action()` is not used for activation: it includes the mouse.
- `FocusManager::validate` drops focus when the focused control is
  hidden, disabled or removed.

### Alternatives

- **Give handlers a `NodeContext`:** the control is borrowed out of the
  tree while `UiRoot` walks it; a context would alias the tree.
  Rejected.
- **Per-control `pressed()` / `take_clicked()` polling by `UiRoot`:**
  hard-codes every widget's signals into the root. Rejected.
- **`emit_deferred`:** delays handlers to the end of the pass for no
  gain. Rejected.

### Consequences

- Custom controls emit any signal type with `base.emit`, no root change.
- No Shift+Tab until `Key` has Shift; Up is the backwards key.
- Handlers targeting `UiRoot` itself are skipped (it is checked out of
  the tree during `process`, ADR-022); connect to another node.

## ADR-037: Exported properties live on `NodeName`; scenes are written from live trees

**Status:** accepted (2026-11-24) — refines [[10-nodes-and-scenes]] §11, [[21-ember-scene]] §9, [[26-ember-macros]]; amends ADR-025, ADR-030

### Context

Phase 2 needs `#[export]` to generate property access and a writer that
saves a tree back to `.escene` (the editor, Phase 6, depends on both).
Since ADR-030 the user owns `impl Node`, so the derive cannot add
`set_property` there. `Node::serialize(&self, &mut SceneWriter)` was a
stub, and a `&dyn Node` cannot name its class (`Object::class_name` is
`Self: Sized`).

### Decision

- **Properties move to `NodeName`**, the half of a node the derive
  writes: `property_list() -> &'static [&'static str]`,
  `get_property(key) -> Option<Variant>`, `set_property(key, &Variant)
  -> bool`, all defaulted. `Node::set_property` and `Node::serialize`
  are removed. Hand-written nodes implement them on `NodeName` if they
  load or save properties.
- **`#[derive(Node)]` generates them** from `#[export]` fields
  (`f3eedef`). A field type must implement `ember_core::VariantValue` (new:
  `bool`, integer types with range checks, `f32`/`f64` — `Int` accepted
  for floats —, `String`, `Vec2`, `Color`, `Option<T>` (= `Nil`),
  `Vec<T>` (= `Array`)). `#[export(skip)]` is accepted; exporting
  `name` is a compile error; `#[export(default = …)]` is not in v0.1.
- **`ClassDB` maps Rust types back to class names**
  (`class_of(TypeId)`, `class_name_of(&dyn Object)`).
- **Writing (`d057b2c`):** `SceneWriter::new(uid)` + `write_node(NodeRecord)`
  / `write_ext_resource` / `write_connection` + `finish() ->
  PackedScene` (validated like a loaded file). `PackedScene::from_tree(
  tree, root, &ClassDB, uid)` saves a subtree: class, name, exported
  properties, `visible`, `z`. Unregistered node types fail with
  `SceneError::NotSaveable`. Nested `instance:` nodes are written
  inline; connections are not written.

### Alternatives

- **New supertrait `Properties` on `Node`:** every hand-written node and
  macro (`ember-ui`, pong, simon, tests) would need an empty impl.
  Rejected; `NodeName` already has to exist on each.
- **Derive generates inherent methods the user forwards to:** easy to
  forget, and scenes would silently ignore properties. Rejected.
- **`Node::class(&self)` on every node:** another hand-written method;
  the `ClassDB` already knows the types it can load. Rejected.

### Consequences

- `ClassDB::instantiate` + `.escene` properties + `from_tree` round-trip
  derived nodes with no hand-written property code
  (`tests/scene_writer.rs`).
- `Handle<T>` fields cannot be exported yet (need the `AssetServer` to
  resolve `uid://`); store the path as a `String`.
- `NodeName` is now a slight misnomer; it stays to avoid churn.

## ADR-038: Font and audio loaders; hot reload by polling

**Status:** accepted (2026-11-24) — refines [[13-assets]] §7–8; amends ADR-026

### Context

Phase 2 listed font and audio loaders and a hot-reload watcher. The
spec's watcher uses the `notify` crate on a background thread; the
engine is single-threaded and its GPU / audio handles must be recreated
on the main thread with the servers at hand.

### Decision (`a048f66`)

- **`Font`** (`.ttf`, `.otf`) → `RenderingServer::create_font`;
  **`AudioClip`** (`.wav`, `.ogg`) → `AudioServer::load_clip`.
  `AssetLoadContext` gains `audio`; `AssetServer` gains `load_audio`,
  `font`, `audio_clip`; `AssetError::NeedsAudio`. Reloads destroy the
  old font / clip.
- **Hot reload = polling:** each cache entry records the file's mtime
  and a type-erased reload function. `AssetServer::reload_changed(
  rendering, audio)` reloads every live asset whose file changed, in
  path order, and returns `(path, Result)` for each. A failed reload
  keeps the old value and is not retried until the file changes again.
  The game (or editor) calls it — every frame in debug, on a timer, or
  on a key.

### Alternatives

- **`notify` watcher thread:** new dependency, platform quirks, and
  events still have to be applied on the main thread. Rejected for
  v0.1; `reload_changed` can later be fed by one.
- **Automatic reload inside `SceneLoop`:** the loop does not own an
  `AssetServer` yet (`ctx.assets()` is not implemented). Deferred.

### Consequences

- One `stat` per live asset per call; fine for v0.1 asset counts.
- Code hot-reload stays out of scope (spec §10.3).

## ADR-039: How a game plugs into the editor

**Status:** accepted (2026-11-26) — M1.3 of [[Next Phase Plan — Migration Close & Editor v2]]

### Context

The Phase 6 editor only knew its own `Player` / `Sprite` classes, and it
was a binary. To open `pong.escene` it needs Pong's classes. Layers
(`check-layers`) forbid `ember-editor` from depending on games.

### Decision (`6aaf9a0`)

- **The editor is a library.** `ember_editor::run(EditorConfig)`, where
  `EditorConfig { classes: ClassDB, scene: PathBuf, viewport:
  Option<Vec2>, print: bool }`. `EditorConfig::from_args(classes)` parses
  `<scene> [--print]`; `with_viewport(w, h)` sets the game's resolution,
  which the preview frames.
- **The dependency points from the game to the editor.** A game that
  wants editing adds `ember-editor` as a **dev-dependency** and an
  `examples/edit.rs` of a few lines that passes its own
  `scene::classes()`. The game binary does not link the editor.
- **`make edit GAME=<game>`** runs `cargo run -p <game> --example edit --
  games/<game>/scenes/<game>.escene`. It fails with a message if the game
  has no `examples/edit.rs`.
- **It edits the game's scene in place**, not a scratch copy. A game
  scene is a source file tracked by git (`git diff` / `git checkout`
  undo it), and the proof needs `cargo run -p <game>` to play what was
  saved. `make editor` keeps its scratch copy, because it edits a
  **test fixture**.
- The `ember-editor` binary stays the demo: it calls `run` with
  `nodes::classes()`.

### Alternatives

- **A registry crate the editor depends on, listing every game's
  classes:** the editor would rebuild with every game, and the layering
  is still wrong. Rejected.
- **Dynamic loading (plugins as `cdylib`):** an unstable Rust ABI, and
  far more work than v0.1 needs. Rejected.
- **An `edit` feature on the game's binary:** it puts editor code into
  the game's build graph. The dev-only example keeps it out. Rejected.

### Consequences

- Each editable game costs one example and one dev-dependency.
- The editor cannot list "all games". It opens whichever game started it.
- The preview frames the resolution but does not scale it yet: Pong's
  800 × 600 is clipped by the panel until ADR-040 (M1.4).

## ADR-040: Camera transform (offset + zoom)

**Status:** accepted (2026-11-26) — M1.4 of [[Next Phase Plan — Migration Close & Editor v2]]; amends [[22-ember-servers]] and [[23-ember-render-macroquad]] §4

### Context

`RenderingServer::set_camera(offset)` could only scroll. The editor
preview must show Pong's 800 × 600 court inside a 540 × 660 panel, and
pagoda, taiji and ember-wars need a zoom to leave macroquad's own camera.

### Decision (`dfa8b04`)

- **`ember_servers::Camera2d { offset: Vec2, zoom: f32 }`.** A world
  point `p` lands at `(p - offset) * zoom`. It has `IDENTITY`,
  `at(offset)` (the old behaviour), `with_zoom`, `placing_origin_at(screen,
  zoom)`, `world_to_screen` / `screen_to_world`, `length`, `rect_to_screen`.
- **The trait takes the whole value:** `set_camera(Camera2d)` and
  `camera() -> Camera2d` replace `set_camera(Vec2)` / `camera_offset()`.
  Saving and restoring a camera is one value.
- **The backend maps every draw call.** Positions go through
  `world_to_screen`, and sizes, radii, thicknesses and font sizes are
  multiplied by `zoom`. **Clip rects stay in screen pixels**, as ADR-031
  says. `measure_text` stays in world units, so layout code does not
  change.
- `DrawContext::camera()` replaces `camera_offset()`;
  `world_to_screen` / `screen_to_world` use the zoom.
- **Editor:** `preview_camera(viewport)` scales the game's resolution
  down to fit the preview (never up) and centres it. Without a viewport
  the POC placement is unchanged. `Document::ready` runs the scene's
  `ready` on the preview's own silent servers (`MockAudio`, a separate
  `LinearPhysics`, no input).

### Alternatives

- **Add `set_camera_zoom` and keep `set_camera(offset)`:** no breaking
  change, but two calls to save and restore. Only the editor and three
  tests called the old API. Rejected.
- **macroquad's `Camera2D` matrix:** scissor rects and `measure_text`
  would then mix screen and world units. Rejected for v0.1.
- **Rotation:** no game needs it yet. Deferred.

### Consequences

- Text rounds its scaled size to whole pixels (`font_size: u16`), so
  zoomed text can be a pixel off in width.
- `ready` in the preview can rewrite a node's exported fields, which
  Save then writes. Pong's `ready` writes back the same values.
- ADR-041 (virtual resolution / letterbox) can build on `Camera2d`.

## ADR-041: Virtual resolution with letterbox

**Status:** accepted (2026-11-26) — M2 of [[Next Phase Plan — Migration Close & Editor v2]]; builds on ADR-040

### Context

Taiji and pagoda draw into a macroquad `render_target` of a fixed size,
then scale it into a resizable window with black bars. The engine had
no render targets, and `SceneLoop` never learned the window size.

### Decision (`ef68ce5`)

- **`SceneLoop::with_virtual_resolution(w, h)`**, or
  `set_virtual_resolution(Option<VirtualResolution { size, bars }>)`.
  Nodes draw on a `w × h` canvas, and `viewport()` is that canvas. Each
  frame the loop:
  1. clears to `bars` (black);
  2. sets `Camera2d::letterbox(window, size)`;
  3. clips to `Camera2d::letterbox_rect(window, size)`;
  4. draws the tree;
  5. restores the identity camera.
- **Input is mapped back:** in `set_input`, `mouse_pos` and every touch
  position go from window pixels to canvas coordinates. The wheel and
  keys are unchanged.
- **`MainLoop::resize(width, height)`**, a no-op by default. The
  macroquad runner calls it before the first frame and whenever
  `screen_width()` / `screen_height()` change. Without a virtual
  resolution, `SceneLoop` only records the size; viewport tracking is
  ADR-044's job.
- **No render target:** scaling is done per draw call through the
  camera (vector scaling). There is no `filter` option, because nothing
  is resampled.

### Alternatives

- **Render targets on `RenderingServer` (`create_render_target`, draw
  into it, blit):** this is what the games did, but it needs a new
  resource type on both backends and on the mock. Rejected while vector
  scaling is enough. It can be added later if a game needs pixel-art
  filtering.
- **Track the window inside `Input`:** the window size is not input.
  Rejected.

### Consequences

- Shapes and text stay sharp at any window size. Text rounds its scaled
  size to whole pixels.
- **A node must not call `set_clear_color`** under a virtual resolution:
  it would paint over the bars. Draw a full-canvas rect instead (taiji
  does).
- A node that sets its own camera replaces the letterbox. To scroll,
  build on `ctx.camera()`: `Camera2d { offset: base.offset + scroll,
  ..base }`.
- Clip rects pushed by nodes are in window pixels (ADR-031), so
  `ember_ui` clipping containers do not line up under a virtual
  resolution yet. No game needs both today.

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

- Architecture → [[01-architecture]]
- Invariants → [[02-principles]]
- Server trait contracts → [[12-servers]]
- Node/scene concepts → [[10-nodes-and-scenes]]
- Roadmap phases → [[40-roadmap]]
- Glossary → [[51-glossary]]
