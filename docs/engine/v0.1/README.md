# Ember Engine — Documentation (v0.1)

> **Status:** 🟡 partial — reviewed 2026-11-19; index and snapshot mirror per-file status headers
> **Audience:** all (entry point)
> **Prerequisites:** none

Ember is a 2D game engine written in Rust, sitting on top of
[macroquad](https://github.com/not-fl3/macroquad) and [miniquad](https://github.com/not-fl3/miniquad).
This directory is the single source of truth for its architecture, API
specifications, formats, and roadmap.

The current workspace already ships **17 playable 2D games**. This
documentation describes how those games get a real engine underneath
them without rewriting their logic — the goal of the v0.1 POC.

**Naming:** specs say `ember_core`, `ember_scene`, …; the actual
crates and directories use hyphens (`ember-core`, `ember-scene`;
backends live in `crates/ember-render/macroquad` and
`crates/ember-audio/macroquad`).

### Implementation snapshot

| Area | State |
|---|---|
| `ember-core`, `ember-scene`, `ember-servers`, `ember-assets`, `ember-input`, macroquad backends | 🟡 implemented, partially (see ADR-022…028) |
| `ember-macros` | 🟡 `#[derive(Node)]` only |
| `ember-ui`, `ember-editor` | ⬜ stubs |
| Games migrated to `NodeTree` | 1 / 17 (`pong`) |

Per-phase detail lives in the *Progress* notes of
[40-roadmap.md](40-roadmap.md).

---

## 1. What this document set is

- **A specification**, not a tutorial. Crate APIs are frozen here *before*
  they are implemented. PRs are reviewed against these files.
- **A roadmap**. Every phase in `40-roadmap.md` has an entry and exit
  criterion; we do not advance by feel.
- **A decision log**. `50-design-decisions.md` records *why* each
  non-obvious choice was made, so future maintainers do not relitigate
  them from memory.
- **A migration manual**. `41-migration-guide.md` tells each game how to
  move from its current `main.rs` to the engine's `MainLoop + SceneTree`.

## 2. What this document set is **not**

- **Not user-facing docs.** A player-facing guide (how to press Space to
  start a game) is a separate workstream, out of scope for v0.1.
- **Not an API reference for `ember_stdlib`.** The existing standard
  library is documented by its own doc-comments; this set covers only the
  *new* engine layers (`ember_scene`, `ember_servers`, etc.).
- **Not a design diary.** Everything that is a *decision* lives in
  `50-design-decisions.md`. Everything else is spec.

---

## 3. Status legend

Every file begins with a status header (line 3) so readers know what they can rely on:

| Status | Meaning | Safe to code against? |
|---|---|---|
| 📋 planned | Documented in this README, not yet written | No |
| ⬜ draft | Written, awaiting review | No — content may change |
| 🟨 review | Reviewed, actively refined | Cautiously — flag mismatches |
| 🟡 partial | Reviewed against code; part of the spec is implemented, the rest is flagged inline | Only the implemented parts |
| ✅ up to date | Reviewed against code and matches it | Yes |
| ✅ frozen | Locked; code must conform; changes need an ADR | Yes |

**Rule:** any file referenced by a crate's public API must be ✅ frozen
before that crate's first PR lands.

---

## 4. File index

### Wave 1 — Foundation

| File | Purpose | Status |
|---|---|---|
| [00-overview.md](00-overview.md) | What Ember is, POC scope, success criteria | 🟡 partial |
| [01-architecture.md](01-architecture.md) | Layer diagram, crate map, dependency rules | 🟡 partial |
| [02-principles.md](02-principles.md) | Invariants, naming, testing, error handling | 🟡 partial |
| [51-glossary.md](51-glossary.md) | Terms and acronyms, cross-referenced | 🟡 partial |

### Wave 2 — Concepts

| File | Purpose | Status |
|---|---|---|
| [10-nodes-and-scenes.md](10-nodes-and-scenes.md) | Node, NodeId, NodeTree, PackedScene, lifecycle | 🟡 partial |
| [11-signals.md](11-signals.md) | Typed signals, connect/emit, ordering | 🟡 partial |
| [12-servers.md](12-servers.md) | Rendering/Audio/Physics trait contracts | 🟡 partial |
| [13-assets.md](13-assets.md) | AssetServer, UIDs, hot-reload, async | 🟡 partial |
| [14-main-loop.md](14-main-loop.md) | MainLoop, fixed step, pause, quit | 🟡 partial |
| [15-input.md](15-input.md) | Input mapping, primary_action, gamepad/touch | 🟡 partial |

### Wave 3 — Crate Specifications

| File | Purpose | Status |
|---|---|---|
| [20-ember-core.md](20-ember-core.md) | math, time, hash, rng, Variant, Object/ClassDB | 🟡 partial |
| [21-ember-scene.md](21-ember-scene.md) | Full Node / SceneTree / PackedScene API | 🟡 partial |
| [22-ember-servers.md](22-ember-servers.md) | Trait signatures, backend registration | 🟡 partial |
| [23-ember-render-macroquad.md](23-ember-render-macroquad.md) | Reference backend implementation | 🟡 partial |
| [24-ember-ui.md](24-ember-ui.md) | Control nodes, layout, focus, theming | ⬜ draft |
| [25-ember-assets.md](25-ember-assets.md) | Loader registry, UID lookup, hot-reload | 🟡 partial |
| [26-ember-macros.md](26-ember-macros.md) | #[derive(Node)], #[signal], #[export] | 🟡 partial |

### Wave 4 — Format, Process & Reference

| File | Purpose | Status |
|---|---|---|
| [30-escene-format.md](30-escene-format.md) | `.escene` v1 spec, examples, migration | 🟡 partial |
| [40-roadmap.md](40-roadmap.md) | 9 phases, entry/exit criteria | 🟡 partial |
| [41-migration-guide.md](41-migration-guide.md) | Per-game checklist, rollback | 🟡 partial |
| [42-acceptance-tests.md](42-acceptance-tests.md) | ember-wars as conformance suite | ⬜ draft |
| [43-testing-strategy.md](43-testing-strategy.md) | Unit/integration/conformance matrix, CI | 🟡 partial |
| [50-design-decisions.md](50-design-decisions.md) | ADRs with alternatives considered | 🟡 partial |

---

## 5. Reading paths

Pick the path that matches what you want to do. Times are for a first read.

### Path A — "I want to write a game in Ember" (~45 min)

1. [00-overview.md](00-overview.md) — what the engine is, what it isn't
2. [10-nodes-and-scenes.md](10-nodes-and-scenes.md) — the mental model
3. [11-signals.md](11-signals.md) — how nodes talk to each other
4. [14-main-loop.md](14-main-loop.md) — the entry point you write
5. [21-ember-scene.md](21-ember-scene.md) — the trait you implement

### Path B — "I want to implement an engine crate" (~2 h)

1. [01-architecture.md](01-architecture.md) — crate map and dependency rules
2. [02-principles.md](02-principles.md) — invariants you must not violate
3. The crate spec for your layer (`20`…`26`)
4. [30-escene-format.md](30-escene-format.md) — if you touch serialization
5. [43-testing-strategy.md](43-testing-strategy.md) — how your crate gets tested

### Path C — "I'm reviewing the plan" (~30 min)

1. [00-overview.md](00-overview.md) — scope and success criteria
2. [40-roadmap.md](40-roadmap.md) — phases and gates
3. [42-acceptance-tests.md](42-acceptance-tests.md) — how we know we're done
4. [50-design-decisions.md](50-design-decisions.md) — the non-obvious choices

### Path D — "I'm a first-time contributor" (~1 h)

1. [README.md](README.md) — you're here
2. [00-overview.md](00-overview.md) — the pitch
3. [01-architecture.md](01-architecture.md) — crate layout
4. [02-principles.md](02-principles.md) — the rules
5. Pick a phase from [40-roadmap.md](40-roadmap.md) and start

---

## 6. Contribution workflow

This is a *spec-first* repository. Every engine crate follows this loop:

1. **Read the spec.** Confirm the section you're implementing is ✅ frozen.
2. **Open a branch.** `feat/<crate>-<feature>` or `docs/<wave>-<topic>`.
3. **Write tests against the spec.** They will fail.
4. **Implement until green.** No deviation from the spec without an ADR.
5. **Update docs if needed.** A spec change is an ADR in
   [50-design-decisions.md](50-design-decisions.md), not a silent edit.
6. **PR is reviewed against the spec.** If the reviewer finds a mismatch
   between code and spec, the spec wins.

### How to mark progress

- After a file is written → ⬜ draft
- After a maintainer reviews it → 🟨 review
- After the corresponding crate's first PR lands → ✅ frozen
- Editing a ✅ file requires an ADR entry first.

### Commit message conventions

- `docs(engine): <wave> — <topic>` for documentation PRs
- `feat(<crate>): <feature>` for engine code
- `refactor(<game>): migrate to <phase>` for game migrations
- `fix(<crate>): …` for bugfixes
- `chore: …` for tooling

---

## 7. How this set evolves

- **v0.1** is the current version. It covers the POC (2D, Rust-only,
  macroquad backend, no editor beyond a scene inspector).
- **v0.2** will be created when the POC completes — same folder pattern,
  `docs/engine/v0.2/`. The v0.1 folder is then frozen forever so future
  readers can see what the POC promised vs. delivered.
- **Between versions**, the top-level `docs/engine/README.md` (not
  versioned) links to the current version and lists the diffs.

---

## 8. Cross-references

- **Repository:** [ember-workspace](.) (root)
- **Continuing CI:** `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo test --workspace --no-fail-fast` must pass on every PR.
- **Architecture quick look:** [01-architecture.md §2](01-architecture.md#2-the-five-layer-model)
- **Phases at a glance:** [40-roadmap.md](40-roadmap.md)
- **Glossary:** [51-glossary.md](51-glossary.md)