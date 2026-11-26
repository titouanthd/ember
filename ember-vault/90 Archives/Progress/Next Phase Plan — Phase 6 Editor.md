---
title: "Next Phase Plan — Phase 6 Editor"
aliases: ["Phase 6 plan"]
tags: ["ember/progress", "ember/plan", "ember/phase-6", "ember/archive"]
created: "2026-11-24"
phase: "6 — Editor POC"
estimate: "3 sessions"
archived: "2026-11-26"
---

> [!warning] Archived 2026-11-26
> Kept for history — Phase 6 (editor POC) is closed; Editor v2 is planned in [[Next Phase Plan — Migration Close & Editor v2]]. Do not update this note. Index: [[Archives]].

# Next phase — Phase 6 · Editor POC

> [!summary]
> A dev-only scene editor in `crates/ember-editor`: open a `.escene`,
> show its node tree, inspect and edit exported properties, see the
> change in a preview, save. Built on Phase 2 (ADR-037: `#[export]`,
> `PackedScene::from_tree`) and Phase 5 (`ember_ui`). Spec:
> [[40-roadmap#9. Phase 6 — Editor POC|roadmap §9]].

## Design

- **Model / view split.** `Document` (pure, no window) owns the opened
  scene as a `NodeTree`, the `ClassDB`, the selection and a dirty flag:
  `open`, `outline`, `properties`, `set_property_text`, `save`. All exit
  criteria are tested on it headlessly.
- **Editable classes.** The CLI registers a small built-in set
  (`Player`, `Sprite`) written with `#[derive(Node)]` + `#[export]`.
  `Editor::with_classes` lets a game register its own.
- **Values as text.** A property is edited as text and parsed back to
  the kind it already has (`64, 64` for a `Vec2`, `r, g, b, a` for a
  `Color`, …), so the inspector is one `TextInput` per property.
- **App = a node tree too.** `EditorApp` (root) + `UiRoot`: outline
  panel (a `Button` per node in a `ScrollContainer`), inspector (a
  `Label` + `TextInput` per property), preview (the scene tree drawn
  inside a clipped viewport), toolbar (Save). Selection and edits arrive
  as `ButtonClicked` / `TextSubmitted` (ADR-036).

## Sessions

| Session | Content | Tests |
|---|---|---|
| **6.1 Model** ✅ `484ead0` | crate setup + layer rules, built-in nodes, `value` (format/parse), `Document`, CLI `--print` | `load_edit_save.rs` (exit criteria on the model), `value` unit tests |
| **6.2 App** ✅ `e4a725a` | `EditorApp` node: outline, inspector, preview, Save; `cargo run -p ember-editor -- scene.escene` | headless `SceneLoop` tests: click a node, edit `size`, save |
| **6.3 Close** ✅ | visual check, docs, crate note, tracker | — |

## Exit criteria ([[40-roadmap]] §9.2)

- [x] opens `player.escene`, shows `Player / Sprite / Hitbox`
- [x] clicking `Sprite` shows `texture` and `size`
- [x] changing `size` from `(32, 32)` to `(64, 64)` updates the preview
- [x] saving round-trips the change through `.escene`
