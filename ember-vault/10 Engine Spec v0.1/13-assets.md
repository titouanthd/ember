---
title: "13 — Assets"
aliases: ["13 — Assets"]
tags: ["ember/spec", "ember/wave-2"]
doc-status: "draft"
former-path: "docs/engine/v0.1/13-assets.md"
imported: "2026-11-22"
wave: "2 — Concepts"
---

# 13 — Assets

> **Status:** ⬜ draft
> **Audience:** game authors, implementers
> **Prerequisites:** [[10-nodes-and-scenes]], [[12-servers]]

## 1. Purpose

Explain how Ember loads, references, and reloads assets (textures, fonts,
audio clips, scenes, level data). This is the file you read when you need
to draw a sprite from a PNG, play an OGG, or load a `.escene`.

The core abstraction is the **`Handle<T>`** — a reference-counted pointer
that keeps an asset alive and lets the engine unload it when no one is
using it.

## 2. Why an AssetServer

Games currently load assets ad-hoc:

```rust
// today
let texture = Texture2D::from_file_with_format(
    include_bytes!("assets/player.png"), None,
);
```

This has three problems:

1. **No sharing.** Two nodes loading the same PNG get two textures.
2. **No unloading.** Textures live until the process exits.
3. **No hot-reload.** Editing a PNG requires a game restart.

An `AssetServer` solves all three by owning a cache:

- `load("player.png")` returns a `Handle<Texture>` — a cheap `Rc`-like
  pointer to a cached `Texture`.
- The cache keeps the asset alive while any `Handle` exists.
- Editing the file on disk, in dev builds, updates the texture in place.

## 3. Handle<T>

### 3.1 What a Handle is

```rust
pub struct Handle<T: Asset> {
    id: AssetId,
    _phantom: PhantomData<T>,
}
```

A `Handle<T>` is:

- **Copy?** No — cloning it bumps the internal refcount. It's `Clone`
  but not `Copy`.
- **Comparable.** `==` compares the underlying `AssetId`.
- **Cheap.** An `AssetId` is a `u64`; cloning a handle is an `Arc::clone`
  plus a bit.

### 3.2 What a Handle is not

- Not a `&T`. You cannot pass it where `&T` is expected.
- Not a reference. It doesn't borrow; it owns a share of the refcount.
- Not nullable. A missing asset is an `Option<Handle<T>>` from `load`,
  or a `Handle<T>` whose `get()` returns `None` during async loading.

### 3.3 Accessing the underlying asset

```rust
let handle: Handle<Texture> = assets.texture("player.png")?;
if let Some(tex) = handle.get() {
    ctx.render().draw_sprite(tex.id(), dst, None, WHITE);
}
```

`get()` returns `Option<&T>`. It is `None` only during async loading.
In synchronous loading, `get()` is always `Some` right after `load`.

## 4. Strong vs weak handles

### 4.1 `Handle<T>` — strong

Keeps the asset alive. As long as any `Handle<T>` exists, the asset
stays in the cache.

### 4.2 `WeakHandle<T>` — weak

Does not keep the asset alive. Upgrading it (`upgrade()`) returns
`Option<Handle<T>>` — `None` if the asset was unloaded.

```rust
let weak: WeakHandle<Texture> = handle.downgrade();
if let Some(strong) = weak.upgrade() {
    // asset is still alive
}
```

Use weak handles when:

- A node caches a texture but should not prevent unloading.
- A background task references an asset that may be reloaded.

For the 80% case, `Handle<T>` is what you want.

## 5. Loading

### 5.1 Synchronous (desktop, blocking)

```rust
let texture: Handle<Texture> = assets.load("player.png")?;
```

Blocks until the file is read and uploaded. Fine for small assets and
for desktop. Returns `Err(AssetError)` if the file is missing or
malformed.

### 5.2 Asynchronous (web, large assets)

```rust
let texture_handle: Handle<Texture> = assets.load_async("player.png");
// handle.get() is None until loaded
```

Non-blocking. Returns immediately with a handle in a "loading" state.
`handle.get()` returns `None` until the asset is ready. On web, this is
the only option — synchronous file reads block the entire page.

### 5.3 Error handling

```rust
match assets.load::<Texture>("player.png") {
    Ok(handle) => { /* use handle */ }
    Err(AssetError::NotFound(path)) => {
        log::warn!("missing texture: {path}");
        // use a fallback handle
    }
    Err(e) => return Err(e.into()),
}
```

`AssetServer::load` returns `Result<Handle<T>, AssetError>` for the
synchronous path, and `Handle<T>` (with an `is_ready()` check) for the
async path.

### 5.4 Caching semantics

Two calls with the same path return handles to the **same** asset:

```rust
let a = assets.load::<Texture>("player.png")?;
let b = assets.load::<Texture>("player.png")?;
assert_eq!(a.id(), b.id());   // same underlying texture
```

This is what makes the cache useful — a level with 30 sprites sharing a
texture gets one upload, not 30.

### 5.5 Paths

- Relative to the game's `assets/` directory.
- Forward slashes only (even on Windows).
- Case-sensitive.
- Leading slashes are rejected (`"/player.png"` is invalid; use
  `"player.png"`).

## 6. UID registry

### 6.1 What UIDs are for

A **UID** is a stable identifier for an asset. `.escene` files reference
assets by UID, not by path:

```ron
(
    ext_resources: [
        (type: "Texture", uid: "uid://player_a3b1", path: "sprites/player.png"),
    ],
)
```

This means renaming or moving the PNG does not break every scene that
uses it — only the path in the asset's own entry needs updating.

### 6.2 Format

```
uid://<base36-encoded 64-bit hash>
```

Example: `uid://player_a3b1c` — the `player_` prefix is convention (not
enforced), the `a3b1c` is a hash of the file's content on first
registration.

### 6.3 UID vs path

| | UID | Path |
|---|---|---|
| Stable under rename | ✅ | ❌ |
| Readable | ❌ (`a3b1c`) | ✅ (`sprites/player.png`) |
| Human-authorable | ❌ | ✅ |
| VCS-friendly | ✅ (no churn on move) | ❌ (every reference updates) |

**In `.escene` files, use UIDs.** In code, use paths — they are readable
and the `AssetServer` resolves them to UIDs internally.

### 6.4 When UIDs are assigned

- First load of a file: `AssetServer` hashes content, registers UID, writes
  it to `assets/uid_registry.ron`.
- Subsequent loads: reads the registry, uses the stored UID.
- File content changes (hot-reload): UID stays the same, only the
  resource is refreshed.

The registry is committed to VCS. It is the source of truth for "which
UID does this path map to".

## 7. Format detection and loaders

### 7.1 By extension

| Extension | Loader | Asset type |
|---|---|---|
| `.png`, `.jpg` | `ImageLoader` | `Texture` |
| `.ogg`, `.wav` | `AudioLoader` | `AudioClip` |
| `.ttf`, `.otf` | `FontLoader` | `Font` |
| `.escene` | `SceneLoader` | `PackedScene` |
| `.ron` | `RonLoader` | `Level` (generic) |

New loaders are registered at startup:

```rust
assets.register_loader::<MyCustomAsset>("custom", CustomLoader);
```

### 7.2 Loader trait

```rust
pub trait AssetLoader<T: Asset>: Send + Sync + 'static {
    fn extensions() -> &'static [&'static str];
    fn load(path: &Path, bytes: Vec<u8>) -> Result<T, AssetError>;
    fn reload(current: &mut T, bytes: Vec<u8>) -> Result<(), AssetError>;
}
```

Each loader parses the raw bytes into the asset type. The `reload`
method supports hot-reload: swap the asset's contents in place.

## 8. Hot-reload

### 8.1 When it is available

- Enabled in **dev builds only** (`cfg(debug_assertions)`).
- Enabled for **assets** (textures, audio, fonts, scenes, levels).
- **Not** enabled for **code** (Rust recompilation still requires a
  restart, in v0.1).

### 8.2 How it works

A file watcher (`notify` crate) runs on a background thread. When a file
under `assets/` changes:

1. Watcher sends a message to the `AssetServer` (a channel).
2. `AssetServer::poll()` (called once per frame by the `MainLoop`)
   dequeues the message.
3. Loader re-reads the file, calls `reload()` on the existing asset.
4. The `Handle<T>`s held by nodes now see the updated asset.

For textures, `reload` re-uploads to the GPU. For scenes, `reload`
replaces the `PackedScene` — new instances reflect the change, existing
ones do not.

### 8.3 Disabling hot-reload

```rust
// in main
let assets = AssetServer::builder()
    .watch(false)   // disable hot-reload
    .build("assets/");
```

Hot-reload is off by default in release builds, on by default in debug
builds. The explicit `.watch(false)` disables it even in debug — useful
for benchmarks.

## 9. Examples

### 9.1 Loading and using a texture

```rust
#[derive(Node)]
#[class_name = "Sprite"]
pub struct Sprite {
    #[export] texture_path: String,
    #[export] size: Vec2,
    texture: Option<Handle<Texture>>,
}

impl Node for Sprite {
    fn ready(&mut self, ctx: &mut NodeContext) {
        self.texture = ctx.assets().load::<Texture>(&self.texture_path).ok();
    }

    fn draw(&self, ctx: &mut DrawContext) {
        if let Some(handle) = &self.texture {
            if let Some(tex) = handle.get() {
                let dst = Rect::new(0.0, 0.0, self.size.x, self.size.y);
                ctx.render().draw_sprite(tex.id(), dst, None, WHITE);
            }
        }
    }
}
```

### 9.2 A loading screen

```rust
#[derive(Node)]
#[class_name = "LoadingScreen"]
pub struct LoadingScreen {
    pending: Vec<WeakHandle<Texture>>,
}

impl Node for LoadingScreen {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        self.pending.retain(|h| h.upgrade().is_none());
        if self.pending.is_empty() {
            ctx.emit::<LoadingComplete>(LoadingComplete);
        }
    }
}
```

### 9.3 Scenes referencing assets

`.escene` files reference assets by UID (see §6). At instantiation, the
loader resolves each UID to a `Handle` and injects it into the node's
`#[export]` field.

## 10. Design notes

### 10.1 Why refcounted handles, not a global cache

A global cache without handles leaks: it holds every asset forever.
Handles make unloading natural — the cache drops an asset when its last
handle drops. This is the same design `wgpu` uses for `TextureView`, and
it's what makes `drop(player_node)` also `drop(player_texture)`.

### 10.2 Why UIDs even though paths work

Because renaming a file is the single most common refactor in a game
project. Every `.escene` that references `sprites/player_v1.png` must
update when it becomes `sprites/characters/player.png`. UIDs absorb the
change in one place.

### 10.3 Why hot-reload is asset-only in v0.1

Code hot-reload (hot-swapping Rust functions) requires `dylib`
reloading, which brings `unsafe`, ABI compatibility, and platform
divergence. It is a v0.2+ feature. Asset hot-reload covers 90% of the
iteration pain (sprites, audio, levels), which is why it ships first.

### 10.4 Why not expose paths on `Handle<T>`

If `Handle<T>` had a `path()` method, games would use it for
comparisons (`if handle.path() == "player.png"`), which breaks when the
asset moves. Comparisons must go through `AssetId` (which handles
already support via `==`).

## 11. Open questions

- **Q1:** Should `load` block in `ready` for large assets? *Provisional:*
  no — `ready` should call `load_async`, and `draw` should check
  `handle.get()`. Synchronous loading is only for `main`, not for nodes.
- **Q2:** Should the UID registry be a separate file, or embedded in each
  asset? *Provisional:* separate file (`assets/uid_registry.ron`), so it
  can be merged in version control.
- **Q3:** Should hot-reload of a scene reset live instances? *Provisional:*
  no — only new instances reflect the change. Live instances keep their
  state. Forcing a reset is a v0.2 "reload scene" button in the inspector.
- **Q4:** How are UID collisions handled? *Provisional:* on registration,
  if the hash collides with an existing UID for a different path, the
  server bumps the hash and logs a warning. Collisions are astronomically
  rare with 64-bit hashes; the fallback exists for correctness.

## 12. Cross-references

- Servers that consume assets → [[12-servers]]
- `.escene` format and UID usage → [[30-escene-format]]
- Full `AssetServer` API → [[25-ember-assets]]
- Hot-reload testing strategy → [[43-testing-strategy]]
