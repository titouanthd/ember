---
title: "02 — Principles"
aliases: ["02 — Principles"]
tags: ["ember/spec", "ember/wave-1"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/02-principles.md"
imported: "2026-11-22"
wave: "1 — Foundation"
---

# 02 — Principles

> **Status:** 🟡 partial — reviewed 2026-11-19; rules are policy and mostly followed; error enums, prelude and profiling differ from the sketches
> **Audience:** implementers, reviewers
> **Prerequisites:** [[01-architecture]]

## 1. Purpose

This is the rulebook. Eight invariants, a naming convention, an error
strategy, a testing philosophy, a deprecation policy. Every PR is reviewed
against this file.

It is deliberately short. If you need to break a rule, the answer is not
"add an exception" — the answer is an ADR in
[[50-design-decisions]] that either changes the
rule or explains why this is a one-off.

## 2. Invariants

These are non-negotiable. A PR that violates one of them is rejected, no
matter how useful the feature.

### I1 — Dependency direction is strictly downward

A crate may import from a strictly lower layer only. No exceptions. See
[[01-architecture#3. Crate map and dependencies|01-architecture.md §3]].

**Violated by:** importing `ember_scene` from `ember_servers` to "reuse a
small helper". Helpers go down, never up.

### I2 — Only the `_macroquad` crates depend on `macroquad`

`ember_render_macroquad` and `ember_audio_macroquad` are the *only* engine
crates allowed to import `macroquad` / `miniquad`. Every other crate talks
through `RenderingServer` / `AudioServer` traits. *Current exception:*
`ember_stdlib` (and the games) still depend on `macroquad`; `xtask
check-macroquad` keeps them on an allowlist until they migrate.

**Violated by:** "just one `draw_rectangle` call in `ember_scene`". Use
`ctx.render().draw_rect(...)` instead.

### I3 — No global mutable state

No `static mut`, no `lazy_static!` for anything mutated at runtime, no
`thread_local!` for game state. All mutable state lives inside a struct
that is threaded through `&mut self` or `&mut NodeContext`.

**Violated by:** a `static CACHE: Mutex<...>` added "for performance" —
profiling comes first, and if a cache is truly needed it belongs to
`AssetServer` or `RenderingServer`, not to a free-standing global.

### I4 — No `unwrap()` / `expect()` in engine crates

Engine crates (`ember_core`, `ember_scene`, `ember_servers`,
`ember_assets`, `ember_ui`, `ember_input`) must not call `unwrap()`,
`expect()`, or `panic!` on inputs that come from the user (game code,
asset files, scene files). Tests may unwrap freely.

The four legitimate panic sites are documented in each crate's `error.rs`:

- Programming errors (slice index out of bounds from internal bug) — use `debug_assert!`
- Impossible states (exhaustive match on a non-`#[non_exhaustive]` enum) — `unreachable!()`
- Startup invariant violations (config file format) — return `Err`, don't panic
- `#[test]` functions — `unwrap()` is fine

**Violated by:** `let tex = assets.load("player.png").unwrap()` inside an
engine function — return `Result<TextureId, AssetError>`.

### I5 — Every public API has a documented contract

Every `pub fn`, `pub struct`, `pub trait`, and `pub enum` must have a doc
comment stating: what it does, what it returns, what it panics on (usually
"never"), what it assumes (usually "nothing"), and one example.

The doc-comment is the spec. If the implementation drifts, one of the two
is wrong; the PR reviewer decides which.

**Violated by:** `pub fn do_thing() { ... }` with no docs.

### I6 — Public types are `#[non_exhaustive]` until v1.0

Every public enum and struct that is expected to grow (server capability
flags, `GameState`, config structs) is `#[non_exhaustive]`. *Partially
applied:* `CoreError`, `ServerError` and `AssetError` are; `SceneError` is not. Exception: fixture structs games construct, e.g. `Input` (ADR-018). Add fields via
new constructors, not by growing the struct.

**Violated by:** adding a fifth field to `GameConfig` and breaking every
caller's struct literal. Use a builder or `Default`.

### I7 — Deprecations get one minor version of grace

An API is marked `#[deprecated(note = "…")]` for one minor version
(e.g. all of v0.1), then removed in the next (v0.2). Deprecation without
a note explaining the replacement is rejected in review.

**Example (applied):** `Cooldown::new` → `Cooldown::running` is a v0.1
deprecation; `new` and `with_duration` carry `#[deprecated]` attributes.

### I8 — Tests are colocated and always run

- Unit tests live in the same file as the code they test, in a
  `#[cfg(test)] mod tests { ... }` block.
- Integration tests live in `crates/<crate>/tests/`.
- A PR that adds a `pub fn` without at least one test for it is rejected.

**Violated by:** a new trait method with "TODO: test later".

## 3. Naming conventions

### 3.1 Type names

| Suffix | Meaning | Example |
|---|---|---|
| `Server` | Trait defining a subsystem | `RenderingServer`, `AudioServer` |
| `Node` | Trait or struct implementing tree node | `SpriteNode`, `Control` |
| `Context` | Borrowed state injected during a call | `NodeContext`, `DrawContext` |
| `Handle<T>` | Owned reference to a loaded asset | `Handle<Texture>`, `Handle<AudioClip>` |
| `Id` | Opaque index into a slotmap | `NodeId`, `TextureId` |
| `Tree` | Ownership container | `NodeTree` |
| `Scene` | Serialisable subtree | `PackedScene` |
| `Config` | Immutable tuning parameters | `GameConfig`, `PhysicsConfig` |
| `State` | Mutable per-run state | `AppState`, `MatchState` |
| `Event` | Signal payload or input event | `HitEvent`, `KeyEvent` |
| `Progress` | Persisted cross-run data | `Progress` |

### 3.2 Function names

| Pattern | Rule | Example |
|---|---|---|
| Constructor | `new` / `with_*` | `Cooldown::running`, `Cooldown::ready` |
| Fallible | `try_*` | `try_spawn`, `try_fire` |
| Query | `is_*` / `has_*` | `is_ready`, `has_child` |
| Conversion | `as_*` (cheap) / `to_*` (allocating) | `as_str`, `to_owned` |
| Iteration | `iter*` | `iter_children` |
| Lifecycle (Node) | `ready`, `process`, `physics_process`, `draw`, `finalize` | fixed — do not rename |

### 3.3 Crate names

- `ember_core` — no suffix
- `ember_scene` — no suffix
- `ember_servers` — plural: defines the set of servers
- `ember_render_macroquad` — `<feature>_<backend>`
- `ember_audio_macroquad` — `<feature>_<backend>`
- Any future backend follows the same `<feature>_<backend>` rule:
  `ember_render_wgpu`, `ember_audio_cpal`, etc.

### 3.4 File names

- One file per public type (or one file per cohesive module)
- Snake_case file names, matching the module name
- `mod.rs` re-exports only — no logic
- `prelude.rs` for the crate's prelude

## 4. Module structure

### 4.1 Standard crate layout

```
crates/ember_x/
├── Cargo.toml
├── README.md           (one-paragraph purpose + link to spec)
├── src/
│   ├── lib.rs          (public API surface, re-exports only)
│   ├── prelude.rs      (curated re-exports for game authors)
│   ├── error.rs        (crate's error enum)
│   └── <module>.rs     (one per concept)
└── tests/
    └── <scenario>.rs   (integration tests)
```

### 4.2 `lib.rs` rules

- Only `pub mod`, `pub use`, and doc comments
- No logic
- No `#[macro_use]` (use `pub use` from `ember_macros` explicitly)

### 4.3 `prelude.rs` rules

Contains the minimum set of types a game needs to start:

```rust
// crates/ember/src/lib.rs (mod prelude) — abridged
pub use ember_scene::prelude::*; // Node, NodeId, NodeTree, NodeContext, DrawContext, Servers, PackedScene, SceneLoop, Signal…
pub use ember_core::math::Rect;
pub use ember_core::{Color, MainLoop, Object};
pub use ember_macros::Node; // derive (`#[signal]` is not re-exported)
pub use glam::Vec2;
```

`Cooldown` is not in the prelude and `#[signal]` (a no-op marker in `ember_macros`) is not re-exported.

The prelude is not the whole crate. Advanced APIs are used with their
full path (`ember_scene::advanced::...`).

## 5. Error handling

### 5.1 Strategy per layer

| Layer | Error strategy |
|---|---|
| L1 · Core | Typed `CoreError` enum; no `anyhow` |
| L2 · Servers | Trait methods return `Result<_, ServerError>`; `ServerError` is a coarse enum |
| L3 · Scene / Assets / UI | Typed enums per crate (`SceneError`, `AssetError`; `UiError` *not yet implemented*) |
| L4 · Main / Editor | May use `anyhow::Result` for the top-level entry point only |

### 5.2 Error crate

Each crate defines its own error in `error.rs`, hand-written (`impl Display` +
`impl std::error::Error`); the workspace does not use `thiserror`:

```rust
// crates/ember-scene/src/error.rs (abridged)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    NotFound(NodeId),
    SelfParent(NodeId),
    IsRoot(NodeId),
    AlreadyAttached(NodeId),
    NotAttached(NodeId),
    Cycle { child: NodeId, parent: NodeId },
    Parse(String),
    UnsupportedFormat(u32),
    ClassNotRegistered(String),
    InvalidName(String),
    Io(String),
    Asset(String),
    InstanceCycle(String),
}
```

### 5.3 Error chaining

Errors are meant to be `#[from]`-chained, never flattened to `String`.
*Not fully applied:* `SceneError::Io`/`Asset` and `AssetError::Io` carry a
`String`. The
goal is that a failing call site can always report *which crate, which
concept, which value* caused the failure.

### 5.4 What is never an error

The following are **never** errors in Ember, and are silently handled:

- Looking up a freed `NodeId` with `NodeTree::get` → `None`
- Disconnecting a signal that isn't connected → no-op
- Drawing outside the viewport → clipped, no warning
- Ticking a `Cooldown` that's already ready → no-op
- Playing a sound that finished → no-op

This rule is what keeps game loops clean. The alternative — a `Result`
for every "node is gone" check — pollutes every system function with
`if let Some(...)`.

### 5.5 What is always an error

- Referencing a freed `NodeId` from outside the tree
- Loading a scene file with a missing `ext_resource`
- Attaching a node that already has a parent (unless detached first)
- A `#[signal]` payload type that isn't `Send + Sync + 'static` (*`#[signal]` is a no-op marker; the bound is enforced by the `Signal` trait*)

These surface at the boundary, not inside the game loop.

## 6. Testing philosophy

### 6.1 Three layers of tests

| Layer | What | Where | Speed |
|---|---|---|---|
| Unit | One `pub fn` | `#[cfg(test)] mod tests` in the same file | ms |
| Integration | One crate's public API | `crates/<crate>/tests/` | 10–100 ms |
| Conformance | A real game migrated to the engine | `games/<game>/tests/` | seconds |

### 6.2 Rules

1. **Unit tests are colocated.** Never put `#[test]` in a separate file
   from the code it tests unless it needs a real network or file system.
2. **Every `pub fn` has at least one unit test.** Enforced in review, not
   by tooling (yet).
3. **Every crate has an integration test.** Even trivial crates get one
   that exercises a plausible use case.
4. **Every game has a `game_scenarios.rs`.** These are the conformance
   suite. When engine internals change, they must still pass.
5. **Scene tests use `MockRenderer` / `MockAudio`** (recording backends in
   `ember_servers`) rather than a headless macroquad backend.
6. **No mocking of `NodeTree`.** If a test needs a tree, build one. The
   tree is fast enough that faking it is wasted effort.

### 6.3 What we do not test

- Rendering pixel-by-pixel (visual regression is a v0.2 feature)
- Audio output (no audible assertions in CI)
- OS window management (macroquad handles it; we trust it)
- Performance regression (no benchmark harness in v0.1)

## 7. Deprecation policy

### 7.1 How to deprecate

*(Applied to `Cooldown::new` / `with_duration` in `ember-core`.)*

```rust
#[deprecated(
    since = "0.1.0",
    note = "use `Cooldown::running(d)` — `new` will be removed in 0.2.0"
)]
pub fn new(d: f32) -> Self { Self::running(d) }
```

Three requirements:

1. `since` — the version the deprecation was introduced
2. `note` — what to use instead, and when the old one will be removed
3. The deprecated function must be **implemented in terms of the
   replacement**, not the other way around

### 7.2 How to remove

- The removal lands in the minor version named in the `note`
  (v0.1 deprecations removed in v0.2.0)
- The removal PR deletes the deprecated function, updates every call site
  in the workspace, and updates the changelog
- The removal PR is its own commit, not bundled with new features

### 7.3 Exception: critical bug deprecations

If a deprecated API is found to be a security or soundness bug (e.g., a
memory-unsafety footgun), it can be removed in the same minor version
with a note in `50-design-decisions.md`. This has not happened in v0.1.

## 8. Performance philosophy

The POC is not performance-critical. The following rules apply:

1. **Correctness before speed.** An O(n²) collision check on 100 nodes is
   fine until profiling says otherwise.
2. **Profile before optimizing.** No speculative `unsafe`, no speculative
   caching.
3. **Profile with real data.** A 60-node scene is the baseline; benchmark
   against a real level, not a synthetic one.
4. **`unsafe` requires justification.** Every `unsafe` block has a
   `// SAFETY:` comment explaining the invariant.
5. **No allocation in hot paths.** `NodeTree::process` should not allocate
   per-node. If it does, either pre-allocate or use a scratch buffer.

Profiling infrastructure:

- `ember_profiling` — *not yet implemented (no such crate)*; planned as a macro crate providing `profile_scope!()` and
  `profile_frame!()`. No-op by default; enabled with a feature flag.
- Profiler UI is v0.2+. For POC, `profile_scope!` writes to
  `tracing` if the `tracing` feature is enabled.

## 9. Documentation philosophy

Every crate has:

1. **`README.md`** — one paragraph, link to spec
2. **Rustdoc on every `pub` item** — for `cargo doc` readers
3. **A section in `docs/engine/v0.1/`** — for architecture readers

These three are different readers. They should not be copy-pasted from
each other. Rustdoc explains *usage*; the spec explains *design*.

### 9.1 Rustdoc conventions

- `///` for items, `//!` for modules
- Examples marked `# Examples` with a runnable snippet
- `# Panics` section if the function can panic (it usually shouldn't)
- `# Errors` section on `Result`-returning functions
- `# Safety` section on `unsafe` functions

### 9.2 Spec conventions

The docs in this folder follow the skeleton in each file's header.
Updates to a `✅` spec file require an ADR in
[[50-design-decisions]].

## 10. Cross-references

- Layering → [[01-architecture]]
- Roadmap gates → [[40-roadmap]]
- Testing matrix → [[43-testing-strategy]]
- Decision log → [[50-design-decisions]]
- Glossary → [[51-glossary]]
