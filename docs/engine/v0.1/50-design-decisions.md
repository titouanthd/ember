# 50 — Design Decisions (ADRs)

> **Status:** ⬜ draft
> **Audience:** all
> **Prerequisites:** [01-architecture.md](01-architecture.md), [02-principles.md](02-principles.md)

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

The workspace's 18 games include:

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
Left Click OR gamepad South. That's three checks per game, times 18
games, times every menu.

### Decision

**`Input::primary_action()`** returns true if any of {Space, Enter,
Left Click, Gamepad South} pressed this frame.

Also `secondary_action()` for Escape / Backspace / Gamepad East.

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

2. Number sequentially (`ADR-016`, etc.).
3. Update `docs/engine/v0.1/README.md` if the ADR is referenced by a
   doc.
4. Do not delete old ADRs — supersede them.

## Cross-references

- Architecture → [01-architecture.md](01-architecture.md)
- Invariants → [02-principles.md](02-principles.md)
- Roadmap phases → [40-roadmap.md](40-roadmap.md)
- Glossary → [51-glossary.md](51-glossary.md)