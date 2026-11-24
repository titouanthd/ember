---
title: "26 — `ember_macros`"
aliases: ["26 — `ember_macros`"]
tags: ["ember/spec", "ember/wave-3"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/26-ember-macros.md"
imported: "2026-11-22"
wave: "3 — Crate Specifications"
---

# 26 — `ember_macros`

> **Status:** 🟡 partial — reviewed 2026-11-19; `#[derive(Node)]` (minimal, ADR-019) and no-op `#[signal]` exist; `#[export]` generates `property_list` / `get_property` / `set_property` on `NodeName` (ADR-037); `#[export(default)]`, generated `new()` and registration are not implemented
> **Audience:** implementers
> **Prerequisites:** [[21-ember-scene]], [[11-signals]]

## 1. Purpose

Specify the procedural macros that reduce boilerplate: `#[derive(Node)]`,
`#[signal]`, `#[export]`, and their associated attributes. This is the
crate that turns a plain Rust struct into a scene node.

> **Implementation status (ADR-019):** only a reduced subset exists.
> `#[derive(Node)]` requires `#[class_name = "..."]` and a user-declared
> `name: String` field, and emits just `impl Object` (`class_name`,
> `as_any`, `as_any_mut`) and `impl NodeName` (`name`, `set_name`) — ADR-030.
> The user writes `impl Node for T { … }` (empty for a node without behaviour). It does
> **not** add fields, generate `new()`, `serialize()` or registration
> functions. `#[signal]` is a no-op marker; `#[export]` is accepted by the
> derive but ignored. Everything below describing more than that is
> *Not yet implemented* (planned). Tests: `crates/ember-scene/tests/derive_works.rs`
> (happy path only; `trybuild` failure tests are deferred).

## 2. Module layout

Actual layout (`crates/ember-macros`):

```
ember-macros/
├── Cargo.toml
└── src/
    ├── lib.rs      entry points: derive(Node), #[signal]
    └── node.rs     derive(Node) implementation
```

The split below (`derive_node.rs`, `signal.rs`, `export.rs`, …) is
*not yet implemented* — planned if the crate grows.

## 3. `#[derive(Node)]`

### 3.1 The attribute set

Implemented form (ADR-019): `#[class_name]` is **required** (compile
error if missing), and the struct must declare `name: String` itself.

```rust
#[derive(Node)]
#[class_name = "Player"]
pub struct Player {
    pub name: String,
    pub speed: f32,
}
```

Planned form (*not yet implemented*):

```rust
#[derive(Node)]
#[class_name = "Player"]     // planned: optional; defaults to struct name
pub struct Player {
    #[export] pub speed: f32,
    #[export] pub hp: f32,
    sprite: NodeId,          // not exported
}
```

### 3.2 What it generates

**Today** only `impl Object` (`class_name`, `as_any`, `as_any_mut`) and
`impl NodeName` (`name`, `set_name`, against the user's `name` field) are
emitted (ADR-030); `impl Node` is always the user's. The rest of this section is the *planned* expansion (not yet
implemented; `SignalTable` was dropped from nodes by ADR-023):

```rust
// === Object impl ===
impl ember_core::Object for Player {
    fn class_name() -> &'static str { "Player" }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

// === Node impl ===
impl ember_scene::Node for Player {
    fn name(&self) -> &str { &self.__node_name }
    fn set_name(&mut self, name: String) { self.__node_name = name; }
    fn signals(&self) -> &SignalTable { &self.__signals }
    fn signals_mut(&mut self) -> &mut SignalTable { &mut self.__signals }
    fn serialize(&self, w: &mut SceneWriter) {
        // for each #[export] field:
        w.set_property("speed", Variant::Float(self.speed as f64));
        w.set_property("hp",    Variant::Float(self.hp as f64));
    }
    // ready, process, physics_process, draw, finalize use defaults
}

// === Inherent impl ===
impl Player {
    pub fn new() -> Self {
        Self {
            __node_name: "Player".to_string(),
            __signals: SignalTable::new(),
            speed: 0.0,
            hp: 0.0,
            sprite: NodeId::NULL,
        }
    }
}

// === Registration ===
pub fn register_player(db: &mut ClassDB) {
    db.register::<Player>();
}
```

### 3.3 The magic fields

*Not yet implemented, and not implementable as a derive* (ADR-019: derive
macros cannot add fields). Planned design:

The macro adds two private fields to the struct:

- `__node_name: String`
- `__signals: SignalTable`

These are why you cannot use a struct literal `Player { speed: 5.0, ... }`
anymore — you must go through `Player::new()` or `Player::with_*()`.

**Consequence:** the derive macro's expansion requires the struct to be
constructed via the generated `new()` — a plain struct literal will fail
to compile because the magic fields are private to the module and must
be initialised.

### 3.4 Field attributes

*Not yet implemented.* `#[export]` is registered as an inert attribute of
the derive, so it compiles but has no effect; `default = ` / `skip` forms
are not parsed.

| Attribute | Effect |
|---|---|
| `#[export]` | Serialize this field to `.escene` files |
| `#[export(default = <expr>)]` | Override the default in `new()` |
| `#[export(skip)]` | Skip this field even if it's public (equivalent to no `#[export]`) |

Fields without `#[export]`:
- Are initialised to `Default::default()` in `new()`.
- Are not serialized.
- Are not shown in the editor inspector.

### 3.5 Default values

Without an explicit `#[export(default = ...)]`, exported fields use
`Default::default()`:

```rust
#[export] pub speed: f32,          // 0.0
#[export] pub name: String,        // ""
#[export] pub handle: Option<Handle<Texture>>,  // None
```

For non-default values, override:

```rust
#[export(default = 100.0)] pub speed: f32,
#[export(default = "Player".to_string())] pub name: String,
```

### 3.6 Supported field types

*Not yet implemented* (no `#[export]` processing exists).

`#[export]` supports:

- Primitives: `bool`, `i8..i64`, `u8..u64`, `f32`, `f64`, `String`
- `Vec2`, `Rect`, `Color` (from `ember_core`)
- `Option<T>` where `T` is an exported type
- `Vec<T>` where `T` is an exported type
- `Handle<T>` (serialized as `Uid`)
- `NodeId` (serialized as a name-based path within the scene)

Types not in this list require a manual `Node::serialize` implementation.

### 3.7 Full example

```rust
#[derive(Node)]
#[class_name = "Catapult"]
pub struct Catapult {
    #[export] pub pos: Vec2,
    #[export(default = 300.0)] pub hp: f32,
    #[export(default = 0.9)] pub cooldown_secs: f32,
    #[export] pub shot_kind: ShotKind,

    // Runtime-only state
    cooldown: Cooldown,
    aim_target: Vec2,
}

impl Catapult {
    pub fn with_facing(pos: Vec2, facing: f32) -> Self {
        let mut c = Self::new();
        c.pos = pos;
        // ... initialise runtime-only fields
        c
    }
}
```

## 4. `#[signal]`

### 4.1 The attribute

```rust
#[signal]
pub struct Damaged {
    pub amount: f32,
    pub source: NodeId,
}
```

### 4.2 What it generates

**Today it generates nothing:** `#[signal]` returns the item unchanged.
`Signal` is satisfied by a blanket impl for `Send + Sync + 'static`
types. The expansion below is *not yet implemented* (planned):

```rust
pub struct Damaged {
    pub amount: f32,
    pub source: NodeId,
}

// Marker (already satisfied by the blanket impl, but generated for clarity)
impl ember_scene::Signal for Damaged {}

// Debug + Display
impl std::fmt::Debug for Damaged { /* derived */ }

// Optional: derive Clone if all fields are Clone
impl Clone for Damaged where /* all fields: Clone */ { /* ... */ }
```

### 4.3 What it does NOT generate

- No trait impls beyond `Signal`.
- No global registry (signals are identified by `TypeId` at runtime).
- No signal name (they're anonymous at runtime).

The `#[signal]` attribute exists mainly as:
1. **Documentation** — grep `#[signal]` to find every signal in the codebase.
2. **Future-proofing** — a v0.2 linter can validate that every
   `#[signal]` type is used in at least one `emit` or `connect`.

For v0.1, `#[signal]` is functionally a no-op marker on top of a plain
struct. You could skip it and the code would work identically (but
unidiomatically).

### 4.4 Signal payload requirements

The macro does not enforce, but the `Signal` trait requires:

```rust
pub trait Signal: Send + Sync + 'static {}
```

Compile errors if you try to use a `!Send` signal (e.g. one containing
an `Rc`). This is intentional — see [[11-signals#3.2 Payload requirements|11-signals.md §3.2]].

## 5. `#[export]`

*Not yet implemented* — this whole section is planned. `ember-scene` nodes
receive properties through hand-written `Node::set_property` (ADR-025).

### 5.1 Standalone usage

`#[export]` is a field attribute, but it can also be used on structs
that don't derive `Node`:

```rust
pub struct Config {
    #[export] pub window_width: u32,
    #[export] pub window_height: u32,
}
```

In this case, `#[export]` has no effect (nothing processes it) — it's a
documentation marker. The attribute only does work when used inside
`#[derive(Node)]`.

### 5.2 Serialization format

Exported fields serialize to `Variant`:

| Rust type | Variant |
|---|---|
| `bool` | `Bool` |
| `i32`, `i64` | `Int` |
| `f32`, `f64` | `Float` |
| `String` | `String` |
| `Vec2` | `Vec2` |
| `Rect` | `Map { x, y, w, h }` |
| `Color` | `Map { r, g, b, a }` |
| `Option<T>` | `Nil` or the inner value |
| `Vec<T>` | `Array` |
| `Handle<T>` | `ObjectRef("uid://...")` |
| `NodeId` | `String` (path within the scene tree) |

### 5.3 NodeId serialization

Serializing a `NodeId` requires the `.escene` writer to have a
root-relative path for the node. The macro generates:

```rust
w.set_property("target", Variant::String(self.target.path_in_scene()));
```

`path_in_scene` is a helper on `NodeContext` that returns the path
relative to the scene root (e.g. `"../../Enemy"`). This is the only
place `NodeId` round-trips through disk.

## 6. `#[class_name]`

### 6.1 Usage

`#[class_name = "X"]` (also accepts `#[class_name("X")]`) is required by
the derive today; there is no default and no PascalCase validation.

```rust
#[derive(Node)]
#[class_name = "MyCustomName"]
pub struct InternalStructName { /* ... */ }
```

Overrides the default class name (which is the struct name). The class
name is what `ClassDB` registers and what `.escene` files reference.

### 6.2 Naming conventions

- **PascalCase.** `"Player"`, `"Catapult"`, `"TextInput"`.
- **Unique across all games.** `"Unit"` from ember-wars is not the same
  as `"Unit"` from jade-garden, but the class name would collide if
  both games used the same class name. Convention: prefix with the game
  name if the node is game-specific (`"EmberWarsUnit"`).

## 7. Macro hygiene

### 7.1 What the macros touch

- The struct being decorated.
- New fields: `__node_name`, `__signals`.
- New methods: `new()`, `register_<snake_case>()`.
- Trait impls: `Object`, `Node`, `Signal`.

### 7.2 What they do not touch

- Other structs in the same file.
- Existing methods on the struct.
- Imports (they use fully-qualified paths).

### 7.3 `#[derive(Node)]` on generic types

**Not supported in v0.1** (the derive forwards generics, but this is
untested and `class_name` is a single static name). A node with a lifetime parameter or type
parameter cannot derive `Node` because `Node::class_name` requires a
unique `'static` name and `ClassDB::register` requires `T: 'static`.

Workaround: define a non-generic wrapper.

### 7.4 `#[derive(Node)]` on enums

**Not supported.** Nodes are structs. An enum that needs node behavior
uses composition (a `Node` struct containing the enum as a field).

## 8. Module-level registration

### 8.1 Auto-registration

The derive macro does **not** generate a `register_<snake_name>` function
(planned, not implemented) and does not auto-register with `ClassDB` —
that requires a call site. `ClassDB::register::<T>()` requires
`T: Object + Default`; otherwise use `register_with(ctor)`.

The plugin's `build` method registers everything:

```rust
impl Plugin for EmberWarsPlugin {
    fn build(class_db: &mut ClassDB, assets: &mut AssetServer) -> Self {
        class_db.register::<Player>();
        class_db.register::<Catapult>();
        class_db.register::<Projectile>();
        // ...
        Self::default()
    }
}
```

### 8.2 Why explicit registration

Because `ClassDB` is not a global — it's passed by `&mut`. Auto-register
via a `linkme`-style crate would reintroduce global mutable state
([[02-principles#I3 — No global mutable state|02-principles I3]]).

The cost is one line per node type in the plugin. For 30 node types,
that's 30 lines in one function, in one file. Acceptable.

## 9. Error messages

Implemented compile errors (all via `syn::Error`):

| Error | Cause |
|---|---|
| `#[derive(Node)] requires the attribute #[class_name = "..."]` | Missing attribute |
| `#[class_name] expects a string literal` / `expected #[class_name = "..."]` | Malformed attribute |
| `#[derive(Node)] requires a field `name: String`` | Missing field |
| `#[derive(Node)] can only be applied to structs` | Enum/union |

Planned, *not yet implemented*:

| Error | Cause |
|---|---|
| Class name must be PascalCase | Validation |
| `#[export]` field type `{T}` is not supported | Unknown type |
| Duplicate `#[class_name]` | Two attributes on same struct |
| Signal struct must be `pub` | Visibility check |

Good error messages are part of the spec — each macro should use
`syn::Error::new_spanned` to point at the offending token.

## 10. Examples

### 10.1 A complete node

*Planned form* — it relies on `#[export(default = …)]`, a generated
`new()` and the unimplemented signal-table API; today the struct needs an
explicit `name: String` and hand-written construction and `set_property`.

```rust
use ember::prelude::*;

#[signal]
pub struct Damaged {
    pub amount: f32,
}

#[derive(Node)]
#[class_name = "Player"]
pub struct Player {
    #[export(default = 100.0)] pub max_hp: f32,
    #[export(default = 5.0)] pub speed: f32,

    hp: f32,
    position: Vec2,
}

impl Node for Player {
    fn ready(&mut self, ctx: &mut NodeContext) {
        self.hp = self.max_hp;
        let id = ctx.current_id();
        ctx.connect::<Damaged>(id, move |ctx, ev| {
            if let Some(p) = ctx.tree().get_mut(id)
                .and_then(|n| n.as_any_mut().downcast_mut::<Player>())
            {
                p.hp -= ev.amount;
            }
        });
    }

    fn physics_process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input();
        let mut mv = Vec2::ZERO;
        if input.action("left")  { mv.x -= 1.0; }
        if input.action("right") { mv.x += 1.0; }
        self.position += mv.normalize_or_zero() * self.speed * dt;
    }
}
```

### 10.2 The .escene produced

If this node is serialized and instantiated, the `.escene` looks like:

```ron
(
    format: 1,
    nodes: [
        (name: "Player", class: "Player",
         properties: { "max_hp": Float(100.0), "speed": Float(5.0) }),
    ],
)
```

### 10.3 Manual impl (no derive)

For a struct that cannot use the derive macro (this is what the derive
actually expands to today, minus the planned hidden fields and
`SignalTable`; `Object::class_name` has a `where Self: Sized` bound):

```rust
pub struct ManualNode {
    __node_name: String,
    __signals: SignalTable,
    pub value: i32,
}

impl Object for ManualNode {
    fn class_name() -> &'static str { "ManualNode" }
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl Node for ManualNode {
    fn name(&self) -> &str { &self.__node_name }
    fn set_name(&mut self, name: String) { self.__node_name = name; }
    fn signals(&self) -> &SignalTable { &self.__signals }
    fn signals_mut(&mut self) -> &mut SignalTable { &mut self.__signals }
}
```

Manual impl is verbose but complete. It's what the derive macro
expands to.

## 11. Design notes

### 11.1 Why hidden `__node_name` and `__signals` fields

Because they are implementation details. Exposing them as public fields
would let game code mutate them in ways that break invariants (rename a
node without updating the tree's name index, replace the signal table).

The `__` prefix is a convention indicating "do not touch from outside
the derive".

### 11.2 Why explicit plugin registration, not `linkme`

`linkme` (and `inventory`) work by populating a global via linker
sections. This is fast and convenient but breaks the "no global mutable
state" invariant and doesn't work on WASM without special linker flags.

The explicit registration is 30 lines per game, runs at startup, and
keeps the architecture honest.

### 11.3 Why `#[class_name]` is separate from `#[derive(Node)]`

Because `#[derive(Node)]` should work without `#[class_name]` — it
defaults to the struct name. `#[class_name]` is for the cases where the
struct name (which might be generic or namespaced) doesn't make a good
class name.

### 11.4 Why no attribute for signals

Because `emit::<T>(T { ... })` doesn't need a name. The type is the
name. A `#[signal]` attribute that generated a name would be dead weight
— it's a documentation marker, not a functional attribute.

## 12. Open questions

- **Q1:** Should `#[derive(Node)]` also implement `Default`? *Provisional:*
  no — `new()` is the constructor, and `Default` might confuse users who
  expect `Node::new()` semantics. Games that need `Default` implement it
  manually as `fn default() -> Self { Self::new() }`.
- **Q2:** Should exported fields with `#[export(skip)]` be visible in
  the editor? *Provisional:* no, same as no `#[export]`. `skip` exists
  only as a documentation marker for public fields that shouldn't be
  serialized.
- **Q3:** Should the macro support `#[derive(Node)]` on structs with
  private fields? *Provisional:* yes — visibility is orthogonal to
  derivation. `new()` initialises all fields via `Default` or explicit
  default.
- **Q4:** Should there be a `#[node(no_register)]` attribute for nodes
  that shouldn't be in `ClassDB`? *Provisional:* no. Registration is
  explicit (plugin's `build`), so a node not registered simply can't be
  instantiated from `.escene`. No macro flag needed.

## 13. Cross-references

- What these macros generate → [[21-ember-scene#4. `Node` trait|21-ember-scene.md §4]]
- Signal type requirements → [[11-signals#3. Signal declaration|11-signals.md §3]]
- `ClassDB` registration → [[20-ember-core#9. `Object` and `ClassDB`|20-ember-core.md §9]]
- `.escene` format → [[30-escene-format]]
