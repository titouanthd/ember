# 25 — `ember_assets`

> **Status:** 🟡 partial — see ADR-026 (no loader registry, no hot-reload watcher, no async)
> **Audience:** implementers
> **Prerequisites:** [13-assets.md](13-assets.md), [22-ember-servers.md](22-ember-servers.md)

## 1. Purpose

Specify the `AssetServer` API: `Handle<T>`, `WeakHandle<T>`, `AssetId`,
the loader registry, UID registry, hot-reload, and error types. This is
the crate that games use to load textures, fonts, audio, scenes, and
level data.

## 2. Module layout

```
ember_assets/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── prelude.rs
│   ├── server.rs       AssetServer
│   ├── handle.rs       Handle, WeakHandle
│   ├── id.rs           AssetId, Uid
│   ├── loader.rs       AssetLoader trait, LoaderRegistry
│   ├── loaders/
│   │   ├── mod.rs
│   │   ├── image.rs    ImageLoader
│   │   ├── audio.rs    AudioLoader
│   │   ├── font.rs     FontLoader
│   │   ├── scene.rs    SceneLoader
│   │   └── ron.rs      RonLoader
│   ├── uid_registry.rs UID assignment and lookup
│   ├── hot_reload.rs   file watcher
│   └── error.rs        AssetError
```

## 3. `AssetId` and `Uid`

### 3.1 `AssetId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct AssetId(u64);

impl AssetId {
    pub fn invalid() -> Self;
    pub fn is_valid(self) -> bool;
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
    arc: Arc<RwLock<AssetData<T>>>,   // shared with WeakHandle and cache
    server: AssetServerRef,            // for get() and drop hook
}

impl<T: Asset> Handle<T> {
    pub fn id(&self) -> AssetId;
    pub fn is_ready(&self) -> bool;
    pub fn get(&self) -> Option<AssetGuard<'_, T>>;
    pub fn downgrade(&self) -> WeakHandle<T>;
}

impl<T: Asset> Clone for Handle<T> { /* clones Arc */ }
impl<T: Asset> Drop for Handle<T> { /* decrements cache refcount */ }
```

### 4.2 `AssetGuard<'a, T>`

```rust
pub struct AssetGuard<'a, T: Asset> {
    guard: std::sync::RwLockReadGuard<'a, AssetData<T>>,
}

impl<'a, T: Asset> std::ops::Deref for AssetGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { &self.guard.value }
}
```

`handle.get()` returns a `Deref` to `T` — the guard prevents the asset
from being swapped out mid-read (hot-reload acquires a write lock).

### 4.3 `WeakHandle<T>`

```rust
pub struct WeakHandle<T: Asset> {
    id: AssetId,
    arc: Weak<RwLock<AssetData<T>>>,
}

impl<T: Asset> WeakHandle<T> {
    pub fn id(&self) -> AssetId;
    pub fn upgrade(&self) -> Option<Handle<T>>;
}
```

Does not keep the asset alive. `upgrade()` returns `None` if the asset
has been unloaded.

### 4.4 Clone and equality semantics

- `Handle::clone` — refcount increment, O(1).
- `Handle ==` — compares `AssetId`, not underlying data.
- `Handle::drop` — refcount decrement; if zero, the asset is queued for
  unloading at `AssetServer::poll()`.

## 5. `Asset` trait

### 5.1 Trait

```rust
pub trait Asset: Send + Sync + 'static {
    fn load(bytes: &[u8], ctx: &AssetLoadContext) -> Result<Self, AssetError>
    where Self: Sized;

    /// Called on hot-reload. Default implementation is to replace.
    fn reload(current: &mut Self, bytes: &[u8], ctx: &AssetLoadContext) -> Result<(), AssetError>;
}
```

### 5.2 `AssetLoadContext`

```rust
pub struct AssetLoadContext<'a> {
    pub path: &'a Path,
    pub uid: &'a Uid,
    pub rendering: &'a mut dyn RenderingServer,
    pub audio: &'a mut dyn AudioServer,
    pub class_db: &'a ClassDB,
}
```

The context gives the loader access to server APIs (e.g. `ImageLoader`
calls `rendering.create_texture`) and to `ClassDB` (e.g. `SceneLoader`
instantiates packed scenes).

### 5.3 Built-in assets

| Asset type | Loader | Format |
|---|---|---|
| `Texture` | `ImageLoader` | PNG, JPG |
| `AudioClip` | `AudioLoader` | OGG, WAV |
| `Font` | `FontLoader` | TTF, OTF |
| `PackedScene` | `SceneLoader` | `.escene` |
| `RonAsset<T>` | `RonLoader` | `.ron` |

Games can define custom assets by implementing `Asset` and registering
a loader.

## 6. `AssetServer`

### 6.1 Struct

```rust
pub struct AssetServer {
    root: PathBuf,
    loaders: LoaderRegistry,
    cache: HashMap<AssetId, Weak<dyn AnyAsset>>,   // type-erased
    path_to_id: HashMap<PathBuf, AssetId>,
    uid_registry: UidRegistry,
    hot_reload: Option<HotReloader>,
    pending: Vec<AssetId>,                         // for unload at poll()
    server_refs: Vec<Weak<ServerHandle>>,          // for Handle drop hooks
}
```

### 6.2 Construction

```rust
impl AssetServer {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, AssetError>;
    pub fn builder() -> AssetServerBuilder;
}

pub struct AssetServerBuilder {
    root: PathBuf,
    watch: bool,
    loaders: LoaderRegistry,
}

impl AssetServerBuilder {
    pub fn new(root: impl Into<PathBuf>) -> Self;
    pub fn watch(mut self, enabled: bool) -> Self;
    pub fn register_loader<T: Asset, L: AssetLoader<T>>(mut self) -> Self;
    pub fn build(self) -> Result<AssetServer, AssetError>;
}
```

### 6.3 Loading

```rust
impl AssetServer {
    // === Synchronous ===
    pub fn load<T: Asset>(&self, path: &str) -> Result<Handle<T>, AssetError>;
    pub fn load_by_uid<T: Asset>(&self, uid: &Uid) -> Result<Handle<T>, AssetError>;

    // === Asynchronous ===
    pub fn load_async<T: Asset>(&self, path: &str) -> Handle<T>;
    pub fn load_async_by_uid<T: Asset>(&self, uid: &Uid) -> Handle<T>;

    // === Typed convenience (via extension traits) ===
    pub fn texture(&self, path: &str) -> Result<Handle<Texture>, AssetError>;
    pub fn audio(&self, path: &str) -> Result<Handle<AudioClip>, AssetError>;
    pub fn font(&self, path: &str) -> Result<Handle<Font>, AssetError>;
    pub fn scene(&self, path: &str) -> Result<Handle<PackedScene>, AssetError>;
    pub fn ron<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        path: &str,
    ) -> Result<Handle<RonAsset<T>>, AssetError>;
}
```

### 6.4 Loading semantics

- **Cached.** Second `load(path)` returns a new `Handle` to the same
  asset — same `AssetId`, same `Arc`.
- **Type-checked.** `load::<Texture>("player.png")` returns
  `Err(AssetError::WrongType)` if the file is registered as a different
  asset type.
- **Path resolution.** `path` is relative to `root`. `.`, `..`, and
  absolute paths are rejected (must be under `root`).

### 6.5 Reload

```rust
impl AssetServer {
    pub fn reload(&self, path: &str) -> Result<(), AssetError>;
    pub fn reload_all(&self);
}
```

`reload` reads the file, calls `Asset::reload` on the live asset. All
existing `Handle`s see the updated data. Hot-reload calls `reload`
automatically when files change.

### 6.6 Poll

```rust
impl AssetServer {
    /// Called once per frame by MainLoop. Processes hot-reload events
    /// and unloads assets whose refcount reached zero.
    pub fn poll(&mut self);
}
```

### 6.7 Unloading

When a `Handle<T>` drops and its refcount reaches zero:

1. The `AssetServer` is notified (via a `Weak<ServerHandle>` in the
   `Handle`).
2. The `AssetId` is added to `pending`.
3. On the next `poll()`, the asset is removed from the cache and dropped.

This defers the actual drop to a safe point outside the frame loop.

## 7. Loader registry

### 7.1 `AssetLoader` trait

```rust
pub trait AssetLoader<T: Asset>: Send + Sync + 'static {
    fn extensions(&self) -> &'static [&'static str];
    fn load(&self, bytes: &[u8], ctx: &mut AssetLoadContext) -> Result<T, AssetError>;
    fn reload(
        &self,
        current: &mut T,
        bytes: &[u8],
        ctx: &mut AssetLoadContext,
    ) -> Result<(), AssetError>;
}
```

### 7.2 Registry

```rust
pub struct LoaderRegistry {
    by_extension: HashMap<&'static str, Box<dyn ErasedLoader>>,
    by_asset_type: HashMap<TypeId, &'static str>,
}

impl LoaderRegistry {
    pub fn new() -> Self;
    pub fn register<T: Asset, L: AssetLoader<T> + 'static>(&mut self, loader: L);
    pub fn loader_for_extension(&self, ext: &str) -> Option<&dyn ErasedLoader>;
}
```

The registry is populated by `AssetServerBuilder::register_loader` and
by the built-in loaders (registered automatically at `build()`).

### 7.3 Registration precedence

Custom loaders override built-in loaders for the same extension. This
lets a game provide a `"png"` loader that preprocesses images, without
forking `ember_assets`.

## 8. UID registry

### 8.1 Format

`assets/uid_registry.ron`:

```ron
(
    assets: {
        "sprites/player.png": "uid://player_a3b1c",
        "sprites/enemy.png": "uid://enemy_b4d2e",
        "levels/forest.escene": "uid://forest_f5e3a",
    },
)
```

### 8.2 Behavior

- **Assign.** First load of a file without a UID entry: hash the content,
  assign a UID, write to the registry.
- **Resolve.** Loading by UID: look up the path, then load by path.
- **Stable.** Renaming a file updates its path in the registry, keeps
  the UID.

### 8.3 API

```rust
impl UidRegistry {
    pub fn load(path: &Path) -> Result<Self, AssetError>;
    pub fn save(&self, path: &Path) -> Result<(), AssetError>;
    pub fn uid_for_path(&self, path: &Path) -> Option<&Uid>;
    pub fn path_for_uid(&self, uid: &Uid) -> Option<&Path>;
    pub fn assign(&mut self, path: &Path, content_hash: u64) -> Uid;
    pub fn rename(&mut self, old: &Path, new: &Path) -> Result<(), AssetError>;
}
```

### 8.4 Why content hash for UID

If the UID is the hash of file content, renaming a file keeps its UID
(the content is unchanged). Moving the file also keeps the UID. Only
editing the content (which changes the hash) would generate a new UID —
but that's wrong, because editing content shouldn't break references.

**Actual rule:** UID is assigned on **first registration** and never
changed, regardless of content or path. The hash is just used to
generate the initial UID (a random-looking stable string). Files
registered twice get the existing UID.

## 9. Hot-reload

### 9.1 Watcher

```rust
pub struct HotReloader {
    watcher: notify::RecommendedWatcher,
    changed: Receiver<PathBuf>,
}
```

The watcher runs on a background thread, sends changed paths to a
`crossbeam` channel. `AssetServer::poll` drains the channel and calls
`reload` for each changed path.

### 9.2 Availability

- **Enabled** in `cfg(debug_assertions)` and when `watch(true)` is
  set explicitly.
- **Disabled** in release builds and when `watch(false)` is set.

### 9.3 Behavior on reload

1. Loader reads new bytes.
2. `Asset::reload` updates the asset in place.
3. All existing `Handle`s see the new data.
4. Live game objects (nodes with `Handle`) are **not** automatically
   updated — they see the new data via the handle's `get()`, but their
   *state* (e.g. a texture's position on screen) is unchanged.

### 9.4 Reload of packed scenes

Reloading a `.escene` file:
- New instances pick up the new content.
- Existing instances are unchanged.

To reset live instances, the game must be restarted (v0.1). A
"reload scene" button in the editor is v0.2.

## 10. Errors

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("asset not found: {0}")]
    NotFound(String),

    #[error("path outside root: {0}")]
    OutsideRoot(String),

    #[error("unknown extension: {0}")]
    UnknownExtension(String),

    #[error("loader failed: {0}")]
    LoaderFailed(String),

    #[error("type mismatch for {0}: registered as {1}, requested as {2}")]
    WrongType(String, &'static str, &'static str),

    #[error("asset not yet loaded: {0:?}")]
    NotLoaded(AssetId),

    #[error("uid not found: {0}")]
    UidNotFound(String),

    #[error("registry error: {0}")]
    Registry(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

## 11. Examples

### 11.1 Loading a texture in `ready`

```rust
impl Node for Sprite {
    fn ready(&mut self, ctx: &mut NodeContext) {
        match ctx.assets().texture("sprites/player.png") {
            Ok(handle) => self.handle = Some(handle),
            Err(e) => log::warn!("failed to load player texture: {e}"),
        }
    }
}
```

### 11.2 Async loading with a loading screen

```rust
struct LoadingScreen { pending: Vec<WeakHandle<Texture>> }

impl Node for LoadingScreen {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        self.pending.retain(|h| h.upgrade().is_none());
        if self.pending.is_empty() {
            ctx.emit::<LoadComplete>(LoadComplete);
        }
    }
}
```

### 11.3 Custom asset type

```rust
#[derive(Clone)]
pub struct LevelConfig {
    pub name: String,
    pub width: f32,
}

impl Asset for LevelConfig {
    fn load(bytes: &[u8], _ctx: &mut AssetLoadContext) -> Result<Self, AssetError> {
        ron::de::from_bytes(bytes).map_err(|e| AssetError::LoaderFailed(e.to_string()))
    }

    fn reload(current: &mut Self, bytes: &[u8], ctx: &mut AssetLoadContext) -> Result<(), AssetError> {
        *current = Self::load(bytes, ctx)?;
        Ok(())
    }
}

// In main:
let mut builder = AssetServer::builder();
builder.register_loader::<LevelConfig, _>(LevelConfigLoader);
let assets = builder.build()?;
```

## 12. Design notes

### 12.1 Why `Arc<RwLock<>>` and not `RefCell`

Because hot-reload runs on a background thread (the file watcher) that
acquires a write lock. `RefCell` is `!Sync` — the reload can't touch it.

### 12.2 Why `AssetGuard` and not `&T`

Because returning `&T` from `handle.get()` would require locking for the
lifetime of the reference — which is the handle's lifetime, not the
caller's. `AssetGuard` bounds the lock to the caller's `let` binding:

```rust
if let Some(tex) = handle.get() {
    // lock held only while `tex` is alive
    ctx.render().draw_sprite(tex.id(), dst, None, WHITE);
}
// lock released
```

### 12.3 Why content hash for UID generation

Because the hash is deterministic (same file, same UID across machines),
avoids needing a UID-authoring tool, and stays stable when files move.

### 12.4 Why no async executor

Because v0.1 targets desktop-first, where `std::fs::read` is fast enough
to block on. `load_async` on web is powered by macroquad's wasm-specific
image loader, not a general async runtime. Full async (with a runtime
like `tokio`) is a v0.2 feature when web target becomes first-class.

### 12.5 Why `poll()` and not automatic unload

Because dropping a `Handle` mid-frame would mutate the cache while it's
being iterated. `poll()` runs once per frame at a safe point, applies
pending unloads, and processes hot-reload events in one place.

## 13. Open questions

- **Q1:** Should `Handle::get` block during hot-reload? *Provisional:*
  yes. Hot-reload is rare; a 1-frame stall is invisible.
- **Q2:** Should loading be reference-counted per path or per asset?
  *Provisional:* per asset. Two paths can point to the same asset
  (via `uid://`); both share the same underlying `Arc`.
- **Q3:** Should `AssetServer` support unloading by tag (e.g. "unload
  all level assets")? *Provisional:* no. Refcounting handles the common
  case; tag-based unloading is v0.2.
- **Q4:** Should `Asset::load` be `async`? *Provisional:* no in v0.1.
  Async loader integration is a v0.2 design.

## 14. Cross-references

- Asset mental model → [13-assets.md](13-assets.md)
- Server APIs called by loaders → [22-ember-servers.md](22-ember-servers.md)
- UID format in `.escene` files → [30-escene-format.md](30-escene-format.md)
- Handle usage in nodes → [21-ember-scene.md](21-ember-scene.md)