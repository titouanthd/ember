//! AssetServer, Handle<T>, UID registry. Games never touch the
//! filesystem directly. Textures, fonts, audio clips, scenes and RON
//! data load through it; `reload_changed` hot-reloads edited files
//! (ADR-026, ADR-038). Async loading is deferred.

mod asset;
mod error;
mod handle;
mod id;
mod server;
mod uid_registry;

pub use asset::{Asset, AssetLoadContext, AudioClip, Font, RonAsset, Texture};
pub use error::AssetError;
pub use handle::{AssetGuard, Handle, WeakHandle};
pub use id::{AssetId, Uid};
pub use server::AssetServer;
pub use uid_registry::UidRegistry;

pub mod prelude {
    pub use crate::{
        Asset, AssetError, AssetServer, AudioClip, Font, Handle, RonAsset, Texture, Uid,
    };
}
