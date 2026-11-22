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
| 011 | [[50-design-decisions#ADR-011: Cooldown::new → running / ready\|Cooldown::new → running / ready]] | ❌ on main · ✅ on unmerged branch |
| 012 | [[50-design-decisions#ADR-012: Handle<T> for assets\|Handle<T> for assets]] | ✅ `Handle<T>` |
| 013 | [[50-design-decisions#ADR-013: No global mutable state\|No global mutable state]] | ✅ no global registry |
| 014 | [[50-design-decisions#ADR-014: Explicit plugin registration\|Explicit plugin registration]] | ❌ no `Plugin` trait yet |
| 015 | [[50-design-decisions#ADR-015: Hot-reload scope — assets only\|Hot-reload scope — assets only]] | 🟡 manual `reload`, no watcher |
| 016 | [[50-design-decisions#ADR-016: RenderingServer / AudioServer extensions\|RenderingServer / AudioServer extensions]] | ✅ trait extended |
| 017 | [[50-design-decisions#ADR-017: PhysicsServer uses opaque u64 handles, not NodeId\|PhysicsServer uses opaque u64 handles, not NodeId]] | ✅ u64 handles |
| 018 | [[50-design-decisions#ADR-018: `Input` is not `#[non_exhaustive]`\|`Input` is not `#[non_exhaustive]`]] | ✅ |
| 019 | [[50-design-decisions#ADR-019: `#[derive(Node)]` scope in v0.1\|`#[derive(Node)]` scope in v0.1]] | ✅ (but blocks behavioural nodes) |
| 020 | [[50-design-decisions#ADR-020: Wasm font/audio loading strategy\|Wasm font/audio loading strategy]] | ✅ `adopt_font`/`adopt_clip` |
| 021 | [[50-design-decisions#ADR-021: Nested folder organization for backend families\|Nested folder organization for backend families]] | ✅ nested backend folders |
| 022 | [[50-design-decisions#ADR-022: Tree passes take `Servers`; nodes are checked out while they run\|Tree passes take `Servers`; nodes are checked out while they run]] | ✅ |
| 023 | [[50-design-decisions#ADR-023: One tree-level `SignalTable`; handlers are `Arc`, dispatch lives on `NodeContext`\|One tree-level `SignalTable`; handlers are `Arc`, dispatch lives on `NodeContext`]] | ✅ |
| 024 | [[50-design-decisions#ADR-024: `ClassDB` is generic over `Object`; node construction is an extension\|`ClassDB` is generic over `Object`; node construction is an extension]] | ✅ |
| 025 | [[50-design-decisions#ADR-025: `.escene` loading takes a `ClassDB` and a tree; properties go through `Node::set_property`\|`.escene` loading takes a `ClassDB` and a tree; properties go through `Node::set_property`]] | ✅ (connections not wired) |
| 026 | [[50-design-decisions#ADR-026: `AssetServer` is type-driven and synchronous; GPU loads take a renderer\|`AssetServer` is type-driven and synchronous; GPU loads take a renderer]] | ✅ (deferred items open) |
| 027 | [[50-design-decisions#ADR-027: Nested `instance:` is resolved by `instantiate_with`\|Nested `instance:` is resolved by `instantiate_with`]] | ✅ |
| 028 | [[50-design-decisions#ADR-028: `LinearPhysics` lives in `ember-servers` and joins `Servers`\|`LinearPhysics` lives in `ember-servers` and joins `Servers`]] | ✅ |

The unmerged branch adds **ADR-029** (game-local `effects.rs` wrappers that `Deref` to the shared `Juice`) — see [[50-design-decisions]].

New ADRs: copy [[ADR Template]].
