---
tags: [ember/game, game/moon-gate]
game: Moon Gate
created: "2026-11-27"
---
# Moon Gate — Design

> Status (2026-11-27, round G1): the rules, undo and the solver are written and tested in `games/moon-gate`; no engine runtime yet. First draft: [[Next Phase Plan — Engine v0.2 & Game 18#Game 18 — first design draft|v0.2 plan]].

## Pitch

A small paper-lantern spirit crosses ink-wash courtyards at night. **Every move slides** the spirit until something stops it. Light every lantern in the courtyard to open the moon gate. Short handcrafted levels, solved in a few moves, with undo and restart always one key away.

## Core loop

1. Pick a level (chapter map, stars earned so far).
2. Swipe or press a direction: the spirit slides to the next obstacle.
3. Lanterns next to the spirit light; the gate opens when all are lit.
4. Solve in **par** moves for a star; undo and restart freely.
5. Next level; each chapter adds one new piece.

## Rules (as built in G1)

Code: `games/moon-gate/src/rules.rs`. Pure functions over `Level` (static board) and `State` (spirit, lit mask, stones, torn mask, won).

1. **Slide.** A move slides the spirit cell by cell until the next cell blocks: wall, lantern, hole, the board edge, or a closed gate.
2. **Lanterns** light when the spirit is orthogonally next to them: at the start, and on every cell it enters during a slide, not only where it stops. They stay lit.
3. **Gate.** Open once every lantern is lit (a level with none starts open). Sliding into the open gate wins, including in the same slide that lit the last lantern.
4. **Stone.** Sliding into a stone pushes it one cell if that cell is plain floor with no stone; the spirit stops where it is. Otherwise the spirit just stops. Stones never go onto pools, bridges, holes, lanterns or the gate.
5. **Ink pool.** Entering a pool sends the spirit out of its twin; the slide goes on in the same direction. Lanterns next to both pools light. A slide that would loop forever (pools in a cycle, or a pool pair on one open line) is refused (`MoveError::Endless`).
6. **Paper bridge.** Floor that tears into a hole once the spirit leaves it (passing over counts).
7. A move that changes nothing is refused (`MoveError::Blocked`) and not counted. Moves after a win are refused.

Every move returns its events in order (`Step`, `Teleport`, `Push`, `Lit`, `Tore`, `GateOpened`, `Won`) so the view (G2/G3) can animate them. `Game` keeps an undo stack (undo, restart, move count, `within_par`).

### Decisions made in G1

| Question | Choice | Why |
|---|---|---|
| When does a lantern light? | Spirit next to it at any time, mid-slide too | "passing next to" in the draft; one rule covers the start and the stop |
| Does a push move the spirit? | No: the stone moves one cell, the spirit stays | a push is a short, readable beat; pushing again moves the stone again |
| Pools and loops | Detect a repeated (cell, lit, torn) during a slide, refuse the move | lit and torn only grow, so a repeat means a true loop |
| Restart | Clears the undo history | simplest; revisit in G4 if players want undo past a restart |
| Level size limits | ≤ 32 lanterns, ≤ 64 bridges (bit masks) | keeps `State` small and hashable for the solver |

## Pieces (one new per chapter, ≈ 10 levels each)

| Chapter | New piece | Char | Draft levels (G1) |
|---|---|---|---|
| 1 — Courtyard | wall, lantern, gate | `#` `L` `G` | 1-01 First Light (par 3), 1-02 Two Lanterns (5) |
| 2 — Garden | stone | `o` | 2-01 Heavy Stone (8), 2-02 Rock Garden (8) |
| 3 — Pond | ink pool | `1`–`9` | 3-01 Ink Pond (4), 3-02 Twin Pools (8) |
| (stretch) 4 — Bridge | paper bridge | `=` (hole `x`) | 4-01 Paper Bridge (5), 4-02 Torn Paths (8) |

The draft levels are in `src/levels.rs` as ASCII. Six of them were picked from a throwaway random search (solver over random boards, kept only those of par 6–8 where removing the chapter's piece leaves no solution). They are placeholders until chapters are painted in the editor (G3).

## Engine bricks used

Planned, by round: `TileMap` for floor and walls (V2.1 → G2), instanced piece scenes for lantern, stone and pool (V3.3), `AnimatedSprite` for the spirit and flames and `Tween` for slides (V2.3 → G3), the input action map for keys + swipe (V1.3), `change_scene` for menu ↔ level (V2.2 → G4), `Persistence<T>` for stars, `AudioServer` + `music`, `fx` for the light-up burst, `ember-ui` for menus, virtual resolution (ADR-041) for the web/mobile shell. G1 uses only `ember_stdlib::Grid` for the board.

## Data files (.ron)

- G1: levels are ASCII strings in `src/levels.rs` (`LevelDef { id, name, par, text }`); `Level::parse` reads them.
- From G2: `scenes/levels/<chapter>-<nn>.escene`, one per level, painted in the editor; par as an export on the root. `scenes/pieces/*.escene` for reusable pieces placed as instances. The ASCII legend stays the test format and maps one-to-one onto tile ids.
- `assets/`: one atlas for tiles + sprites (procedural ink-wash at first, open question 2 of the plan).

## Tests

- `src/level.rs`: parser (every piece, edges, bad input).
- `src/solver.rs`: shortest solution, unsolvable level, state limit.
- `tests/rules.rs`: one scenario per rule, events, undo / restart / par.
- `tests/levels.rs`: **every level is solvable in exactly its par** (BFS) and the solution replays through `Game`; each chapter's new piece is needed (removing it leaves no solution in par); ids unique and in order.
- Later: `tests/scene.rs` (`SceneLoop` on mocks, menu → level → win → next) in G2/G4, `tests/editor.rs` (open, paint, save, reload a level) in G2/G3.

## Open questions

- **Art** (plan open question 2): procedural ink-wash throughout, or drawn sprite sheets from G3?
- **Mobile** (plan open question 4): swipe only, or on-screen arrows too?
- Should stones be pushable onto paper bridges (and tear them, or fall into holes and fill them)? Not in G1; a chapter-4 idea if it needs more depth.
- Par from the solver vs. a hand-set par: today par = the BFS optimum, checked by the test. Keep that, or allow a looser par for early levels?
