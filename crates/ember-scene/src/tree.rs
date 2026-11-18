//! `NodeTree`: the ownership container for every live node.
//!
//! Structural changes made through `NodeContext` (`spawn`, `despawn`)
//! are queued and applied by `flush`, so a pass never iterates a tree
//! that changes under it. See `21-ember-scene.md §5`.

use crate::signal::{ScopedConnection, SignalTable};
use crate::{DrawContext, Node, NodeContext, NodeId, SceneError, Servers, Signal};
use slotmap::SlotMap;

struct NodeSlot {
    /// `None` while the node's callback is running (checked out).
    node: Option<Box<dyn Node>>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    ready: bool,
    /// Spawned through a context but not yet flushed in.
    pending: bool,
}

enum DeferredCmd {
    Activate(NodeId),
    Attach { child: NodeId, parent: NodeId },
    Despawn(NodeId),
}

/// Upper bound on ready/flush cascades (a `ready` that spawns nodes whose
/// `ready` spawns nodes ...). Hitting it means an unbounded spawn loop.
const MAX_FLUSH_ROUNDS: usize = 1000;

#[derive(Default)]
pub struct NodeTree {
    nodes: SlotMap<NodeId, NodeSlot>,
    root: Option<NodeId>,
    pending: Vec<DeferredCmd>,
    frame: u64,
    needs_ready: bool,
    quit: bool,
    signals: SignalTable,
    deferred_emits: Vec<DeferredEmit>,
}

type DeferredEmit = Box<dyn FnOnce(&mut NodeContext<'_>) + Send>;

impl NodeTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// A tree whose root is `node`.
    pub fn with_root(node: impl Node) -> Self {
        let mut tree = Self::new();
        let id = tree.insert(node);
        tree.set_root(id);
        tree
    }

    // --- Insert / remove ---

    /// Insert a floating node. Attach it or make it the root to bring it
    /// into the live tree.
    pub fn insert(&mut self, node: impl Node) -> NodeId {
        self.insert_boxed(Box::new(node))
    }

    pub fn insert_boxed(&mut self, node: Box<dyn Node>) -> NodeId {
        self.nodes.insert(NodeSlot {
            node: Some(node),
            parent: None,
            children: Vec::new(),
            ready: false,
            pending: false,
        })
    }

    /// Remove `id` and all its descendants, running `finalize` on each
    /// bottom-up. Returns the removed node itself.
    pub fn remove(&mut self, id: NodeId) -> Option<Box<dyn Node>> {
        if !self.contains(id) {
            return None;
        }
        let order = self.postorder(id);
        if let Some(parent) = self.nodes[id].parent
            && let Some(p) = self.nodes.get_mut(parent)
        {
            p.children.retain(|c| *c != id);
        }
        if self.root == Some(id) {
            self.root = None;
        }
        let mut removed = None;
        for nid in order {
            let Some(slot) = self.nodes.remove(nid) else {
                continue;
            };
            self.signals.remove_target(nid);
            if let Some(mut node) = slot.node {
                node.finalize();
                if nid == id {
                    removed = Some(node);
                }
            }
        }
        removed
    }

    /// Remove every node, finalizing each.
    pub fn clear(&mut self) {
        let all: Vec<NodeId> = self.nodes.keys().collect();
        for id in all {
            self.remove(id);
        }
        self.pending.clear();
        self.deferred_emits.clear();
        self.root = None;
    }

    // --- Access ---

    pub fn get(&self, id: NodeId) -> Option<&dyn Node> {
        let slot = self.nodes.get(id)?;
        if slot.pending {
            return None;
        }
        slot.node.as_deref()
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut dyn Node> {
        let slot = self.nodes.get_mut(id)?;
        if slot.pending {
            return None;
        }
        slot.node.as_deref_mut()
    }

    /// True if `id` is live (not stale, not merely queued by `spawn`).
    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.get(id).is_some_and(|s| !s.pending)
    }

    /// Number of live nodes.
    pub fn len(&self) -> usize {
        self.nodes.values().filter(|s| !s.pending).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // --- Structure ---

    pub fn attach(&mut self, child: NodeId, parent: NodeId) -> Result<(), SceneError> {
        if !self.contains(child) {
            return Err(SceneError::NotFound(child));
        }
        if !self.contains(parent) {
            return Err(SceneError::NotFound(parent));
        }
        if child == parent {
            return Err(SceneError::SelfParent(child));
        }
        if self.root == Some(child) {
            return Err(SceneError::IsRoot(child));
        }
        if self.nodes[child].parent.is_some() {
            return Err(SceneError::AlreadyAttached(child));
        }
        if self.is_ancestor(child, parent) {
            return Err(SceneError::Cycle { child, parent });
        }
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.push(child);
        self.needs_ready = true;
        Ok(())
    }

    /// Make `child` a floating subtree. Its own children stay attached
    /// to it; the node is not removed.
    pub fn detach(&mut self, child: NodeId) -> Result<(), SceneError> {
        if !self.contains(child) {
            return Err(SceneError::NotFound(child));
        }
        let parent = self.nodes[child]
            .parent
            .take()
            .ok_or(SceneError::NotAttached(child))?;
        if let Some(p) = self.nodes.get_mut(parent) {
            p.children.retain(|c| *c != child);
        }
        Ok(())
    }

    /// Move `child` under `new_parent`, appended last. Leaves the tree
    /// untouched on error.
    pub fn reparent(&mut self, child: NodeId, new_parent: NodeId) -> Result<(), SceneError> {
        if !self.contains(child) {
            return Err(SceneError::NotFound(child));
        }
        if !self.contains(new_parent) {
            return Err(SceneError::NotFound(new_parent));
        }
        if child == new_parent {
            return Err(SceneError::SelfParent(child));
        }
        if self.root == Some(child) {
            return Err(SceneError::IsRoot(child));
        }
        if self.is_ancestor(child, new_parent) {
            return Err(SceneError::Cycle {
                child,
                parent: new_parent,
            });
        }
        if self.nodes[child].parent.is_some() {
            self.detach(child)?;
        }
        self.attach(child, new_parent)
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id)?.parent
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.nodes.get(id).map_or(&[], |s| &s.children)
    }

    /// Depth-first, top-down, in child order; excludes `id` itself.
    pub fn descendants(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.preorder(id).into_iter().skip(1)
    }

    // --- Root ---

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Make `id` the root. Ignored if `id` is not live or has a parent.
    pub fn set_root(&mut self, id: NodeId) {
        if self.contains(id) && self.nodes[id].parent.is_none() {
            self.root = Some(id);
            self.needs_ready = true;
        }
    }

    // --- Queries ---

    /// Resolve a path: `""`/`"."` self, `".."` parent, `"A/B"` children
    /// by name.
    pub fn find(&self, from: NodeId, path: &str) -> Option<NodeId> {
        if !self.contains(from) {
            return None;
        }
        let mut cur = from;
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => cur = self.parent(cur)?,
                name => {
                    cur = self
                        .children(cur)
                        .iter()
                        .copied()
                        .find(|c| self.get(*c).is_some_and(|n| n.name() == name))?;
                }
            }
        }
        Some(cur)
    }

    /// Live nodes in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &dyn Node)> + '_ {
        self.nodes
            .iter()
            .filter(|(_, s)| !s.pending)
            .filter_map(|(id, s)| s.node.as_deref().map(|n| (id, n)))
    }

    /// Ask the loop to stop after the current frame.
    pub fn request_quit(&mut self) {
        self.quit = true;
    }

    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    // --- Signals ---

    /// Connect `handler` to signals of type `T`, run on behalf of
    /// `target`. Handlers run in connection order.
    pub fn connect<T: Signal>(
        &mut self,
        target: NodeId,
        handler: impl Fn(&mut NodeContext<'_>, &T) + Send + Sync + 'static,
    ) {
        self.signals.connect(target, handler);
    }

    /// Like `connect`; dropping the returned handle disconnects.
    pub fn connect_scoped<T: Signal>(
        &mut self,
        target: NodeId,
        handler: impl Fn(&mut NodeContext<'_>, &T) + Send + Sync + 'static,
    ) -> ScopedConnection {
        self.signals.connect_scoped(target, handler)
    }

    /// Remove every `T` handler targeting `target`.
    pub fn disconnect<T: Signal>(&mut self, target: NodeId) {
        self.signals.disconnect_type_target::<T>(target);
    }

    pub fn signals(&self) -> &SignalTable {
        &self.signals
    }

    pub(crate) fn signals_mut(&mut self) -> &mut SignalTable {
        &mut self.signals
    }

    pub(crate) fn queue_emit(&mut self, emit: DeferredEmit) {
        self.deferred_emits.push(emit);
    }

    /// True while `id` has a slot, even one still queued by `spawn`.
    pub(crate) fn is_allocated(&self, id: NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    // --- Frame loop ---

    /// Variable-step pass: top-down, child insertion order.
    pub fn process(&mut self, servers: &mut Servers, dt: f32) {
        self.flush(servers);
        self.run_pass(servers, dt, |node, ctx| node.process(dt, ctx));
        self.flush(servers);
        self.frame += 1;
    }

    /// Fixed-step pass; same order as `process`.
    pub fn physics_process(&mut self, servers: &mut Servers, fixed_dt: f32) {
        self.flush(servers);
        self.run_pass(servers, fixed_dt, |node, ctx| {
            node.physics_process(fixed_dt, ctx)
        });
        self.flush(servers);
    }

    /// Top-down, tree order. Each node sets the server's z first. A node
    /// with `visible() == false` hides its whole subtree.
    pub fn draw(&self, ctx: &mut DrawContext) {
        let Some(root) = self.root else { return };
        self.draw_subtree(root, ctx);
    }

    /// Apply queued structural changes, then call `ready` bottom-up on
    /// every newly reachable node. Repeats until nothing is pending.
    pub fn flush(&mut self, servers: &mut Servers) {
        let mut rounds = 0;
        loop {
            self.apply_pending();
            self.signals.tick();
            if !self.deferred_emits.is_empty() {
                self.run_deferred_emits(servers);
            } else if self.needs_ready {
                self.needs_ready = false;
                self.ready_pass(servers);
            } else {
                break;
            }
            rounds += 1;
            assert!(
                rounds < MAX_FLUSH_ROUNDS,
                "ready/spawn cascade never settled"
            );
        }
    }

    // --- Crate-internal: deferred commands ---

    pub(crate) fn spawn(&mut self, node: Box<dyn Node>, parent: Option<NodeId>) -> NodeId {
        let id = self.nodes.insert(NodeSlot {
            node: Some(node),
            parent: None,
            children: Vec::new(),
            ready: false,
            pending: true,
        });
        self.pending.push(DeferredCmd::Activate(id));
        if let Some(parent) = parent {
            self.pending.push(DeferredCmd::Attach { child: id, parent });
        }
        id
    }

    pub(crate) fn despawn(&mut self, id: NodeId) {
        self.pending.push(DeferredCmd::Despawn(id));
    }

    // --- Internals ---

    fn apply_pending(&mut self) {
        for cmd in std::mem::take(&mut self.pending) {
            match cmd {
                DeferredCmd::Activate(id) => {
                    if let Some(slot) = self.nodes.get_mut(id) {
                        slot.pending = false;
                        self.needs_ready = true;
                    }
                }
                // A queued attach whose target vanished or became invalid
                // is dropped; the spawned node stays a floating node.
                DeferredCmd::Attach { child, parent } => {
                    let _ = self.attach(child, parent);
                }
                DeferredCmd::Despawn(id) => {
                    self.remove(id);
                }
            }
        }
    }

    /// FIFO. Emits queued by these emits wait for the next round.
    fn run_deferred_emits(&mut self, servers: &mut Servers) {
        let emits = std::mem::take(&mut self.deferred_emits);
        let mut ctx = NodeContext::new(self, servers, NodeId::default(), 0.0);
        for emit in emits {
            emit(&mut ctx);
        }
    }

    fn ready_pass(&mut self, servers: &mut Servers) {
        let Some(root) = self.root else { return };
        for id in self.postorder(root) {
            let due = self.nodes.get(id).is_some_and(|s| !s.ready && !s.pending);
            if !due {
                continue;
            }
            self.nodes[id].ready = true;
            self.with_node(id, servers, 0.0, |node, ctx| node.ready(ctx));
        }
    }

    fn run_pass(
        &mut self,
        servers: &mut Servers,
        delta: f32,
        mut call: impl FnMut(&mut dyn Node, &mut NodeContext),
    ) {
        let Some(root) = self.root else { return };
        for id in self.preorder(root) {
            self.with_node(id, servers, delta, &mut call);
        }
    }

    /// Check `id`'s node out of its slot, run `call` with a context, put
    /// it back. A node removed during its own callback is finalized.
    fn with_node(
        &mut self,
        id: NodeId,
        servers: &mut Servers,
        delta: f32,
        mut call: impl FnMut(&mut dyn Node, &mut NodeContext),
    ) {
        let Some(slot) = self.nodes.get_mut(id) else {
            return;
        };
        if slot.pending {
            return;
        }
        let Some(mut node) = slot.node.take() else {
            return;
        };
        {
            let mut ctx = NodeContext::new(self, servers, id, delta);
            call(node.as_mut(), &mut ctx);
        }
        match self.nodes.get_mut(id) {
            Some(slot) => slot.node = Some(node),
            None => node.finalize(),
        }
    }

    fn draw_subtree(&self, id: NodeId, ctx: &mut DrawContext) {
        let Some(node) = self.get(id) else { return };
        if !node.visible() {
            return;
        }
        ctx.render().set_z(node.z());
        node.draw(ctx);
        for child in self.children(id) {
            self.draw_subtree(*child, ctx);
        }
    }

    /// True if `ancestor` is `node` or one of its ancestors.
    fn is_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut cur = Some(node);
        while let Some(id) = cur {
            if id == ancestor {
                return true;
            }
            cur = self.parent(id);
        }
        false
    }

    fn preorder(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        if self.nodes.contains_key(id) {
            let mut stack = vec![id];
            while let Some(n) = stack.pop() {
                out.push(n);
                stack.extend(self.children(n).iter().rev().copied());
            }
        }
        out
    }

    fn postorder(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        if self.nodes.contains_key(id) {
            self.postorder_into(id, &mut out);
        }
        out
    }

    fn postorder_into(&self, id: NodeId, out: &mut Vec<NodeId>) {
        for child in self.children(id) {
            self.postorder_into(*child, out);
        }
        out.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Dummy;

    #[test]
    fn attach_rejects_cycles_and_double_parents() {
        let mut t = NodeTree::new();
        let a = t.insert(Dummy::named("a"));
        let b = t.insert(Dummy::named("b"));
        let c = t.insert(Dummy::named("c"));
        t.attach(b, a).unwrap();
        t.attach(c, b).unwrap();
        assert_eq!(
            t.attach(a, c),
            Err(SceneError::Cycle {
                child: a,
                parent: c
            })
        );
        assert_eq!(t.attach(a, a), Err(SceneError::SelfParent(a)));
        assert_eq!(t.attach(c, a), Err(SceneError::AlreadyAttached(c)));
    }

    #[test]
    fn find_resolves_relative_paths() {
        let mut t = NodeTree::new();
        let root = t.insert(Dummy::named("root"));
        let a = t.insert(Dummy::named("a"));
        let b = t.insert(Dummy::named("b"));
        t.attach(a, root).unwrap();
        t.attach(b, root).unwrap();
        assert_eq!(t.find(root, "a"), Some(a));
        assert_eq!(t.find(a, "../b"), Some(b));
        assert_eq!(t.find(a, ".."), Some(root));
        assert_eq!(t.find(a, "."), Some(a));
        assert_eq!(t.find(root, "nope"), None);
    }

    #[test]
    fn reparent_moves_and_keeps_tree_on_error() {
        let mut t = NodeTree::new();
        let a = t.insert(Dummy::named("a"));
        let b = t.insert(Dummy::named("b"));
        let c = t.insert(Dummy::named("c"));
        t.attach(b, a).unwrap();
        t.attach(c, b).unwrap();
        assert!(t.reparent(a, c).is_err());
        assert_eq!(t.children(a), &[b]);
        t.reparent(c, a).unwrap();
        assert_eq!(t.children(a), &[b, c]);
        assert!(t.children(b).is_empty());
    }

    #[test]
    fn remove_takes_the_subtree() {
        let mut t = NodeTree::new();
        let a = t.insert(Dummy::named("a"));
        let b = t.insert(Dummy::named("b"));
        t.attach(b, a).unwrap();
        assert!(t.remove(a).is_some());
        assert!(t.is_empty());
        assert!(t.get(b).is_none());
    }
}
