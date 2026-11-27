---
title: "ADR Index"
tags: ["ember/moc", "ember/adr"]
generated: "2026-11-22"
---

# Architecture Decision Records

All ADRs live in [[50-design-decisions]]. Each link below jumps to its heading. The *Code* column is this vault's assessment against the `main` branch.

| ADR | Decision | Code |
|---|---|---|
| 001 | [[50-design-decisions#ADR-001: Scene tree over pure ECS\|Scene tree over pure ECS]] | ✅ reflected (tree-first) |
| 002 | [[50-design-decisions#ADR-002: RON for scene format\|RON for scene format]] | ✅ `.escene` is RON |
| 003 | [[50-design-decisions#ADR-003: macroquad for v0.1, wgpu deferred\|macroquad for v0.1, wgpu deferred]] | ✅ macroquad backend |
| 004 | [[50-design-decisions#ADR-004: Typed signals over string-keyed\|Typed signals over string-keyed]] | ✅ typed signals |
| 005 | [[50-design-decisions#ADR-005: Rust-only, no scripting language\|Rust-only, no scripting language]] | ✅ |
| 006 | [[50-design-decisions#ADR-006: Trait crates + impl crates\|Trait crates + impl crates]] | ✅ trait + impl crates |
| 007 | [[50-design-decisions#ADR-007: slotmap for NodeId\|slotmap for NodeId]] | ✅ slotmap `NodeId` |
| 008 | [[50-design-decisions#ADR-008: Fixed-step physics, variable-step process\|Fixed-step physics, variable-step process]] | ✅ `FIXED_DT` accumulator in runner |
| 009 | [[50-design-decisions#ADR-009: Deferred spawn / structural changes\|Deferred spawn / structural changes]] | ✅ deferred spawn/despawn |
| 010 | [[50-design-decisions#ADR-010: primary_action convenience method\|primary_action convenience method]] | ✅ `primary_action` in ember-input |
| 011 | [[50-design-decisions#ADR-011: Cooldown::new → running / ready\|Cooldown::new → running / ready]] | ✅ `running`/`ready(d)`, old names `#[deprecated]` (merged 2026-11-22) |
| 012 | [[50-design-decisions#ADR-012: Handle<T> for assets\|Handle<T> for assets]] | ✅ `Handle<T>` |
| 013 | [[50-design-decisions#ADR-013: No global mutable state\|No global mutable state]] | ✅ no global registry |
| 014 | [[50-design-decisions#ADR-014: Explicit plugin registration\|Explicit plugin registration]] | ❌ no `Plugin` trait yet |
| 015 | [[50-design-decisions#ADR-015: Hot-reload scope — assets only\|Hot-reload scope — assets only]] | 🟡 manual `reload`, no watcher |
| 016 | [[50-design-decisions#ADR-016: RenderingServer / AudioServer extensions\|RenderingServer / AudioServer extensions]] | ✅ trait extended |
| 017 | [[50-design-decisions#ADR-017: PhysicsServer uses opaque u64 handles, not NodeId\|PhysicsServer uses opaque u64 handles, not NodeId]] | ✅ u64 handles |
| 018 | [[50-design-decisions\|Input is not non_exhaustive]] (ADR-018) | ✅ |
| 019 | [[50-design-decisions\|derive(Node) scope in v0.1]] (ADR-019) | ✅ amended by ADR-030 |
| 020 | [[50-design-decisions#ADR-020: Wasm font/audio loading strategy\|Wasm font/audio loading strategy]] | ✅ `adopt_font`/`adopt_clip`; amended `7bdf040` (bytes load on wasm) |
| 021 | [[50-design-decisions#ADR-021: Nested folder organization for backend families\|Nested folder organization for backend families]] | ✅ nested backend folders |
| 022 | [[50-design-decisions#ADR-022: Tree passes take `Servers`; nodes are checked out while they run\|Tree passes take `Servers`; nodes are checked out while they run]] | ✅ |
| 023 | [[50-design-decisions#ADR-023: One tree-level `SignalTable`; handlers are `Arc`, dispatch lives on `NodeContext`\|One tree-level `SignalTable`; handlers are `Arc`, dispatch lives on `NodeContext`]] | ✅ |
| 024 | [[50-design-decisions#ADR-024: `ClassDB` is generic over `Object`; node construction is an extension\|`ClassDB` is generic over `Object`; node construction is an extension]] | ✅ |
| 025 | [[50-design-decisions#ADR-025: `.escene` loading takes a `ClassDB` and a tree; properties go through `Node::set_property`\|`.escene` loading takes a `ClassDB` and a tree; properties go through `Node::set_property`]] | ✅ (connections wired since M4.1) |
| 026 | [[50-design-decisions#ADR-026: `AssetServer` is type-driven and synchronous; GPU loads take a renderer\|`AssetServer` is type-driven and synchronous; GPU loads take a renderer]] | ✅ (deferred items open) |
| 027 | [[50-design-decisions#ADR-027: Nested `instance:` is resolved by `instantiate_with`\|Nested `instance:` is resolved by `instantiate_with`]] | ✅ |
| 028 | [[50-design-decisions#ADR-028: `LinearPhysics` lives in `ember-servers` and joins `Servers`\|`LinearPhysics` lives in `ember-servers` and joins `Servers`]] | ✅ |

| 029 | [[50-design-decisions#ADR-029: Shared `ember_stdlib::juice`; game-specific effects wrap it\|Shared juice; game effects wrap it]] | ✅ merged 2026-11-22 |
| 030 | [[50-design-decisions\|Derive generates `NodeName`; users own `impl Node`]] (ADR-030) | ✅ `8cc3112` |
| 031 | [[50-design-decisions#ADR-031: Clip-rect stack on `RenderingServer`\|Clip-rect stack on RenderingServer]] | ✅ `a1072f5` (+ camera sign fix `e190c21`) |
| 032 | [[50-design-decisions#ADR-032: Typed text, mouse wheel and editing keys in `Input`\|Typed text, mouse wheel, editing keys]] | ✅ `7bde4b0` |
| 033 | [[50-design-decisions#ADR-033: Phase 4's macroquad-isolation exit moves to Phase 7\|Phase 4 isolation exit moves to Phase 7]] | ✅ `c0fde0c` |
| 034 | [[50-design-decisions#ADR-034: How `ember_ui` controls live in the scene tree\|How ember_ui controls live in the scene tree]] | ✅ `cdbc078` |
| 035 | [[50-design-decisions#ADR-035: Post-children draw hook and children clip for controls\|Post-children draw hook and children clip]] | ✅ `a8f18f8` |
| 036 | [[50-design-decisions#ADR-036: `UiRoot` routes input and emits control signals\|UiRoot routes input and emits control signals]] | ✅ `45e44da` |
| 037 | [[50-design-decisions#ADR-037: Exported properties live on `NodeName`; scenes are written from live trees\|Exported properties on NodeName; scene writer]] | ✅ `f3eedef` `d057b2c` |
| 038 | [[50-design-decisions#ADR-038: Font and audio loaders; hot reload by polling\|Font/audio loaders; polling hot reload]] | ✅ `a048f66` |
| 039 | [[50-design-decisions#ADR-039: How a game plugs into the editor\|How a game plugs into the editor]] | ✅ `6aaf9a0` |
| 040 | [[50-design-decisions#ADR-040: Camera transform (offset + zoom)\|Camera transform (offset + zoom)]] | ✅ `dfa8b04` |
| 041 | [[50-design-decisions#ADR-041: Virtual resolution with letterbox\|Virtual resolution with letterbox]] | ✅ `ef68ce5` · amended `bebe16d` (viewport following the window) |
| 042 | [[50-design-decisions#ADR-042: Polygon, arc and ellipse shapes\|Polygon, arc and ellipse shapes]] | ✅ `20a1c21` |
| 043 | [[50-design-decisions#ADR-043: Looped clips, clip readiness and MP3\|Looped clips, clip readiness, MP3]] | ✅ `d5ba842` `a855e43` |
| 046 | [[50-design-decisions#ADR-046: `ember_stdlib` may use `ember_servers`; engine-typed juice (`fx`)\|stdlib may use servers; `fx`]] | ✅ `9fa73de` |
| 047 | [[50-design-decisions#ADR-047: Window options (`run_window_with`)\|Window options (`run_window_with`)]] | ✅ `45392e9` |
| 048 | [[50-design-decisions#ADR-048: Texture filtering (`set_texture_filter`)\|Texture filtering]] | ✅ `fcf7a8e` |

New ADRs: copy [[ADR Template]].
