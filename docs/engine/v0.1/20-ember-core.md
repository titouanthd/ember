# 20 — `ember_core`

> **Status:** 🟡 partial — reviewed 2026-11-19; math, `Color`, `Cooldown`, `TickTimer`, hash, `Rng`, `Variant`, `ClassDB`, `MainLoop` exist with a smaller API than specced; `Stopwatch`, `Plugin`, prelude and several helpers are not implemented.
> **Audience:** implementers
> **Prerequisites:** [01-architecture.md](01-architecture.md), [02-principles.md](02-principles.md)

## 1. Purpose

Specify the API of `ember_core`: math types, time utilities, hashing,
random numbers, the `Variant` tagged union, and the `Object`/`ClassDB`
registry. Every other crate depends on `ember_core`; nothing in
`ember_core` depends on any other Ember crate.

> **Reading note.** The signatures below were reconciled with the code on
> 2026-11-19. Anything marked *Not yet implemented* is still planned; the
> shipped API is smaller and in places differs (e.g. `Aabb` is min/max,
> `Rng` is xorshift32).

## 2. Module layout

```
ember-core/src/
├── lib.rs          module declarations + re-exports (no prelude.rs)
├── error.rs        CoreError (Parse only)
├── math/
│   ├── mod.rs      re-exports Aabb, Circle, Rect
│   ├── vec.rs      `reflect` helper (Vec2 itself is glam::Vec2)
│   ├── rect.rs     Rect
│   ├── aabb.rs     Aabb
│   └── circle.rs   Circle
├── color.rs        Color
├── image.rs        Image (RGBA8 buffer)
├── time/
│   ├── mod.rs
│   ├── cooldown.rs Cooldown
│   └── tick.rs     TickTimer (fixed accumulator)
├── hash.rs         deterministic hash
├── rng.rs          seeded xorshift32 Rng
├── variant.rs      Variant
├── object.rs       Object trait
├── class_db.rs     ClassDB
├── main_loop.rs    MainLoop trait, FIXED_DT, MAX_FRAME_DT
├── app/            GameState (Start/Playing/LevelCleared/GameOver/Win)
└── io.rs           save_to_file / load_from_file (RON), IoError
```

*Not yet implemented:* `prelude.rs`, `plugin.rs`, `time/stopwatch.rs`.

## 3. Math

### 3.1 `Vec2`

`Vec2` is `glam::Vec2` used directly: there is no `ember_core::math::Vec2`
re-export, so crates depend on `glam` themselves. All the methods of the
original sketch come from glam (with glam names: `length_squared`,
`distance_squared`, `normalize_or_zero`, `perp_dot`, `lerp`, ...).
`glam::Vec2::normalize` does not panic on zero (it yields NaN), so the
"panics" policy of §3.5 does not apply.

`ember_core::math::reflect(v, n)` reflects `v` about the unit normal `n`
(glam has no `Vec2::reflect`).

### 3.2 `Rect`

```rust
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self;
    pub fn from_min_size(min: Vec2, size: Vec2) -> Self;
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self;

    pub fn min(&self) -> Vec2;
    pub fn max(&self) -> Vec2;
    pub fn center(&self) -> Vec2;
    pub fn size(&self) -> Vec2;

    pub fn contains(&self, p: Vec2) -> bool;              // edges inclusive
    pub fn intersects(&self, other: &Rect) -> bool;       // positive area
    pub fn intersect(&self, other: &Rect) -> Option<Rect>; // None if touching
    pub fn to_aabb(&self) -> Aabb;
    pub fn from_aabb(aabb: &Aabb) -> Self;
}
```

Methods take `&self` / `&Rect`. *Not yet implemented:* `Rect::ZERO`,
`from_corners`, `contains_strict`, `union`, `expand`, `translate`
(use `Rect::default()` for zero).

### 3.3 `Aabb`

Axis-aligned bounding box stored as **`min` / `max`** corners (not
center + half extent). `Rect` is the top-left + size spelling.

```rust
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb { pub min: Vec2, pub max: Vec2 }

impl Aabb {
    pub fn from_corners(a: Vec2, b: Vec2) -> Self;        // normalises
    pub fn from_center_half(center: Vec2, half: Vec2) -> Self;
    pub fn from_top_left_size(top_left: Vec2, size: Vec2) -> Self;

    pub fn center(&self) -> Vec2;
    pub fn size(&self) -> Vec2;
    pub fn half_size(&self) -> Vec2;
    pub fn width(&self) -> f32;
    pub fn height(&self) -> f32;

    pub fn contains(&self, point: Vec2) -> bool;          // edges inclusive
    pub fn contains_strict(&self, point: Vec2) -> bool;
    pub fn overlaps(&self, other: &Aabb) -> bool;         // edges inclusive
    pub fn overlaps_strict(&self, other: &Aabb) -> bool;
    /// `(axis, depth)`; axis is unit, from `other` towards `self`.
    pub fn penetration(&self, other: &Aabb) -> Option<(Vec2, f32)>;
    pub fn distance_to_point(&self, point: Vec2) -> f32;
    pub fn closest_point(&self, point: Vec2) -> Vec2;
}
```

Convert with `Rect::to_aabb` / `Rect::from_aabb`. `penetration` is strict
(touching edges give `None`) and returns an axis plus depth, not an MTD
vector.

### 3.4 `Circle`

```rust
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Circle { pub center: Vec2, pub radius: f32 }

impl Circle {
    pub fn new(center: Vec2, radius: f32) -> Self;
    pub fn diameter(&self) -> f32;
    pub fn area(&self) -> f32;
    pub fn circumference(&self) -> f32;
    pub fn contains(&self, point: Vec2) -> bool;
    pub fn contains_strict(&self, point: Vec2) -> bool;
    pub fn overlaps(&self, other: &Circle) -> bool;
    pub fn overlaps_aabb(&self, aabb: &Aabb) -> bool;
    pub fn closest_point_on_edge(&self, point: Vec2) -> Vec2;
}
```

### 3.5 Math panics policy

`ember_core`'s own math has no documented panic sites. Everything
returns `Option` / `Result` or a well-defined default (see
[02-principles I4](02-principles.md#i4--no-unwrapexpect-in-engine-crates)).

## 4. `Color`

```rust
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }

impl Color {
    pub const WHITE: Self;
    pub const BLACK: Self;
    pub const TRANSPARENT: Self;
    pub const RED: Self;
    pub const GREEN: Self;
    pub const BLUE: Self;
    pub const YELLOW: Self;

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self;
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self;      // a = 1.0
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self;

    pub fn with_alpha(self, a: f32) -> Self;
    pub fn to_array(self) -> [f32; 4];
    pub fn from_array(a: [f32; 4]) -> Self;
}
```

*Not yet implemented:* `CYAN`, `MAGENTA`, `from_hex`, `from_u8`, `lerp`.

Same field layout as `macroquad::prelude::Color`. The macroquad backend
converts field-by-field with internal helpers; there are no public `From`
impls (see [23-ember-render-macroquad.md](23-ember-render-macroquad.md)).

`ember_core::Image` (RGBA8 buffer: `new`, `from_rgba8`, `width`,
`height`, `as_bytes`, `set_pixel`) is the pixel type passed to
`RenderingServer::create_texture`.

## 5. Time

### 5.1 `Cooldown`

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Cooldown { remaining: f32, duration: f32 }

impl Cooldown {
    /// Running: `remaining == duration`.
    pub fn running(duration: f32) -> Self;
    /// Ready now, remembers `duration` for `reset()`.
    pub fn ready(duration: f32) -> Self;
    #[deprecated] pub fn new(duration: f32) -> Self;           // = running
    #[deprecated] pub fn with_duration(duration: f32) -> Self; // = ready
    /// Running with `remaining` left; duration is extended if smaller.
    pub fn from_remaining(remaining: f32, duration: f32) -> Self;

    pub fn duration(&self) -> f32;
    pub fn remaining(&self) -> f32;
    pub fn is_ready(&self) -> bool;
    pub fn is_active(&self) -> bool;
    pub fn fraction(&self) -> f32;            // elapsed, 0..1; 1.0 when ready
    pub fn remaining_fraction(&self) -> f32;  // 1..0

    pub fn tick(&mut self, dt: f32);           // clamps at 0; dt <= 0 ignored
    pub fn tick_returning(&mut self, dt: f32) -> bool;  // true on the expiring tick
    pub fn trigger(&mut self, duration: f32);  // restart with new duration
    pub fn reset(&mut self);                   // restart with stored duration
    pub fn clear(&mut self);                   // force ready
}
```

> **Implemented (ADR-011).** `running(d)` / `ready(d)` are the constructors;
> `new` and `with_duration` are `#[deprecated]` aliases to be removed in
> v0.2. The no-arg `ready()` is gone (use `Cooldown::default()`).
> `from_remaining` takes `(remaining, duration)`. `Cooldown` derives `Copy`
> and serde.

| Constructor | Initial `is_ready()` |
|---|---|
| `running(d)` | `d <= 0.0` |
| `ready(d)` | `true` |
| `default()` | `true` |

**Negative dt:** `tick(-1.0)` is a no-op.

**Fixed-step safety:** because `Cooldown::tick` clamps at zero,
calling it with `1/60` in `physics_process` produces identical behaviour
across frame rates.

### 5.2 `TickTimer` — fixed-step accumulator

The type is named `TickTimer` (not `Tick`).

```rust
#[derive(Debug, Clone)]
pub struct TickTimer { /* accumulator, tick_duration */ }

impl TickTimer {
    pub const MAX_TICKS_PER_ADVANCE: u32 = 8;

    pub fn new(tick_duration: f32) -> Self;      // clamped to >= 1e-6
    pub fn duration(&self) -> f32;
    pub fn set_duration(&mut self, duration: f32);

    /// Advances and returns whole ticks elapsed, capped at
    /// `MAX_TICKS_PER_ADVANCE` (surplus dropped). Negative dt → 0.
    pub fn advance(&mut self, dt: f32) -> u32;

    pub fn progress(&self) -> f32;     // 0..1 within the current tick
    pub fn accumulator(&self) -> f32;
    pub fn reset(&mut self);
}
```

### 5.3 `Stopwatch`

*Not yet implemented.* Planned API:

```rust
pub struct Stopwatch { /* elapsed, running */ }
// new, start, stop, reset, tick(dt), elapsed(), is_running()
```

### 5.4 `MainLoop` and fixed step

`ember_core::main_loop` defines the `MainLoop` trait (`init`, `process`,
`physics_process`, `draw(&mut self)`, `finalize`, see
[14-main-loop.md](14-main-loop.md)) and the constants `FIXED_DT = 1/60` and
`MAX_FRAME_DT = 0.25`.

## 6. Hashing

```rust
// ember_core::hash (re-exported at the crate root)

/// MurmurHash3-finalizer based; `(seed, x)` -> 32 bits.
pub fn hash_u32(seed: u32, x: u32) -> u32;
pub fn hash_u32_2(seed: u32, x: u32, y: u32) -> u32;
pub fn hash_u32_3(seed: u32, x: u32, y: u32, z: u32) -> u32;
```

*Not yet implemented:* `hash_2d`, `hash_3d`, `hash_with_seed`,
`hash_to_f32` (use `Rng` for floats).

**Contract:** every function is deterministic for the same input (no
seeding by OS entropy), produces well-distributed output (avalanche test
passes), and is safe for zero input.

## 7. RNG

### 7.1 `Rng`

An **xorshift32** generator (single `u32` state), not xoshiro256**.

```rust
#[derive(Clone, Debug)]
pub struct Rng { state: u32 }

impl Rng {
    pub const ZERO_SEED_FALLBACK: u32 = 0xDEAD_BEEF;

    pub fn new(seed: u32) -> Self;             // 0 -> fallback
    pub fn from_state(state: u32) -> Self;
    pub fn state(&self) -> u32;
    pub fn set_state(&mut self, state: u32);   // 0 -> fallback

    pub fn next_u32(&mut self) -> u32;
    pub fn next_f32(&mut self) -> f32;                       // [0, 1)
    pub fn next_range(&mut self, n: usize) -> usize;         // [0, n), 0 if n == 0
    pub fn next_f32_range(&mut self, min: f32, max: f32) -> f32;
    pub fn next_bool(&mut self) -> bool;
}
```

*Not yet implemented:* `next_u64`/`next_f64`, `range_*`, `choice`,
`shuffle`, `seed()`, 64-bit/xoshiro state.

**Contract:**
- Same seed → same sequence.
- Seed/state `0` does not lock up (replaced by `ZERO_SEED_FALLBACK`).
- `next_f32()` is in `[0, 1)`, never `1.0`.

### 7.2 Free helpers

Bridges for code that stores the RNG state as a bare `u32`:

```rust
pub fn rand01(state: &mut u32) -> f32;
pub fn rand_usize(state: &mut u32, n: usize) -> usize;
```

They advance `*state` in place (they are not index-driven).

## 8. `Variant`

A tagged union for dynamic values. Used at the `Object`/`ClassDB` and
`.escene` serialization boundaries only. Game code should prefer typed
structs.

```rust
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Variant {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2(Vec2),
    Color(Color),
    Array(Vec<Variant>),
    Map(BTreeMap<String, Variant>),
    ObjectRef(String),   // uid://...
}

impl Variant {
    pub fn nil() -> Self;
    pub fn as_bool(&self) -> Option<bool>;
    pub fn as_i64(&self) -> Option<i64>;
    pub fn as_f64(&self) -> Option<f64>;
    pub fn as_str(&self) -> Option<&str>;
    pub fn as_vec2(&self) -> Option<Vec2>;
    pub fn as_color(&self) -> Option<Color>;

    pub fn type_name(&self) -> &'static str;
    pub fn from_ron(s: &str) -> Result<Self, CoreError>;
    pub fn to_ron(&self) -> String;
}
```

RON syntax is tuple-style: `Float(1.0)`, `Vec2(1.0, 2.0)`,
`Color(r, g, b, a)`. `from_ron` errors are `CoreError::Parse`.

**Why not `serde_json::Value`:** because `Variant` is `Send + Sync +
'static`, has a fixed set of primitives that maps to `Object` properties,
and round-trips through RON (which the scene format uses).

## 9. `Object` and `ClassDB`

### 9.1 `Object` trait

```rust
pub trait Object: Send + Sync + 'static {
    fn class_name() -> &'static str where Self: Sized;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

Every `Node` implements `Object`. Non-node objects (future `Resource`
types) can also implement `Object`.

### 9.2 `ClassDB`

A plain value (no global, thread-local or `static mut` state) mapping
class names to constructors. Games create one and pass it by reference.
`#[derive(Node)]` does **not** generate a `register` function; games call
`register::<T>()` / `register_node::<T>()` explicitly (ADR-019, ADR-024).

```rust
#[derive(Default)]
pub struct ClassDB { /* BTreeMap<&'static str, Entry> */ }

impl ClassDB {
    pub fn new() -> Self;

    /// Register under `T::class_name()`. Idempotent: first registration wins.
    pub fn register<T: Object + Default>(&mut self);
    pub fn register_with<T: Object>(&mut self, ctor: fn() -> T);
    pub fn is_registered(&self, class_name: &str) -> bool;

    /// Fresh instance by name; `None` if unregistered. Takes `&self`.
    pub fn instantiate(&self, class_name: &str) -> Option<Box<dyn Object>>;

    /// Registered class names, sorted.
    pub fn names(&self) -> Vec<&'static str>;

    /// Per-class extension values keyed by type (ADR-024).
    pub fn set_extension<E: Any + Send + Sync>(&mut self, class_name: &str, ext: E) -> bool;
    pub fn extension<E: Any + Send + Sync>(&self, class_name: &str) -> Option<&E>;
}
```

`ember_scene::ClassDbNodes` adds `register_node`, `register_node_with`
and `instantiate_node` for `Box<dyn Node>` (see
[21-ember-scene.md](21-ember-scene.md)).

**Threading model:** no global mutable state (see
[02-principles I3](02-principles.md#i3--no-global-mutable-state)).

## 10. `Plugin` trait

*Not yet implemented.* There is no `Plugin` trait in `ember-core` (it
would need `AssetServer` and `Node`, which live in higher layers). Games
currently build a `ClassDB` and a `NodeTree` in `main` and hand a
`SceneLoop` to `run_window`. The planned shape:

```rust
pub trait Plugin: Sized + 'static {
    fn build(class_db: &mut ClassDB, assets: &mut AssetServer) -> Self;
    fn root(&mut self) -> Box<dyn Node>;
    fn shutdown(&mut self) {}
}
```

## 11. Error type

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoreError {
    Parse(String),
}
```

Implements `Display` and `std::error::Error` by hand (no `thiserror`).
*Not yet implemented:* `OutOfRange`, `TypeMismatch`, `ClassNotRegistered`.

`ember_core::io` also provides `save_to_file` / `load_from_file` (RON)
with its own `IoError`.

## 12. Prelude

*Not yet implemented.* `ember_core` has no `prelude` module; it
re-exports its main types at the crate root: `ClassDB`, `Color`,
`CoreError`, `GameState`, `Image`, `MainLoop`, `Object`, `Rng`,
`Variant`, `Aabb`, `Circle`, `reflect`, `Cooldown`, `TickTimer`,
`hash_u32{,_2,_3}`. The `ember` facade crate provides the game-facing
prelude.

## 13. Design notes

### 13.1 Why `Vec2` is glam's, not a newtype

Every game already uses `glam::Vec2` extensively. A newtype would require
`From`/`Into` conversions everywhere, so v0.1 uses `glam::Vec2` directly.
`ember_core` does not re-export it yet, so a future swap is not a
one-line change.

### 13.2 Why `Cooldown` was renamed, not patched

The former `Cooldown::new` semantics (starts active) were *correct* for
what the function does — it's the *name* that's wrong. A programmer
who reads `Cooldown::new(0.5)` thinks "a fresh cooldown of 0.5s" and
expects `is_ready() == true`. Renaming to `running` / `ready` makes the
semantics impossible to misread. The 31-test `ember-wars` bug is the
proof.

### 13.3 Why `Variant` is `#[non_exhaustive]`

Because it will grow (`Nil` might get a companion `Undefined`,
`ObjectRef` might become a `NodeRef` variant in v0.2). `#[non_exhaustive]`
prevents downstream exhaustive matches from breaking on every addition.

## 14. Cross-references

- Invariants that constrain this crate → [02-principles.md §2](02-principles.md#2-invariants)
- Scene layer that uses these types → [21-ember-scene.md](21-ember-scene.md)
- `.escene` format that uses `Variant` → [30-escene-format.md](30-escene-format.md)
- `#[derive(Node)]` macro → [26-ember-macros.md](26-ember-macros.md)