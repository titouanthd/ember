---
title: "13 — Assets"
aliases: ["13 — Assets"]
tags: ["ember/spec", "ember/branch-snapshot", "ember/wave-2"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/13-assets.md"
imported: "2026-11-22"
wave: "2 — Concepts"
---

# 13 — Assets

> **Status:** 🟡 partial — reviewed 2026-11-19; synchronous cached loading, `Handle`/`WeakHandle`, UID registry, PNG/RON/scene assets work; async loading, hot-reload watcher, audio/font loaders and `ctx.assets()` are not implemented.
> **Audience:** game authors, implementers
> **Prerequisites:** [[10-nodes-and-scenes (branch)]], [[12-servers (branch)]]

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

- `load_with::<Texture>("player.png", renderer)` (or `texture(path, renderer)`)
  returns a `Handle<Texture>` — a cheap `Arc`-like pointer to a cached
  `Texture`.
- The cache keeps the asset alive while any `Handle` exists.
- `reload` updates a loaded asset in place. *Not yet implemented:*
  automatic reload when the file changes on disk (§8).

> **Implementation note (ADR-026).** v0.1 `AssetServer` is synchronous and
> type-driven. Details below that differ from the original design are
> marked.

## 3. Handle<T>

### 3.1 What a Handle is

```rust
pub struct Handle<T: Asset> {
    id: AssetId,
    uid: Uid,
    arc: Arc<RwLock<T>>,
}
```

A `Handle<T>` is:

- **Copy?** No — cloning it bumps the internal refcount. It's `Clone`
  but not `Copy`.
- **Comparable.** `==` compares the underlying `AssetId`.
- **Cheap.** An `AssetId` is a `u64`; cloning a handle is an `Arc::clone`
  plus a `Uid` string clone.
- **Identifiable.** `id()` returns the runtime `AssetId`, `uid()` the
  persistent `Uid`.

### 3.2 What a Handle is not

- Not a `&T`. You cannot pass it where `&T` is expected.
- Not a reference. It doesn't borrow; it owns a share of the refcount.
- Not nullable. A missing asset is an `Err(AssetError)` from `load`.

### 3.3 Accessing the underlying asset

```rust
let handle: Handle<Texture> = assets.texture("player.png", renderer)?;
let tex = handle.get();   // read guard
ctx.render().draw_sprite(tex.id, dst, None, Color::WHITE);
```

`get()` returns an `AssetGuard<'_, T>` (a `RwLockReadGuard`), always
valid; a reload waits for the guard to drop. `Texture` exposes public
fields `id`, `width`, `height`. Do not hold the guard across a reload.

## 4. Strong vs weak handles

### 4.1 `Handle<T>` — strong

Keeps the asset alive. As long as any `Handle<T>` exists, the asset
stays in the cache.

### 4.2 `WeakHandle<T>` — weak

Does not keep the asset alive. Upgrading it (`upgrade()`) returns
`Option<Handle<T>>` — `None` if every strong handle dropped.

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
// CPU-only assets (scenes, RON):
let stats: Handle<RonAsset<Stats>> = assets.load("stats.ron")?;
// GPU assets take the renderer (ADR-026):
let texture: Handle<Texture> = assets.load_with("player.png", renderer)?;
let texture = assets.texture("player.png", renderer)?; // same thing
```

Blocks until the file is read (and uploaded). `load::<Texture>` without a
renderer fails with `AssetError::NeedsRenderer`. Returns
`Err(AssetError)` if the file is missing or malformed. `load_by_uid` /
`load_by_uid_with` load by `Uid`.

### 5.2 Asynchronous (web, large assets)

*Not yet implemented.* There is no `load_async`; v0.1 loads are
synchronous (ADR-026). The sketch below is the planned design:

```rust
let texture_handle: Handle<Texture> = assets.load_async("player.png");
// handle.get() is None until loaded
```

Non-blocking. Returns immediately with a handle in a "loading" state.
On web, this will be the only option — synchronous file reads block the
entire page.

### 5.3 Error handling

```rust
match assets.texture("player.png", renderer) {
    Ok(handle) => { /* use handle */ }
    Err(AssetError::NotFound(path)) => {
        // use a fallback handle
    }
    Err(e) => return Err(e.into()),
}
```

`load` / `load_with` / `texture` return `Result<Handle<T>, AssetError>`.
`AssetError` variants: `NotFound`, `Io`, `InvalidPath`, `WrongType`,
`Parse`, `InvalidUid`, `UnknownUid`, `NeedsRenderer`, `Server`.

### 5.4 Caching semantics

Two calls with the same path return handles to the **same** asset:

```rust
let a = assets.texture("player.png", renderer)?;
let b = assets.texture("player.png", renderer)?;
assert_eq!(a.id(), b.id());   // same underlying texture
```

This is what makes the cache useful — a level with 30 sprites sharing a
texture gets one upload, not 30. The cache holds a `Weak` per path;
loading a path already cached as a different asset type is
`AssetError::WrongType`. `poll()` prunes entries whose handles are all
gone.

### 5.5 Paths

- Relative to the game's `assets/` directory.
- Forward slashes only (even on Windows).
- Case-sensitive (extension matching is case-insensitive).
- Absolute paths, `.`/`..` components and empty paths are rejected
  (`AssetError::InvalidPath`).

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
uid://<file stem>_<5 hex digits of content hash>
```

Example: `uid://player_a3b1c` — the stem is the file name without
extension (whitespace becomes `_`), the hex suffix is the low 20 bits of
an FNV-1a hash of the file's content on first registration.
`Uid::parse` only requires `uid://` followed by a non-empty string with
no whitespace.

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
  it to `uid_registry.ron` in the asset root.
- Subsequent loads: reads the registry, uses the stored UID.
- File content changes (hot-reload): UID stays the same, only the
  resource is refreshed.

`UidRegistry::rename(old, new)` moves a path while keeping its UID (the
caller must `save`). The registry is committed to VCS. It is the source of truth for "which
UID does this path map to".

## 7. Format detection and loaders

### 7.1 By extension

v0.1 is type-driven (ADR-026): the requested type's `Asset::extensions()`
must contain the file's extension, else `WrongType`.

| Extension | Asset type | Status |
|---|---|---|
| `.png` | `Texture` | ✅ (needs a renderer; JPEG *not yet implemented*) |
| `.escene` | `PackedScene` (impl in `ember-scene`) | ✅ via `SceneAssets::scene` |
| `.ron` | `RonAsset<T>` (any `DeserializeOwned`) | ✅ |
| `.ogg`, `.wav` | `AudioClip` | *Not yet implemented* (use `AudioServer::load_clip`) |
| `.ttf`, `.otf` | `Font` | *Not yet implemented* (use `RenderingServer::create_font`) |

There is no loader registry; a custom asset is a type implementing
`Asset`. *Not yet implemented:* `register_loader`.

### 7.2 Asset trait

```rust
pub trait Asset: Send + Sync + Sized + 'static {
    fn extensions() -> &'static [&'static str];
    fn load(bytes: &[u8], ctx: &mut AssetLoadContext) -> Result<Self, AssetError>;
    fn reload(current: &mut Self, bytes: &[u8], ctx: &mut AssetLoadContext)
        -> Result<(), AssetError> { /* default: replace */ }
}
```

`AssetLoadContext` carries `path`, `uid` and `rendering:
Option<&mut dyn RenderingServer>`. The `reload` method swaps the asset's
contents in place (`Texture::reload` destroys the old GPU texture).

## 8. Hot-reload

> *Not yet implemented.* There is no file watcher, `notify` dependency,
> `watch(false)` builder, or per-frame `poll()` reload. What exists:
> `AssetServer::reload::<T>(path, Option<&mut dyn RenderingServer>)`
> re-reads a loaded asset and updates it in place (every handle sees the
> new data), called manually. `poll()` only prunes dead cache entries.
> The rest of this section is the planned design.

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
        // `ctx.assets()` is *not yet implemented*; today the game
        // loads through an `AssetServer` it owns, with the renderer:
        self.texture = self.assets.texture(&self.texture_path, ctx.render()).ok();
    }

    fn draw(&self, ctx: &mut DrawContext) {
        if let Some(handle) = &self.texture {
            let tex = handle.get();
            let dst = Rect::new(0.0, 0.0, self.size.x, self.size.y);
            ctx.render().draw_sprite(tex.id, dst, None, Color::WHITE);
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

`.escene` files reference assets by UID (see §6). *Not yet implemented:*
`ext_resources` are parsed and kept, but not resolved to `Handle`s, and
`#[export]` does not exist yet. Nested scenes (`instance:`) are loaded
through `SceneAssets::scene` by `PackedScene::instantiate_with` (ADR-027).

## 10. Design notes

### 10.1 Why refcounted handles, not a global cache

A global cache without handles leaks: it holds every asset forever.
Handles make unloading natural — the cache drops an asset when its last
handle drops. This is the same design `wgpu` uses for `TextureView`, and
it's what makes `drop(player_node)` also `drop(player_texture)` (the
cache entry; GPU `destroy_texture` on drop is *not yet implemented*,
ADR-026).

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
  no — `ready` should call `load_async` (*not yet implemented*). Until
  then, load synchronously in `main`.
- **Q2:** Should the UID registry be a separate file, or embedded in each
  asset? *Provisional:* separate file (`uid_registry.ron` in the asset root), so it
  can be merged in version control.
- **Q3:** Should hot-reload of a scene reset live instances? *Provisional:*
  no — only new instances reflect the change. Live instances keep their
  state. Forcing a reset is a v0.2 "reload scene" button in the inspector.
- **Q4:** How are UID collisions handled? *Provisional:* on registration,
  if the candidate UID is already taken, the registry increments the hash
  until it is free (no warning is logged). With only 20 hash bits in the
  suffix, collisions across same-named files are plausible, so the
  fallback matters.

## 12. Cross-references

- Servers that consume assets → [[12-servers (branch)]]
- `.escene` format and UID usage → [[30-escene-format (branch)]]
- Full `AssetServer` API → [[25-ember-assets (branch)]]
- Hot-reload testing strategy → [[43-testing-strategy (branch)]]
