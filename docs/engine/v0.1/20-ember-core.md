# 20 — `ember_core`

> **Status:** ⬜ draft
> **Audience:** implementers
> **Prerequisites:** [01-architecture.md](01-architecture.md), [02-principles.md](02-principles.md)

## 1. Purpose

Specify the API of `ember_core`: math types, time utilities, hashing,
random numbers, the `Variant` tagged union, and the `Object`/`ClassDB`
registry. Every other crate depends on `ember_core`; nothing in
`ember_core` depends on any other Ember crate.

## 2. Module layout

```
ember_core/
├── Cargo.toml
├── src/
│   ├── lib.rs          re-exports only
│   ├── prelude.rs      curated re-exports
│   ├── error.rs        CoreError
│   ├── math/
│   │   ├── mod.rs
│   │   ├── vec.rs      Vec2
│   │   ├── rect.rs     Rect
│   │   ├── aabb.rs     Aabb
│   │   └── circle.rs   Circle
│   ├── color.rs        Color
│   ├── time/
│   │   ├── mod.rs
│   │   ├── cooldown.rs Cooldown
│   │   ├── tick.rs     Tick (fixed accumulator)
│   │   └── stopwatch.rs
│   ├── hash.rs         deterministic hash
│   ├── rng.rs          seeded xoshiro
│   ├── variant.rs      Variant
│   ├── class_db.rs     Object, ClassDB
│   └── plugin.rs       Plugin trait
```

## 3. Math

### 3.1 `Vec2`

```rust
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vec2 { pub x: f32, pub y: f32 }

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    pub const fn new(x: f32, y: f32) -> Self;
    pub const fn splat(v: f32) -> Self;

    pub fn length_sq(self) -> f32;
    pub fn length(self) -> f32;
    pub fn distance(self, other: Self) -> f32;
    pub fn distance_sq(self, other: Self) -> f32;
    pub fn dot(self, other: Self) -> f32;
    pub fn cross(self, other: Self) -> f32;   // z-component of 3D cross
    pub fn normalized(self) -> Self;           // panics if zero (see §3.5)
    pub fn normalize_or_zero(self) -> Self;    // safe for zero vectors
    pub fn rotate(self, radians: f32) -> Self;
    pub fn angle(self) -> f32;                 // radians, -π..π
    pub fn lerp(self, other: Self, t: f32) -> Self;
    pub fn clamp_length(self, max: f32) -> Self;
}
```

**Operators:** `Add`, `Sub`, `Mul<f32>`, `Div<f32>`, `Neg`, `AddAssign`,
`SubAssign`, `MulAssign<f32>`, `Index<usize>` (read-only).

**Note:** `glam::Vec2` is re-exported under the hood. The `ember_core::math::Vec2`
type is a newtype or a direct re-export — decision in
[50-design-decisions.md](50-design-decisions.md) (currently direct
re-export via `pub use glam::Vec2`).

### 3.2 `Rect`

```rust
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

impl Rect {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };

    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self;
    pub const fn from_min_size(min: Vec2, size: Vec2) -> Self;
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self;
    pub fn from_corners(a: Vec2, b: Vec2) -> Self;   // normalises

    pub fn min(self) -> Vec2;
    pub fn max(self) -> Vec2;
    pub fn center(self) -> Vec2;
    pub fn size(self) -> Vec2;

    pub fn contains(self, point: Vec2) -> bool;      // edges inclusive
    pub fn contains_strict(self, point: Vec2) -> bool; // edges exclusive
    pub fn overlaps(self, other: Rect) -> bool;      // edges inclusive
    pub fn intersect(self, other: Rect) -> Option<Rect>;
    pub fn union(self, other: Rect) -> Rect;
    pub fn expand(self, amount: f32) -> Rect;
    pub fn translate(self, offset: Vec2) -> Rect;
}
```

### 3.3 `Aabb`

Axis-aligned bounding box, stored as **center + half_extent** internally
(distinct from `Rect` which stores top-left + size).

```rust
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb { pub center: Vec2, pub half_extent: Vec2 }

impl Aabb {
    pub fn new(center: Vec2, half_extent: Vec2) -> Self;
    pub fn from_corners(a: Vec2, b: Vec2) -> Self;
    pub fn from_rect(r: Rect) -> Self;
    pub fn to_rect(self) -> Rect;

    pub fn min(self) -> Vec2;
    pub fn max(self) -> Vec2;
    pub fn size(self) -> Vec2;
    pub fn contains(self, point: Vec2) -> bool;
    pub fn overlaps(self, other: Aabb) -> bool;
    pub fn closest_point(self, point: Vec2) -> Vec2;
    pub fn penetration(self, other: Aabb) -> Option<Vec2>;  // MTD vector
}
```

### 3.4 `Circle`

```rust
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Circle { pub center: Vec2, pub radius: f32 }

impl Circle {
    pub fn new(center: Vec2, radius: f32) -> Self;
    pub fn contains(self, point: Vec2) -> bool;
    pub fn overlaps(self, other: Circle) -> bool;
    pub fn overlaps_aabb(self, aabb: Aabb) -> bool;
    pub fn closest_point_on_edge(self, from: Vec2) -> Vec2;
}
```

### 3.5 Math panics policy

Functions documented with `# Panics` are the only permitted panic sites
in `ember_core` (see [02-principles I4](02-principles.md#i4--no-unwrapexpect-in-engine-crates)):

- `Vec2::normalized` — panics if `length_sq() == 0.0`. Callers who can't
  guarantee non-zero use `normalize_or_zero`.

That's it. Everything else returns `Option` / `Result` or produces a
well-defined default.

## 4. `Color`

```rust
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }

impl Color {
    pub const WHITE: Self;
    pub const BLACK: Self;
    pub const TRANSPARENT: Self;
    pub const RED: Self;
    pub const GREEN: Self;
    pub const BLUE: Self;
    pub const YELLOW: Self;
    pub const CYAN: Self;
    pub const MAGENTA: Self;

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self;      // a = 1.0
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self;
    pub fn from_hex(hex: u32) -> Self;                     // 0xRRGGBB or 0xRRGGBBAA
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self;

    pub fn with_alpha(self, a: f32) -> Self;
    pub fn lerp(self, other: Self, t: f32) -> Self;
    pub fn to_array(self) -> [f32; 4];                     // macroquad interop
}
```

Compatible with `macroquad::prelude::Color` — same field layout, same
defaults, same `From` conversions (see [23-ember-render-macroquad.md](23-ember-render-macroquad.md)).

## 5. Time

### 5.1 `Cooldown` — the renamed API

```rust
#[derive(Clone, Debug)]
pub struct Cooldown {
    duration: f32,
    remaining: f32,
}

impl Cooldown {
    /// Ready immediately. After `.trigger()`, becomes not-ready for `duration`.
    pub fn ready(duration: f32) -> Self;

    /// Already running. Becomes ready after `duration`.
    pub fn running(duration: f32) -> Self;

    /// Ready now with `duration` of 0.
    pub fn immediate() -> Self;

    /// Reconstructs from a saved `remaining` value, clamping to valid range.
    pub fn from_remaining(duration: f32, remaining: f32) -> Self;

    // --- Deprecated (removed in v0.2) ---
    #[deprecated(since = "0.1.0", note = "use `Cooldown::running(d)`")]
    pub fn new(duration: f32) -> Self { Self::running(duration) }

    #[deprecated(since = "0.1.0", note = "use `Cooldown::ready(d)`")]
    pub fn with_duration(duration: f32) -> Self { Self::ready(duration) }

    // --- Queries ---
    pub fn duration(&self) -> f32;
    pub fn remaining(&self) -> f32;
    pub fn is_ready(&self) -> bool;
    pub fn is_active(&self) -> bool;
    pub fn fraction(&self) -> f32;            // 0..1, 1.0 when ready

    // --- Mutation ---
    pub fn tick(&mut self, dt: f32);           // reduces remaining, clamps at 0
    pub fn tick_returning(&mut self, dt: f32) -> bool;  // true on frame it expires
    pub fn trigger(&mut self, duration: f32);  // restart with new duration
    pub fn reset(&mut self);                   // restart with stored duration
    pub fn clear(&mut self);                   // force ready
}
```

**Semantic contract:**

| Constructor | Initial `is_ready()` |
|---|---|
| `ready(d)` | `true` |
| `running(d)` | `d <= 0.0` (immediately ready if zero/negative duration) |
| `immediate()` | `true` |

**Negative dt:** `tick(-1.0)` is a no-op (matches `ember_core::time::Cooldown::tick_ignores_negative_dt`).

**Fixed-step safety:** because `Cooldown::tick` clamps at zero,
calling it with `1/60` in `physics_process` produces identical behaviour
across frame rates.

### 5.2 `Tick` — fixed-step accumulator

```rust
#[derive(Clone, Debug)]
pub struct Tick {
    duration: f32,
    accumulator: f32,
}

impl Tick {
    pub fn new(duration: f32) -> Self;
    pub fn duration(&self) -> f32;
    pub fn set_duration(&mut self, duration: f32);

    /// Advances and returns how many whole ticks elapsed.
    /// Capped at `max_ticks` per call (default 8).
    pub fn advance(&mut self, dt: f32, max_ticks: u32) -> u32;

    pub fn progress(&self) -> f32;   // 0..1 within the current tick
    pub fn reset(&mut self);
}
```

### 5.3 `Stopwatch`

```rust
#[derive(Clone, Debug, Default)]
pub struct Stopwatch {
    elapsed: f32,
    running: bool,
}

impl Stopwatch {
    pub fn new() -> Self;
    pub fn start(&mut self);
    pub fn stop(&mut self);
    pub fn reset(&mut self);
    pub fn tick(&mut self, dt: f32);
    pub fn elapsed(&self) -> f32;
    pub fn is_running(&self) -> bool;
}
```

## 6. Hashing

```rust
// ember_core::hash

pub fn hash_u32(input: u32) -> u32;
pub fn hash_2d(x: i32, y: i32) -> u32;
pub fn hash_3d(x: i32, y: i32, z: i32) -> u32;

/// Deterministic, avalanche-distributed hash of a seed + input.
pub fn hash_with_seed(seed: u32, input: u32) -> u32;

/// 0..1 float from a hash. Uses the top 24 bits for the mantissa.
pub fn hash_to_f32(h: u32) -> f32;
```

**Contract:** every function is deterministic for the same input (no
seeding by OS entropy), produces well-distributed output (avalanche test
passes), and is safe for zero input.

## 7. RNG

### 7.1 `Rng`

```rust
#[derive(Clone, Debug)]
pub struct Rng { /* xoshiro256** state */ }

impl Rng {
    pub fn new(seed: u64) -> Self;

    // --- State ---
    pub fn state(&self) -> [u64; 4];
    pub fn set_state(&mut self, state: [u64; 4]);
    pub fn seed(&self) -> u64;

    // --- Basic ---
    pub fn next_u32(&mut self) -> u32;
    pub fn next_u64(&mut self) -> u64;
    pub fn next_f32(&mut self) -> f32;         // [0, 1)
    pub fn next_f64(&mut self) -> f64;         // [0, 1)

    // --- Range ---
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32;      // [lo, hi)
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize;
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32;      // [lo, hi)
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32;

    // --- Choice ---
    pub fn bool(&mut self) -> bool;
    pub fn choice<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T>;
    pub fn shuffle<T>(&mut self, slice: &mut [T]);
}
```

**Contract:**
- Same seed → same sequence, forever (this is a compatibility guarantee).
- `seed(0)` does not lock up (special-cased).
- `next_f32()` is in `[0, 1)`, never `1.0`.

### 7.2 Legacy helpers

For compatibility with existing `ember_stdlib::rng` code:

```rust
pub fn rand01(seed: u64, index: u64) -> f32;
pub fn rand_usize(seed: u64, index: u64, n: usize) -> usize;
```

These are stateless (index-driven), used by `ember_stdlib::grid` for
deterministic tile generation.

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

A global (thread-local, no `static mut`) registry of type names to
constructors. Populated at startup by the `#[derive(Node)]` macro's
generated `register` function.

```rust
pub struct ClassDB { /* HashMap<&'static str, fn() -> Box<dyn Object>> */ }

impl ClassDB {
    pub fn new() -> Self;

    /// Register a class. Idempotent — same name registered twice is a no-op.
    pub fn register<T: Object>(&mut self) where T: Default;

    /// Register with a custom constructor.
    pub fn register_with<T: Object>(&mut self, ctor: fn() -> T);

    /// Instantiate by name. Returns `None` if unregistered.
    pub fn instantiate(&mut self, class_name: &str) -> Option<Box<dyn Object>>;

    /// List all registered class names (sorted).
    pub fn names(&self) -> Vec<&'static str>;
}
```

**Threading model:** `ClassDB` is created once per thread and passed
around by reference. `#[derive(Node)]` generates a `register_class(db)`
function that each crate's `init` calls. No global mutable state (see
[02-principles I3](02-principles.md#i3--no-global-mutable-state)).

## 10. `Plugin` trait

The single extension point between games and the engine.

```rust
pub trait Plugin: Sized + 'static {
    /// Called once at startup. Register classes, load assets, create root.
    fn build(class_db: &mut ClassDB, assets: &mut AssetServer) -> Self;

    /// Construct the initial root node from the plugin's state.
    fn root(&mut self) -> Box<dyn Node>;

    /// Optional cleanup before shutdown.
    fn shutdown(&mut self) {}
}
```

A game's `lib.rs` typically looks like:

```rust
pub struct EmberWarsPlugin { config: GameConfig }

impl Plugin for EmberWarsPlugin {
    fn build(class_db: &mut ClassDB, assets: &mut AssetServer) -> Self {
        class_db.register::<Unit>();
        class_db.register::<Catapult>();
        // ...
        let config = assets.load("config/levels.ron").unwrap_or_default();
        Self { config }
    }

    fn root(&mut self) -> Box<dyn Node> {
        Box::new(GameRoot::new(self.config.clone()))
    }
}
```

## 11. Error type

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("value out of range: {0}")]
    OutOfRange(String),

    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: &'static str, actual: &'static str },

    #[error("class not registered: {0}")]
    ClassNotRegistered(String),
}
```

## 12. Prelude

```rust
pub mod prelude {
    pub use crate::math::{Vec2, Rect, Aabb, Circle};
    pub use crate::color::Color;
    pub use crate::time::{Cooldown, Tick, Stopwatch};
    pub use crate::rng::Rng;
    pub use crate::hash::{hash_2d, hash_3d};
    pub use crate::class_db::{Object, ClassDB};
    pub use crate::plugin::Plugin;
    pub use crate::error::CoreError;
}
```

## 13. Design notes

### 13.1 Why `Vec2` is a re-export, not a newtype

Every game already uses `glam::Vec2` extensively. A newtype would require
`From`/`Into` conversions everywhere. A direct `pub use glam::Vec2` keeps
the code identical while routing through `ember_core::math::Vec2` so that
if we ever switch to `mint` or a custom SIMD type, the re-export is a
one-line change.

### 13.2 Why `Cooldown` is renamed, not patched

The current `Cooldown::new` semantics (starts active) are *correct* for
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