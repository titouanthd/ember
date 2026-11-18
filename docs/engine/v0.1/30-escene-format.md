# 30 — The `.escene` Format (v1)

> **Status:** 🟡 partial — see ADR-025 (no `instance:`, connections parsed but not wired)
> **Audience:** implementers, game authors
> **Prerequisites:** [21-ember-scene.md](21-ember-scene.md), [26-ember-macros.md](26-ember-macros.md)

## 1. Purpose

Specify the on-disk format for scene files. This is a **contract with the
future**: every scene file ever written to disk must remain loadable by
every future version of the engine that claims to support v1.

This is the file you read when you need to:

- Understand what a `.escene` file contains.
- Hand-edit a scene outside the editor.
- Write a tool that reads or writes scenes.
- Migrate a scene from a hand-authored struct to a file.

## 2. Format selection: RON

`.escene` files are **RON** (Rusty Object Notation). RON is:

- **Human-readable** (like JSON, but less punctuation noise).
- **VCS-friendly** (line-oriented enough to diff reasonably).
- **Type-rich** (distinguishes `1` from `1.0`, supports tuples, structs).
- **Already used** by the workspace (`levels.ron`, `units.ron`, `chapters.ron`).

The reason for RON over TOML, YAML, or JSON:

| Format | Readable | VCS diff | Rust types | Verdict |
|---|---|---|---|---|
| JSON | ✅ | ✅ | ❌ (no tuples, no enums) | No — JSON has no tuple type, which we need for `Vec2` |
| TOML | ✅ | ✅ | ❌ (tabular only) | No — TOML is config-shaped, not data-shaped |
| YAML | ⚠️ | ⚠️ | ❌ (soft types) | No — YAML's type ambiguity is a bug factory |
| RON | ✅ | ✅ | ✅ (matches Rust syntax) | **Yes** |

## 3. Format structure

A `.escene` file has **five sections, in order**:

1. **File header** — format version, unique id
2. **External resources** — assets referenced by the scene
3. **Internal resources** — sub-resources (shapes, materials, sub-scenes)
4. **Nodes** — the node tree, top-down
5. **Connections** — signal connections between nodes

Not all sections are required. An empty scene with one node needs only the
node section.

### 3.1 Minimal example

```ron
(
    format: 1,
    uid: "uid://scene_empty_a1b2c",
    nodes: [
        (name: "Root", class: "Node"),
    ],
)
```

### 3.2 Complete example

```ron
(
    format: 1,
    uid: "uid://scene_player_0a1b2",

    ext_resources: [
        (
            type: "Texture",
            uid: "uid://sprite_player_a3b1c",
            path: "sprites/player.png",
        ),
    ],

    sub_resources: [
        (
            type: "CircleShape2D",
            id: "shape_circle_a",
            props: (radius: 16.0),
        ),
    ],

    nodes: [
        (
            name: "Player",
            class: "Player",
            properties: {
                "max_hp": Float(100.0),
                "speed":  Float(5.0),
            },
        ),
        (
            name: "Sprite",
            class: "Sprite",
            parent: ".",
            properties: {
                "texture": ObjectRef("uid://sprite_player_a3b1c"),
                "size":    Vec2(32.0, 32.0),
            },
        ),
        (
            name: "Hitbox",
            class: "Hitbox",
            parent: ".",
            properties: {
                "shape": String("shape_circle_a"),
            },
        ),
    ],

    connections: [
        (
            signal: "Damaged",
            from: "..",
            to: "Sprite",
            method: "flash_red",
        ),
    ],
)
```

## 4. Section 1: File header

### 4.1 Fields

| Field | Type | Required | Purpose |
|---|---|---|---|
| `format` | `u32` | ✅ | Format version (currently `1`) |
| `uid` | `String` | ✅ | Unique scene id (`"uid://scene_<name>_<hash>"`) |

### 4.2 Version compatibility

The engine **rejects** any file whose `format` is greater than the max
supported version:

```
format version 2 not supported (max 1)
```

Downgrade (older `format` than current) is not permitted in v1 — there
are no older formats yet. When v2 arrives, it will define a migration
path from v1.

### 4.3 UID format

Scene UIDs follow the same scheme as asset UIDs (see
[13-assets.md §6.2](13-assets.md#62-format)):

```
uid://<category>_<base36-hash>
```

Categories: `scene_<name>_`, `sub_<type>_`, `uid_<hash>_` (fallback).
The category prefix is a convention for human readability, not a
correctness requirement.

## 5. Section 2: External resources

### 5.1 Purpose

External resources are files elsewhere on disk (textures, audio clips,
fonts, other scenes). Each entry records the file's **UID**, **type**,
and **path**. At load time, the scene resolves each UID to a
`Handle<T>`.

### 5.2 Fields per entry

| Field | Type | Required | Purpose |
|---|---|---|---|
| `type` | `String` | ✅ | Asset type: `"Texture"`, `"AudioClip"`, `"Font"`, `"PackedScene"` |
| `uid` | `String` | ✅ | Asset UID |
| `path` | `String` | ✅ | Relative path within `assets/` |

### 5.3 Why both UID and path

- **UID** is what the scene actually references in node properties.
  Stable across renames.
- **Path** is what the loader uses to find the file. It is metadata for
  the asset server, not the actual reference.

If a file is renamed on disk, only `path` changes in the scene; `uid`
stays. The reverse (UID changes but path stays) is impossible — UIDs are
content-stable.

### 5.4 Missing asset behavior

If an `ext_resource` is declared but the file doesn't exist:

- **Load succeeds** with a warning, and the asset's `Handle` is
  unresolved (`get()` returns `None`).
- Nodes referencing the missing asset still instantiate; their
  `#[export]` fields get `Handle::invalid()`.

This lets a scene load (with placeholders) even when some assets are
missing — important for iterating on a scene without all its art.

## 6. Section 3: Internal resources

### 6.1 Purpose

Sub-resources are small data structures embedded in the scene. They
don't get their own file. Used for:

- Collision shapes
- Materials
- Animation curves
- Sub-scene templates (rare)

### 6.2 Fields per entry

| Field | Type | Required | Purpose |
|---|---|---|---|
| `type` | `String` | ✅ | Sub-resource type: `"CircleShape2D"`, `"AabbShape2D"`, `"Material"` |
| `id` | `String` | ✅ | Local id (unique within the file, `"shape_<name>_<hash>"`) |
| `props` | `Map<String, Variant>` | optional | Serialized fields |

### 6.3 Referencing sub-resources

A node property can reference a sub-resource by its `id`:

```ron
properties: {
    "shape": String("shape_circle_a"),
}
```

At load time, the engine resolves this to a concrete `Shape` and passes
it to the node's `#[export]` field.

### 6.4 Sub-resource vs external resource

- **External** — file on disk, has its own UID, loaded via `AssetServer`.
- **Internal** — in-memory only, identified by file-local `id`.

The rule of thumb: if it's reusable across scenes, it's external. If
it's specific to this scene, it's internal.

## 7. Section 4: Nodes

### 7.1 Purpose

The node tree. Every entry is a node template. Entries are listed
**top-down, parent-before-children**. The first entry with no `parent`
is the scene root.

### 7.2 Fields per entry

| Field | Type | Required | Purpose |
|---|---|---|---|
| `name` | `String` | ✅ | Node name (unique within parent) |
| `class` | `String` | ✅ | Class name (must be registered in `ClassDB`) |
| `parent` | `String` | optional | Parent path (`"."` = immediate parent by tree order, `".."` = grandparent, `"Name"` = named sibling) |
| `instance` | `String` | optional | Path to a nested `.escene` (mutually exclusive with `class`) |
| `properties` | `Map<String, Variant>` | optional | Exported field values |
| `visible` | `bool` | optional | Default `true` |
| `z` | `f32` | optional | Default `0.0` |

### 7.3 Parent path semantics

The `parent` field refers to a previously-declared node by path. Paths
follow the same syntax as `NodeTree::find`:

- `"."` — the node declared immediately above (must be its parent)
- `".."` — two levels up
- `"Name"` — a sibling named `Name`
- `"Name/Child"` — the `Child` child of a sibling `Name`

Because node entries are listed parent-before-children, a `parent`
always refers to an **earlier entry**.

### 7.4 Nested scene instancing

An entry can either:
- Declare a `class` (create a new node of that class), or
- Declare an `instance` (instantiate a nested `.escene`).

```ron
(
    name: "Player",
    instance: "res://player.escene",
    parent: ".",
)
```

The nested scene is recursively instantiated at load time. Cycles are
detected and rejected:

```
error: cycle detected instancing res://player.escene
       (chain: player.escene → weapon.escene → player.escene)
```

### 7.5 Property serialization

`properties` keys are exported field names. Values are `Variant`:

| Rust type | Variant syntax in `.escene` |
|---|---|
| `bool` | `Bool(true)` |
| `i64` | `Int(42)` |
| `f64`, `f32` | `Float(3.14)` |
| `String` | `String("hello")` |
| `Vec2` | `Vec2(1.0, 2.0)` |
| `Color` | `Color(1.0, 0.0, 0.0, 1.0)` |
| `Option<T>` | `Nil` or the inner value |
| `Vec<T>` | `Array([...])` |
| `Handle<T>` | `ObjectRef("uid://...")` |
| `NodeId` | `String("path/to/node")` |
| Sub-resource ref | `String("shape_circle_a")` |

### 7.6 Node names

Node names must be:

- Non-empty.
- Unique within the immediate parent.
- Free of `/` (path separator).
- Free of `..` (path syntax conflict).

The engine validates these at load time and returns
`SceneError::Parse` on violation.

## 8. Section 5: Connections

### 8.1 Purpose

Signal connections declared inside the scene. At instantiation,
the engine resolves `from` and `to` to `NodeId`s and calls
`ctx.connect::<T>(target, handler)` where the handler dispatches to
`method`.

### 8.2 Fields per entry

| Field | Type | Required | Purpose |
|---|---|---|---|
| `signal` | `String` | ✅ | Signal type name (must match a registered `#[signal]` struct) |
| `from` | `String` | ✅ | Path to the emitting node |
| `to` | `String` | ✅ | Path to the receiving node |
| `method` | `String` | ✅ | Method name on the receiver |

### 8.3 Method dispatch

The `method` field names a method the receiver must have. The method is
resolved by a **dispatch trait** (`SignalReceiver`), which the
`#[derive(Node)]` macro implements for every node that wants to receive
scene-declared signals.

```rust
pub trait SignalReceiver {
    fn call_method(&mut self, method: &str, ctx: &mut NodeContext, payload: &Variant);
}
```

Nodes that don't implement `SignalReceiver` cannot be targets of
scene connections. At load time, this returns
`SceneError::ClassNotRegistered` or similar.

### 8.4 Connection timing

Connections are established **after all nodes are instantiated**, in the
order they appear in the file. This matches `ready` timing — a node can
emit a signal in `ready` and its connections are already live.

## 9. Round-trip guarantees

### 9.1 Idempotence

`PackedScene::from_file(path)?.to_ron()` produces a file that parses to
an equivalent `PackedScene`. The text may differ (whitespace, ordering
of `properties` keys) but the semantics are identical.

### 9.2 Unknown properties

A `.escene` file may contain properties that the current `Node` class
doesn't recognise (e.g. a property added by a future version). The
engine:

- **Warns** at load time (`log::warn!`).
- **Skips** the unknown property.
- **Preserves** it on re-serialization (if the node's `serialize`
  round-trips unknown fields).

Preserving unknown properties is what allows files to survive a
downgrade — a newer scene loads in an older engine without losing data.

### 9.3 Property ordering

`properties` is a `BTreeMap<String, Variant>` — keys are sorted
alphabetically. This makes diffs stable: adding a property doesn't
reorder existing ones.

## 10. Migration from structs

### 10.1 Why migrate at all

Before `.escene`, each game's level data lived in hand-written Rust
structs or a single `.ron` file with a custom format. That works but:

- No editor can display it.
- No cross-scene reuse.
- No nesting (a `PackedScene` inside another `PackedScene`).

`.escene` is a superset — it handles single-scene cases that were
previously structs, and also enables nesting and editor tooling.

### 10.2 Migration strategy

Per-game, incremental:

1. **Keep the current format in place.** Games can continue loading
   their hand-authored `.ron` files indefinitely.
2. **Add a `.escene` alternative** for a single scene (e.g. the player).
3. **Switch load sites** from the old format to `PackedScene::instantiate`.
4. **Delete the old format** when no more callers use it.

This is exactly what `41-migration-guide.md` describes per game.

## 11. Design notes

### 11.1 Why not TSCN's exact syntax

Godot's `.tscn` format is:

```
[gd_scene format=3 uid="uid://…"]
[ext_resource type="Script" path="res://player.gd" id="1"]
[sub_resource type="CircleShape2D" id="Circle_abc"]
[node name="Player" type="CharacterBody2D"]
[node name="Sprite" type="Sprite2D" parent="."]
[connection signal="pressed" from="Button" to="." method="_on_pressed"]
```

This is genuinely good. But it requires a custom parser and doesn't
leverage Rust's type system. Our RON-based variant gets the same 5-section
layout, the same UID-based referencing, and the same connection model,
but uses `serde` for parsing — no custom parser to maintain.

The cost is verbosity (`properties: { "speed": Float(5.0) }` instead of
`speed = 5.0`). The benefit is that the whole format round-trips through
Rust types without hand-written parsing code.

### 11.2 Why UIDs and not paths in references

Because renaming a file is the single most common refactor. Path-based
references break every referencing file. UID-based references are stable.

The path is kept alongside the UID in `ext_resources` for exactly one
purpose: letting the asset loader find the file without a full
UID → path lookup at load time.

### 11.3 Why parent paths, not explicit parent ids

Because parent-before-children ordering makes parent ids implicit.
`parent: "."` means "the node declared immediately above me", which is
almost always what the author means.

Explicit ids would require naming each node with a unique id in addition
to its name — redundant complexity for a rare case.

### 11.4 Why preserve unknown properties

Because scene files are the seam between versions. A scene saved in
v0.2 that references a `material` property can still load in v0.1 (with
a warning). Re-saving in v0.1 preserves the property. When the file is
re-opened in v0.2, the property is intact.

Without this, files would silently drop unknown data, forcing every
save to happen in the newest engine — a maintenance burden worse than
the small amount of extra code.

## 12. Open questions

- **Q1:** Should `.escene` support node groups (a la Godot)?
  *Provisional:* no in v0.1. Groups are a v0.2 feature (see
  [50-design-decisions.md](50-design-decisions.md)).
- **Q2:** Should the format support comments? *Provisional:* RON supports
  `//` line comments. We accept them but re-serialize without them
  (comments in scene files are documentation for the author, not for
  the engine).
- **Q3:** Should `.escene` files store timestamps or authorship?
  *Provisional:* no. That's what VCS is for.
- **Q4:** Should the `format` field bump on every schema change?
  *Provisional:* yes. `format: 1` is frozen; `format: 2` will be a new
  version with a defined migration path.

## 13. Cross-references

- Node/Scene mental model → [10-nodes-and-scenes.md](10-nodes-and-scenes.md)
- `PackedScene` API → [21-ember-scene.md §8](21-ember-scene.md#8-packedscene)
- Signal dispatch through connections → [11-signals.md §8.3](11-signals.md#83-a-scene-connecting-button--restart)
- UID assignment and lookup → [25-ember-assets.md §8](25-ember-assets.md#8-uid-registry)
- `#[derive(Node)]` serialization → [26-ember-macros.md §5](26-ember-macros.md#5-export)