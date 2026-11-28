# moon-gate

A sliding grid puzzle, Game 18 of the workspace and the first one designed
for the Ember engine rather than ported to it. A paper-lantern spirit slides
across ink-wash courtyards until something stops it; light every lantern to
open the moon gate, then slide through it.

**Status:** rules only (round G1 of the v0.2 plan). The crate is pure Rust
with no engine runtime and no binary yet: the rules, an undo history, a BFS
solver and eight draft levels. The node tree, `main.rs` and the `.escene`
levels painted in the editor come in round G2.

## Rules

| Piece | Char | Rule |
|---|---|---|
| floor | `.` | the spirit slides over it |
| wall | `#` | blocks; so does the board edge |
| spirit | `@` | starts here; every move slides until blocked |
| lantern | `L` | blocks; lights when the spirit is next to it |
| moon gate | `G` | blocks until every lantern is lit; sliding in wins |
| stone | `o` | a slide into it pushes it one floor cell and stops you |
| ink pool | `1`–`9` | entering one sends you out of its twin, same direction |
| paper bridge | `=` | crossed once, then a hole |
| hole | `x` | blocks |

A move that changes nothing is not counted. The fewest moves that win is
the level's par; winning in par earns the star.

## Tests

```sh
cargo test -p moon-gate
```

- `src/*` unit tests: the level parser and the solver.
- `tests/rules.rs`: one scenario per rule, plus undo / restart / par.
- `tests/levels.rs`: every level is solvable in exactly its par, and each
  chapter's new piece is needed.

Design notes live in the vault: `ember-vault/30 Games/Moon Gate/`.
