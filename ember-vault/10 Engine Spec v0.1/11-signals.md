---
title: "11 — Signals"
aliases: ["11 — Signals"]
tags: ["ember/spec", "ember/wave-2"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/11-signals.md"
imported: "2026-11-22"
wave: "2 — Concepts"
---

# 11 — Signals

> **Status:** 🟡 partial — reviewed 2026-11-19; typed signals, connect/emit/deferred/scoped implemented (ADR-023); `#[signal]` is a no-op marker not in the prelude, scene-declared connections are not wired
> **Audience:** game authors, implementers
> **Prerequisites:** [[10-nodes-and-scenes]]

## 1. Purpose

Explain how nodes talk to each other **without holding references to
each other**. Signals are Ember's observer pattern — the equivalent of
Godot's `signal` / `connect` / `emit`, but with a crucial difference:
**signals are typed, not string-keyed.**

If you take one thing from this file, it is: **a signal is a type, and
its payload is the type**. You cannot emit a `BodyEntered` event into a
`BodyExited` receiver — the compiler prevents it.

## 2. Why typed signals

Godot's signals are declared by name and connected by string:

```gdscript
signal body_entered(body)
connect("body_entered", self, "_on_body_entered")
```

This works, but has three failure modes:

1. **Typo.** `connect("body_entred", ...)` compiles, fails silently at
   runtime.
2. **Wrong arity.** Receiver expects one arg, sender passes two — runtime
   error.
3. **Refactor drift.** Rename the signal, nothing catches the stale
   `connect` call.

Rust can eliminate all three with zero ergonomic cost:

```rust
#[signal]
pub struct BodyEntered { pub body: NodeId }

ctx.connect::<BodyEntered>(target, |ctx, ev| { ... });
ctx.emit::<BodyEntered>(BodyEntered { body: player_id });
```

The signal is a struct. The struct's type is the signal's identity. The
receiver's closure signature is checked at compile time. Rename the
struct and every site updates in one search-and-replace.

## 3. Signal declaration

### 3.1 The `#[signal]` attribute

```rust
#[signal]
pub struct BodyEntered {
    pub body: NodeId,
}

#[signal]
pub struct Damaged {
    pub amount: f32,
    pub source: NodeId,
}
```

The attribute is a no-op marker — it does not transform the struct. It
exists in `ember_macros` (`ember_macros::signal`) but is **not** re-exported
by `ember::prelude`, and is optional: any `Send + Sync + 'static` type is a
signal (blanket impl), so the attribute is purely documentation. Its sole purpose is to make the intent grep-able and to enable an
optional linter (`cargo ember lint`, *not yet implemented*, v0.2) that warns about structs
used only in `emit`/`connect` calls.

### 3.2 Payload requirements

Every signal payload type must satisfy:

```rust
pub trait Signal: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Signal for T {}
```

In practice, this means:

- **No lifetimes.** Signal payloads are owned.
- **No `!Send` types.** `Rc`, raw pointers not allowed.
- **No references.** Use `NodeId`, not `&Node`.

This is what allows the signal table to be stored in a `HashMap` without
lifetime complications. `Signal` lives in `ember_scene::signal` and is
re-exported by `ember_scene::prelude`.

### 3.3 Where signals live

Signals are declared at the **module level**, not inside a node. Multiple
nodes can emit or receive the same signal type. This is deliberate — a
`Damaged` event emitted by a projectile to a health bar is not
"owned" by either.

## 4. Connection API

### 4.1 Connecting

```rust
ctx.connect::<BodyEntered>(target_id, |ctx, ev| {
    // ctx.current_id() == target_id here
    if let Some(health) = ctx.tree().get_mut(ctx.current_id()) { /* ... */ }
});
```

`connect` exists on both `NodeContext` and `NodeTree` (for setup code); see [[10-nodes-and-scenes#2.4 NodeContext|10-nodes-and-scenes.md §2.4]]).
It takes:

- `target: NodeId` — who receives the signal
- `handler: impl Fn(&mut NodeContext, &T) + Send + Sync + 'static`

Handlers are stored as `Arc` in one tree-level `SignalTable`, in a `Vec` per
signal `TypeId`, in connection order (ADR-023). If the target is the node
whose callback is currently running, it is checked out of the tree
(ADR-022), so `ctx.tree().get_mut(target)` returns `None`.

### 4.2 Disconnecting

```rust
ctx.disconnect::<BodyEntered>(target_id);
```

Removes **all** handlers of that type connected to `target_id`. Also
available on `NodeTree`. To disconnect a single handler, use a scoped
connection (§6).

### 4.3 Disconnection semantics

- Disconnecting a non-connected signal is a **no-op** (see
  [[02-principles#5.4 What is never an error|02-principles §5.4]]).
- Disconnecting is idempotent.
- Removing a node auto-disconnects everything targeting it.

## 5. Emission API

### 5.1 Emitting

```rust
ctx.emit::<Damaged>(Damaged { amount: 10.0, source: attacker_id });
```

The emitter does not specify **who** receives. The tree's `SignalTable`
holds the handlers for every signal type `T`. `NodeContext::emit`
snapshots the `T` handlers in connection order, releases the table, then
calls each with the same `&mut NodeContext` whose `current_id()` is set to
the handler's target. (There is no `SignalTable::emit`.)

### 5.2 Emission order

Emission order is **deterministic**: the order in which handlers were
connected. Two connection calls in the same `ready` result in the same
dispatch order.

Across frames, the order is stable unless connect/disconnect calls
modify the list. This is what makes signal-driven tests reproducible.

### 5.3 What the handler sees

Inside a handler:

- `ctx.current_id()` is the **target**'s id, not the emitter's.
- `ctx.tree()` returns the full tree.
- `ctx.spawn` / `spawn_child` / `despawn` are queued until the next flush;
  direct `ctx.tree().attach/remove` calls are immediate and safe.
- Emitting again is legal and runs immediately (nested).

Nested emission — a handler that emits another signal — is legal. The
inner signal's handlers run to completion before the outer signal's
dispatch continues. This is a **depth-first** dispatch order.

### 5.4 What the handler cannot do

- Hold a reference past the handler body.
- Spawn or despawn **immediately** via `ctx.spawn`/`despawn` (deferred
  to the next flush, see [[10-nodes-and-scenes#8.2 NodeContext and the tree|10-nodes-and-scenes §8.2]]).

## 6. Scoped connections

For handlers that should only live for a bounded lifetime (a menu, a
temporary effect), use the scoped connection API:

```rust
let conn = ctx.connect_scoped::<ButtonClicked>(self.id, |ctx, ev| {
    // ...
});
// ... later:
drop(conn);   // automatically disconnects on drop
```

`connect_scoped` returns a `#[must_use]` `ScopedConnection` handle. Dropping
the handle disconnects the specific closure (it wraps a connection id
internally); the removal is applied at the tree's next `flush` or at the
start of the next `emit`, whichever is first. `forget()` keeps the handler
connected for good. Also available on `NodeTree`.

This is the v0.1 way to disconnect a single handler. If you do not need
fine-grained control, `connect` + `disconnect` is enough.

## 7. Memory model

### 7.1 What the signal table holds

Internally, `SignalTable` is:

```rust
pub struct SignalTable {
    handlers: HashMap<TypeId, Vec<HandlerEntry>>,
    next_id: u64,
    dropped: Arc<Mutex<Vec<u64>>>,   // ids from dropped ScopedConnections
}

struct HandlerEntry {
    id: u64,
    target: NodeId,
    handler: Arc<dyn Any + Send + Sync>,  // an Arc<dyn Fn(&mut NodeContext, &T)>, T erased
}
```

`Handlers` hold a `NodeId`, not an `Rc<Node>`. This means:

- Disconnecting a node's signals when it despawns is a `HashMap` scan
  filtered on `target == id`. Fast.
- Holding a signal handler does **not** keep the target alive. If the
  target despawns and you forget to disconnect, `emit` will skip it (see
  §7.2).

### 7.2 Emitting to a dead target

If a handler targets a `NodeId` that has been freed:

- The handler is **not** called.
- The stale entry is removed from the table (a target that is merely queued
  and not yet live is skipped but kept).

This is a self-healing invariant — you never need to manually disconnect
when a node despawns, but explicit disconnection keeps the table smaller.

### 7.3 Cycle behaviour

Signals **can** form cycles (A emits to B, B emits to A). The depth-first
dispatch will recurse. There is no re-entrancy guard in v0.1.

**The safe pattern:** cycle handlers should be *pure* (no further emits
in the same call). If a handler must emit in response, dispatch the
second emit as deferred (`ctx.emit_deferred::<T>(payload)`), which queues
the emission on the tree; `flush` runs the queue FIFO, one round at a time,
alongside the ready cascade (same 1000-round cap).

```rust
ctx.emit_deferred::<Damaged>(Damaged { amount: 5.0, source: self.id });
```

Deferred emission preserves order (FIFO queue) and prevents stack
overflow from A→B→A cycles.

## 8. Examples

### 8.1 A health bar listening to damage

```rust
#[signal]
pub struct Damaged { pub amount: f32 }

#[derive(Node)]
#[class_name = "HealthBar"]
pub struct HealthBar {
    owner_id: NodeId,
    health: f32,
    max_health: f32,
}

impl Node for HealthBar {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let owner = ctx.current_id();
        ctx.connect::<Damaged>(owner, move |ctx, ev| {
            // adjust health on the target
            if let Some(bar) = ctx.tree().get_mut(owner)
                .and_then(|n| n.as_any_mut().downcast_mut::<HealthBar>())
            {
                bar.health = (bar.health - ev.amount).max(0.0);
            }
        });
    }
}
```

### 8.2 A button emitting on click

```rust
#[signal]
pub struct ButtonClicked;

#[derive(Node)]
#[class_name = "Button"]
pub struct Button {
    rect: Rect,
    label: String,
}

impl Node for Button {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        let mouse = ctx.input().mouse_pos;
        if ctx.input().is_mouse_pressed(MouseButton::Left)
            && self.rect.contains(mouse)
        {
            ctx.emit::<ButtonClicked>(ButtonClicked);
        }
    }
}
```

### 8.3 A scene connecting button → restart

```ron
(
    nodes: [
        (name: "Menu", class: "Menu"),
        (name: "PlayButton", class: "Button", parent: "Menu"),
    ],
    connections: [
        (signal: "ButtonClicked", from: "Menu/PlayButton", to: "Menu", method: "_on_play"),
    ],
)
```

*Not yet implemented.* Today `PackedScene::instantiate` validates that both
endpoints exist but does not wire connections (ADR-025: the string-named
`method` has no typed equivalent yet). Connect in code instead, e.g. from
`ready`, with `ctx.connect::<ButtonClicked>(menu_id, ...)`.

## 9. Design notes

### 9.1 Why TypeId dispatch instead of string

A `HashMap<TypeId, Vec<Handler>>` is faster than `HashMap<&str, _>` (no
hashing strings), and — the real win — **it cannot be typo'd**. A missing
signal name in a `.tscn` is a runtime crash; a missing signal type is a
compile error.

### 9.2 Why Fn and not method pointers

`Fn(&mut NodeContext, &T)` is what handlers need: access to the tree and
the payload. Method pointers (`fn(&mut Self, &T)`) would require an
explicit `&mut Self` and a way to route it — either through the tree (so
the method becomes `fn(&mut NodeContext, &T)` after downcast) or through
a callback registry (extra machinery for the same effect). Closures are
strictly simpler.

### 9.3 Why deferred emission is opt-in

Automatic deferral would break the common case (a `ButtonClicked`
handler that does work synchronously). Opting in when needed is what
most engines do, and it keeps the common path fastest.

### 9.4 Why `disconnect_all` at node removal

The alternative is a leak. If a node despawns and its connections stay,
the signal table grows unboundedly. Auto-cleanup is cheap: one filtered
`Vec::retain` per removal, in the same pass that already walks the tree.

## 10. Open questions

- **Q1:** Should `disconnect` accept a `ScopedConnection` handle rather
  than a type? *Provisional:* `disconnect::<T>(target)` for the coarse
  case, `drop(ScopedConnection)` for the fine case.
- **Q2:** Should handlers be `FnMut` instead of `Fn`? *Provisional:* no.
  State belongs on the node; handlers should stay stateless.
- **Q3:** Do we need a priority ordering for handlers (e.g. UI before
  game logic)? *Provisional:* not in v0.1. If needed, split into two
  signals (`ButtonClickedPre` / `ButtonClickedPost`) — ugly, but the
  alternative (sorting handler lists) is worse.
- **Q4:** Should emit be able to fail (e.g. if the target is gone)? 
  *Provisional:* no. Silence is correct (see
  [[02-principles#5.4 What is never an error|02-principles §5.4]]).

## 11. Cross-references

- Node lifecycle and `NodeContext` → [[10-nodes-and-scenes]]
- `.escene` connection syntax → [[30-escene-format]]
- `#[signal]` macro → [[26-ember-macros]]
- Signal testing patterns → [[43-testing-strategy]]
