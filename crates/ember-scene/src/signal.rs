//! Signals: the `Signal` marker trait, the tree's `SignalTable` and
//! `ScopedConnection`. See `11-signals.md` and ADR-023.

use crate::{NodeContext, NodeId};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Any `Send + Sync + 'static` type is a signal (blanket impl below).
pub trait Signal: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> Signal for T {}

/// A connected handler. Runs with `ctx.current_id()` set to its target.
pub type Handler<T> = Arc<dyn Fn(&mut NodeContext<'_>, &T) + Send + Sync>;

struct HandlerEntry {
    id: u64,
    target: NodeId,
    // An `Arc<Handler<T>>` with `T` erased.
    handler: Arc<dyn Any + Send + Sync>,
}

/// One snapshot entry: connection id, target, handler.
pub(crate) type Listener<T> = (u64, NodeId, Arc<Handler<T>>);

/// Every connection of one tree, per signal type, in connection order.
#[derive(Default)]
pub struct SignalTable {
    handlers: HashMap<TypeId, Vec<HandlerEntry>>,
    next_id: u64,
    // Ids of dropped `ScopedConnection`s, applied by `tick`.
    dropped: Arc<Mutex<Vec<u64>>>,
}

impl SignalTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Connect `handler` to signals of type `T` on behalf of `target`.
    /// Returns the connection id.
    pub fn connect<T: Signal>(
        &mut self,
        target: NodeId,
        handler: impl Fn(&mut NodeContext<'_>, &T) + Send + Sync + 'static,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let handler: Handler<T> = Arc::new(handler);
        self.handlers
            .entry(TypeId::of::<T>())
            .or_default()
            .push(HandlerEntry {
                id,
                target,
                handler: Arc::new(handler),
            });
        id
    }

    /// Like `connect`, returning a handle whose drop disconnects.
    pub fn connect_scoped<T: Signal>(
        &mut self,
        target: NodeId,
        handler: impl Fn(&mut NodeContext<'_>, &T) + Send + Sync + 'static,
    ) -> ScopedConnection {
        let id = self.connect(target, handler);
        ScopedConnection {
            id,
            queue: Arc::clone(&self.dropped),
            armed: true,
        }
    }

    pub fn disconnect_by_id(&mut self, id: u64) {
        self.disconnect_ids(&[id]);
    }

    /// Remove every `T` handler targeting `target`.
    pub fn disconnect_type_target<T: Signal>(&mut self, target: NodeId) {
        if let Some(list) = self.handlers.get_mut(&TypeId::of::<T>()) {
            list.retain(|e| e.target != target);
        }
    }

    /// Remove every handler targeting `target`, of any type.
    pub fn remove_target(&mut self, target: NodeId) {
        for list in self.handlers.values_mut() {
            list.retain(|e| e.target != target);
        }
    }

    pub fn is_connected(&self, id: u64) -> bool {
        self.handlers.values().flatten().any(|e| e.id == id)
    }

    pub fn len(&self) -> usize {
        self.handlers.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Apply disconnects queued by dropped `ScopedConnection`s.
    pub fn tick(&mut self) {
        let dropped = std::mem::take(&mut *self.dropped.lock().unwrap());
        self.disconnect_ids(&dropped);
    }

    pub(crate) fn disconnect_ids(&mut self, ids: &[u64]) {
        if ids.is_empty() {
            return;
        }
        for list in self.handlers.values_mut() {
            list.retain(|e| !ids.contains(&e.id));
        }
    }

    /// The `T` handlers in connection order. Cloned out so dispatch can
    /// hold `&mut NodeContext` while they run.
    pub(crate) fn snapshot<T: Signal>(&mut self) -> Vec<Listener<T>> {
        self.tick();
        self.handlers
            .get(&TypeId::of::<T>())
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let h = Arc::clone(&e.handler).downcast::<Handler<T>>().ok()?;
                Some((e.id, e.target, h))
            })
            .collect()
    }
}

/// Dropping this disconnects its handler; the removal is applied by the
/// tree's next `flush` or `emit`.
#[must_use = "dropping a ScopedConnection disconnects it immediately"]
pub struct ScopedConnection {
    id: u64,
    queue: Arc<Mutex<Vec<u64>>>,
    armed: bool,
}

impl ScopedConnection {
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Keep the handler connected for good.
    pub fn forget(mut self) {
        self.armed = false;
    }
}

impl Drop for ScopedConnection {
    fn drop(&mut self) {
        if self.armed
            && let Ok(mut q) = self.queue.lock()
        {
            q.push(self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_struct_is_a_signal() {
        struct MyEvent {
            _amount: u32,
        }
        fn assert_signal<T: Signal>() {}
        assert_signal::<MyEvent>();
    }

    #[test]
    fn a_primitive_is_a_signal() {
        fn assert_signal<T: Signal>() {}
        assert_signal::<u32>();
        assert_signal::<String>();
    }
}
