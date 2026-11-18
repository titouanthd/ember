//! AssetServer, Handle<T>, UID registry. Games never touch the
//! filesystem directly. Hot-reload watching and async loading are
//! deferred (ADR-026).

mod asset;
mod error;
mod handle;
mod id;
mod server;
mod uid_registry;

pub use asset::{Asset, AssetLoadContext, RonAsset, Texture};
pub use error::AssetError;
pub use handle::{AssetGuard, Handle, WeakHandle};
pub use id::{AssetId, Uid};
pub use server::AssetServer;
pub use uid_registry::UidRegistry;

pub mod prelude {
    pub use crate::{Asset, AssetError, AssetServer, Handle, RonAsset, Texture, Uid};
}
