---
title: "12 — Servers"
aliases: ["12 — Servers"]
tags: ["ember/spec", "ember/wave-2"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/12-servers.md"
imported: "2026-11-22"
wave: "2 — Concepts"
---

# 12 — Servers

> **Status:** 🟡 partial — reviewed 2026-11-19; three server traits, mocks and macroquad backends exist; `AssetServer`-driven texture lifecycle, shader API and `MainLoop::new(servers)` creation are not implemented.
> **Audience:** game authors, implementers
> **Prerequisites:** [[10-nodes-and-scenes]]

## 1. Purpose

Explain the three **servers** — `RenderingServer`, `AudioServer`,
`PhysicsServer` — that sit between game code and the platform. This is
the file you read when you need to draw something, play a sound, or
trace a ray.

The critical property to remember: **servers are traits, not singletons.**
Games never import a concrete backend. Games never call `macroquad` or
`miniquad`. Games call `ctx.render().draw_sprite(...)`, and the backend
is a detail chosen by the engine's `main`.

## 2. Why servers

Without servers, every game would import `macroquad` directly:

```rust
// today
use macroquad::prelude::*;
fn draw() { draw_rectangle(0.0, 0.0, 100.0, 50.0, RED); }
```

This works until you want to:

1. Add a second rendering backend (wgpu for desktop, WebGL for web).
2. Record draw calls for a visual debugger.
3. Run in headless CI without a window.
4. Profile the renderer without a GPU.

Every one of these becomes a code change in 17 games, × every draw call.
Servers reduce the change to one crate:

```rust
// with servers
ctx.render().draw_rect(Rect::new(0.0, 0.0, 100.0, 50.0), RED);
```

Same call, no macroquad. The backend is chosen at startup by `main` and
passed in via `NodeContext`.

## 3. Server lifecycle

```mermaid
sequenceDiagram
    participant Main as main()
    participant Loop as MainLoop
    participant R as RenderingServer
    participant A as AudioServer
    participant P as PhysicsServer

    Main->>R: MacroquadRenderer::new()
    Main->>A: MacroquadAudio::new()
    Main->>Loop: SceneLoop::new(tree, R, A, viewport)
    Note over Loop,P: SceneLoop creates and owns LinearPhysics itself
    loop frame
        Loop->>R: begin_frame(viewport)
        Loop->>Loop: process / draw
        Loop->>R: end_frame()
        Loop->>A: flush()
    end
    Main->>Loop: finalize()
    Main->>R: drop()
    Main->>A: drop()
    Main->>P: drop()
```

### 3.1 Creation

Servers are created **once at startup** by the top-level `main`.
Implementations are concrete types (`MacroquadRenderer`,
`MacroquadAudio`, `LinearPhysics`). `ember_scene::SceneLoop<R, A>` is
generic over the rendering and audio backends and owns them by value (not
`Box<dyn Trait>`); it is the only place they are stored. Everything else
sees them as `&mut dyn Trait`.

### 3.2 Ownership

`SceneLoop` owns the servers for its lifetime. Each pass bundles them
into a `Servers` struct and the tree builds one `NodeContext` per
callback, borrowing them one level deep (ADR-022):

```rust
pub struct Servers<'a> {
    pub rendering: &'a mut dyn RenderingServer,
    pub audio: &'a mut dyn AudioServer,
    pub physics: &'a mut dyn PhysicsServer,
    pub input: &'a Input,
}
```

Because `NodeContext` carries `&mut`, you **cannot** call
`ctx.render().draw_sprite(...)` from two places at once — which is what
prevents race conditions in the future thread pool.

### 3.3 Destruction

Servers drop with the `SceneLoop` after `MainLoop::finalize()` (which
clears the tree). `Node::finalize(&mut self)` has no `ctx`, so a node
cannot use a server there; calling `ctx.render()` would not compile.

## 4. RenderingServer

### 4.1 Trait definition

```rust
pub trait RenderingServer {
    // --- Frame ---
    fn begin_frame(&mut self, viewport: Rect);
    fn end_frame(&mut self);
    fn set_clear_color(&mut self, color: Color);
    fn viewport(&self) -> Rect;

    // --- Camera ---
    fn set_camera(&mut self, camera: Camera2d);  // { offset, zoom } — ADR-040
    fn camera(&self) -> Camera2d;

    // --- Z ordering ---
    fn set_z(&mut self, z: f32);
    fn current_z(&self) -> f32;

    // --- Clipping (ADR-031): screen pixels, not moved by the camera ---
    fn push_clip_rect(&mut self, rect: Rect);
    fn pop_clip_rect(&mut self);
    fn clip_rect(&self) -> Option<Rect>;

    // --- Primitives ---
    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_rect_outline(&mut self, rect: Rect, thickness: f32, color: Color);
    fn draw_rect_rotated(&mut self, rect: Rect, rotation_radians: f32, color: Color);
    fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color);
    fn draw_circle_outline(&mut self, center: Vec2, radius: f32, thickness: f32, color: Color);
    fn draw_line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: Color);
    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color);

    // --- Sprites ---
    fn draw_sprite(&mut self, texture: TextureId, dst: Rect, src: Option<Rect>, tint: Color);

    // --- Text ---
    fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color);
    fn draw_text_with_font(&mut self, font: FontId, text: &str, pos: Vec2, size: f32, color: Color);
    fn measure_text(&self, text: &str, size: f32) -> TextMetrics;
    fn measure_text_with_font(&self, font: FontId, text: &str, size: f32) -> TextMetrics;
    fn set_default_font(&mut self, font: Option<FontId>);

    // --- Textures and fonts ---
    fn create_texture(&mut self, image: &Image) -> Result<TextureId, ServerError>;
    fn destroy_texture(&mut self, id: TextureId);
    fn create_font(&mut self, bytes: &[u8]) -> Result<FontId, ServerError>;
    fn destroy_font(&mut self, id: FontId);

    // --- Capability probing ---
    fn supports(&self, feature: RenderFeature) -> bool;
}

#[non_exhaustive]
pub enum RenderFeature {
    AlphaBlending,
    CustomShaders,
    TextureArray,
    Instancing,
}
```

The method set beyond the original floor (fallible `create_texture`,
`draw_triangle`, `draw_circle_outline`, `draw_rect_rotated`, font
methods, `viewport`, `destroy_font`) is recorded in ADR-016. `set_z`
only stores the value (`current_z()`); the macroquad backend does not yet
reorder draws by z.

### 4.2 Naming convention

- `draw_*` — immediate draw call, resets at `end_frame`
- `set_*` — changes persistent state (camera, z, clear color, default font)
- `create_*` / `destroy_*` — resource lifecycle
- `supports` — capability probe

### 4.3 Batch semantics

`begin_frame` opens a batching window. `end_frame` flushes it. Within a
batch, the server may reorder draw calls if ordering constraints allow.
The one hard constraint: **`draw_*` calls are visible in their call
order** within a z-level.

This matches macroquad's model and is what makes the trait implementable
on both immediate-mode backends (macroquad) and retained-mode backends
(wgpu, where batching is explicit). In the macroquad backend
`begin_frame`/`end_frame` are markers only: draws execute immediately in
call order, with no reordering.

### 4.4 Texture lifecycle

`create_texture` returns a `TextureId`, an opaque handle. The server
owns the underlying GPU resource and may fail (`ServerError::Backend`,
e.g. an empty image). `destroy_texture` must be called explicitly: there
is no drop hook, so dropping a `Handle<Texture>` does not destroy the
GPU texture (ADR-026). Unknown ids are ignored by `destroy_texture` and
`draw_sprite`.

Games normally go through the `AssetServer` (see
[[13-assets]]); `AssetServer::texture` takes the renderer
and calls `create_texture` for you.

## 5. AudioServer

### 5.1 Trait definition

```rust
pub trait AudioServer {
    // --- Playback ---
    fn play(&mut self, clip: AudioClipId, volume: f32, pitch: f32) -> SoundId;
    fn stop(&mut self, sound: SoundId);
    fn stop_all(&mut self);

    // --- Volume ---
    fn set_master_volume(&mut self, volume: f32);
    fn master_volume(&self) -> f32;

    // --- Lifecycle ---
    fn flush(&mut self);   // flush pending buffers to the device

    // --- Clips (ADR-016) ---
    fn load_clip(&mut self, bytes: &[u8]) -> Result<AudioClipId, ServerError>;
    fn destroy_clip(&mut self, clip: AudioClipId);

    // --- Capability probing ---
    fn supports(&self, feature: AudioFeature) -> bool;
}

#[non_exhaustive]
pub enum AudioFeature {
    PitchShift,
    Streaming,
    PositionalAudio,
}
```

The macroquad backend reports none of these features: `pitch` is
accepted and ignored.

### 5.2 Playback model

`play` returns a `SoundId`, which is a handle you can `stop` with.
Calling `stop` on an unknown or already-stopped sound is a **no-op**
(see [[02-principles#5.4 What is never an error|02-principles §5.4]]).
Playing an unknown clip still returns a `SoundId`, but nothing plays.
*Not yet implemented:* auto-removal of naturally finished sounds in the
macroquad backend (the `SoundId` stays tracked until `stop`/`stop_all`).

### 5.3 Why no `loop` parameter

Loops are game logic, not server logic. A game that wants looping music
calls `play` again from its own logic (for example on a timer) or uses a
`Loop` wrapper in the game crate. The server stays simple. (There is no
`is_playing` query yet; the macroquad backend always plays with
`looped: false`.)

## 6. PhysicsServer

### 6.1 Status

**Implemented in v0.1 as `LinearPhysics`** (ADR-017, ADR-028): the trait
uses opaque `u64` handles rather than `NodeId` (which lives in the L3
`ember_scene`). `LinearPhysics` lives in `ember-servers` and is written
against `ember_core::math::{Aabb, Circle}`; `ember_stdlib::collider` is
not migrated. Spatial indexing and swept tests are *not yet implemented*.

```rust
pub trait PhysicsServer {
    fn register(&mut self, shape: Shape, mask: CollisionMask) -> u64;
    fn unregister(&mut self, handle: u64);
    fn update(&mut self, handle: u64, shape: Shape); // move/reshape
    fn query_overlaps(&self, shape: &Shape, mask: CollisionMask) -> Vec<u64>;
    fn raycast(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Option<RayHit>;
    fn rebuild(&mut self);  // no-op for LinearPhysics
}

pub struct RayHit {
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}
```

`Shape` (exported as `PhysicsShape`) is `Aabb(Aabb)` or `Circle(Circle)`.

### 6.2 Why a trait if `collider` is enough

Because some games (bullet-hell, asteroids) do collision resolution
hundreds of times per frame. A linear scan at 300 entities is O(n²). A
spatial hash makes it O(n·k). The trait leaves room for a spatial-hash
backend in v0.2 without changing game code.

For v0.1, `LinearPhysics` walks its registry per query (O(n), fine below
~300 shapes). A spatial-hash backend is *not yet implemented*.

## 7. Backend registration

### 7.1 Creation API

```rust
// At the top of the game's main():
let scene_loop = SceneLoop::new(
    tree,
    MacroquadRenderer::new(),
    MacroquadAudio::new(),
    Rect::new(0.0, 0.0, 800.0, 600.0),
);
// Opens a window and drives the loop (ember_render_macroquad::run_window).
run_window("My game", 800, 600, scene_loop);
```

`SceneLoop` creates its own `LinearPhysics`. Node code only sees
`&mut dyn RenderingServer` via `ctx.render()`, so switching backends is a
change in `main`. `MainLoop` is a trait (`ember_core::MainLoop`), not a
struct; there is no `MainLoop::new`.

### 7.2 Backend crate convention

- `ember_render_macroquad` exports `MacroquadRenderer` (plus the `run` / `run_window` runners and `poll_input`)
- `ember_audio_macroquad` exports `MacroquadAudio`
- A future `ember_render_wgpu` exports `WgpuRenderer`
- No crate exports more than one concrete backend type

This is enforced by review, not tooling — but it matches the
`<feature>_<backend>` naming rule from
[[02-principles#3.3 Crate names|02-principles §3.3]].

### 7.3 The "one backend" rule for v0.1

Only `macroquad` backends ship in v0.1. The seam exists so v0.2 can add
wgpu without touching games. Do not add a second backend in v0.1 even
if it's tempting — the trait shape must stabilise first.

## 8. Capability probing

### 8.1 When to use it

If a game uses `RenderFeature::CustomShaders` and the backend doesn't
support it, the game should either:

1. Fall back to a simpler rendering path, or
2. Refuse to run and explain why.

The `supports(feature)` method is how the game finds out.

```rust
if ctx.render().supports(RenderFeature::CustomShaders) {
    // shader path (*not yet implemented*: no `use_shader` on the trait)
} else {
    ctx.render().draw_rect(rect, color);  // fallback
}
```

### 8.2 What v0.1 macroquad supports

| Feature | Supported |
|---|---|
| `AlphaBlending` | ✅ |
| `CustomShaders` | flag reported ✅, but the trait has no shader API yet |
| `TextureArray` | ❌ |
| `Instancing` | ❌ (v0.2 candidate) |

`MacroquadAudio` reports no `AudioFeature`s.

A game that queries `Instancing` gets `false` and must take the
slow path. This is honest and correct.

## 9. Passing servers through NodeContext

Servers are borrowed, not stored, inside `NodeContext`. This is what
prevents "I had a reference to the renderer in one node and now it's
gone" bugs.

```rust
// Inside process:
let renderer = ctx.render();          // &mut dyn RenderingServer
renderer.draw_rect(rect, RED);
renderer.draw_rect(rect2, BLUE);
// renderer borrow ends here
```

You can also call `ctx.render().draw_rect(...)` inline. The borrow lives
exactly for that statement.

## 10. Examples

### 10.1 Drawing a sprite

```rust
impl Node for Sprite {
    fn draw(&self, ctx: &mut DrawContext) {
        if let Some(tex) = self.texture.get() {
            let dst = Rect::new(self.pos.x, self.pos.y, self.size.x, self.size.y);
            ctx.render().draw_sprite(tex.id(), dst, None, Color::WHITE);
        }
    }
}
```

### 10.2 Playing a sound effect

```rust
impl Node for Button {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        if ctx.input().primary_action() {
            // `self.click` is an `AudioClipId` obtained earlier from
            // `AudioServer::load_clip`. (`ctx.assets()` and an audio asset
            // loader are *not yet implemented*.)
            ctx.audio().play(self.click, 0.8, 1.0);
        }
    }
}
```

### 10.3 A raycast

```rust
impl Node for Player {
    fn physics_process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        let origin = self.pos;
        let target = origin + self.aim_dir * 100.0;
        if let Some(hit) = ctx.physics().raycast(origin, target, CollisionMask::from_bits(ENEMY_LAYER)) {
            self.hit_point = Some(hit.point);
        }
    }
}
```

## 11. Design notes

### 11.1 Why traits and not singletons

Godot's servers are singletons (`RenderingServer.draw_rect(...)` from
anywhere). This is ergonomic but hostile to testing and multi-threading.
Traits give:

- **Testability.** A test can create a `MockRenderer` that records
  draw calls. No `#[cfg(test)]` branching in the server crate.
- **Thread safety.** In v0.2, `&mut dyn RenderingServer` can become
  `&(dyn RenderingServer + Sync)` for a threaded renderer.
- **Explicitness.** A function that draws takes `&mut dyn RenderingServer`.
  The dependency is in the signature, not hidden in a global.

### 11.2 Why `&mut dyn` and not `&dyn`

Because rendering and audio mutate internal state (batch buffers, sound
lists). Sharing them immutably would require interior mutability
(`Mutex`, `RefCell`), which is slower and *easier to get wrong*. The
borrow checker catches double-rendering at compile time.

If a future backend needs shared read-only access (e.g. for a
compute shader), the trait can grow a `fn read_state(&self) -> ...`
method without changing the borrow structure.

### 11.3 Why the server owns textures, not assets

The `AssetServer` owns `Handle<Texture>`, which is a reference-counted
pointer to a `Texture` struct. The `Texture` struct holds a
`TextureId` — the server's handle. `Handle<T>` is `Arc<RwLock<T>>`, so
`AssetServer::reload` can swap the `TextureId` inside the shared
`Texture` (destroying the old one) without every node updating its
`Handle`. Dropping the last handle does **not** call `destroy_texture`
yet (ADR-026); that is *not yet implemented*.

### 11.4 Why a handle-based `PhysicsServer`

The trait shipped in v0.1 with `LinearPhysics` (ADR-028). It keys shapes
by `u64` handles to keep `NodeId` out of L2 (ADR-017); the scene layer
will own any `NodeId ↔ u64` map.

## 12. Open questions

- **Q1:** Should `RenderingServer` include a `draw_texture_rect` (no
  tint, no src) fast path? *Provisional:* no — `draw_sprite` with `None`
  src and `WHITE` tint is the fast path in macroquad, and the compiler
  inlines it.
- **Q2:** Should `AudioServer::play` return `Result<SoundId, AudioError>`?
  *Provisional:* no. Failed playback is silent
  ([[02-principles#5.4 What is never an error|02-principles §5.4]]).
- **Q3:** Should `RenderingServer` support a custom shader registry?
  *Provisional:* not in v0.1. `RenderFeature::CustomShaders` exists as a
  flag but there is no `use_shader` method; a shader pipeline is v0.2.
- **Q4:** Should the trait be `Send + Sync`? *Provisional:* no, `&mut`
  is enough. Add `Send + Sync` bounds when threading arrives.

## 13. Cross-references

- How `NodeContext` carries servers → [[10-nodes-and-scenes#2.4 NodeContext|10-nodes-and-scenes.md §2.4]]
- Full trait definitions → [[22-ember-servers]]
- The macroquad renderer → [[23-ember-render-macroquad]]
- Asset handles and textures → [[13-assets]]
- Why traits over singletons (ADR) → [[50-design-decisions]]
