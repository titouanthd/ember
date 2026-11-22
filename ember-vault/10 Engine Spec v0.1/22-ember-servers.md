---
title: "22 — `ember_servers`"
aliases: ["22 — `ember_servers`"]
tags: ["ember/spec", "ember/wave-3"]
doc-status: "draft"
former-path: "docs/engine/v0.1/22-ember-servers.md"
imported: "2026-11-22"
wave: "3 — Crate Specifications"
---

# 22 — `ember_servers`

> **Status:** ⬜ draft
> **Audience:** implementers
> **Prerequisites:** [[12-servers]], [[20-ember-core]]

## 1. Purpose

Specify the full trait signatures for `RenderingServer`, `AudioServer`,
and `PhysicsServer`, plus every supporting type (ids, feature flags,
errors). This crate is **trait definitions only** — no implementations.
Reference implementations live in the `_macroquad` crates.

## 2. Module layout

```
ember_servers/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── prelude.rs
│   ├── error.rs        ServerError
│   ├── rendering/
│   │   ├── mod.rs
│   │   ├── trait.rs    RenderingServer
│   │   ├── types.rs    TextureId, RenderFeature, ShaderId
│   │   └── image.rs    Image
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── trait.rs    AudioServer
│   │   └── types.rs    AudioClipId, SoundId, AudioFeature
│   └── physics/
│       ├── mod.rs
│       ├── trait.rs    PhysicsServer
│       └── types.rs    Shape, CollisionMask, RayHit
```

## 3. Shared types

### 3.1 `ServerError`

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    #[error("invalid resource id: {0:?}")]
    InvalidId(u64),

    #[error("feature not supported: {0:?}")]
    UnsupportedFeature(&'static str),

    #[error("GPU error: {0}")]
    Gpu(String),

    #[error("audio device error: {0}")]
    Audio(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

**Note:** server methods that "just work" (draw, play, tick) return
`()` or their natural type. Methods that can genuinely fail (create
texture, load audio) return `Result`.

## 4. `RenderingServer`

### 4.1 Trait

```rust
pub trait RenderingServer {
    // === Frame ===
    fn begin_frame(&mut self, viewport: Rect);
    fn end_frame(&mut self);
    fn set_clear_color(&mut self, color: Color);

    // === Camera ===
    fn set_camera(&mut self, offset: Vec2);
    fn camera_offset(&self) -> Vec2;

    // === Z ordering ===
    fn set_z(&mut self, z: f32);
    fn current_z(&self) -> f32;

    // === Primitives ===
    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_rect_outline(&mut self, rect: Rect, thickness: f32, color: Color);
    fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color);
    fn draw_line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: Color);
    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color);

    // === Sprites ===
    fn draw_sprite(
        &mut self,
        texture: TextureId,
        dst: Rect,
        src: Option<Rect>,   // None = full texture
        tint: Color,
    );

    // === Text ===
    fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color);
    fn measure_text(&self, text: &str, size: f32) -> Vec2;

    // === Textures ===
    fn create_texture(&mut self, image: &Image) -> Result<TextureId, ServerError>;
    fn destroy_texture(&mut self, id: TextureId);
    fn texture_size(&self, id: TextureId) -> Option<Vec2>;

    // === Shaders (v0.1 stub) ===
    fn register_shader(&mut self, name: &str, source: &str) -> Result<ShaderId, ServerError>;
    fn use_shader(&mut self, shader: Option<ShaderId>);

    // === Capability ===
    fn supports(&self, feature: RenderFeature) -> bool;

    // === Info ===
    fn name(&self) -> &'static str;
    fn draw_call_count(&self) -> u32;   // for profiling
}
```

### 4.2 `TextureId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct TextureId(u64);

impl TextureId {
    pub fn invalid() -> Self;
    pub fn is_valid(self) -> bool;
}
```

Opaque. Games never construct one — they get it from
`Handle<Texture>::id()` (see [[25-ember-assets]]).

### 4.3 `ShaderId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ShaderId(u32);
```

Opaque. `register_shader` returns one; `use_shader(Some(id))` activates
it for subsequent draw calls until cleared.

**v0.1 scope:** the macroquad backend supports a limited shader set
(grayscale, outline, drop shadow). Arbitrary GLSL compilation is v0.2.

### 4.4 `RenderFeature`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum RenderFeature {
    AlphaBlending,
    CustomShaders,
    TextureArray,
    Instancing,
    RenderToTexture,
}
```

Games query `supports(feature)` and either take the slow path or refuse
to run.

### 4.5 `Image`

```rust
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub data: Vec<u8>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ImageFormat {
    Rgba8,
    Rgb8,
    R8,
}

impl Image {
    pub fn new(width: u32, height: u32, format: ImageFormat) -> Self;
    pub fn from_bytes(bytes: &[u8], format: ImageFormat) -> Result<Self, ServerError>;
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, ServerError>;
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color);
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Color>;
}
```

`Image` is what `AssetServer` loads into before uploading to the GPU via
`create_texture`.

### 4.6 Contract notes

- **`begin_frame`/`end_frame`:** every draw call between them is queued.
  `end_frame` may reorder within a z-level for batching, but not across
  z-levels.
- **`set_z`:** takes effect for subsequent draw calls until the next
  `set_z`. Default `0.0`. Stored per-renderer, not per-node — the
  scene walk sets it before each node's `draw`.
- **`draw_sprite`:** `src = None` means the entire texture. `tint =
  Color::WHITE` means no tint. This is the hot path — implementations
  must batch consecutive sprites with the same texture.
- **`measure_text`:** returns pixel bounds of the rendered string at
  `size`. Used by `ember_ui` for layout.

## 5. `AudioServer`

### 5.1 Trait

```rust
pub trait AudioServer {
    // === Playback ===
    fn play(&mut self, clip: AudioClipId, volume: f32, pitch: f32) -> Result<SoundId, ServerError>;
    fn play_looped(&mut self, clip: AudioClipId, volume: f32) -> Result<SoundId, ServerError>;
    fn stop(&mut self, sound: SoundId);
    fn stop_all(&mut self);
    fn is_playing(&self, sound: SoundId) -> bool;

    // === Volume ===
    fn set_master_volume(&mut self, volume: f32);
    fn master_volume(&self) -> f32;
    fn set_muted(&mut self, muted: bool);
    fn is_muted(&self) -> bool;

    // === Audio clips ===
    fn load_clip(&mut self, bytes: &[u8]) -> Result<AudioClipId, ServerError>;
    fn destroy_clip(&mut self, clip: AudioClipId);

    // === Lifecycle ===
    fn flush(&mut self);

    // === Capability ===
    fn supports(&self, feature: AudioFeature) -> bool;
    fn name(&self) -> &'static str;
}
```

### 5.2 `AudioClipId` and `SoundId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct AudioClipId(u64);

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SoundId(u64);
```

- `AudioClipId` — the loaded clip data (shared, reference-counted).
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
    Reverb,
}
```

### 5.4 Contract notes

- **`play`:** returns `Err` if the device is unavailable. Games should
  treat this as "no audio, continue playing silently".
- **`stop`:** a no-op if the sound already finished. Silently ignored
  per [[02-principles#5.4 What is never an error|02-principles §5.4]].
- **`flush`:** called once per frame by `MainLoop`. Implementations
  push accumulated commands to the OS audio device.

## 6. `PhysicsServer`

### 6.1 Trait

```rust
pub trait PhysicsServer {
    // === Registration ===
    fn register(&mut self, id: NodeId, shape: Shape, mask: CollisionMask, position: Vec2);
    fn update(&mut self, id: NodeId, position: Vec2);
    fn unregister(&mut self, id: NodeId);

    // === Queries ===
    fn overlaps_shape(&self, shape: &Shape, mask: CollisionMask) -> Vec<NodeId>;
    fn overlaps_point(&self, point: Vec2, mask: CollisionMask) -> Vec<NodeId>;
    fn raycast(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Option<RayHit>;
    fn raycast_all(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Vec<RayHit>;

    // === Reconstruction ===
    /// Recompute internal indices after bulk `update` calls.
    fn rebuild(&mut self);

    // === Capability ===
    fn name(&self) -> &'static str;
}
```

### 6.2 `Shape`

```rust
#[derive(Clone, Debug)]
pub enum Shape {
    Aabb { center: Vec2, half_extent: Vec2 },
    Circle { center: Vec2, radius: f32 },
    Line { from: Vec2, to: Vec2, thickness: f32 },
}
```

### 6.3 `CollisionMask`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct CollisionMask(u32);

impl CollisionMask {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(u32::MAX);

    pub const fn from_bits(bits: u32) -> Self;
    pub fn bits(self) -> u32;
    pub fn contains(self, layer: u32) -> bool;
    pub fn overlaps(self, other: Self) -> bool;
}
```

Layers are user-defined (0–31). A shape belongs to one layer; a query
specifies which layers it cares about.

### 6.4 `RayHit`

```rust
#[derive(Copy, Clone, Debug)]
pub struct RayHit {
    pub node: NodeId,
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}
```

### 6.5 v0.1 scope

The `LinearPhysics` implementation (in `ember_stdlib`, re-exported by
`ember_servers`) does O(n) `overlaps` checks per query. Sufficient for
games with < 300 registered shapes. Spatial indexing (grid hash,
BVH) is v0.2.

## 7. Backend registration

### 7.1 Factory pattern

Backends expose a `new()` returning `Result<Box<dyn Trait>, ServerError>`:

```rust
// ember_render_macroquad
pub struct MacroquadRenderer { /* ... */ }

impl MacroquadRenderer {
    pub fn new() -> Result<Box<dyn RenderingServer>, ServerError>;
}
```

Games never see `MacroquadRenderer`. The engine's `main` chooses a
backend and passes the boxed trait to the `MainLoop`.

### 7.2 Startup template

```rust
// games/ember-wars/src/main.rs
#[macroquad::main("Ember Wars")]
async fn main() {
    let rendering = ember_render_macroquad::MacroquadRenderer::new()
        .expect("failed to create renderer");
    let audio = ember_audio_macroquad::MacroquadAudio::new()
        .expect("failed to create audio");
    let physics = ember_servers::LinearPhysics::new();

    ember::run::<EmberWarsPlugin>(rendering, audio, physics).await;
}
```

### 7.3 The "one backend" rule

v0.1 ships exactly one backend per server. `Cargo.toml` for a game
depends on:

```toml
[dependencies]
ember = "0.1"
ember_render_macroquad = "0.1"
ember_audio_macroquad = "0.1"
```

A v0.2 game that wants wgpu swaps one line:

```toml
ember_render_wgpu = "0.2"
```

The `ember::run` call is unchanged.

## 8. Feature probing contract

### 8.1 What `supports` means

`supports(feature)` returns `true` if the backend can execute the feature
**today** on the **current platform**. It is conservative — a feature
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
an `Image`; `RenderingServer::create_texture` consumes it. Putting it
in either crate forces a cyclic dependency. Middle layer = shared.

### 9.4 Why `PhysicsServer` is a trait despite v0.1 linear impl

Because the seam is what matters. A game written against
`PhysicsServer` today migrates to `SpatialHashPhysics` in v0.2 by
swapping one line in `main.rs`. A game written against
`ember_stdlib::collider` directly would need to rewrite every collision
check.

## 10. Open questions

- **Q1:** Should `RenderingServer` expose a `push_clip_rect` / `pop_clip_rect`
  pair? *Provisional:* yes, needed by `ember_ui`'s scroll containers.
  Add to v0.1 trait.
- **Q2:** Should `AudioServer` support 3D positional audio? *Provisional:*
  no. v0.2 feature, gated behind `AudioFeature::PositionalAudio`.
- **Q3:** Should `PhysicsServer::rebuild` be automatic? *Provisional:*
  no. Games call it explicitly after bulk updates — automatic rebuild
  wastes work.
- **Q4:** Should `RenderingServer` expose low-level vertices? *Provisional:*
  no. Primitives + sprites cover 18/18 games.

## 11. Cross-references

- Why servers at all → [[12-servers]]
- The macroquad renderer → [[23-ember-render-macroquad]]
- `NodeContext` server access → [[21-ember-scene#6. `NodeContext`|21-ember-scene.md §6]]
- Assets that produce textures → [[25-ember-assets]]
