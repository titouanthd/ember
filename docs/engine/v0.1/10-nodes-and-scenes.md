# 10 — Nodes and Scenes

> **Status:** 🟡 partial — reviewed 2026-11-19; `Node`/`NodeTree`/contexts/`PackedScene` implemented (ADR-022..027); `#[export]`, serialization, signal wiring from scenes and `find_by_class` are pending
> **Audience:** game authors, implementers
> **Prerequisites:** [01-architecture.md](01-architecture.md)

## 1. Purpose

Explain the mental model that every Ember game uses: **everything is a
node; nodes nest into scenes; a scene is just a node subtree**. This is
the file you read before writing your first game.

If you already know Godot, most of this will feel familiar. The
differences are subtle and deliberate — typed signals instead of string
signals, `NodeId` instead of raw `Node*`, explicit `&mut NodeContext`
instead of implicit `get_node()` calls. Every divergence is explained.

## 2. Concepts

### 2.1 Node

A **Node** is the smallest unit of composition. A sprite is a node. A
button is a node. A player is a node that owns a sprite child and a
collision child. A level is a node that owns a player and a tilemap.

There is no special "entity" type distinct from "UI element" — both are
nodes. This is what makes scenes nest freely and makes reuse natural.

At the Rust level, a node is any type that implements the `Node` trait:

```rust
pub trait Object: Send + Sync + 'static {
    fn class_name() -> &'static str where Self: Sized;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait Node: Object {
    // Identity
    fn name(&self) -> &str;
    fn set_name(&mut self, name: String);
    fn z(&self) -> f32 { 0.0 }          // draw layer
    fn visible(&self) -> bool { true }  // false hides the whole subtree from draw

    // Lifecycle — all defaulted
    fn ready(&mut self, _ctx: &mut NodeContext) {}
    fn process(&mut self, _dt: f32, _ctx: &mut NodeContext) {}
    fn physics_process(&mut self, _fixed_dt: f32, _ctx: &mut NodeContext) {}
    fn draw(&self, _ctx: &mut DrawContext) {}
    fn finalize(&mut self) {}

    // Scene loading / serialization
    fn set_property(&mut self, _key: &str, _value: &Variant) -> bool { false }
    fn serialize(&self, _out: &mut SceneWriter) {} // stub: SceneWriter is empty
}
```

`name`/`set_name` are required (the derive generates them); every other `Node`
method has a default. You only implement what your node cares about.
`Node::serialize` is a stub and `SceneWriter` is empty — *serialization is not
yet implemented*.

### 2.2 NodeId

`NodeId` is an **opaque, generational index** into the tree's internal
arena. It is `Copy`, cheap to compare, and safe to hold across frames —
if the node it referred to has been removed, looking it up returns `None`
instead of a dangling reference (that's the "generational" part — the
slot number is reused, but the generation counter is bumped, so a stale
`NodeId` is detected).

```rust
// slotmap key: Copy, Clone, Eq, Hash, Debug, Default
pub struct NodeId { /* opaque */ }
impl NodeId { pub fn is_null(self) -> bool; } // NodeId::default() is the never-issued null id
```

You never construct a `NodeId` yourself. You get one from
`tree.insert(node)`, `ctx.spawn(node)` or `ctx.current_id()` (inside a `process` call) and
you pass it around.

> **Naming note:** `NodeId` is what Godot would call a node reference. We
> prefer an opaque index over `Rc<RefCell<...>>` for two reasons:
> (1) it is `Copy`, so patterns like "store the id of my target" are
> ergonomic; (2) it makes the borrow checker your friend inside
> `process` — you can hold a `NodeId` while mutating the tree.

### 2.3 NodeTree

`NodeTree` is the **ownership container**. It owns every node in the
program, keyed by `NodeId`. Nodes never own each other directly — a
parent node holds `Vec<NodeId>` of children, and the tree holds the
actual data.

```rust
pub struct NodeTree { /* slotmap + root + pending queue + SignalTable */ }

impl NodeTree {
    pub fn new() -> Self;
    pub fn with_root(node: impl Node) -> Self;
    pub fn insert(&mut self, node: impl Node) -> NodeId;   // floating node
    pub fn remove(&mut self, id: NodeId) -> Option<Box<dyn Node>>; // + descendants, finalize bottom-up
    pub fn clear(&mut self);
    pub fn get(&self, id: NodeId) -> Option<&dyn Node>;
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut dyn Node>;
    pub fn attach(&mut self, child: NodeId, parent: NodeId) -> Result<(), SceneError>;
    pub fn detach(&mut self, child: NodeId) -> Result<(), SceneError>;
    pub fn children(&self, id: NodeId) -> &[NodeId];
    pub fn parent(&self, id: NodeId) -> Option<NodeId>;
    pub fn root(&self) -> Option<NodeId>;
    pub fn set_root(&mut self, id: NodeId);
    // Passes take the server bundle (ADR-022); the tree builds a NodeContext per callback.
    pub fn process(&mut self, servers: &mut Servers, dt: f32);
    pub fn physics_process(&mut self, servers: &mut Servers, fixed_dt: f32);
    pub fn flush(&mut self, servers: &mut Servers);
    pub fn draw(&self, ctx: &mut DrawContext);
}
```

While a node's own callback runs, it is **checked out** of the tree:
`tree.get(current)` is `None` (ADR-022).

You interact with the tree through `NodeContext` inside lifecycle
callbacks — you don't hold a `&mut NodeTree` yourself.

### 2.4 NodeContext

`NodeContext` is the **single injection point** for everything a node
needs beyond itself. It is passed as `&mut NodeContext` to `ready`,
`process`, and `physics_process`.

```rust
pub struct NodeContext<'a> {
    tree: &'a mut NodeTree,
    rendering: &'a mut dyn RenderingServer,
    audio: &'a mut dyn AudioServer,
    physics: &'a mut dyn PhysicsServer,
    input: &'a Input,
    current: NodeId,
    delta: f32,         // dt (process), fixed step (physics_process), 0.0 (ready)
}

impl<'a> NodeContext<'a> {
    pub fn current_id(&self) -> NodeId;
    pub fn parent_id(&self) -> Option<NodeId>;
    pub fn delta(&self) -> f32;

    pub fn tree(&mut self) -> &mut NodeTree;
    pub fn find(&self, path: &str) -> Option<NodeId>;
    pub fn render(&mut self) -> &mut dyn RenderingServer;
    pub fn audio(&mut self) -> &mut dyn AudioServer;
    pub fn physics(&mut self) -> &mut dyn PhysicsServer;
    pub fn input(&self) -> &Input;
    pub fn quit(&mut self);

    pub fn spawn(&mut self, node: impl Node) -> NodeId;
    pub fn spawn_child(&mut self, parent: NodeId, node: impl Node) -> NodeId;
    pub fn despawn(&mut self, id: NodeId);

    pub fn emit<T: Signal>(&mut self, payload: T);
    pub fn emit_deferred<T: Signal>(&mut self, payload: T);
    pub fn connect<T: Signal>(&mut self, target: NodeId, handler: impl Fn(&mut NodeContext, &T) + Send + Sync + 'static);
    pub fn connect_scoped<T: Signal>(&mut self, target: NodeId, handler: impl Fn(&mut NodeContext, &T) + Send + Sync + 'static) -> ScopedConnection;
    pub fn disconnect<T: Signal>(&mut self, target: NodeId);
}
```

The context does **not** carry an `AssetServer`; load assets in game setup
(`AssetServer` is a separate crate-level object). The four server fields are
the `Servers` bundle (`rendering`, `audio`, `physics`, `input`).

`NodeContext` is the analogue of Godot's `get_tree()` / `get_node()` —
but instead of returning global state, it is an explicit borrow you
receive in each callback. This is what makes the borrow checker cooperate
instead of fight.

### 2.5 DrawContext

`DrawContext` is passed as `&mut DrawContext` to `draw()`. It provides
access to the rendering server but *not* to the mutable tree — drawing is
read-only by construction.

```rust
pub struct DrawContext<'a> {
    rendering: &'a mut dyn RenderingServer,
}

impl<'a> DrawContext<'a> {
    pub fn new(rendering: &'a mut dyn RenderingServer) -> Self;
    pub fn render(&mut self) -> &mut dyn RenderingServer;
    pub fn viewport(&self) -> Rect;        // forwarded from the renderer
    pub fn camera_offset(&self) -> Vec2;   // forwarded from the renderer
    pub fn current_z(&self) -> f32;
    pub fn world_to_screen(&self, world: Vec2) -> Vec2;
    pub fn screen_to_world(&self, screen: Vec2) -> Vec2;
}
```

### 2.6 PackedScene

A **PackedScene** is a serialized node subtree that can be instantiated
any number of times. This is Ember's analogue of a Godot `.tscn` and its
dual role as prefab and level.

```rust
pub struct PackedScene { /* parsed template, no live NodeId */ }

impl PackedScene {
    pub fn from_ron(text: &str) -> Result<Self, SceneError>;
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, SceneError>;
    pub fn to_ron(&self) -> String;
    // Builds nodes immediately; root is returned detached (ADR-025).
    pub fn instantiate(&self, classes: &ClassDB, tree: &mut NodeTree) -> Result<NodeId, SceneError>;
    // Also resolves `instance:` entries through the AssetServer (ADR-027).
    pub fn instantiate_with(&self, classes: &ClassDB, assets: &AssetServer, tree: &mut NodeTree) -> Result<NodeId, SceneError>;
    pub fn instantiate_into(&self, classes: &ClassDB, tree: &mut NodeTree, parent: NodeId) -> Result<NodeId, SceneError>;
}
```

Node classes are built through `ClassDB` (`ClassDbNodes::register_node` /
`register_node_with`, ADR-024); a `SceneAssets::scene(path)` extension on
`AssetServer` loads a `Handle<PackedScene>`.

The instantiated subtree is a **deep copy** — modifying the instance does
not modify the packed scene. Node names are preserved; node ids are new.

### 2.7 Object

`Object` is a smaller trait that `Node` extends. It exists so that
non-node objects (assets, resources, future utilities) can share the
dynamic-typing facility used by the scene format and the `Variant`
serialization layer.

```rust
pub trait Object: Send + Sync + 'static {
    fn class_name() -> &'static str where Self: Sized;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

You rarely interact with `Object` directly. `Node: Object` means any
`Node` is also a valid `Object`.

## 3. Lifecycle

A node's life has six states, visited in this order:

```mermaid
stateDiagram-v2
    [*] --> Constructed: Node::new()
    Constructed --> Attached: tree.insert() + tree.attach()
    Attached --> Ready: next flush, after all children Ready
    Ready --> Running: every frame
    Running --> Running: process / physics_process / draw
    Running --> Detached: tree.detach() or tree.remove()
    Detached --> Finalized: tree.remove()
    Finalized --> [*]

    note right of Attached
        Child ready() fires before parent ready()
        (bottom-up, like Godot)
    end note

    note right of Running
        draw() is read-only.
        process() and physics_process()
        may mutate tree via ctx.
    end note
```

### 3.1 The six transitions in detail

| Transition | What runs | What the node has access to |
|---|---|---|
| `Constructed` → `Attached` | nothing (plain construction) | nothing — the node is unattached, cannot interact with the tree |
| `Attached` → `Ready` | `Node::ready(ctx)` (at the next `flush`: start/end of a pass, or `SceneLoop::init`) | full `NodeContext`: tree, servers, input |
| `Ready` → `Running` | nothing (this is just the first frame) | — |
| `Running` (per frame) | `physics_process(fixed_dt)` N times, then `process(dt)` once, then `draw(ctx)` | varies |
| `Running` → `Detached` | nothing | — |
| `Detached` → `Finalized` | `Node::finalize()` on `tree.remove` / `clear` / despawn, bottom-up | the node only (no context) |

### 3.2 Ready order

`ready` fires **bottom-up**: children are readied before their parents,
so a parent can assume its children are fully initialised when its own
`ready` runs. This is the opposite of `process` order, which is
top-down.

This matches Godot's `_ready()` semantics and is the most requested
property in scene-tree engines — an entity can safely look up its
sprite child in `ready`.

### 3.3 Process order

Per frame, the tree is walked **depth-first, top-down, in child insertion
order**. Two sibling nodes always process in the order they were attached.

Ordering is **deterministic**: same tree, same frame, same order. This is
what allows reproducible physics and reproducible replays, and it is
what makes `ember-wars`'s combat resolution testable.

### 3.4 Draw order

Draw runs after all `process` calls have finished. Two rules:

1. **Draw is read-only.** The tree is not passed to `draw`, only the
   `DrawContext`. You cannot mutate from `draw`, which prevents
   surprising interactions.
2. **Draw order is tree order, then z.** Within a node, children draw
   after their parent (so parents render "behind"). Two children at the
   same z draw in insertion order. Z is an `f32` from `Node::z()` (default
   `0.0`); the tree calls `RenderingServer::set_z` with it before each
   node's `draw`. A node with `visible() == false` skips its whole subtree.

## 4. Node trait — full signature

The authoritative signature is the one in §2.1 (including `z`, `visible`
and `set_property`). The spec's earlier `signals()` / `signals_mut()`
methods do **not** exist: signals live in one tree-level `SignalTable`
(ADR-023, see [11-signals.md](11-signals.md)).

### 4.1 What the derive macro gives you

```rust
#[derive(Node)]
#[class_name = "Player"]
pub struct Player {
    #[export] pub speed: f32,
    sprite: NodeId,
    health: f32,
}
```

The macro currently expands to:

- `impl Object for Player` with `class_name()` returning the
  `#[class_name = "..."]` string (required)
- `impl Node for Player` with `name`/`set_name` on a **required**
  `name: String` field (compile error if missing)

It does **not** yet generate `new()`, `#[export]` handling, `set_property`
or serialization — *not yet implemented*. `#[export]` attributes in the
snippets below are aspirational. The macro does not verify that
`class_name` matches the struct name.

You can also implement `Node` by hand — the macro is convenience, not
requirement.

## 5. NodeId semantics

### 5.1 What a `NodeId` guarantees

- **Copy.** Pass it around freely.
- **Hashable.** Use it as a `HashMap` key.
- **Stale-safe.** If the node was removed, `tree.get(id)` returns `None`.
- **Frame-stable.** If the node is still alive, the same `NodeId` refers
  to it across frames.

### 5.2 What a `NodeId` does not guarantee

- **Uniqueness across runs.** A `NodeId` from a previous game run is
  meaningless in the current run. Do not persist `NodeId`s to disk.
- **Ability to construct.** You get ids from the tree, never from
  literals (`NodeId::default()` is a null id that is never issued).

### 5.3 Why not `Rc<RefCell<Node>>`?

Because the two patterns that dominate game code are:

1. **"Store a reference to my target."** A unit that chases an enemy
   stores `target: Option<NodeId>`. With `Rc<RefCell<>>`, that requires
   weak refs and runtime borrow checking; with `NodeId`, it's a `Copy`
   field.
2. **"Mutate a sibling while iterating children."** With `NodeId`, you
   call `ctx.tree().get_mut(target)` inside `process` and the borrow is
   scoped. With `Rc<RefCell<>>`, you get runtime panics at exactly the
   wrong moment.

`NodeId` makes both patterns compile-time-safe.

## 6. NodeTree API

Beyond the sketch in §2.3, a few more operations matter:

```rust
impl NodeTree {
    /// Number of nodes in the tree (including queued ones' slots).
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn contains(&self, id: NodeId) -> bool;

    /// Iterates live nodes in insertion order (excludes not-yet-flushed spawns).
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &dyn Node)>;

    /// Iterates a node's descendants depth-first, top-down (excludes `id`).
    pub fn descendants(&self, id: NodeId) -> impl Iterator<Item = NodeId>;

    /// Resolves a path relative to a starting node.
    pub fn find(&self, from: NodeId, path: &str) -> Option<NodeId>;

    /// Detaches `child` and appends it under `new_parent`; the tree is
    /// untouched on error. (No transforms exist, so nothing is "preserved".)
    pub fn reparent(&mut self, child: NodeId, new_parent: NodeId) -> Result<(), SceneError>;

    pub fn request_quit(&mut self);
    pub fn quit_requested(&self) -> bool;
    pub fn frame(&self) -> u64;
}
```

*`find_by_class` is not yet implemented.*

`find` accepts a subset of Godot's path syntax: `""`/`"."` (self), `"Child"`,
`"Child/SubChild"`, `".."` (parent), `"../Sibling"`. It does not accept `$`, `%`, or node
groups in v0.1 — those are v0.2 candidates.

## 7. PackedScene and instancing

### 7.1 Instancing rules

- **Deep copy.** Instances never share mutable state.
- **Names preserved.** A child called `Sprite` in the packed scene is
  called `Sprite` in every instance.
- **Ids fresh.** Every node in every instance has a unique `NodeId`.
- **Signals re-wired.** *Not yet implemented:* connections in a scene file
  are validated but not wired (ADR-025).
- **Properties applied.** Each property goes through `Node::set_property`;
  unknown or mistyped keys warn and are skipped but stay in the template.
  The `#[export]` derive support is *not yet implemented*.
- **Private fields default-constructed.** Nodes are built by the
  constructor registered in `ClassDB` (`T::default()` or a custom `fn`).

### 7.2 Nested scenes

A packed scene may itself contain a `PackedScene` instance as a child
(Godot's "instance a scene as a child" pattern):

```ron
(
    nodes: [
        (name: "Root", class: "Level"),
        (name: "Player", instance: "res://player.escene", parent: "."),
    ],
)
```

At load time (`instantiate_with`), the nested scene is recursively
instantiated, with the entry's `name`, `properties`, `visible` and `z` as
overrides (ADR-027). Cycles are rejected with `SceneError::InstanceCycle`;
plain `instantiate` fails on `instance:` with `SceneError::Asset`.

### 7.3 Ownership of loaded scenes

A `PackedScene` returned by `AssetServer::load` (or `assets.scene(path)`) is
**reference-counted**. The `Handle<PackedScene>` keeps it alive. Drop all
handles and the scene is unloaded on the next `AssetServer::poll()`.

## 8. NodeContext in depth

### 8.1 What NodeContext owns

`NodeContext` is a **reborrow** of state that belongs to the `MainLoop`.
It does not own anything; it borrows from the loop's fields. This means:

- You cannot store a `NodeContext` in a field. It is `'a`-lifetimed
  exactly to the callback.
- Every mutation goes through `&mut NodeContext`. There is no
  "get and release" pattern.
- Passing `ctx` down to helper functions is normal; storing it is not.

### 8.2 NodeContext and the tree

Inside `process`, calling `ctx.spawn(...)` or `ctx.despawn(...)` is legal.
The tree handles this by **deferring structural changes**: the new node's
id is valid immediately, but the node joins the tree (and gets `ready`) at
the next `flush`, which runs at the start and end of every `process` and
`physics_process` pass. Direct `ctx.tree().attach/remove` calls are
immediate; passes walk an id snapshot and skip ids that are no longer live
(ADR-022).

This is what prevents the classic "iterator invalidation" bug: you can
spawn a projectile mid-frame without breaking the loop that's iterating
units.

```rust
pub fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
    if self.cooldown.is_ready() {
        let me = ctx.current_id();
        ctx.spawn_child(me, Projectile::new(self.pos)); // attached at flush
        self.cooldown.reset();
    }
}
```

## 9. DrawContext in depth

### 9.1 Coordinates

`DrawContext` uses **world coordinates** by default. The
`world_to_screen` / `screen_to_world` helpers convert explicitly. The
default camera is `(0, 0)` — origin at top-left, +Y down, matching
macroquad's convention.

### 9.2 Z ordering

Nodes render in tree order. Each node's draw layer is `Node::z()` (default
`0.0`), which the tree forwards to `RenderingServer::set_z` before `draw`.
Z is **absolute** per node — children are not offset by their parent's z
(`#[export(z = ...)]` does not exist).

In v0.1, z is `f32`. A `u16`-based z (à la Godot 4) is a v0.2 candidate
if profiling shows float z is a hotspot.

## 10. Common patterns

### 10.1 A sprite node

```rust
#[derive(Node)]
#[class_name = "Sprite"]
pub struct Sprite {
    #[export] texture: Handle<Texture>,
    #[export] size: Vec2,
}

impl Node for Sprite {
    fn draw(&self, ctx: &mut DrawContext) {
        if let Some(tex) = self.texture.get() {
            let rect = Rect::new(0.0, 0.0, self.size.x, self.size.y);
            ctx.render().draw_sprite(tex, rect, None, WHITE);
        }
    }
}
```

### 10.2 A player with children

```rust
#[derive(Node)]
#[class_name = "Player"]
pub struct Player {
    #[export] speed: f32,
    #[export] hp: f32,
    sprite: NodeId,   // set in ready
    hitbox: NodeId,
}

impl Node for Player {
    fn ready(&mut self, ctx: &mut NodeContext) {
        // children exist by the time ready fires (bottom-up ready order)
        self.sprite = ctx.find("Sprite").unwrap_or_default();
        self.hitbox = ctx.find("Hitbox").unwrap_or_default();
    }

    fn physics_process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input();
        let mut mv = Vec2::ZERO;
        if input.is_key_down(Key::W) { mv.y -= 1.0; }
        if input.is_key_down(Key::S) { mv.y += 1.0; }
        if input.is_key_down(Key::A) { mv.x -= 1.0; }
        if input.is_key_down(Key::D) { mv.x += 1.0; }
        // The running node is checked out of the tree, so mutate `self`
        // directly (tree.get(current) is None here, ADR-022).
        self.pos += mv.normalize_or_zero() * self.speed * dt;
    }
}
```

### 10.3 A level scene

```rust
#[derive(Node)]
#[class_name = "Level"]
pub struct Level {
    #[export] name: String,
    #[export] width: f32,
}

// In main:
let level = PackedScene::from_file("levels/forest.escene")?;
let root = level.instantiate(&classes, &mut tree)?; // classes: ClassDB with register_node::<Level>()
tree.set_root(root);
```

## 11. Serialization

Every node that participates in `.escene` files is constructible via a
registered `ClassDB` constructor and receives properties via
`Node::set_property`. Writing nodes back (`Node::serialize` / `SceneWriter`)
and generating both from `#[export]` is *not yet implemented*; the intended
shape is:

```rust
#[derive(Node)]
#[class_name = "Sprite"]
pub struct Sprite {
    #[export] texture: Handle<Texture>,
    #[export] size: Vec2,
    // no #[export]: not serialized, reinitialised in new()
    runtime_cache: Option<TextureCache>,
}
```

Fields *without* `#[export]` are not serialized. When the scene is
loaded, they come from the registered constructor — so it should always
produce a fully valid node.

See [30-escene-format.md](30-escene-format.md) for the exact file format.

## 12. Design notes

### 12.1 Why bottom-up ready but top-down process

`ready` is the moment a node learns about its children — so children
must be ready first. `process` is the moment a parent updates and
broadcasts to children — so parents must process first. This is the
Godot convention, and it works.

### 12.2 Why deferred structural changes

Immediate insertion during iteration breaks every loop in the engine.
Deferring costs one extra `Vec` per pass and saves all the pain. The
deferred queue is flushed at the start and end of `process` and
`physics_process` (draw has no structural changes because it's `&self`).

### 12.3 Why Any-based downcast

A `NodeId` gives you `&dyn Node`. To call `Player::take_damage`, you
downcast: `tree.get_mut(id)?.as_any_mut().downcast_mut::<Player>()`. This
is one pattern; the alternative is a visitor trait for every node type,
which is worse. Rust's `downcast_mut` is monomorphised and cheap.

Downcasting is checked at runtime (`downcast_mut` returns `None` on a type
mismatch). The derive requires a `#[class_name]` but does not check it
against the struct name, and `find_by_class` does not exist yet.

### 12.4 Why no `parent()` back-reference on the node

If nodes held `Option<NodeId>` for their parent, a child would need to be
mutated when reparented. Storing parent on the tree side means reparenting
touches exactly one struct. It's also what makes `descendants` cheap.

## 13. Open questions

- **Q1:** Should `NodeId` be exposed as `u32` + `u32` (slot, generation)
  or as an opaque struct? *Resolved:* opaque `slotmap` key.
- **Q2:** Should `ready` receive `&mut NodeContext` or a read-only
  variant? *Resolved:* mutable — nodes commonly register signals in
  ready.
- **Q3:** Do we need a `Node::enter_tree` / `Node::exit_tree` pair
  (Godot has both)? *Provisional:* not in v0.1; `ready` and `finalize`
  cover the cases we've needed.
- **Q4:** Should `find` accept Godot's full path syntax (`%`, `$`,
  `..`)? *Provisional:* subset only in v0.1 (`Child`, `..`, `../Sibling`).
- **Q5:** Should `NodeTree::iter` iterate in insertion order or in
  tree-traversal order? *Provisional:* insertion order for `iter`, tree
  order for `descendants`. Games that need tree order use `descendants`.

## 14. Cross-references

- Typed signals → [11-signals.md](11-signals.md)
- Full trait signatures → [21-ember-scene.md](21-ember-scene.md)
- Serialization format → [30-escene-format.md](30-escene-format.md)
- Servers injected via `NodeContext` → [12-servers.md](12-servers.md)
- The main loop that drives this → [14-main-loop.md](14-main-loop.md)