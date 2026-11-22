---
title: "22 — `ember_servers`"
aliases: ["22 — `ember_servers`"]
tags: ["ember/spec", "ember/wave-3"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/22-ember-servers.md"
imported: "2026-11-22"
wave: "3 — Crate Specifications"
---

# 22 — `ember_servers`

> **Status:** 🟡 partial — reviewed 2026-11-19; all three traits, ids, mocks and `LinearPhysics` exist with a smaller/different API than first specced; shaders, `RenderToTexture`, `Reverb`, mute and looped playback are not implemented.
> **Audience:** implementers
> **Prerequisites:** [[12-servers]], [[20-ember-core]]

## 1. Purpose

Specify the full trait signatures for `RenderingServer`, `AudioServer`,
and `PhysicsServer`, plus every supporting type (ids, feature flags,
errors). The crate holds the traits plus test doubles (`MockRenderer`,
`MockAudio`, `tests/contract.rs`) and the CPU `LinearPhysics` (ADR-028).
Real backends live in the `macroquad` crates.

## 2. Module layout

```
ember-servers/src/
├── lib.rs
├── error.rs        ServerError
├── types.rs        TextureId, FontId, SoundId, AudioClipId,
│                   RenderFeature, AudioFeature, TextMetrics
├── rendering.rs    RenderingServer
├── audio.rs        AudioServer
├── physics/
│   ├── mod.rs      PhysicsServer, Shape, CollisionMask, RayHit
│   └── linear.rs   LinearPhysics
└── mock.rs         MockRenderer, MockAudio (+ RecordedCall, RecordedAudioCall)
```

`Image` lives in `ember_core` (not here), there is no `prelude.rs`.
*Not yet implemented:* `ShaderId`.

## 3. Shared types

### 3.1 `ServerError`

```rust
#[derive(Debug)]
#[non_exhaustive]
pub enum ServerError {
    Backend(String),     // backend/driver failure (bad image, decode error)
    UnknownHandle,
    NotSupported,        // e.g. blocking font load on wasm (ADR-020)
    Io(std::io::Error),  // From<io::Error>
}
```

`Display`/`Error` are hand-written (no `thiserror`).

**Note:** server methods that "just work" (draw, play, tick) return
`()` or their natural type (`play` returns a `SoundId`, never a `Result`).
Methods that can genuinely fail (create texture/font, load clip) return
`Result`.

## 4. `RenderingServer`

### 4.1 Trait

```rust
pub trait RenderingServer {
    // === Frame ===
    fn begin_frame(&mut self, viewport: Rect);
    fn end_frame(&mut self);
    fn set_clear_color(&mut self, color: Color);
    fn viewport(&self) -> Rect;

    // === Camera ===
    fn set_camera(&mut self, offset: Vec2);
    fn camera_offset(&self) -> Vec2;

    // === Z ordering ===
    fn set_z(&mut self, z: f32);
    fn current_z(&self) -> f32;

    // === Primitives ===
    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_rect_outline(&mut self, rect: Rect, thickness: f32, color: Color);
    fn draw_rect_rotated(&mut self, rect: Rect, rotation_radians: f32, color: Color);
    fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color);
    fn draw_circle_outline(&mut self, center: Vec2, radius: f32, thickness: f32, color: Color);
    fn draw_line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: Color);
    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color);

    // === Sprites ===
    fn draw_sprite(&mut self, texture: TextureId, dst: Rect, src: Option<Rect>, tint: Color);

    // === Text ===
    fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color);
    fn draw_text_with_font(&mut self, font: FontId, text: &str, pos: Vec2, size: f32, color: Color);
    fn measure_text(&self, text: &str, size: f32) -> TextMetrics;
    fn measure_text_with_font(&self, font: FontId, text: &str, size: f32) -> TextMetrics;
    fn set_default_font(&mut self, font: Option<FontId>);

    // === Resources ===
    fn create_texture(&mut self, image: &Image) -> Result<TextureId, ServerError>;
    fn destroy_texture(&mut self, id: TextureId);
    fn create_font(&mut self, bytes: &[u8]) -> Result<FontId, ServerError>;
    fn destroy_font(&mut self, id: FontId);

    // === Capability ===
    fn supports(&self, feature: RenderFeature) -> bool;
}
```

*Not yet implemented:* `texture_size`, `register_shader` / `use_shader`,
`name`, `draw_call_count`, `push_clip_rect`/`pop_clip_rect`.
`Image` is `ember_core::Image`; `TextMetrics` is
`{ width, height, offset_y }` (ADR-016).

### 4.2 `TextureId`, `FontId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct TextureId(u64);   // FontId, SoundId, AudioClipId are the same shape

impl TextureId {
    pub const fn new(raw: u64) -> Self;
    pub const fn raw(self) -> u64;
}
```

Opaque. Games get one from `Handle<Texture>::id()` (see
[[25-ember-assets]]). There is no `invalid()` /
`is_valid()` on the server ids (`AssetId` has them).

### 4.3 `ShaderId`

*Not yet implemented.* There is no shader API; `RenderFeature::CustomShaders`
is only a capability flag. The planned design: `register_shader` returns a
`ShaderId(u32)`, `use_shader(Some(id))` activates it until cleared.

### 4.4 `RenderFeature`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum RenderFeature {
    AlphaBlending,
    CustomShaders,
    TextureArray,
    Instancing,
}
```

*Not yet implemented:* `RenderToTexture`. Games query `supports(feature)`
and either take the slow path or refuse to run.

### 4.5 `Image`

`Image` lives in `ember_core` (RGBA8 only: `new`, `from_rgba8`, `width`,
`height`, `as_bytes`, `set_pixel`; see [[20-ember-core]]).
The planned `ImageFormat` (`Rgb8`, `R8`) and `get_pixel` are *Not yet
implemented*. `AssetServer` decodes a PNG into an `Image` before uploading
via `create_texture`.

### 4.6 Contract notes

- **`begin_frame`/`end_frame`:** the contract allows queueing and
  per-z reordering, but the macroquad backend draws immediately; its
  `begin_frame`/`end_frame` are markers and `set_z` only stores the value.
- **`set_z`:** takes effect for subsequent draw calls until the next
  `set_z`. Default `0.0`. Stored per-renderer, not per-node — the
  scene walk sets it before each node's `draw`.
- **`draw_sprite`:** `src = None` means the entire texture. `tint =
  Color::WHITE` means no tint. This is the hot path — implementations
  must batch consecutive sprites with the same texture.
- **`measure_text`:** returns `TextMetrics` (pixel bounds) of the rendered
  string at `size`. Used by `ember_ui` for layout.

## 5. `AudioServer`

### 5.1 Trait

```rust
pub trait AudioServer {
    // === Playback ===
    fn play(&mut self, clip: AudioClipId, volume: f32, pitch: f32) -> SoundId;
    fn stop(&mut self, sound: SoundId);
    fn stop_all(&mut self);

    // === Volume ===
    fn set_master_volume(&mut self, volume: f32);
    fn master_volume(&self) -> f32;

    // === Clips ===
    fn load_clip(&mut self, bytes: &[u8]) -> Result<AudioClipId, ServerError>;
    fn destroy_clip(&mut self, clip: AudioClipId);

    // === Lifecycle ===
    fn flush(&mut self);

    // === Capability ===
    fn supports(&self, feature: AudioFeature) -> bool;
}
```

*Not yet implemented:* `play_looped`, `is_playing`, `set_muted`/`is_muted`,
`name`. `play` is infallible.

### 5.2 `AudioClipId` and `SoundId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct AudioClipId(u64);   // new(raw) / raw()

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SoundId(u64);
```

- `AudioClipId` — the loaded clip data (released with `destroy_clip`).
- `SoundId` — one active playback instance (unique per `play` call).

A game with a `"click.ogg"` clip has one `AudioClipId` and many
`SoundId`s (one per click).

### 5.3 `AudioFeature`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum AudioFeature {
    PitchShift,
    Streaming,
    PositionalAudio,
}
```

*Not yet implemented:* `Reverb`. The macroquad backend reports no features.

### 5.4 Contract notes

- **`play`:** infallible; an unknown clip still returns a `SoundId`
  (nothing is played). The macroquad backend ignores `pitch` and never
  loops.
- **`stop`:** a no-op if the sound already finished. Silently ignored
  per [[02-principles#5.4 What is never an error|02-principles §5.4]].
- **`flush`:** called once per frame by `SceneLoop::draw`. A no-op in the
  macroquad backend.

## 6. `PhysicsServer`

### 6.1 Trait

```rust
/// Handles are plain `u64` (monotonic from 1), not `NodeId` (ADR-017).
pub trait PhysicsServer {
    fn register(&mut self, shape: Shape, mask: CollisionMask) -> u64;
    fn unregister(&mut self, handle: u64);
    /// Replace the shape (position included) of `handle`.
    fn update(&mut self, handle: u64, shape: Shape);

    fn query_overlaps(&self, shape: &Shape, mask: CollisionMask) -> Vec<u64>;
    fn raycast(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Option<RayHit>;

    /// Recompute internal indices after bulk updates (no-op in `LinearPhysics`).
    fn rebuild(&mut self);
}
```

*Not yet implemented:* `overlaps_point`, `raycast_all`, `name`; shapes carry
their own position (no separate `position` argument).

### 6.2 `Shape`

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Shape {
    Aabb(Aabb),      // ember_core::math::Aabb (min/max)
    Circle(Circle),  // ember_core::math::Circle
}
```

`ember_servers::PhysicsShape` is an alias for this type. *Not yet
implemented:* `Line` shape.

### 6.3 `CollisionMask`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct CollisionMask(pub u32);

impl CollisionMask {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(u32::MAX);

    pub const fn from_bits(bits: u32) -> Self;
    pub const fn bits(self) -> u32;
    pub const fn overlaps(self, other: Self) -> bool;   // bits & bits != 0
}
```

*Not yet implemented:* `contains(layer)`.

Layers are user-defined (0–31). A shape belongs to one layer; a query
specifies which layers it cares about.

### 6.4 `RayHit`

```rust
#[derive(Copy, Clone, Debug)]
pub struct RayHit {
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}
```

### 6.5 v0.1 scope

`LinearPhysics` (in `ember-servers`, ADR-028; `new`, `len`, `is_empty`)
does O(n) checks per query; `raycast` returns the nearest hit.
`SceneLoop` owns one. Sufficient for
games with < 300 registered shapes. Spatial indexing (grid hash,
BVH) is v0.2.

## 7. Backend registration

### 7.1 Construction

Backends are plain structs built with an infallible `new()`:

```rust
// ember-render-macroquad
impl MacroquadRenderer { pub fn new() -> Self; }
// ember-audio-macroquad
impl MacroquadAudio { pub fn new() -> Self; }
```

They are passed **by value** (generics `SceneLoop<R, A>`), not boxed
(see [[21-ember-scene|21-ember-scene.md §12.1]]). The planned
`Result<Box<dyn Trait>, ServerError>` factory is *Not yet implemented*.

### 7.2 Startup template

```rust
// games/<game>/src/main.rs
fn main() {
    let scene_loop = SceneLoop::new(
        tree,
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    );
    ember_render_macroquad::run_window("Game", 800, 600, scene_loop);
}
```

(`ember::run` / `ember::run_window` are re-exported by the `ember` facade;
`ember::run::<Plugin>(…)` is *Not yet implemented*, see
[[20-ember-core|20-ember-core.md §10]].)

### 7.3 The "one backend" rule

v0.1 ships exactly one backend per server. `Cargo.toml` for a game
depends on:

```toml
[dependencies]
ember = "0.1"
ember-render-macroquad = "0.1"
ember-audio-macroquad = "0.1"
```

A v0.2 game that wants wgpu swaps one line:

```toml
ember-render-wgpu = "0.2"   # hypothetical
```

The `run` call is unchanged.

## 8. Feature probing contract

### 8.1 What `supports` means

`supports(feature)` returns `true` if the backend can execute the feature
**today** (the macroquad renderer reports `AlphaBlending` and the
`CustomShaders` flag only; there is no shader API yet) on the **current platform**. It is conservative — a feature
supported on desktop but not web returns `false` when running on web.

### 8.2 What `supports` does not mean

- Not "will support in a future release"
- Not "supports on other platforms"
- Not "is enabled by default"

Games must probe explicitly and choose a fallback.

### 8.3 Recommended probe pattern

```rust
struct MyNode {
    use_shader: bool,
}

impl Node for MyNode {
    fn ready(&mut self, ctx: &mut NodeContext) {
        self.use_shader = ctx.render().supports(RenderFeature::CustomShaders);
    }

    fn draw(&self, ctx: &mut DrawContext) {
        if self.use_shader {
            // shader path
        } else {
            // simpler path
        }
    }
}
```

## 9. Design notes

### 9.1 Why traits, not singletons (recap)

Godot's servers are global. This works in GDScript (which has no
borrow checker) and breaks in Rust:

- `RenderingServer::draw_rect(...)` from any node at any time is a
  global mutable borrow — impossible to express safely without
  `Mutex`, which is slow.
- A test that needs to check draw calls can't intercept a global
  singleton.

Traits + `&mut dyn` inside `NodeContext` express exactly the right
borrow: one mutable borrow at a time, scoped to the callback.

### 9.2 Why `&mut dyn` and not generic `<R: RenderingServer>`

Generics would monomorphise every `draw` call. A scene with 200 node
types and 50 draw calls each produces 10,000 monomorphised versions of
`draw_rect`. Binary size explodes; compile times explode; and the
optimisation benefit is negligible because drawing is dominated by the
GPU.

`&mut dyn` gives vtable dispatch (2 ns overhead per call) and is
invisible in profiles.

### 9.3 Why `Image` is in `ember_servers` and not `ember_assets`

Because it is the wire format between the two: `AssetServer` produces
an `Image`; `RenderingServer::create_texture` consumes it. In the
shipped code it lives one layer lower, in `ember_core`, which both
crates already depend on.

### 9.4 Why `PhysicsServer` is a trait despite v0.1 linear impl

Because the seam is what matters. A game written against
`PhysicsServer` today migrates to `SpatialHashPhysics` in v0.2 by
swapping one line in `main.rs`. A game written against
collision helpers directly would need to rewrite every collision
check.

## 10. Open questions

- **Q1:** Should `RenderingServer` expose a `push_clip_rect` / `pop_clip_rect`
  pair? *Provisional:* yes, needed by `ember_ui`'s scroll containers.
  Not in the trait yet.
- **Q2:** Should `AudioServer` support 3D positional audio? *Provisional:*
  no. v0.2 feature, gated behind `AudioFeature::PositionalAudio`.
- **Q3:** Should `PhysicsServer::rebuild` be automatic? *Provisional:*
  no. Games call it explicitly after bulk updates — automatic rebuild
  wastes work.
- **Q4:** Should `RenderingServer` expose low-level vertices? *Provisional:*
  no. Primitives + sprites cover 17/17 games.

## 11. Cross-references

- Why servers at all → [[12-servers]]
- The macroquad renderer → [[23-ember-render-macroquad]]
- `NodeContext` server access → [[21-ember-scene#6. `NodeContext`|21-ember-scene.md §6]]
- Assets that produce textures → [[25-ember-assets]]
