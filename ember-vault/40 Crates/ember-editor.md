---
title: "ember-editor"
tags: ["ember/crate"]
layer: "L4 · Main"
path: "crates/ember-editor"
loc: 19
tests: 1
generated: "2026-11-22"
---

# ember-editor

**Layer:** L4 · Main · **Path:** `crates/ember-editor` · **Files:** 2 · **Lines:** 19 · **`#[test]`:** 1

## Purpose

Scene inspector POC (planned)

## Dependencies (Cargo.toml)

—

## Implementation status

🟢 Phase 6 POC, closed 2026-11-24 (sessions 6.1 `484ead0`, 6.2 `e4a725a`). `make editor` opens a scratch copy of `player.escene` in `target/editor/`. `Document` (open / outline / properties as text / edit / save via `PackedScene::from_tree`), built-in `Player` and `Sprite` (`#[derive(Node)]` + `#[export]`), `app::EditorApp` window: toolbar (Save, status), outline, inspector (one `TextInput` per property), clipped preview of the scene. `make editor SCENE=…`, `--print` for a text outline. 13 tests (`load_edit_save.rs`, `app.rs`) + `value` unit tests. **Limits:** knows only its own `Player` / `Sprite` classes, no add/remove/rename of nodes, no undo, textures not drawn, connections and ext resources not saved, at most 16 properties per node. **Next:** Editor v2 (Phase 9) — games plug their classes in, proof = opens Pong; see [[Next Phase Plan — Migration Close & Editor v2]].

## Spec

- No dedicated spec file (covered by [[01-architecture]]).

## See also

- [[Crates MOC]] · [[Roadmap Tracker]] · [[Handover 2026-11-26]]
