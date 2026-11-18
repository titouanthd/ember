//! `Handle<T>` and `WeakHandle<T>`.

use crate::{Asset, AssetId, Uid};
use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};

pub type AssetGuard<'a, T> = RwLockReadGuard<'a, T>;

/// A strong reference. The asset stays cached while any handle lives;
/// `AssetServer::poll` drops entries whose handles are all gone.
pub struct Handle<T: Asset> {
    id: AssetId,
    uid: Uid,
    arc: Arc<RwLock<T>>,
}

impl<T: Asset> Handle<T> {
    pub(crate) fn new(id: AssetId, uid: Uid, arc: Arc<RwLock<T>>) -> Self {
        Self { id, uid, arc }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn uid(&self) -> &Uid {
        &self.uid
    }

    /// Read access. A reload waits for the guard to drop.
    pub fn get(&self) -> AssetGuard<'_, T> {
        self.arc.read().expect("asset lock poisoned")
    }

    pub fn downgrade(&self) -> WeakHandle<T> {
        WeakHandle {
            id: self.id,
            uid: self.uid.clone(),
            arc: Arc::downgrade(&self.arc),
        }
    }
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            uid: self.uid.clone(),
            arc: Arc::clone(&self.arc),
        }
    }
}

impl<T: Asset> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle({:?}, {})", self.id, self.uid)
    }
}

impl<T: Asset> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Asset> Eq for Handle<T> {}

pub struct WeakHandle<T: Asset> {
    id: AssetId,
    uid: Uid,
    arc: Weak<RwLock<T>>,
}

impl<T: Asset> WeakHandle<T> {
    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn upgrade(&self) -> Option<Handle<T>> {
        Some(Handle::new(self.id, self.uid.clone(), self.arc.upgrade()?))
    }
}

impl<T: Asset> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            uid: self.uid.clone(),
            arc: Weak::clone(&self.arc),
        }
    }
}
