---
title: "Ember — Home"
tags: ["ember/moc"]
aliases: ["Home", "Index"]
generated: "2026-11-22"
---

# 🔥 Ember — Knowledge Vault

A 2D game engine in Rust (macroquad backend) built on top of **17 games**. **This vault is the only home of the project documentation** — the only docs outside it are the `README.md` files (repo root and 5 games). It also holds generated notes that map the code and evaluate progress.

> [!summary] Where things stand (2026-11-22)
> Phases 0, 2, 4, 5 (UI) and 6 (editor POC) are **closed**; **5 / 17** games run on the engine (snake, simon, memory, minesweeper; pong partly). Overall v0.1 POC ≈ **60 %**, the rest is mostly migration. → [[Handover 2026-11-26]]

## Start here
- [[Handover 2026-11-26]] — latest state, how to push, what to do next
- [[Handover 2026-11-22]] — the vault move, Phase 0 close
- [[Roadmap Review 2026-11-22]] — latest phase-by-phase status and re-plan (≈ 37 %)
- [[Baseline 2026-11-22]] — gate results after the Phase 0 merge (2,065 tests green)
- [[Progress Evaluation 2026-11-22]] — status, scorecard, risks, next steps
- [[Roadmap Tracker]] — every exit criterion as a checkbox
- [[Doc–Code Discrepancies]] — where docs and code disagree
- [[Engine Spec Index]] — the original spec entry point (reading paths A–D)

## Workflow
- [[Commit & Push]] — commit message format, pre-push checks, push / merge / hotfix flow
- [[Next Phase Plan — Phase 6 Editor]], [[Next Phase Plan — Phase 5 UI]] — past phase plans (Phase 7 follows [[41-migration-guide]])

## Maps of content
- [[Crates MOC]] — 12 engine crates, layers, LOC, test counts
- [[Games MOC]] — 17 games in migration order
- [[ADR Index]] — 28 decisions with implementation state
- [[51-glossary|Glossary]]

## Engine spec v0.1 (`docs/engine/v0.1/`)
- [[00-overview]]
- [[01-architecture]]
- [[02-principles]]
- [[10-nodes-and-scenes]]
- [[11-signals]]
- [[12-servers]]
- [[13-assets]]
- [[14-main-loop]]
- [[15-input]]
- [[20-ember-core]]
- [[21-ember-scene]]
- [[22-ember-servers]]
- [[23-ember-render-macroquad]]
- [[24-ember-ui]]
- [[25-ember-assets]]
- [[26-ember-macros]]
- [[30-escene-format]]
- [[40-roadmap]]
- [[41-migration-guide]]
- [[42-acceptance-tests]]
- [[43-testing-strategy]]
- [[50-design-decisions]]
- [[51-glossary]]

## Workspace docs
- [[Browser Build (WASM)]] · [[Project Recap (version.txt)]] · repo README: `README.md` (outside the vault)


## Vault layout
| Folder | Contents |
|---|---|
| `10 Engine Spec v0.1` | 24 spec files from `docs/engine/v0.1` (links converted to wikilinks) |
| `20 Workspace` | former `BROWSER_BUILD.md` and `version.txt` |
| `30 Games` | one folder per game: hub note + Design / Session / Release notes (READMEs and credits stay in `games/<game>/README.md`) |
| `40 Crates` | generated note per engine crate |
| `50 Decisions` | ADR index |
| `60 Progress` | evaluation, tracker, discrepancies |
| `80 Templates` | [[ADR Template]] and [[Game Design Template]] (core Templates plugin points here) |
