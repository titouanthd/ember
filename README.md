# Ember

A 2D game engine in Rust, built incrementally through 7 small games.

## Architecture

Three layers:

- **`ember-core`** — pure primitives (math, time, app state, I/O). No graphics dependency.
- **`ember-stdlib`** — reusable game bricks (transform, sprite, collider, config, grid, input, ui, audio, text). Depends on macroquad.
- **`games/*`** — one self-contained, data-driven game per folder.

## Games

| Game | Engine bricks exercised |
|---|---|
| **Pong** | AABB collisions, simple AI, match state |
| **Breakout** | RON levels, substepping, angular bounce |
| **Snake** | Grid logic, fixed-step timer, level progression |
| **Asteroids** | Rotation, inertia, wrap-around, high score persistence |
| **Bullet Hell** | Data-driven waves, 4 emitters, graze multiplier, screen shake, hitstop |
| **Minesweeper** | Grid<T>, mouse input, UI widgets, flood fill, chord, difficulty menu, best times persistence |
| **Simon** | Audio playback, circle buttons, sequence timers, best score persistence |

## Engine bricks (ember-stdlib)

- **transform** — `Transform { position, rotation, scale }`
- **sprite** — `Sprite { color, visible }` (a tint, not an image)
- **collider** — `Shape::Aabb` / `Shape::Circle`, `collides`, `collide_contact`, swept collision
- **config** — `.env` loaders (`env_f32`, `env_i32`, `env_u32`, `env_color`)
- **grid** — `Grid<T>` with 4/8-neighbor iteration
- **input** — `Input` frame snapshot, decoupled from macroquad
- **ui** — `Button`, `CircleButton`, `Label`, `Panel`, `TimerDisplay`
- **audio** — `AudioClip`, WAV loading and playback
- **graphics** — `draw_centered`, `draw_centered_shadowed`

## Principles

- **Rule of Three** — don't extract to the engine until a pattern has appeared 3 times.
- **Data-driven** — game content lives in `.ron` files.
- **Clippy gate** — `cargo clippy --workspace --all-targets -- -D warnings` must stay green.
- **Tests required** — 309 tests across the workspace.
- **State structs** — each game owns a single struct holding all mutable state; `main.rs` holds one variable.

## Build and run

```bash
cargo run -p pong
cargo run -p breakout
cargo run -p snake
cargo run -p asteroids
cargo run -p bullet-hell
cargo run -p minesweeper
cargo run -p simon