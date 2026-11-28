//! `NodeTree`: the ownership container for every live node.
//!
//! Structural changes made through `NodeContext` (`spawn`, `despawn`)
//! are queued and applied by `flush`, so a pass never iterates a tree
//! that changes under it. See `21-ember-scene.md §5`.

use crate::scene_change::{SceneChange, SceneLibrary};
use crate::signal::{ScopedConnection, SignalTable};
use crate::{DrawContext, Node, NodeContext, NodeId, SceneError, Servers, Signal};
use slotmap::SlotMap;
use std::collections::{BTreeMap, HashSet};

struct NodeSlot {
    /// `None` while the node's callback is running (checked out).
    node: Option<Box<dyn Node>>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    ready: bool,
    /// Spawned through a context but not yet flushed in.
    pending: bool,
    /// Groups this node is in, in join order (ADR-051).
    groups: Vec<String>,
}

enum DeferredCmd {
    Activate(NodeId),
    Attach { child: NodeId, parent: NodeId },
    Despawn(NodeId),
}

/// Upper bound on ready/flush cascades (a `ready` that spawns nodes whose
/// `ready` spawns nodes ...). Hitting it means an unbounded spawn loop.
const MAX_FLUSH_ROUNDS: usize = 1000;

/// Upper bound on scene changes applied in one go (a new scene whose
/// `ready` asks for another, ...). Hitting it means a change loop.
const MAX_SCENE_CHANGES: usize = 64;

#[derive(Default)]
pub struct NodeTree {
    nodes: SlotMap<NodeId, NodeSlot>,
    root: Option<NodeId>,
    pending: Vec<DeferredCmd>,
    frame: u64,
    /// Seconds of `process` time so far (ADR-050).
    time: f64,
    needs_ready: bool,
    quit: bool,
    signals: SignalTable,
    deferred_emits: Vec<DeferredEmit>,
    /// Members of each group, in join order (ADR-051). A group with no
    /// members has no entry.
    groups: BTreeMap<String, Vec<NodeId>>,
    /// Requested by `change_scene`, applied by `apply_scene_change`
    /// (ADR-054). A later request replaces an earlier one.
    scene_change: Option<SceneChange>,
}

// `Sync` too (a signal payload is `Send + Sync`), so a whole `NodeTree`
// is `Send + Sync` and can live inside a node, as the editor's
// document does.
type DeferredEmit = Box<dyn FnOnce(&mut NodeContext<'_>) + Send + Sync>;

const _: () = {
    fn send_sync<T: Send + Sync>() {}
    let _ = send_sync::<NodeTree>;
};

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
            groups: Vec::new(),
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
            for group in &slot.groups {
                self.leave_index(group, nid);
            }
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
        self.groups.clear();
        self.scene_change = None;
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

    /// The node at `id` as its concrete type `T`; `None` if `id` is not
    /// live, is checked out (the node whose callback is running), or is
    /// not a `T` (ADR-051).
    pub fn get_as<T: Node>(&self, id: NodeId) -> Option<&T> {
        self.get(id)?.as_any().downcast_ref::<T>()
    }

    /// Mutable [`get_as`](Self::get_as).
    pub fn get_as_mut<T: Node>(&mut self, id: NodeId) -> Option<&mut T> {
        self.get_mut(id)?.as_any_mut().downcast_mut::<T>()
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

    /// Like [`Self::attach`], placing `child` at `index` among the
    /// parent's children (clamped to the end). ADR-045.
    pub fn attach_at(
        &mut self,
        child: NodeId,
        parent: NodeId,
        index: usize,
    ) -> Result<(), SceneError> {
        self.attach(child, parent)?;
        let children = &mut self.nodes[parent].children;
        let last = children.pop().expect("just attached");
        children.insert(index.min(children.len()), last);
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

    /// The first descendant of `from` (not `from` itself) whose type is
    /// `T`, in [`descendants`](Self::descendants) order: depth-first,
    /// top-down, child order (ADR-051). A checked-out node is not seen.
    pub fn find_by_class<T: Node>(&self, from: NodeId) -> Option<NodeId> {
        self.descendants(from)
            .find(|id| self.get_as::<T>(*id).is_some())
    }

    /// Every descendant of `from` whose type is `T`, in
    /// [`descendants`](Self::descendants) order.
    pub fn find_all_by_class<T: Node>(&self, from: NodeId) -> Vec<NodeId> {
        self.descendants(from)
            .filter(|id| self.get_as::<T>(*id).is_some())
            .collect()
    }

    /// The direct children of `id` whose type is `T`, in child order,
    /// already downcast.
    pub fn children_of<T: Node>(&self, id: NodeId) -> impl Iterator<Item = (NodeId, &T)> + '_ {
        self.children(id)
            .iter()
            .filter_map(|c| Some((*c, self.get_as::<T>(*c)?)))
    }

    // --- Groups (ADR-051) ---

    /// Put `id` in `group`. Works on a node queued by `spawn` too, so a
    /// node can tag what it spawns. Returns `false` if `id` has no slot,
    /// `group` is empty, or `id` is in it already.
    pub fn add_to_group(&mut self, id: NodeId, group: &str) -> bool {
        let Some(slot) = self.nodes.get_mut(id) else {
            return false;
        };
        if group.is_empty() || slot.groups.iter().any(|g| g == group) {
            return false;
        }
        slot.groups.push(group.to_string());
        self.groups.entry(group.to_string()).or_default().push(id);
        true
    }

    /// Take `id` out of `group`. Returns `false` if it was not in it.
    pub fn remove_from_group(&mut self, id: NodeId, group: &str) -> bool {
        let Some(slot) = self.nodes.get_mut(id) else {
            return false;
        };
        let Some(at) = slot.groups.iter().position(|g| g == group) else {
            return false;
        };
        slot.groups.remove(at);
        self.leave_index(group, id);
        true
    }

    /// True if `id` is live and in `group`.
    pub fn is_in_group(&self, id: NodeId, group: &str) -> bool {
        self.nodes
            .get(id)
            .is_some_and(|s| !s.pending && s.groups.iter().any(|g| g == group))
    }

    /// The live members of `group`, in the order they joined; attached
    /// or floating. A node queued by `spawn` shows up once flushed.
    pub fn nodes_in_group(&self, group: &str) -> Vec<NodeId> {
        self.groups.get(group).map_or_else(Vec::new, |members| {
            members
                .iter()
                .copied()
                .filter(|id| self.contains(*id))
                .collect()
        })
    }

    /// The groups `id` is in, in the order it joined them. Empty for a
    /// stale id.
    pub fn groups_of(&self, id: NodeId) -> &[String] {
        self.nodes.get(id).map_or(&[], |s| &s.groups)
    }

    /// Every group with at least one member (live or queued), sorted
    /// by name.
    pub fn group_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.groups.keys().map(String::as_str)
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

    /// Completed `process` passes. During the first pass it is `0`.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Seconds since the tree started (ADR-050): the sum of every
    /// `process` `dt` so far, the current one included, so it reads the
    /// same in `process`, `physics_process` and `draw` of one frame.
    /// `0.0` before the first `process`; a negative `dt` adds nothing.
    /// `f64`, so it keeps millisecond precision over days of play.
    pub fn time(&self) -> f64 {
        self.time
    }

    // --- Scene change (ADR-054) ---

    /// Ask for the current scene to be replaced. Nothing happens until
    /// the owner of the frame boundary calls
    /// [`apply_scene_change`](Self::apply_scene_change) (`SceneLoop` does
    /// at the end of each `process`). A second request before then
    /// replaces the first: the last one wins.
    pub fn change_scene(&mut self, change: impl Into<SceneChange>) {
        self.scene_change = Some(change.into());
    }

    /// True while a requested change waits to be applied.
    pub fn scene_change_pending(&self) -> bool {
        self.scene_change.is_some()
    }

    /// Drop a requested change without applying it.
    pub fn cancel_scene_change(&mut self) -> Option<SceneChange> {
        self.scene_change.take()
    }

    /// Apply the requested change, if any; call it between passes. The
    /// new scene is built first, as a floating subtree; on success every
    /// old node is finalized (the old root's subtree bottom-up, then any
    /// other old node in insertion order), queued emits are dropped, the
    /// new root becomes the root and is readied by a `flush`. A change
    /// the new scene asks for in its `ready` is applied next, in the same
    /// call (chained changes). On error nothing of the new scene is
    /// left, the current scene stays, and the error is returned.
    ///
    /// `None` when no change was pending; else the last result.
    pub fn apply_scene_change(
        &mut self,
        servers: &mut Servers,
        library: &SceneLibrary,
    ) -> Option<Result<NodeId, SceneError>> {
        let mut result = None;
        let mut applied = 0;
        while let Some(change) = self.scene_change.take() {
            applied += 1;
            assert!(applied <= MAX_SCENE_CHANGES, "scene changes never settled");
            match self.replace_scene(change, servers, library) {
                Ok(root) => result = Some(Ok(root)),
                Err(e) => return Some(Err(e)),
            }
        }
        result
    }

    fn replace_scene(
        &mut self,
        change: SceneChange,
        servers: &mut Servers,
        library: &SceneLibrary,
    ) -> Result<NodeId, SceneError> {
        let old: HashSet<NodeId> = self.nodes.keys().collect();
        let built = change.run(self, library).and_then(|root| {
            let fresh = !old.contains(&root) && self.contains(root);
            if fresh && self.nodes[root].parent.is_none() {
                Ok(root)
            } else {
                Err(SceneError::SceneChange(format!(
                    "the builder returned {root:?}, which is not a new floating node"
                )))
            }
        });
        let new_root = match built {
            Ok(root) => root,
            Err(e) => {
                let leaked: Vec<NodeId> =
                    self.nodes.keys().filter(|id| !old.contains(id)).collect();
                for id in leaked {
                    self.remove(id);
                }
                return Err(e);
            }
        };
        // Queued spawns become live first so they are finalized too.
        self.apply_pending();
        if let Some(root) = self.root {
            self.remove(root);
        }
        let rest: Vec<NodeId> = self.nodes.keys().filter(|id| old.contains(id)).collect();
        for id in rest {
            self.remove(id);
        }
        self.pending.clear();
        self.deferred_emits.clear();
        self.set_root(new_root);
        self.flush(servers);
        Ok(new_root)
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
        self.time += f64::from(dt.max(0.0));
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

    /// Top-down, tree order. Each node sets the server's z first, then
    /// `draw`, its children, and `draw_after_children` (ADR-035). A node
    /// with `visible() == false` hides its whole subtree. `ctx.time()`
    /// reads this tree's [`time`](Self::time).
    pub fn draw(&self, ctx: &mut DrawContext) {
        ctx.set_time(self.time);
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
            groups: Vec::new(),
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

    fn leave_index(&mut self, group: &str, id: NodeId) {
        if let Some(members) = self.groups.get_mut(group) {
            members.retain(|m| *m != id);
            if members.is_empty() {
                self.groups.remove(group);
            }
        }
    }

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
        let children = self.children(id);
        for child in children {
            self.draw_subtree(*child, ctx);
        }
        if !children.is_empty() {
            ctx.render().set_z(node.z());
        }
        node.draw_after_children(ctx);
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
