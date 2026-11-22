---
title: "21 — `ember_scene`"
aliases: ["21 — `ember_scene`"]
tags: ["ember/spec", "ember/wave-3"]
doc-status: "partial"
former-path: "docs/engine/v0.1/21-ember-scene.md"
imported: "2026-11-22"
wave: "3 — Crate Specifications"
---

# 21 — `ember_scene`

> **Status:** 🟡 partial — tree, signals, `PackedScene` implemented (ADR-022 to 025)
> **Audience:** implementers
> **Prerequisites:** [[10-nodes-and-scenes]], [[20-ember-core]]

## 1. Purpose

Specify the full API of `ember_scene`: the `Node` trait, `NodeId`,
`NodeTree`, `NodeContext`, `DrawContext`, `PackedScene`, `SceneWriter`,
`SceneReader`, and `SignalTable`. This is the crate that every game
interacts with most.

## 2. Module layout

```
ember_scene/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── prelude.rs
│   ├── error.rs        SceneError
│   ├── node.rs         Node, NodeId
│   ├── tree.rs         NodeTree
│   ├── context.rs      NodeContext, DrawContext
│   ├── packed.rs       PackedScene
│   ├── writer.rs       SceneWriter
│   ├── reader.rs       SceneReader
│   ├── signal.rs       SignalTable, ScopedConnection
│   └── commands.rs     DeferredCmd queue
```

## 3. `NodeId`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct NodeId(u64);   // opaque; internals are (slot: u32, generation: u32)

impl NodeId {
    /// For debugging only. Not stable across runs.
    pub fn debug_bits(self) -> u64;
    pub fn is_null(self) -> bool;   // see §3.2
}

impl std::fmt::Display for NodeId { /* prints "Node#<slot>:<gen>" */ }
```

### 3.1 Contract

- `NodeId` is `Copy`. It is not `Clone` (identity is the value).
- A `NodeId` obtained from `tree.insert()` is valid until `tree.remove()`
  of that specific node.
- After removal, `tree.get(id)` returns `None`.
- Node ids are stable across frames as long as the node is alive.

### 3.2 `is_null()`

`NodeId(0)` — with slot=0, gen=0 — is the "null" sentinel used in
`Option`-free APIs (e.g. struct fields that store "no target"). It is
never issued by `tree.insert()`.

```rust
pub const NULL: NodeId = NodeId(0);
```

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

    // === Signals ===
    fn signals(&self) -> &SignalTable { &EMPTY_SIGNALS }
    fn signals_mut(&mut self) -> &mut SignalTable { unimplemented!() }

    // === Serialization ===
    fn serialize(&self, _w: &mut SceneWriter) {}

    /// Called after deserialization. Default no-op.
    fn post_deserialize(&mut self) {}
}
```

### 4.1 Defaults and the derive macro

Every method except `name`, `set_name`, `signals` (defaulted),
`signals_mut` (panics by default) is usable as-is. The `#[derive(Node)]`
macro generates all of them, plus `Object::class_name`, `as_any`,
`as_any_mut`, and a `new()` constructor.

### 4.2 Why `signals_mut` panics by default

Because 90% of nodes never emit or receive signals. A default `&mut
SignalTable` would require every node to allocate one. Instead:

- Nodes that never touch signals: `signals()` returns `&EMPTY_SIGNALS`,
  `signals_mut()` never called.
- Nodes that do: derive macro or manual impl overrides both.

`NodeContext::connect` / `emit` handle the `signals_mut` call internally
— game code never calls `signals_mut` directly.

## 5. `NodeTree`

### 5.1 Struct

```rust
pub struct NodeTree {
    nodes: SlotMap<NodeId, NodeSlot>,
    root: Option<NodeId>,
    pending: Vec<DeferredCmd>,
    frame: u64,
}

struct NodeSlot {
    node: Box<dyn Node>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    signal_table: SignalTable,   // owned here, borrowed via &mut
    ready: bool,
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
    pub fn find_by_class(&self, from: NodeId, class: &str) -> Option<NodeId>;
    pub fn find_all_by_class(&self, class: &str) -> Vec<NodeId>;

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
}
```

### 5.3 Attach / detach semantics

**`attach(child, parent)`:**

- Fails if `child` is the root.
- Fails if `child == parent`.
- Fails if `parent` is a descendant of `child` (would create a cycle).
- Fails if `child` already has a parent (must `detach` first).
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
- Runs `Node::finalize` for each (bottom-up).
- Disconnects all signals targeting each removed node.
- Returns the root's `Box<dyn Node>` (descendants are dropped).

### 5.4 Process order

`process(dt)`:

1. Reset `ctx.delta = dt`.
2. Walk the tree depth-first, top-down, in `children` order.
3. For each node: `node.process(dt, &mut ctx)`.
4. After the walk: `flush()` (applies `DeferredCmd`s).
5. Advance `frame`.

`physics_process(fixed_dt)` is identical but calls `physics_process` on
each node.

`draw(ctx)`:

1. Walk tree in same order.
2. Skip nodes where `visible() == false` (children still process).
3. For each visible node: `node.draw(ctx)`.

### 5.5 Ready order

When `flush()` runs, any newly-attached node whose subtree is fully
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
pub struct NodeContext<'a> {
    tree: &'a mut NodeTree,
    rendering: &'a mut dyn RenderingServer,
    audio: &'a mut dyn AudioServer,
    physics: &'a mut dyn PhysicsServer,
    assets: &'a AssetServer,
    input: &'a Input,
    current: NodeId,
    delta: f32,
    input_pressed: HashSet<Key>,   // for pressed_frame semantics
}

impl<'a> NodeContext<'a> {
    // === Identity ===
    pub fn current_id(&self) -> NodeId;
    pub fn delta(&self) -> f32;

    // === Server access ===
    pub fn render(&mut self) -> &mut dyn RenderingServer;
    pub fn audio(&mut self) -> &mut dyn AudioServer;
    pub fn physics(&mut self) -> &mut dyn PhysicsServer;
    pub fn assets(&self) -> &AssetServer;
    pub fn input(&self) -> &Input;

    // === Tree access ===
    pub fn tree(&mut self) -> &mut NodeTree;
    pub fn parent_id(&self) -> Option<NodeId>;
    pub fn find(&mut self, path: &str) -> Option<NodeId>;

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
    );
    pub fn connect_scoped<T: Signal>(
        &mut self,
        target: NodeId,
        handler: impl Fn(&mut NodeContext, &T) + Send + Sync + 'static,
    ) -> ScopedConnection;
    pub fn disconnect<T: Signal>(&mut self, target: NodeId);
}
```

### 6.1 Spawn is deferred

`ctx.spawn(node)` returns the `NodeId` **immediately**, but the node is
not yet inserted into the tree's live structure. The insertion is queued
in `pending` and applied at `flush()`. Between `spawn` and `flush`:

- `tree.get(id)` returns `None`.
- `ctx.find(id, ...)` won't find it.
- But `tree.attach(new_id, parent_id)` also queues, so a
  spawn-then-attach sequence works.

This is what prevents iterator-invalidation bugs.

### 6.2 `connect` timing

`connect` is deferred too — the handler is registered in the target's
`signal_table` at `flush()`. Between `connect` and `flush`, emitting the
signal has no effect on that receiver. This is intentional: a `ready`
function that connects should not receive events in the same frame.

## 7. `DrawContext`

```rust
pub struct DrawContext<'a> {
    rendering: &'a mut dyn RenderingServer,
    viewport: Rect,
    camera: Vec2,
    current_z: f32,
}

impl<'a> DrawContext<'a> {
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
pub struct PackedScene {
    root_template: NodeTemplate,
    ext_resources: Vec<ExtResource>,
    connections: Vec<ConnectionTemplate>,
}

impl PackedScene {
    pub fn from_file(path: &Path, assets: &AssetServer) -> Result<Self, SceneError>;
    pub fn from_ron(ron: &str) -> Result<Self, SceneError>;
    pub fn to_ron(&self) -> String;

    /// Instantiate as a standalone tree root.
    pub fn instantiate(&self, ctx: &mut NodeContext) -> Result<NodeId, SceneError>;

    /// Instantiate as a child of `parent`.
    pub fn instantiate_into(
        &self,
        ctx: &mut NodeContext,
        parent: NodeId,
    ) -> Result<NodeId, SceneError>;

    pub fn root_template_class(&self) -> &str;
}
```

### 8.1 Instantiation algorithm

1. Walk `root_template` recursively.
2. For each node template:
   - Look up `class_name` in `ClassDB`.
   - Construct via `ClassDB::instantiate`.
   - Set `#[export]` fields via `Variant` deserialization.
   - Call `post_deserialize`.
3. Attach children to parents (deferred via `ctx.spawn_child`).
4. Apply `connections` (deferred via `ctx.connect`).
5. Return the root id.

All steps run through `NodeContext`, so the tree updates are deferred
and consistent with the frame loop.

### 8.2 Deep-copy guarantee

Each `instantiate` produces fresh node instances. Exported values are
copied. `PackedScene` holds no live node references.

## 9. `SceneWriter` / `SceneReader`

### 9.1 `SceneWriter`

```rust
pub struct SceneWriter {
    format: u32,
    ext_resources: Vec<ExtResource>,
    sub_resources: Vec<SubResource>,
    nodes: Vec<NodeBlock>,
    connections: Vec<Connection>,
}

impl SceneWriter {
    pub fn new(format: u32) -> Self;
    pub fn write_ext_resource(&mut self, class: &str, uid: &str, path: &str);
    pub fn write_node(
        &mut self,
        name: &str,
        class: &str,
        parent: Option<&str>,
        properties: BTreeMap<String, Variant>,
    );
    pub fn write_connection(&mut self, signal: &str, from: &str, to: &str, method: &str);
    pub fn finish(self) -> String;   // RON text
}
```

`Node::serialize` receives `&mut SceneWriter` and calls these methods.
The writer accumulates everything, then `finish()` produces the file.

### 9.2 `SceneReader`

```rust
pub struct SceneReader {
    format: u32,
    ext_resources: Vec<ExtResource>,
    sub_resources: Vec<SubResource>,
    nodes: Vec<NodeBlock>,
    connections: Vec<Connection>,
}

impl SceneReader {
    pub fn parse(ron: &str) -> Result<Self, SceneError>;
    pub fn format(&self) -> u32;
    pub fn ext_resources(&self) -> &[ExtResource];
    pub fn nodes(&self) -> &[NodeBlock];
    pub fn connections(&self) -> &[Connection];
}
```

The reader does not construct nodes; `PackedScene::instantiate` does.
This separation is what makes the format inspectable by the editor
without touching runtime types.

## 10. Signals

### 10.1 `SignalTable`

```rust
pub struct SignalTable {
    handlers: HashMap<TypeId, Vec<HandlerEntry>>,
    next_id: u64,
}

impl SignalTable {
    pub fn new() -> Self;
    pub fn connect<T: Signal>(&mut self, target: NodeId, handler: Handler<T>) -> u64;
    pub fn disconnect_by_id(&mut self, connection_id: u64);
    pub fn disconnect_type_target<T: Signal>(&mut self, target: NodeId);
    pub fn emit<'a, T: Signal>(&mut self, payload: T, ctx: &mut NodeContext<'a>);
    pub fn tick(&mut self);   // remove stale entries
}
```

`Handler<T>` is `Box<dyn Fn(&mut NodeContext, &T) + Send + Sync + 'static>`.

### 10.2 `ScopedConnection`

```rust
pub struct ScopedConnection {
    table_id: u64,
    connection_id: u64,
    // reference to tree's signal tables so Drop can disconnect
}
impl Drop for ScopedConnection { /* disconnects */ }
```

Dropping a `ScopedConnection` removes the handler on the next `flush()`.

### 10.3 `Signal` trait

```rust
pub trait Signal: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Signal for T {}
```

Blanket impl — any `Send + Sync + 'static` type is a valid signal
payload. This is why the payload struct is the "signal identifier".

## 11. Errors

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SceneError {
    #[error("node not found: {0:?}")]
    NodeNotFound(NodeId),

    #[error("node already attached: {0:?}")]
    AlreadyAttached(NodeId),

    #[error("would create cycle: {0:?} as child of {1:?}")]
    CycleDetected(NodeId, NodeId),

    #[error("cannot attach root")]
    CannotAttachRoot,

    #[error("class not registered: {0}")]
    ClassNotRegistered(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("format version {found} not supported (max {max})")]
    UnsupportedFormat { found: u32, max: u32 },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

## 12. Prelude

```rust
pub mod prelude {
    pub use crate::node::{Node, NodeId};
    pub use crate::tree::NodeTree;
    pub use crate::context::{NodeContext, DrawContext};
    pub use crate::packed::PackedScene;
    pub use crate::signal::{Signal, ScopedConnection};
}
```

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
information uses `ctx.tree().find_path_debug(...)` (v0.2).

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
