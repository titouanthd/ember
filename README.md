# Ember

A 2D game engine in Rust, built incrementally through 5 small games.

## Architecture

Three layers:

- **`ember-core`** — pure primitives (math, time, app state, I/O).
- **`ember-stdlib`** — reusable game bricks (transform, sprite, collider, config, text). Depends on macroquad.
- **`games/*`** — one self-contained, data-driven game per folder.

## Games

| Game | Engine bricks exercised |
|---|---|
| **Pong** | AABB collisions, simple AI, match state |
| **Breakout** | RON levels, substepping, angular bounce |
| **Snake** | Grid logic, fixed-step timer, level progression |
| **Asteroids** | Rotation, inertia, wrap-around, high score persistence |
| **Bullet Hell** | Data-driven waves, 4 emitters, graze multiplier, screen shake, hitstop |

## Principles

- **Rule of Three** — don't extract to the engine until a pattern has appeared 3 times.
- **Data-driven** — game content lives in `.ron` files.
- **Clippy gate** — `cargo clippy --workspace --all-targets -- -D warnings` must stay green.
- **Tests required** — 198 tests across the workspace.

## Build and run

```bash
cargo run -p pong
cargo run -p breakout
cargo run -p snake
cargo run -p asteroids
cargo run -p bullet-hell