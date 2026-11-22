---
title: "12 — Servers"
aliases: ["12 — Servers"]
tags: ["ember/spec", "ember/wave-2"]
doc-status: "draft"
former-path: "docs/engine/v0.1/12-servers.md"
imported: "2026-11-22"
wave: "2 — Concepts"
---

# 12 — Servers

> **Status:** ⬜ draft
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

Every one of these becomes a code change in 18 games, × every draw call.
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

    Main->>R: create(RenderingBackend::Macroquad)
    Main->>A: create(AudioBackend::Macroquad)
    Main->>P: create(PhysicsBackend::Collider)
    Main->>Loop: new(game, R, A, P)
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
`MacroquadAudio`, `ColliderPhysics`), but once created they are stored as
`Box<dyn Trait>` and passed around by trait object.

### 3.2 Ownership

The `MainLoop` owns the servers for its lifetime. Each `NodeContext`
borrows them, one level deep:

```rust
pub struct NodeContext<'a> {
    rendering: &'a mut dyn RenderingServer,
    audio: &'a mut dyn AudioServer,
    // physics not in v0.1; see §6
    // ...
}
```

Because `NodeContext` carries `&mut`, you **cannot** call
`ctx.render().draw_sprite(...)` from two places at once — which is what
prevents race conditions in the future thread pool.

### 3.3 Destruction

Servers drop after `MainLoop::finalize()`. Nothing in a node's
`finalize` should try to use a server — the servers are already
unborrowed at that point, and calling `ctx.render()` would fail to
compile (no `ctx` in `finalize`'s signature).

## 4. RenderingServer

### 4.1 Trait definition

```rust
pub trait RenderingServer {
    // --- Frame ---
    fn begin_frame(&mut self, viewport: Rect);
    fn end_frame(&mut self);
    fn set_clear_color(&mut self, color: Color);

    // --- Camera ---
    fn set_camera(&mut self, offset: Vec2);
    fn camera_offset(&self) -> Vec2;

    // --- Z ordering ---
    fn set_z(&mut self, z: f32);
    fn current_z(&self) -> f32;

    // --- Primitives ---
    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_rect_outline(&mut self, rect: Rect, thickness: f32, color: Color);
    fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color);
    fn draw_line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: Color);

    // --- Sprites ---
    fn draw_sprite(&mut self, texture: TextureId, dst: Rect, src: Option<Rect>, tint: Color);

    // --- Text ---
    fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color);

    // --- Textures ---
    fn create_texture(&mut self, image: &Image) -> TextureId;
    fn destroy_texture(&mut self, id: TextureId);

    // --- Capability probing ---
    fn supports(&self, feature: RenderFeature) -> bool;
}

#[non_exhaustive]
pub enum RenderFeature {
    AlphaBlending,
    CustomShaders,
    TextureArray,
    // v0.2: Instancing, Compute, ...
}
```

### 4.2 Naming convention

- `draw_*` — immediate draw call, resets at `end_frame`
- `set_*` — changes persistent state (camera, z, clear color)
- `create_*` / `destroy_*` — resource lifecycle
- `supports` — capability probe

### 4.3 Batch semantics

`begin_frame` opens a batching window. `end_frame` flushes it. Within a
batch, the server may reorder draw calls if ordering constraints allow.
The one hard constraint: **`draw_*` calls are visible in their call
order** within a z-level.

This matches macroquad's model and is what makes the trait implementable
on both immediate-mode backends (macroquad) and retained-mode backends
(wgpu, where batching is explicit).

### 4.4 Texture lifecycle

`create_texture` returns a `TextureId`, an opaque handle. The server
owns the underlying GPU resource. `destroy_texture` is optional — the
server drops unreferenced textures when the `AssetServer` drops the
corresponding `Handle<Texture>`.

Games never call `create_texture` directly. They go through the
`AssetServer` (see [[13-assets]]). The direct API exists
for the `AssetServer` itself to use.

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

    // --- Capability probing ---
    fn supports(&self, feature: AudioFeature) -> bool;
}

#[non_exhaustive]
pub enum AudioFeature {
    PitchShift,
    Streaming,
    PositionalAudio,
    // v0.2: Effects
}
```

### 5.2 Playback model

`play` returns a `SoundId`, which is a handle you can `stop` with.
Finished sounds (`SoundId`s that completed naturally) are auto-removed
from the server's active list; calling `stop` on a finished sound is a
**no-op** (see [[02-principles#5.4 What is never an error|02-principles §5.4]]).

### 5.3 Why no `loop` parameter

Loops are game logic, not server logic. A game that wants looping music
calls `play` from its `process` when `is_playing(music_id)` returns
false. Or it uses a `Loop` wrapper in the game crate. The server stays
simple.

## 6. PhysicsServer

### 6.1 Status

**Deferred to v0.2.** The current `ember_stdlib::collider` module covers
what the 18 games need (AABB overlap, circle-vs-circle, circle-vs-AABB,
swept AABB). It is a pure function library, not a server.

The `PhysicsServer` trait is sketched here so the seam exists, but the
v0.1 implementation is a thin shim over `collider`:

```rust
pub trait PhysicsServer {
    fn query_overlaps(&self, shape: &Shape, mask: CollisionMask) -> Vec<NodeId>;
    fn raycast(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Option<RayHit>;
    fn register(&mut self, id: NodeId, shape: Shape, mask: CollisionMask);
    fn unregister(&mut self, id: NodeId);
    fn rebuild(&mut self);  // recompute spatial index
}

pub struct RayHit {
    pub node: NodeId,
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}
```

### 6.2 Why a trait if `collider` is enough

Because some games (bullet-hell, asteroids) do collision resolution
hundreds of times per frame. A `Vec<(NodeId, Shape)>` linear scan at 300
entities is O(n²). A spatial hash makes it O(n·k). The trait leaves
room for `PhysicsBackend::SpatialHash` in v0.2 without changing game
code.

For v0.1, `PhysicsBackend::Linear` walks the registry and calls
`collider::overlaps`. It is what the games already do.

## 7. Backend registration

### 7.1 Creation API

```rust
// At the top of the game's main():
let rendering: Box<dyn RenderingServer> =
    Box::new(MacroquadRenderer::new().await);
let audio: Box<dyn AudioServer> =
    Box::new(MacroquadAudio::new());
let physics: Box<dyn PhysicsServer> =
    Box::new(LinearPhysics::new());

// The MainLoop takes them.
let mut loop_ = MainLoop::new::<MyGame>(rendering, audio, physics);
loop_.run().await;
```

Games never see `MacroquadRenderer` — they only see `Box<dyn RenderingServer>`.
Switching backends is a one-line change in `main`.

### 7.2 Backend crate convention

- `ember_render_macroquad` exports `MacroquadRenderer`
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
    ctx.render().use_shader(...);
} else {
    ctx.render().draw_rect(...);  // fallback
}
```

### 8.2 What v0.1 macroquad supports

| Feature | Supported |
|---|---|
| `AlphaBlending` | ✅ |
| `CustomShaders` | ✅ (via `macroquad::material`) |
| `TextureArray` | ❌ |
| `Instancing` | ❌ (v0.2 candidate) |

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
            ctx.render().draw_sprite(tex.id(), dst, None, WHITE);
        }
    }
}
```

### 10.2 Playing a sound effect

```rust
impl Node for Button {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        if ctx.input().primary_action() {
            let clip = ctx.assets().audio("click.ogg");
            if let Some(clip) = clip.get() {
                ctx.audio().play(clip.id(), 0.8, 1.0);
            }
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
        if let Some(hit) = ctx.physics().raycast(origin, target, CollisionMask::ENEMY) {
            self.target = Some(hit.node);
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
`TextureId` — the server's handle. When the last `Handle<Texture>`
drops, the `Texture` drops, calling `rendering.destroy_texture(id)`.

This split (asset owns the handle, server owns the resource) means
reloading an asset (`AssetServer::reload(path)`) can swap the
`TextureId` without every node needing to update its `Handle`.

### 11.4 Why the deferred `PhysicsServer`

Because the games don't need it yet. Shipping a trait we don't use is
worse than deferring it. The `collider` module covers 18/18 games today;
the trait lands when a game needs spatial indexing.

## 12. Open questions

- **Q1:** Should `RenderingServer` include a `draw_texture_rect` (no
  tint, no src) fast path? *Provisional:* no — `draw_sprite` with `None`
  src and `WHITE` tint is the fast path in macroquad, and the compiler
  inlines it.
- **Q2:** Should `AudioServer::play` return `Result<SoundId, AudioError>`?
  *Provisional:* no. Failed playback is silent
  ([[02-principles#5.4 What is never an error|02-principles §5.4]]).
- **Q3:** Should `RenderingServer` support a custom shader registry?
  *Provisional:* v0.1 exposes `use_shader(&str)` for named shaders
  registered by the game. This is a stub; a full shader pipeline is v0.2.
- **Q4:** Should the trait be `Send + Sync`? *Provisional:* no, `&mut`
  is enough. Add `Send + Sync` bounds when threading arrives.

## 13. Cross-references

- How `NodeContext` carries servers → [[10-nodes-and-scenes#2.4 NodeContext|10-nodes-and-scenes.md §2.4]]
- Full trait definitions → [[22-ember-servers]]
- The macroquad renderer → [[23-ember-render-macroquad]]
- Asset handles and textures → [[13-assets]]
- Why traits over singletons (ADR) → [[50-design-decisions]]
