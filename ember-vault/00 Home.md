---
title: "Ember — Home"
tags: ["ember/moc"]
aliases: ["Home", "Index"]
generated: "2026-11-22"
updated: "2026-11-26"
---

# 🔥 Ember — Knowledge Vault

A 2D game engine in Rust (macroquad backend) built on top of **17 games**. **This vault is the only home of the project documentation** — the only docs outside it are the `README.md` files (repo root and 5 games). It also holds generated notes that map the code and evaluate progress.

> [!summary] Where things stand (2026-11-27)
> **v0.1 is done:** Phases 0–8 are **closed**, `CHANGELOG.md` is on `main`; Phase 9 (Editor v2) has one box left, moved to v0.2. **17 / 17** games run on the engine (snake, simon, memory, minesweeper, freecell, breakout, asteroids, air-hockey, bullet-hell, pong, taiji, pagoda, zhuo-ji, jade-garden, accretion, bore-inc, ember-wars); no game imports macroquad. Ember-wars runs as a node tree (`0ad070c`) and **passed its acceptance test** (all 20 items of [[42-acceptance-tests]], 2026-11-27).
> **Goal now:** Ember v0.2 (tiles, animation, input actions, scene changes, an editor you paint levels in) and **Game 18**, a grid puzzle built in the editor. → [[Next Phase Plan — Engine v0.2 & Game 18]]

## Start here
- [[Handover 2026-11-26]] — latest state, how to push, gotchas
- [[Next Phase Plan — Engine v0.2 & Game 18]] — **the current plan**: v0.2 features (ADR-050+), Game 18's design draft, rounds V0–V3 and G1–G5
- [[Next Phase Plan — Migration Close & Editor v2]] — the v0.1 close-out plan (M1–M5), done
- [[Roadmap Tracker]] — every exit criterion as a checkbox (Phases 0–9)
- [[Doc–Code Discrepancies]] — where docs and code disagree
- [[Engine Spec Index]] — the original spec entry point (reading paths A–D)
- [[Archives]] — old handovers, evaluations, closed phase plans, pre-engine logs

## Workflow
- [[Commit & Push]] — commit message format, pre-push checks, push / merge / hotfix flow
- Rounds: one game or one engine feature per round. Claude commits locally, you run `make pre-push`, play the game and push.
- [[Token Budget]] — one session per round, plan before building, small tool output (what the 10-round session cost, and why)

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
- [[Browser Build (WASM)]] · repo README: `README.md` (outside the vault) · the old `version.txt` recap is archived ([[Project Recap (version.txt)]])


## Vault layout
| Folder | Contents |
|---|---|
| `10 Engine Spec v0.1` | 24 spec files from `docs/engine/v0.1` (links converted to wikilinks) |
| `20 Workspace` | former `BROWSER_BUILD.md` |
| `30 Games` | one folder per game: hub note + Design / Session / Release notes (READMEs and credits stay in `games/<game>/README.md`) |
| `40 Crates` | generated note per engine crate |
| `50 Decisions` | ADR index |
| `60 Progress` | latest handover, current plan, tracker, discrepancies |
| `80 Templates` | [[ADR Template]] and [[Game Design Template]] (core Templates plugin points here) |
| `90 Archives` | superseded snapshots, closed plans and pre-engine logs — see [[Archives]] |
