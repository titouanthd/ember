---
title: "21 — `ember_scene`"
aliases: ["21 — `ember_scene`"]
tags: ["ember/spec", "ember/wave-3"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/21-ember-scene.md"
imported: "2026-11-22"
wave: "3 — Crate Specifications"
---

# 21 — `ember_scene`

> **Status:** 🟡 partial — reviewed 2026-11-19; tree, signals, `NodeContext`/`DrawContext`, `SceneLoop`, `PackedScene` exist; `SceneWriter`/`SceneReader`, `ctx.assets()`, scene connections and several tree queries are not implemented.
> **Audience:** implementers
> **Prerequisites:** [[10-nodes-and-scenes]], [[20-ember-core]]

## 1. Purpose

Specify the full API of `ember_scene`: the `Node` trait, `NodeId`,
`NodeTree`, `NodeContext`, `DrawContext`, `PackedScene`, `SceneLoop`, `SceneWriter` (stub),
and `SignalTable`. This is the crate that every game
interacts with most.

## 2. Module layout

```
ember-scene/src/
├── lib.rs          re-exports + prelude module
├── error.rs        SceneError
├── node.rs         Node, NodeId (slotmap key)
├── tree.rs         NodeTree (+ deferred commands, flush)
├── context.rs      NodeContext, DrawContext, Servers
├── class_db.rs     ClassDbNodes (Node registration on ember_core::ClassDB)
├── packed.rs       PackedScene, SceneAssets
├── scene.rs        SceneWriter (empty stub)
├── scene_loop.rs   SceneLoop<R, A>: MainLoop + InputSink
└── signal.rs       Signal, SignalTable, ScopedConnection
```

*Not yet implemented:* `reader.rs` (`SceneReader`); the deferred queue lives
in `tree.rs`, not `commands.rs`.

## 3. `NodeId`

```rust
// slotmap::new_key_type!
pub struct NodeId;   // Copy, Eq, Ord, Hash, Debug, Default

impl NodeId {
    pub fn is_null(self) -> bool;   // see §3.2
}
```

`NodeId` is a `slotmap` key (slot index + generation); there is no
`debug_bits` and no custom `Display`.

### 3.1 Contract

- `NodeId` is `Copy` (and therefore `Clone`).
- A `NodeId` obtained from `tree.insert()` is valid until `tree.remove()`
  of that specific node.
- After removal, `tree.get(id)` returns `None`.
- Node ids are stable across frames as long as the node is alive.

### 3.2 `is_null()`

`NodeId::default()` is the "null" sentinel used in `Option`-free APIs
(e.g. struct fields that store "no target"). It is never issued by
`tree.insert()`. There is no `NodeId::NULL` constant.

## 4. `Node` trait

```rust
pub trait Node: Object {
    // === Identity ===
    fn name(&self) -> &str;
    fn set_name(&mut self, name: String);
    fn z(&self) -> f32 { 0.0 }
    fn visible(&self) -> bool { true }

    // === Lifecycle ===
    fn ready(&mut self, ctx: &mut NodeContext) {}
    fn process(&mut self, dt: f32, ctx: &mut NodeContext) { let _ = (dt, ctx); }
    fn physics_process(&mut self, fixed_dt: f32, ctx: &mut NodeContext) {
        let _ = (fixed_dt, ctx);
    }
    fn draw(&self, ctx: &mut DrawContext) { let _ = ctx; }
    fn finalize(&mut self) {}

    // === Serialization ===
    /// Stub in v0.1 (`SceneWriter` is empty).
    fn serialize(&self, _w: &mut SceneWriter) {}

    /// Apply one `.escene` property. Returns `false` if the key or
    /// variant type is not understood (the loader warns and skips it).
    fn set_property(&mut self, _key: &str, _value: &Variant) -> bool { false }
}
```

*Not yet implemented:* `Node::signals` / `signals_mut` / `post_deserialize`
(signals live on the tree, ADR-023).

### 4.1 Defaults and the derive macro

`Node` requires `name` and `set_name`; everything else has a default. The
`#[derive(Node)]` macro (see [[26-ember-macros]])
generates the `Object` impl and `name`/`set_name`.

### 4.2 Signals are not on the node

There is no per-node `SignalTable`: ADR-023 keeps a single tree-level
table, so nodes never override signal methods. `NodeContext::connect` /
`emit` are the only entry points.

## 5. `NodeTree`

### 5.1 Struct

```rust
pub struct NodeTree {
    nodes: SlotMap<NodeId, NodeSlot>,   // slot.node is Option<Box<dyn Node>>:
                                        // checked out during callbacks
    root: Option<NodeId>,
    pending: Vec<DeferredCmd>,
    signals: SignalTable,               // one table for the whole tree
    deferred_emits: Vec<DeferredEmit>,
    frame: u64,
    quit: bool,
    /* ... */
}
```

`SlotMap` is the [`slotmap`](https://crates.io/crates/slotmap) crate's
`SlotMap<NodeId, V>`. We use it directly (see
[[50-design-decisions]]).

### 5.2 API

```rust
impl NodeTree {
    // === Construction ===
    pub fn new() -> Self;
    pub fn with_root(node: impl Node) -> Self;

    // === Insert / remove ===
    pub fn insert(&mut self, node: impl Node) -> NodeId;
    pub fn insert_boxed(&mut self, node: Box<dyn Node>) -> NodeId;
    pub fn remove(&mut self, id: NodeId) -> Option<Box<dyn Node>>;
    pub fn clear(&mut self);

    // === Access ===
    pub fn get(&self, id: NodeId) -> Option<&dyn Node>;
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut dyn Node>;
    pub fn contains(&self, id: NodeId) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;

    // === Tree structure ===
    pub fn attach(&mut self, child: NodeId, parent: NodeId) -> Result<(), SceneError>;
    pub fn detach(&mut self, child: NodeId) -> Result<(), SceneError>;
    pub fn reparent(&mut self, child: NodeId, new_parent: NodeId) -> Result<(), SceneError>;
    pub fn parent(&self, id: NodeId) -> Option<NodeId>;
    pub fn children(&self, id: NodeId) -> &[NodeId];
    pub fn descendants(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_;

    // === Root ===
    pub fn root(&self) -> Option<NodeId>;
    pub fn set_root(&mut self, id: NodeId);

    // === Queries ===
    pub fn find(&self, from: NodeId, path: &str) -> Option<NodeId>;
    // Not yet implemented: find_by_class, find_all_by_class

    // === Iteration ===
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &dyn Node)> + '_;

    // === Frame loop ===
    pub fn process(&mut self, servers: &mut Servers, dt: f32);       // ADR-022
    pub fn physics_process(&mut self, servers: &mut Servers, fixed_dt: f32);
    pub fn draw(&self, ctx: &mut DrawContext);

    /// Flush deferred commands (insertions, removals, signal connects).
    /// Called automatically at the end of process/physics_process.
    pub fn flush(&mut self, servers: &mut Servers);

    pub fn frame(&self) -> u64;

    // === Quit / signals ===
    pub fn request_quit(&mut self);
    pub fn quit_requested(&self) -> bool;
    pub fn connect<T: Signal>(&mut self, target: NodeId, handler: Handler<T>) -> u64;
    pub fn connect_scoped<T: Signal>(&mut self, target: NodeId, handler: Handler<T>) -> ScopedConnection;
    pub fn disconnect<T: Signal>(&mut self, target: NodeId);
    pub fn signals(&self) -> &SignalTable;
}
```

### 5.3 Attach / detach semantics

**`attach(child, parent)`:**

- `Err(IsRoot)` if `child` is the root.
- `Err(SelfParent)` if `child == parent`.
- `Err(Cycle { child, parent })` if `parent` is a descendant of `child`.
- `Err(AlreadyAttached)` if `child` already has a parent (`detach` first).
- `Err(NotFound)` for unknown ids; `detach` of a parentless node is `Err(NotAttached)`.
- On success, pushes `child` onto `parent.children`, sets `child.parent`.
- If `parent` is in the tree and `child.ready == false`, the tree marks
  `child` for `ready` at next flush.

**`detach(child)`:**

- Removes from `parent.children`.
- Sets `child.parent = None`.
- Does not recursively detach children (they stay attached to `child`,
  which is now a floating subtree).
- Does not remove the node — it stays in the `SlotMap`, unreachable until
  either `attach`ed again or `remove`d.

**`remove(id)`:**

- Recursively removes `id` and all descendants.
- Disconnects all signals targeting each removed node.
- Returns the root's `Box<dyn Node>` (descendants are dropped).
- *Not yet verified/implemented:* `Node::finalize` is called by
  `SceneLoop::finalize` on shutdown, not per removed node.

### 5.4 Process order

`process(dt)`:

1. `flush()` (apply pending commands, run `ready`).
2. Walk the tree depth-first, top-down, in `children` order; each node is
   checked out of its slot while its callback runs (ADR-022).
3. For each node: `node.process(dt, &mut ctx)`.
4. `flush()` again.
5. Advance `frame` (`process` only; `physics_process` does not).

`physics_process(fixed_dt)` is identical but calls `physics_process` on
each node.

`draw(ctx)`:

1. Walk tree in same order.
2. Skip nodes where `visible() == false`; **the whole subtree is hidden**
   (ADR-022). Each node sets the server z (`set_z(node.z())`) first.
3. For each visible node: `node.draw(ctx)`.

### 5.5 Ready order

When `flush()` runs (it loops until no commands, deferred emits or ready
nodes remain), any newly-attached node whose subtree is fully
attached and whose `ready == false` gets its `ready` called **bottom-up**
(children first, then parent). This matches Godot.

### 5.6 `find` path syntax

Supported in v0.1:

| Path | Meaning |
|---|---|
| `""` | the starting node itself |
| `"."` | the starting node itself |
| `"Child"` | direct child named `Child` |
| `"Child/Grandchild"` | grandchild |
| `".."` | parent |
| `"../Sibling"` | parent's child named `Sibling` |

Not supported (v0.2 candidates): `%Name` (unique names), `$Node`
(shorthand), `@Group` (groups), `*` (wildcards).

## 6. `NodeContext`

```rust
/// Bundle of the backends, handed to every tree pass (ADR-022).
pub struct Servers<'a> {
    pub rendering: &'a mut dyn RenderingServer,
    pub audio: &'a mut dyn AudioServer,
    pub physics: &'a mut dyn PhysicsServer,
    pub input: &'a Input,
}

pub struct NodeContext<'a> { /* tree, servers, current id, parent, delta */ }

impl<'a> NodeContext<'a> {
    // === Identity ===
    pub fn current_id(&self) -> NodeId;
    pub fn parent_id(&self) -> Option<NodeId>;
    pub fn delta(&self) -> f32;

    // === Server access ===
    pub fn render(&mut self) -> &mut dyn RenderingServer;
    pub fn audio(&mut self) -> &mut dyn AudioServer;
    pub fn physics(&mut self) -> &mut dyn PhysicsServer;
    pub fn input(&self) -> &Input;

    // === Tree access ===
    pub fn tree(&mut self) -> &mut NodeTree;
    pub fn find(&mut self, path: &str) -> Option<NodeId>;   // relative to current
    pub fn quit(&mut self);

    // === Spawn / despawn (deferred) ===
    pub fn spawn(&mut self, node: impl Node) -> NodeId;
    pub fn spawn_child(&mut self, parent: NodeId, node: impl Node) -> NodeId;
    pub fn despawn(&mut self, id: NodeId);

    // === Signals ===
    pub fn emit<T: Signal>(&mut self, payload: T);
    pub fn emit_deferred<T: Signal>(&mut self, payload: T);
    pub fn connect<T: Signal>(
        &mut self,
        target: NodeId,
        handler: impl Fn(&mut NodeContext, &T) + Send + Sync + 'static,
    );  // see context.rs for the exact generic bounds
    pub fn connect_scoped<T: Signal>(
        &mut self,
        target: NodeId,
        handler: impl Fn(&mut NodeContext, &T) + Send + Sync + 'static,
    ) -> ScopedConnection;
    pub fn disconnect<T: Signal>(&mut self, target: NodeId);
}
```

*Not yet implemented:* `ctx.assets()`. Assets are loaded by the game
before building the `SceneLoop` (see [[25-ember-assets]]).

### 6.1 Spawn is deferred

`ctx.spawn(node)` returns the `NodeId` **immediately**, but the node is
not yet inserted into the tree's live structure. The insertion is queued
in `pending` and applied at `flush()`. Between `spawn` and `flush`:

- `tree.get(id)` returns `None`.
- `ctx.find(path)` won't find it.
- But `tree.attach(new_id, parent_id)` also queues, so a
  spawn-then-attach sequence works.

This is what prevents iterator-invalidation bugs.

### 6.2 `connect` timing

`connect` registers the handler in the tree's single `SignalTable`
immediately (ADR-023). `emit` dispatches synchronously on a snapshot of
the listeners; `emit_deferred` queues the emission until the next
`flush`. Dropping a `ScopedConnection` queues a disconnect applied at the
next flush or emit.

## 7. `DrawContext`

```rust
pub struct DrawContext<'a> {
    rendering: &'a mut dyn RenderingServer,
    viewport: Rect,
    camera: Vec2,
    current_z: f32,
}

impl<'a> DrawContext<'a> {
    pub fn new(rendering: &'a mut dyn RenderingServer, /* viewport, camera */) -> Self;
    pub fn render(&mut self) -> &mut dyn RenderingServer;
    pub fn viewport(&self) -> Rect;
    pub fn camera_offset(&self) -> Vec2;
    pub fn world_to_screen(&self, world: Vec2) -> Vec2;
    pub fn screen_to_world(&self, screen: Vec2) -> Vec2;
    pub fn current_z(&self) -> f32;
}
```

`DrawContext` provides **read-only** access to the tree — there is no
`ctx.tree()` method. Draw is a pure rendering pass; mutation happens in
`process` / `physics_process`.

## 8. `PackedScene`

```rust
pub struct PackedScene { /* parsed `.escene` file (private serde model) */ }

impl PackedScene {
    pub fn from_ron(text: &str) -> Result<Self, SceneError>;
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, SceneError>;
    pub fn to_ron(&self) -> String;
    pub fn uid(&self) -> &str;
    pub fn root_template_class(&self) -> &str;

    /// Builds nodes in `tree`; the root is returned detached.
    /// `instance:` entries fail with `SceneError::Asset`.
    pub fn instantiate(&self, classes: &ClassDB, tree: &mut NodeTree)
        -> Result<NodeId, SceneError>;

    /// Same, resolving nested `instance:` through the `AssetServer`
    /// (cycles → `InstanceCycle`, ADR-027).
    pub fn instantiate_with(&self, classes: &ClassDB, assets: &AssetServer,
        tree: &mut NodeTree) -> Result<NodeId, SceneError>;

    /// `instantiate` then `attach` to `parent`.
    pub fn instantiate_into(&self, classes: &ClassDB, tree: &mut NodeTree,
        parent: NodeId) -> Result<NodeId, SceneError>;
}
```

`PackedScene` implements `ember_assets::Asset` (extension `escene`,
format 1), so `AssetServer::load::<PackedScene>` works. The
`SceneAssets` trait (`scene(path)`) is implemented for `AssetServer`.
Instantiation is **synchronous on the tree**, not via `NodeContext`.

### 8.1 Instantiation algorithm

1. Verify every non-instance class is registered (`ClassNotRegistered`).
2. Walk the node entries in order; construct via
   `ClassDbNodes::instantiate_node`, apply `visible`/`z` and each
   property through `Node::set_property`. Unknown or mistyped properties
   are warned and skipped (ADR-025).
3. Attach children to their parents (`parent` path); nested `instance:`
   entries are built recursively (overrides reach only the nested root,
   ADR-027).
4. `connections` are parsed and validated but **not wired**
   (*Not yet implemented*, ADR-025); `ext_resources` are parsed but not
   resolved.
5. Return the root id (detached).

### 8.2 Deep-copy guarantee

Each `instantiate` produces fresh node instances. Exported values are
copied. `PackedScene` holds no live node references.

## 9. `SceneWriter` / `SceneReader`

*Not yet implemented.* `SceneWriter` exists only as an empty struct
(`pub struct SceneWriter {}`) so that `Node::serialize` has a stable
signature; `SceneReader` does not exist. `PackedScene::to_ron` /
`from_ron` handle (de)serialisation of the file model today. The planned
writer API:

```rust
impl SceneWriter {
    pub fn new(format: u32) -> Self;
    pub fn write_ext_resource(&mut self, class: &str, uid: &str, path: &str);
    pub fn write_node(&mut self, name: &str, class: &str, parent: Option<&str>,
                      properties: BTreeMap<String, Variant>);
    pub fn write_connection(&mut self, signal: &str, from: &str, to: &str, method: &str);
    pub fn finish(self) -> String;
}
```

## 10. Signals

### 10.1 `SignalTable`

```rust
pub struct SignalTable { /* HashMap<TypeId, Vec<HandlerEntry>>, next_id, ... */ }

impl SignalTable {
    pub fn new() -> Self;
    pub fn connect<T: Signal>(&mut self, target: NodeId, handler: Handler<T>) -> u64;
    pub fn connect_scoped<T: Signal>(&mut self, target: NodeId, handler: Handler<T>) -> ScopedConnection;
    pub fn disconnect_by_id(&mut self, id: u64);
    pub fn disconnect_type_target<T: Signal>(&mut self, target: NodeId);
    pub fn remove_target(&mut self, target: NodeId);
    pub fn is_connected(&self, id: u64) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn tick(&mut self);   // apply queued scoped disconnects
}
```

`Handler<T>` is `Arc<dyn Fn(&mut NodeContext<'_>, &T) + Send + Sync>`.
Emission is driven by `NodeContext::emit` (there is no `SignalTable::emit`).

### 10.2 `ScopedConnection`

```rust
pub struct ScopedConnection { /* connection id + shared drop queue */ }
impl ScopedConnection {
    pub fn id(&self) -> u64;
    pub fn forget(self);   // keep the connection alive after drop
}
impl Drop for ScopedConnection { /* queues a disconnect */ }
```

Dropping a `ScopedConnection` removes the handler at the next `flush()` or emit.

### 10.3 `Signal` trait

```rust
pub trait Signal: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Signal for T {}
```

Blanket impl — any `Send + Sync + 'static` type is a valid signal
payload. This is why the payload struct is the "signal identifier".

## 11. Errors

```rust
#[derive(Debug)]
#[non_exhaustive]
pub enum SceneError {
    NotFound(NodeId),
    SelfParent(NodeId),
    IsRoot(NodeId),
    AlreadyAttached(NodeId),
    NotAttached(NodeId),
    Cycle { child: NodeId, parent: NodeId },
    Parse(String),
    UnsupportedFormat(u32),
    ClassNotRegistered(String),
    InvalidName(String),
    Io(std::io::Error),
    Asset(String),            // nested instance load failed
    InstanceCycle(String),    // scene A instances A (ADR-027)
}
```

`Display`/`Error` are hand-written (no `thiserror`).

## 12. Prelude

The prelude (`ember_scene::prelude`) re-exports the main types; check
`lib.rs` for the current list (it includes `Node`, `NodeId`, `NodeTree`,
`NodeContext`, `DrawContext`, `PackedScene`, `Signal`, `ScopedConnection`).

### 12.1 `SceneLoop`

```rust
pub struct SceneLoop<R: RenderingServer, A: AudioServer> {
    pub tree: NodeTree, /* backends, LinearPhysics, Input, viewport */
}
impl<R, A> SceneLoop<R, A> {
    pub fn new(tree: NodeTree, rendering: R, audio: A, viewport: Rect) -> Self;
    pub fn input_mut(&mut self) -> &mut Input;
    pub fn set_viewport(&mut self, viewport: Rect);
    pub fn rendering(&self) -> &R;  pub fn rendering_mut(&mut self) -> &mut R;
    pub fn audio(&self) -> &A;
    pub fn physics_mut(&mut self) -> &mut LinearPhysics;
    pub fn request_quit(&mut self);
}
```

Implements `ember_core::MainLoop` and `ember_input::InputSink`. `draw` =
`begin_frame`, `tree.draw`, `end_frame`, `audio.flush`; `finalize` clears
the tree. `ClassDbNodes` (extension trait on `ClassDB`) provides
`register_node`, `register_node_with`, `instantiate_node`.

## 13. Design notes

### 13.1 Why `slotmap` and not `Arena` + manual generational indices

`slotmap` already does what we need, is mature, and has zero unsafe in
its public API. Rolling our own is 300 lines of `unsafe` for no
performance gain.

### 13.2 Why signals live on the tree, not the node

If `Node` owned its `SignalTable`, then `Node::signals_mut` would be
called on every emit — but emit needs to reach into *another* node. That
would require `&mut NodeTree` plus `&mut SignalTable` on a node inside
it — a double-borrow. Owning signal tables on the tree side keeps the
borrow scoped to `NodeContext`.

### 13.3 Why `find` returns `Option`, not `Result`

Because "node not found" is normal (a path may be conditional). Returning
`Result` would force every call site to handle an error that the game
usually doesn't care about. The rare caller that needs diagnostic
information uses `ctx.tree().find_path_debug(...)` (*Not yet implemented*, v0.2).

### 13.4 Why `spawn` is deferred but returns a live id

Because two patterns are common:

1. **Fire-and-forget.** `ctx.spawn(Projectile::new(...))` — no need for
   the id, the projectile manages itself.
2. **Reference.** `let enemy = ctx.spawn(Enemy::new(...)); self.target =
   Some(enemy);` — the caller stores the id and uses it later.

In case (2), the caller does **not** dereference the id on the same
frame — the `spawn` is committed at `flush()`, and the next `process`
sees a live node. Deferring the insertion is invisible to legitimate
callers.

### 13.5 Why draw doesn't see the tree

Because drawing that mutates is the source of half the bugs in game
code. A `draw` that spawns particles, plays sounds, or emits signals is
a bug waiting to happen. `DrawContext` makes it structurally impossible.

## 14. Open questions

- **Q1:** Should `NodeTree::iter` iterate pending nodes? *Provisional:*
  no. Pending nodes are not yet part of the tree.
- **Q2:** Should `remove` be recursive by default? *Provisional:* yes —
  matches Godot's `queue_free`.
- **Q3:** Should `NodeContext` allow re-entrant `process`? *Provisional:*
  no — `process` calls `&mut self`, so re-entrancy would need `RefCell`
  and would reintroduce the exact problem `&mut` avoids.
- **Q4:** Should `NodeId` implement `serde::Serialize`? *Provisional:*
  no. Node ids are runtime-only; saving them would be a bug.
- **Q5:** Should we expose `tree.children_mut()` for bulk mutation?
  *Provisional:* no. Mutation goes through `attach` / `detach`.

## 15. Cross-references

- Node/scene mental model → [[10-nodes-and-scenes]]
- Signals deep dive → [[11-signals]]
- `.escene` format → [[30-escene-format]]
- `#[derive(Node)]` expansion → [[26-ember-macros]]
- Server traits used by `NodeContext` → [[22-ember-servers]]
