---
title: "43 — Testing Strategy"
aliases: ["43 — Testing Strategy"]
tags: ["ember/spec", "ember/wave-4"]
doc-status: "draft"
former-path: "docs/engine/v0.1/43-testing-strategy.md"
imported: "2026-11-22"
wave: "4 — Format, Process & Reference"
---

# 43 — Testing Strategy

> **Status:** ⬜ draft
> **Audience:** implementers, reviewers
> **Prerequisites:** [[02-principles#6. Testing philosophy|02-principles.md §6]]

## 1. Purpose

Define the testing approach across all Ember crates. Covers the three
test layers (unit, integration, conformance), what each crate must have,
what CI runs, and what "good test" means in this codebase.

This is the file you read before writing tests for a new crate, or
before reviewing a PR that changes test infrastructure.

## 2. The three layers

| Layer | Unit | Integration | Conformance |
|---|---|---|---|
| **What** | One `pub fn` | One crate's public API | A real game on the engine |
| **Where** | `#[cfg(test)] mod tests` in the same file | `crates/<crate>/tests/` | `games/<game>/tests/game_scenarios.rs` |
| **Speed** | < 1 ms | 10–100 ms | 100 ms – 2 s |
| **Runs** | Every `cargo test` | Every `cargo test` | Every `cargo test` |
| **Who writes** | Crate author | Crate author | Game author |
| **Blocks merge** | Yes | Yes | Yes |

### 2.1 Coverage expectations per crate

| Crate | Unit | Integration | Notes |
|---|---|---|---|
| `ember_core` | 80%+ line coverage | 1 end-to-end test | Math/time/rng — pure functions, easy to test |
| `ember_macros` | Compile-time tests (`trybuild`) | — | Test that misuse fails with the expected error |
| `ember_servers` | — | Trait-level tests via mock | No impl, only traits |
| `ember_render_macroquad` | — | Headless render tests | Requires a window; uses `#[ignore]` unless enabled |
| `ember_audio_macroquad` | — | Similar to render |  |
| `ember_input` | 80% | — | Pure logic |
| `ember_scene` | 80% | Node tree scenarios | Ordering, cycles, deferred ops |
| `ember_ui` | 80% | Layout tests | Similar to scene |
| `ember_assets` | 80% | Round-trip tests | Load → access → reload |
| `ember_editor` | 60% (POC) | Load-edit-save | Lower coverage acceptable |
| `ember` | — | End-to-end smoke | Just runs a scene |

## 3. Unit tests

### 3.1 Where they live

Every crate's source file has a `#[cfg(test)] mod tests` at the bottom:

```rust
// crates/ember_core/src/time/cooldown.rs

pub struct Cooldown { /* ... */ }

impl Cooldown { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_is_ready() {
        let c = Cooldown::ready(1.0);
        assert!(c.is_ready());
    }

    #[test]
    fn running_is_not_ready() {
        let c = Cooldown::running(1.0);
        assert!(!c.is_ready());
    }
}
```

### 3.2 What a good unit test looks like

| Rule | Reason |
|---|---|
| Tests one behavior | Failing test names the behavior that broke |
| No mocks if possible | Mocks drift; prefer real objects |
| No `thread::sleep` | Non-deterministic; use `tick(dt)` |
| No file I/O | Use fixtures or tempdirs |
| Fast | < 1 ms per test |

### 3.3 Naming

`<behavior>_<condition>` or `<should>_when_<condition>`:

- `ready_is_ready` ✅
- `running_is_not_ready` ✅
- `tick_reduces_remaining` ✅
- `test_cooldown_1` ❌ (meaningless)
- `cooldown_works` ❌ (too vague)

The test name is the test's documentation. If the name doesn't tell you
what broke, it's wrong.

## 4. Integration tests

### 4.1 Where they live

`crates/<crate>/tests/<scenario>.rs`. Each file is one scenario.
`cargo test` runs each as a separate binary.

```
crates/ember_scene/
├── src/
└── tests/
    ├── tree_ordering.rs
    ├── ready_ordering.rs
    ├── attach_cycle_rejected.rs
    └── spawn_deferred.rs
```

### 4.2 What an integration test looks like

```rust
// crates/ember_scene/tests/tree_ordering.rs

use ember_scene::prelude::*;

#[derive(Default)]
struct Recorder { order: Vec<String> }

impl Node for Recorder {
    fn name(&self) -> &str { "" }
    fn set_name(&mut self, _: String) {}
    fn process(&mut self, _dt: f32, _ctx: &mut NodeContext) {
        self.order.push("process".into());
    }
}

#[test]
fn process_is_top_down() {
    let mut tree = NodeTree::new();
    let parent = tree.insert(Recorder::default());
    let child = tree.insert(Recorder::default());
    tree.attach(child, parent).unwrap();
    tree.set_root(parent);

    // verify order
}
```

### 4.3 When to write one

Write an integration test when:

- The behavior depends on **more than one** `pub` function in the crate.
- The behavior is a **contract** with another crate.
- The behavior is hard to unit-test in isolation (e.g., tree invariants).

### 4.4 Common integration tests per crate

- **`ember_scene`:** tree ordering, ready ordering, cycle detection,
  spawn deferral, signal dispatch order.
- **`ember_assets`:** load → access → reload → drop → unload.
- **`ember_ui`:** layout engine produces expected rects, focus changes
  correctly, scroll clips content.
- **`ember_servers`:** mock server records and replays draw calls.

## 5. Conformance tests

### 5.1 What they are

`games/<game>/tests/game_scenarios.rs` — the game runs on the engine
and produces the expected scenario outcomes. These are the POC's
**integration tests**, but at the full-game level.

### 5.2 Why they matter

Because they catch the class of bug that unit tests can't:

- "The tree ordering is right, but the game's systems no longer fire
  in the right order."
- "The renderer is right, but the game's draw calls are in the wrong z
  order."
- "The signal dispatch is right, but a handler is missed because the
  target was despawned at the wrong time."

### 5.3 The pattern

```rust
#[test]
fn enemy_catapult_fires_when_player_unit_in_range() {
    // Set up game state
    let mut game = Game::new_test_level("sh_02");

    // Spawn a player unit within enemy catapult range
    game.spawn_unit_at(Team::Player, "grunt", pos(500.0, 400.0));

    // Advance fixed steps until the catapult should have fired
    for _ in 0..120 {
        game.physics_step(1.0 / 60.0);
    }

    // Assert on the outcome
    assert!(
        game.projectiles.iter().any(|p| p.team == Team::Enemy),
        "enemy catapult should fire at player unit"
    );
}
```

### 5.4 Per-game test count expectations

| Game | Scenario tests |
|---|---|
| snake | 4 |
| pong | 13 |
| simon | 8 |
| memory | 4 |
| minesweeper | 8 |
| freecell | 8 |
| breakout | 1 (POC) |
| asteroids | 10 |
| air-hockey | 10 |
| bullet-hell | 11 |
| pagoda | 0 (should add) |
| taiji | varies |
| zhuo-ji | varies |
| bore-inc | 21 |
| jade-garden | 22 + 1 balance |
| accretion | 2 balance |
| ember-wars | 20 |

Games with 0 or 1 scenario test are candidates for enrichment during
migration (Phase 7).

## 6. The CI pipeline

### 6.1 What CI runs

```yaml
# .github/workflows/ci.yml (outline)
on: [push, pull_request]

jobs:
  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace --no-fail-fast

  check-layers:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo run -p xtask -- check-layers

  macroquad-isolation:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check macroquad isolation
        run: |
          violations=$(grep -rn "use macroquad" --include='*.rs' crates/ games/ \
            | grep -v "ember_render_macroquad\|ember_audio_macroquad\|ember_stdlib" || true)
          if [ -n "$violations" ]; then
            echo "macroquad imported outside backend crates:"
            echo "$violations"
            exit 1
          fi
```

### 6.2 What CI does NOT run

- **Visual regression tests.** Requires a GPU. v0.2 feature.
- **Performance benchmarks.** No benchmark harness in v0.1.
- **Cross-platform.** v0.1 targets Linux + macOS. Windows CI is v0.2.
- **WASM builds.** Deferred.
- **Publish to crates.io.** v0.1 is a monorepo POC.

### 6.3 Fail-fast behavior

CI is not fail-fast. All jobs run, so a single run reveals all issues.
This is slower but better for PR review (one CI run per PR, not N).

## 7. Special test cases

### 7.1 `trybuild` for macros

`ember_macros` uses `trybuild` to verify compile errors:

```rust
#[test]
fn class_name_requires_derive() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/class_name_without_derive.rs");
}
```

`tests/ui/*.rs` are expected to fail compilation; `.stderr` files
contain the expected error text.

### 7.2 Headless rendering

`ember_render_macroquad` tests are `#[ignore]` by default — they need a
window. CI can run them with `xvfb`:

```yaml
- run: xvfb-run cargo test -p ember_render_macroquad -- --ignored
```

Local developers run them with `cargo test -p ember_render_macroquad -- --ignored`.

### 7.3 Timing-sensitive tests

Physics tests that depend on frame step must use deterministic helpers:

```rust
fn physics_step(game: &mut Game, fixed_dt: f32) {
    game.physics_process(fixed_dt);
}
```

Never `std::thread::sleep`. Never `Instant::now().elapsed()`.

### 7.4 Asset round-trip

`ember_assets` has a `round_trip.rs` scenario test:

```rust
#[test]
fn png_roundtrips_through_texture() {
    let assets = AssetServer::new("test_assets/").unwrap();
    let handle = assets.texture("1x1_red.png").unwrap();
    let tex = handle.get().unwrap();
    assert_eq!(tex.size(), Vec2::new(1.0, 1.0));
}
```

`test_assets/` is committed to the repo (small fixtures).

## 8. What makes a test "good" in this codebase

### 8.1 It fails loudly when the behavior breaks

A test that passes for the wrong reason is worse than no test. If you
change `Cooldown::ready(1.0)` to return a running cooldown and the test
still passes, the test is wrong.

### 8.2 It reads like a specification

A reader who knows nothing about the code can read the test and
understand the contract:

```rust
#[test]
fn ready_cooldown_does_not_fire_on_first_tick() {
    let mut c = Cooldown::ready(1.0);
    assert!(!c.tick_returning(0.0));  // ready, but not "just expired"
    c.trigger(1.0);
    assert!(!c.tick_returning(0.5));
    assert!(c.tick_returning(0.5));    // exactly at expiry
    assert!(!c.tick_returning(0.1));   // already fired
}
```

### 8.3 It's fast

A test that takes 100 ms is suspicious. A test that takes 1 s is a bug.
Unit tests should be < 1 ms; integration < 100 ms; conformance < 2 s.

### 8.4 It uses real types

No mocks except for the three server traits. Even then, prefer a real
mock implementation (a struct that records calls) over a mocking
framework.

### 8.5 It has one assertion

Where possible. A test with 5 assertions fails on assertion 3, and
you have to fix it, run again, fail on 5, etc. One assertion per test.

**Exception:** assertion clusters that check one concept:
```rust
assert!(tree.contains(a));
assert!(tree.contains(b));
assert_eq!(tree.len(), 2);
```
These are three assertions but one check ("the tree has both nodes").

## 9. Running tests locally

### 9.1 The fast loop

```bash
cargo test -p <crate>          # one crate
cargo test -p <crate> <filter>  # one test
```

### 9.2 The full check

```bash
cargo test --workspace --no-fail-fast
```

`--no-fail-fast` runs all tests even if some fail — useful for a full
workspace state, not for iterating.

### 9.3 The pre-push ritual

```bash
cargo clippy --workspace --all-targets -- -D warnings \
  && cargo test --workspace --no-fail-fast
```

If this passes locally, CI will pass.

## 10. Cross-references

- Testing philosophy → [[02-principles#6. Testing philosophy|02-principles.md §6]]
- Per-game migration → [[41-migration-guide]]
- Acceptance tests → [[42-acceptance-tests]]
- Error handling and testing → [[02-principles#5. Error handling|02-principles.md §5]]
