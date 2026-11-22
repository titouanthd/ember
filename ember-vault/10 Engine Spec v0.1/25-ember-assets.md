---
title: "25 — `ember_assets`"
aliases: ["25 — `ember_assets`"]
tags: ["ember/spec", "ember/wave-3"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/25-ember-assets.md"
imported: "2026-11-22"
wave: "3 — Crate Specifications"
---

# 25 — `ember_assets`

> **Status:** 🟡 partial — reviewed 2026-11-19; `AssetServer`, handles, UID registry, `Texture`/`RonAsset`/`PackedScene` loading work (ADR-026); loader registry, async loading, hot-reload and audio/font assets are not implemented.
> **Audience:** implementers
> **Prerequisites:** [[13-assets]], [[22-ember-servers]]

## 1. Purpose

Specify the `AssetServer` API: `Handle<T>`, `WeakHandle<T>`, `AssetId`,
the `Asset` trait, UID registry and error types. This is the crate that
games use to load textures and RON/scene data. Fonts and audio clips are
created directly through the servers for now (§5.3). Hot-reload watching,
async loading and a loader registry are *Not yet implemented* (ADR-026).

## 2. Module layout

```
ember-assets/src/
├── lib.rs          re-exports + prelude
├── asset.rs        Asset trait, AssetLoadContext, Texture, RonAsset<T>
├── server.rs       AssetServer
├── handle.rs       Handle, WeakHandle, AssetGuard
├── id.rs           AssetId, Uid
├── uid_registry.rs UidRegistry
└── error.rs        AssetError
```

*Not yet implemented:* `loader.rs` / `loaders/*` (loader registry),
`hot_reload.rs`. Loading is driven by the `Asset` trait itself, and
`PackedScene` implements `Asset` in `ember-scene` (ADR-026). The prelude
exports `Asset`, `AssetError`, `AssetServer`, `Handle`, `RonAsset`,
`Texture`, `Uid`.

## 3. `AssetId` and `Uid`

### 3.1 `AssetId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct AssetId(u64);

impl AssetId {
    pub fn invalid() -> Self;
    pub fn is_valid(self) -> bool;
    // `new` is pub(crate): only the server issues ids
}
```

Runtime handle. Assigned by the server when an asset is first loaded.
Stable for the lifetime of the server process only.

### 3.2 `Uid`

```rust
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Uid(String);   // e.g. "uid://player_a3b1c"

impl Uid {
    pub fn parse(s: &str) -> Result<Self, AssetError>;
    pub fn as_str(&self) -> &str;
}

impl std::fmt::Display for Uid { /* prints uid://... */ }
```

Persistent identifier. Written to `assets/uid_registry.ron`. Used in
`.escene` files to reference assets by stable name.

### 3.3 Why both

`AssetId` is a fast integer for runtime comparisons. `Uid` is a
persistent string for serialized references. Each serves a different
purpose; conflating them would compromise one.

## 4. `Handle<T>` and `WeakHandle<T>`

### 4.1 `Handle<T>`

```rust
pub struct Handle<T: Asset> {
    id: AssetId,
    uid: Uid,
    arc: Arc<RwLock<T>>,   // shared with WeakHandle and the cache
}

impl<T: Asset> Handle<T> {
    pub fn id(&self) -> AssetId;
    pub fn uid(&self) -> &Uid;
    pub fn get(&self) -> AssetGuard<'_, T>;   // read lock, never `None`
    pub fn downgrade(&self) -> WeakHandle<T>;
}

impl<T: Asset> Clone for Handle<T> { /* clones Arc */ }
impl<T: Asset> PartialEq for Handle<T> { /* compares AssetId */ }
```

Assets are loaded synchronously, so a `Handle` always holds loaded data:
there is no `is_ready` and no custom `Drop`.

### 4.2 `AssetGuard<'a, T>`

```rust
pub type AssetGuard<'a, T> = std::sync::RwLockReadGuard<'a, T>;
```

`handle.get()` returns a read guard that derefs to `T`; it prevents the
asset from being swapped out mid-read (`reload` takes the write lock).

### 4.3 `WeakHandle<T>`

```rust
pub struct WeakHandle<T: Asset> { /* id, Weak<RwLock<T>> */ }

impl<T: Asset> WeakHandle<T> {
    pub fn id(&self) -> AssetId;
    pub fn upgrade(&self) -> Option<Handle<T>>;
}
```

Does not keep the asset alive. `upgrade()` returns `None` once every
strong handle has dropped.

### 4.4 Clone and equality semantics

- `Handle::clone` — `Arc` increment, O(1).
- `Handle ==` — compares `AssetId`, not underlying data.
- Dropping the last `Handle` frees the asset's memory immediately; the
  cache only holds a `Weak`, and the stale entry is pruned by
  `AssetServer::poll()`. There is no drop hook, so GPU resources (e.g.
  textures) are **not** released automatically: call
  `RenderingServer::destroy_texture` (ADR-026).

## 5. `Asset` trait

### 5.1 Trait

```rust
pub trait Asset: Send + Sync + Sized + 'static {
    fn extensions() -> &'static [&'static str];
    fn load(bytes: &[u8], ctx: &mut AssetLoadContext) -> Result<Self, AssetError>;
    /// Called by `AssetServer::reload`. Default: replace with `load`.
    fn reload(current: &mut Self, bytes: &[u8], ctx: &mut AssetLoadContext)
        -> Result<(), AssetError>;
}
```

### 5.2 `AssetLoadContext`

```rust
pub struct AssetLoadContext<'a> {
    pub path: &'a Path,
    pub uid: &'a Uid,
    pub rendering: Option<&'a mut dyn RenderingServer>,
}
```

`rendering` is `Some` only through the `*_with` / `texture` entry points
(GPU assets). Loading a `Texture` through plain `load` fails with
`AssetError::NeedsRenderer`. There is no `audio` or `class_db` in the
context.

### 5.3 Built-in assets

| Asset type | Format | Notes |
|---|---|---|
| `Texture { id: TextureId, width, height }` | PNG only | via the `png` crate; `reload` destroys the old GPU texture |
| `RonAsset<T>(pub T)` | `.ron` | |
| `PackedScene` (in `ember-scene`) | `.escene` | |

*Not yet implemented:* JPG, `AudioClip` / `Font` assets (use
`RenderingServer::create_font` and `AudioServer::load_clip` with bytes
read by the game). Custom assets implement `Asset`; no registration step
is needed.

## 6. `AssetServer`

### 6.1 Struct

Private state behind a mutex: `root`, a cache keyed by normalised path
(each entry a `Weak` plus `TypeId`), the next `AssetId`, and the
`UidRegistry`.

### 6.2 Construction

```rust
impl AssetServer {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, AssetError>;
    pub fn root(&self) -> &Path;
}
```

*Not yet implemented:* `AssetServerBuilder`, `watch`, `register_loader`.

### 6.3 Loading

```rust
impl AssetServer {
    pub fn load<T: Asset>(&self, path: &str) -> Result<Handle<T>, AssetError>;
    pub fn load_with<T: Asset>(&self, path: &str,
        rendering: &mut dyn RenderingServer) -> Result<Handle<T>, AssetError>;
    pub fn load_by_uid<T: Asset>(&self, uid: &Uid) -> Result<Handle<T>, AssetError>;
    pub fn load_by_uid_with<T: Asset>(&self, uid: &Uid,
        rendering: &mut dyn RenderingServer) -> Result<Handle<T>, AssetError>;
    pub fn texture(&self, path: &str,
        rendering: &mut dyn RenderingServer) -> Result<Handle<Texture>, AssetError>;
    pub fn uid_of(&self, path: &str) -> Option<Uid>;
}
```

*Not yet implemented:* `load_async*`, `audio`, `font`, and the `scene` /
`ron` shortcuts on `AssetServer` (use `load::<PackedScene>` /
`load::<RonAsset<T>>`; `ember_scene::SceneAssets::scene(path)` is
implemented for `AssetServer`).

### 6.4 Loading semantics

- **Cached.** Second `load(path)` returns a new `Handle` to the same
  asset — same `AssetId`, same `Arc` — while any handle is alive.
- **Type-checked.** `load::<T>` of a cached path as another type returns
  `Err(AssetError::WrongType { path, expected })`.
- **Path resolution.** `path` is relative to `root`. Absolute paths,
  `.`/`..` components and empty paths return `AssetError::InvalidPath`.
- The UID registry is saved only the first time a path is loaded.

### 6.5 Reload

```rust
impl AssetServer {
    pub fn reload<T: Asset>(&self, path: &str,
        rendering: Option<&mut dyn RenderingServer>) -> Result<(), AssetError>;
}
```

`reload` re-reads the file and calls `Asset::reload` under the write
lock; existing `Handle`s see the updated data. It is manual:
nothing calls it on file change. *Not yet implemented:* `reload_all`.

### 6.6 Poll

```rust
impl AssetServer {
    /// Drops cache entries nobody holds a handle to; returns how many.
    pub fn poll(&self) -> usize;
    /// Cache entry count, including ones `poll` has not pruned yet.
    pub fn cached(&self) -> usize;
}
```

`poll` does not process hot-reload events (none exist). Nothing calls it
automatically; games call it when convenient.

### 6.7 Unloading

Dropping the last `Handle<T>` drops the data (the cache holds a `Weak`).
`poll()` only removes the dead cache entries. There is no drop
notification to the server (ADR-026).

## 7. Loader registry

*Not yet implemented.* ADR-026 replaced `AssetLoader` / `LoaderRegistry`
with the type-driven `Asset` trait (§5): the requested type `T` selects
the loader, and `Asset::extensions()` documents accepted file extensions.
There is no precedence/override mechanism.

## 8. UID registry

### 8.1 Format

`assets/uid_registry.ron`:

```ron
(
    assets: {
        "sprites/player.png": "uid://player_a3b1c",
        "levels/forest.escene": "uid://forest_f5e3a",
    },
)
```

UIDs look like `uid://<file stem>_<5 hex chars>` (FNV-1a of the content
hash).

### 8.2 Behavior

- **Assign.** First load of a file without a UID entry: hash the content,
  assign a UID, write to the registry.
- **Resolve.** Loading by UID: look up the path, then load by path
  (`AssetError::UnknownUid` if missing).
- **Stable.** `rename(old, new)` updates the path, keeps the UID.

### 8.3 API

```rust
impl UidRegistry {
    pub fn load(file: &Path) -> Result<Self, AssetError>;
    pub fn save(&self, file: &Path) -> Result<(), AssetError>;
    pub fn uid_for_path(&self, path: &str) -> Option<&Uid>;
    pub fn path_for_uid(&self, uid: &Uid) -> Option<&str>;
    pub fn assign(&mut self, path: &str, content_hash: u64) -> Uid;
    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), AssetError>;
}
```

(Asset paths are normalised `&str`s.)

### 8.4 UID rule

UID is assigned on **first registration** and never changed, regardless
of content or path. The hash only seeds the initial string. Files
registered twice get the existing UID.

## 9. Hot-reload

*Not yet implemented* (ADR-026). There is no `HotReloader`, no `notify`
dependency and no `watch` option. Reloading is manual via
`AssetServer::reload` (§6.5). When it is called:

1. The file is re-read and `Asset::reload` updates the asset in place.
2. All existing `Handle`s see the new data.
3. Live nodes' own state is unchanged.

Reloading a `.escene` replaces the stored `PackedScene` only; new
instantiations use it, existing instances are unchanged.

## 10. Errors

```rust
#[derive(Debug)]
#[non_exhaustive]
pub enum AssetError {
    NotFound(String),
    Io(std::io::Error),
    InvalidPath(String),
    WrongType { path: String, expected: &'static str },
    Parse(String),
    InvalidUid(String),
    UnknownUid(Uid),
    NeedsRenderer(String),
    Server(ServerError),
}
```

`Display`/`Error` are hand-written (no `thiserror`).

## 11. Examples

### 11.1 Loading a texture at start-up

`NodeContext` has no `assets()` (*Not yet implemented*), so games load
before building the tree:

```rust
let assets = AssetServer::new("assets")?;
let tex: Handle<Texture> = assets.texture("sprites/player.png", &mut renderer)?;
// later, in draw: ctx.render().draw_sprite(tex.get().id, dst, None, Color::WHITE);
```

### 11.2 Async loading with a loading screen

*Not yet implemented* (`load_async`). Loading is synchronous; a loading
screen can load one asset per frame instead.

### 11.3 Custom asset type

```rust
#[derive(Clone)]
pub struct LevelConfig { pub name: String, pub width: f32 }

impl Asset for LevelConfig {
    fn extensions() -> &'static [&'static str] { &["level"] }
    fn load(bytes: &[u8], _ctx: &mut AssetLoadContext) -> Result<Self, AssetError> {
        // parse bytes (e.g. with ron) and map errors to AssetError::Parse
    }
}

let level: Handle<LevelConfig> = assets.load("levels/one.level")?;
```

(`reload` has a default implementation that replaces the value.) For
plain serde data, `RonAsset<T>` needs no impl.

## 12. Design notes

### 12.1 Why `Arc<RwLock<>>` and not `RefCell`

`Asset: Send + Sync`, handles are shared and `reload` mutates in place
under a write lock; `RefCell` is `!Sync`.

### 12.2 Why `AssetGuard` and not `&T`

Returning `&T` would require holding the lock for the handle's lifetime.
The guard bounds the lock to the caller's binding:

```rust
let tex = handle.get();          // lock held while `tex` is alive
ctx.render().draw_sprite(tex.id, dst, None, Color::WHITE);
```

### 12.3 Why content hash for UID generation

Deterministic (same file, same UID across machines), needs no authoring
tool, and stable when files move.

### 12.4 Why no async executor

v0.1 targets desktop-first, where `std::fs::read` is fast enough to block
on. Async loading is a v0.2 design.

### 12.5 Why `poll()` and not automatic unload

Memory is freed on the last handle drop; `poll()` only prunes dead cache
entries at a point the game chooses.

## 13. Open questions

- **Q1:** Should `Handle::get` block during hot-reload? *Provisional:*
  yes (reload takes a write lock). Watcher-driven reload is not implemented.
- **Q2:** Should loading be reference-counted per path or per asset?
  *Provisional:* per asset. Two paths can point to the same asset
  (via `uid://`); both share the same underlying `Arc`.
- **Q3:** Should `AssetServer` support unloading by tag (e.g. "unload
  all level assets")? *Provisional:* no. Refcounting handles the common
  case; tag-based unloading is v0.2.
- **Q4:** Should `Asset::load` be `async`? *Provisional:* no in v0.1.
  Async loader integration is a v0.2 design.

## 14. Cross-references

- Asset mental model → [[13-assets]]
- Server APIs called by loaders → [[22-ember-servers]]
- UID format in `.escene` files → [[30-escene-format]]
- Handle usage in nodes → [[21-ember-scene]]
