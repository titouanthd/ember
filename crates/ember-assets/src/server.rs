//! `AssetServer`: cached, type-checked, UID-aware loading (ADR-026) with
//! change-polling hot reload (ADR-038).

use crate::{
    Asset, AssetError, AssetId, AssetLoadContext, AudioClip, Font, Handle, Texture, Uid,
    UidRegistry,
};
use ember_servers::{AudioServer, RenderingServer};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::SystemTime;

const REGISTRY_FILE: &str = "uid_registry.ron";

trait ErasedWeak: Send + Sync {
    fn alive(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Asset> ErasedWeak for Weak<RwLock<T>> {
    fn alive(&self) -> bool {
        self.strong_count() > 0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The servers a load or reload may need.
struct Backends<'a> {
    rendering: Option<&'a mut dyn RenderingServer>,
    audio: Option<&'a mut dyn AudioServer>,
}

impl Backends<'_> {
    fn none() -> Self {
        Self {
            rendering: None,
            audio: None,
        }
    }
}

/// `AssetServer::reload_inner::<T>`, with `T` erased for the cache.
type ReloadFn = fn(&AssetServer, &str, &mut Backends<'_>) -> Result<(), AssetError>;

fn reload_erased<T: Asset>(
    server: &AssetServer,
    key: &str,
    backends: &mut Backends<'_>,
) -> Result<(), AssetError> {
    server.reload_inner::<T>(key, backends)
}

struct Entry {
    type_id: TypeId,
    id: AssetId,
    weak: Box<dyn ErasedWeak>,
    /// File modification time when last (re)loaded.
    modified: Option<SystemTime>,
    reload: ReloadFn,
}

struct State {
    registry: UidRegistry,
    cache: HashMap<String, Entry>,
    next_id: u64,
}

pub struct AssetServer {
    root: PathBuf,
    state: Mutex<State>,
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Rejects absolute paths and `.` / `..`; returns the `/`-joined key.
fn normalize(path: &str) -> Result<String, AssetError> {
    let mut parts = Vec::new();
    for c in Path::new(path).components() {
        match c {
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            _ => return Err(AssetError::InvalidPath(path.to_string())),
        }
    }
    if parts.is_empty() {
        return Err(AssetError::InvalidPath(path.to_string()));
    }
    Ok(parts.join("/"))
}

impl AssetServer {
    /// `root` is the asset directory; `uid_registry.ron` lives in it.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, AssetError> {
        let root = root.into();
        let registry = UidRegistry::load(&root.join(REGISTRY_FILE))?;
        Ok(Self {
            root,
            state: Mutex::new(State {
                registry,
                cache: HashMap::new(),
                next_id: 1,
            }),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().expect("asset state poisoned")
    }

    fn read(&self, key: &str) -> Result<Vec<u8>, AssetError> {
        std::fs::read(self.root.join(key)).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AssetError::NotFound(key.to_string()),
            _ => AssetError::Io(format!("{key}: {e}")),
        })
    }

    fn modified(&self, key: &str) -> Option<SystemTime> {
        std::fs::metadata(self.root.join(key)).ok()?.modified().ok()
    }

    fn check_extension<T: Asset>(key: &str) -> Result<(), AssetError> {
        let ext = Path::new(key)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if T::extensions().contains(&ext.as_str()) {
            Ok(())
        } else {
            Err(AssetError::WrongType {
                path: key.to_string(),
                expected: std::any::type_name::<T>(),
            })
        }
    }

    /// Loads a CPU-only asset (scenes, RON). GPU assets need `load_with`.
    pub fn load<T: Asset>(&self, path: &str) -> Result<Handle<T>, AssetError> {
        self.load_inner(path, &mut Backends::none())
    }

    pub fn load_with<T: Asset>(
        &self,
        path: &str,
        rendering: &mut dyn RenderingServer,
    ) -> Result<Handle<T>, AssetError> {
        let mut b = Backends {
            rendering: Some(rendering),
            audio: None,
        };
        self.load_inner(path, &mut b)
    }

    /// Loads an asset that needs the audio backend (`AudioClip`).
    pub fn load_audio<T: Asset>(
        &self,
        path: &str,
        audio: &mut dyn AudioServer,
    ) -> Result<Handle<T>, AssetError> {
        let mut b = Backends {
            rendering: None,
            audio: Some(audio),
        };
        self.load_inner(path, &mut b)
    }

    pub fn load_by_uid<T: Asset>(&self, uid: &Uid) -> Result<Handle<T>, AssetError> {
        let path = self.path_of(uid)?;
        self.load_inner(&path, &mut Backends::none())
    }

    pub fn load_by_uid_with<T: Asset>(
        &self,
        uid: &Uid,
        rendering: &mut dyn RenderingServer,
    ) -> Result<Handle<T>, AssetError> {
        let path = self.path_of(uid)?;
        self.load_with(&path, rendering)
    }

    fn path_of(&self, uid: &Uid) -> Result<String, AssetError> {
        self.state()
            .registry
            .path_for_uid(uid)
            .map(str::to_string)
            .ok_or_else(|| AssetError::UnknownUid(uid.to_string()))
    }

    pub fn texture(
        &self,
        path: &str,
        rendering: &mut dyn RenderingServer,
    ) -> Result<Handle<Texture>, AssetError> {
        self.load_with(path, rendering)
    }

    pub fn font(
        &self,
        path: &str,
        rendering: &mut dyn RenderingServer,
    ) -> Result<Handle<Font>, AssetError> {
        self.load_with(path, rendering)
    }

    pub fn audio_clip(
        &self,
        path: &str,
        audio: &mut dyn AudioServer,
    ) -> Result<Handle<AudioClip>, AssetError> {
        self.load_audio(path, audio)
    }

    /// The UID registered for `path`, if it was ever loaded.
    pub fn uid_of(&self, path: &str) -> Option<Uid> {
        let key = normalize(path).ok()?;
        self.state().registry.uid_for_path(&key).cloned()
    }

    fn load_inner<T: Asset>(
        &self,
        path: &str,
        backends: &mut Backends<'_>,
    ) -> Result<Handle<T>, AssetError> {
        let key = normalize(path)?;
        Self::check_extension::<T>(&key)?;
        let mut state = self.state();

        if let Some(entry) = state.cache.get(&key) {
            if entry.type_id != TypeId::of::<T>() {
                return Err(AssetError::WrongType {
                    path: key,
                    expected: std::any::type_name::<T>(),
                });
            }
            let live = entry
                .weak
                .as_any()
                .downcast_ref::<Weak<RwLock<T>>>()
                .and_then(Weak::upgrade);
            if let Some(arc) = live {
                let uid = state.registry.uid_for_path(&key).cloned().expect("cached");
                return Ok(Handle::new(entry.id, uid, arc));
            }
        }

        let bytes = self.read(&key)?;
        let modified = self.modified(&key);
        let known = state.registry.uid_for_path(&key).is_some();
        let uid = state.registry.assign(&key, fnv1a(&bytes));
        let mut ctx = AssetLoadContext {
            path: Path::new(&key),
            uid: &uid,
            rendering: backends
                .rendering
                .as_deref_mut()
                .map(|r| r as &mut dyn RenderingServer),
            audio: backends
                .audio
                .as_deref_mut()
                .map(|a| a as &mut dyn AudioServer),
        };
        let value = T::load(&bytes, &mut ctx)?;
        if !known {
            state.registry.save(&self.root.join(REGISTRY_FILE))?;
        }
        let arc = Arc::new(RwLock::new(value));
        let id = AssetId::new(state.next_id);
        state.next_id += 1;
        state.cache.insert(
            key,
            Entry {
                type_id: TypeId::of::<T>(),
                id,
                weak: Box::new(Arc::downgrade(&arc)),
                modified,
                reload: reload_erased::<T>,
            },
        );
        Ok(Handle::new(id, uid, arc))
    }

    /// Re-reads `path` and updates the live asset in place; every
    /// existing handle sees the new data. Errors if it is not loaded.
    /// Pass the renderer for GPU assets (`Texture`, `Font`).
    pub fn reload<T: Asset>(
        &self,
        path: &str,
        rendering: Option<&mut dyn RenderingServer>,
    ) -> Result<(), AssetError> {
        let key = normalize(path)?;
        let mut b = Backends {
            rendering,
            audio: None,
        };
        self.reload_inner::<T>(&key, &mut b)
    }

    fn reload_inner<T: Asset>(
        &self,
        key: &str,
        backends: &mut Backends<'_>,
    ) -> Result<(), AssetError> {
        let (arc, uid) = {
            let state = self.state();
            let arc = state
                .cache
                .get(key)
                .filter(|e| e.type_id == TypeId::of::<T>())
                .and_then(|e| e.weak.as_any().downcast_ref::<Weak<RwLock<T>>>())
                .and_then(Weak::upgrade)
                .ok_or_else(|| AssetError::NotFound(key.to_string()))?;
            let uid = state.registry.uid_for_path(key).cloned().expect("cached");
            (arc, uid)
        };
        let bytes = self.read(key)?;
        let modified = self.modified(key);
        let mut ctx = AssetLoadContext {
            path: Path::new(key),
            uid: &uid,
            rendering: backends
                .rendering
                .as_deref_mut()
                .map(|r| r as &mut dyn RenderingServer),
            audio: backends
                .audio
                .as_deref_mut()
                .map(|a| a as &mut dyn AudioServer),
        };
        let mut guard = arc.write().expect("asset lock poisoned");
        T::reload(&mut guard, &bytes, &mut ctx)?;
        drop(guard);
        if let Some(e) = self.state().cache.get_mut(key) {
            e.modified = modified;
        }
        Ok(())
    }

    /// Hot reload (ADR-038): re-reads every live asset whose file changed
    /// on disk since it was loaded, in path order, and returns each path
    /// with its result. Call it every frame or on a timer; pass the
    /// servers GPU and audio assets need (without them those reloads
    /// fail with `NeedsRenderer` / `NeedsAudio`). A failed reload keeps
    /// the old value and is not retried until the file changes again.
    pub fn reload_changed(
        &self,
        rendering: Option<&mut dyn RenderingServer>,
        audio: Option<&mut dyn AudioServer>,
    ) -> Vec<(String, Result<(), AssetError>)> {
        let mut changed: Vec<(String, ReloadFn, Option<SystemTime>)> = {
            let state = self.state();
            state
                .cache
                .iter()
                .filter(|(_, e)| e.weak.alive())
                .filter_map(|(key, e)| {
                    let now = self.modified(key)?;
                    (Some(now) != e.modified).then(|| (key.clone(), e.reload, Some(now)))
                })
                .collect()
        };
        changed.sort_by(|a, b| a.0.cmp(&b.0));
        let mut b = Backends {
            rendering: rendering.map(|r| r as &mut dyn RenderingServer),
            audio: audio.map(|a| a as &mut dyn AudioServer),
        };
        let mut out = Vec::with_capacity(changed.len());
        for (key, reload, now) in changed {
            let result = reload(self, &key, &mut b);
            if result.is_err()
                && let Some(e) = self.state().cache.get_mut(&key)
            {
                e.modified = now; // don't retry a broken file every frame
            }
            out.push((key, result));
        }
        out
    }

    /// Drops cache entries nobody holds a handle to. Returns how many.
    pub fn poll(&self) -> usize {
        let mut state = self.state();
        let before = state.cache.len();
        state.cache.retain(|_, e| e.weak.alive());
        before - state.cache.len()
    }

    /// Number of cache entries, including ones `poll` has not pruned yet.
    pub fn cached(&self) -> usize {
        self.state().cache.len()
    }
}
