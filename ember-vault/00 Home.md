---
title: "Ember — Home"
tags: ["ember/moc"]
aliases: ["Home", "Index"]
generated: "2026-11-22"
---

# 🔥 Ember — Knowledge Vault

A 2D game engine in Rust (macroquad backend) built on top of **17 games**. **This vault is the only home of the project documentation** — the only docs outside it are the `README.md` files (repo root and 5 games). It also holds generated notes that map the code and evaluate progress.

> [!summary] Where things stand (2026-11-22)
> Engine core (scene tree, `.escene`, signals, servers) is **~70 % done**; UI and editor **not started**; **1 / 17** games migrated (pong). Overall v0.1 POC ≈ **30 %**. → [[Progress Evaluation 2026-11-22]]

## Start here
- [[Handover 2026-11-22]] — what changed today, how to push, what to do next
- [[Progress Evaluation 2026-11-22]] — status, scorecard, risks, next steps
- [[Roadmap Tracker]] — every exit criterion as a checkbox
- [[Doc–Code Discrepancies]] — where docs and code disagree
- [[Engine Spec Index]] — the original spec entry point (reading paths A–D)

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

## Branch snapshot
`titouanthd-docs-crates-games-plan` (local, unpushed) has a reviewed copy of every spec file → [[Engine Spec Index]]. Kept separate so the `main` versions stay authoritative.

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
| `90 Branch Snapshot` | docs from the unmerged branch |
