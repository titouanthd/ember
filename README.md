# Ember

A 2D game engine in Rust, built incrementally through 14 games (13 functional, 1 abandoned jam game).

- **Language**: Rust, edition 2024, resolver 3
- **Rendering / input / window**: macroquad 0.4
- **Math**: glam 0.27
- **Serialization**: serde + RON
- **Native config**: dotenvy (`.env`)
- **Targets**: native (macOS, Linux, Windows) + `wasm32-unknown-unknown`
- **Tests**: ~1661 green (`cargo test --workspace`)
- **Quality gate**: `cargo clippy --workspace --all-targets -- -D warnings`

## Architecture

Three layers:

- **`ember-core`** — pure primitives (math, time, app state, I/O). No graphics dependency.
- **`ember-stdlib`** — reusable game bricks (transform, sprite, collider, config, grid, input, ui, audio, text). Depends on macroquad.
- **`games/*`** — one self-contained, data-driven game per folder.

## Games

| Game | Engine bricks exercised | Type |
|---|---|---|
| **Pong** | AABB collisions, simple AI, match state | Native |
| **Breakout** | RON levels, substepping, angular bounce | Native |
| **Snake** | Grid logic, fixed-step timer, level progression | Native |
| **Asteroids** | Rotation, inertia, wrap-around, high score persistence | Native |
| **Bullet Hell** | Data-driven waves, 4 emitters, graze multiplier, screen shake, hitstop | Native |
| **Minesweeper** | Grid<T>, mouse input, UI widgets, flood fill, chord, difficulty menu, best times persistence | Native |
| **Simon** | Audio playback, circle buttons, sequence timers, best score persistence | Native |
| **Memory** | Grid<T> clicking, 2-step selection, best time persistence | Native |
| **FreeCell** | Drag-and-drop state machine, super-moves, undo/redo, DejaVu Sans Mono | Native |
| **Air Hockey** | GameMode PvP/PvAI, goalkeeper IA, hitstop guard, font_scale countdown | Native |
| **Zhuo Ji** | 108 tiles, 4-player turn-based, shanten, claim window, Dou/Ji scoring | Native |
| **ember-wars** | Catapult, auto-combat, 28-node upgrade tree, procedural textures, juice | Native |
| **jade-garden** | Match-3, cascade, Tween/Easing, CJK subset, parallax, WASM | Native + WASM |
| **bore-inc** | Incremental mining, pixel-perfect rendering, procedural sprites | Abandoned |

## Engine bricks (ember-core + ember-stdlib)

- **transform** — `Transform { position, rotation, scale }`
- **sprite** — `Sprite { color, visible }` (a tint, not an image)
- **collider** — `Shape::Aabb` / `Shape::Circle`, `collides`, `collide_contact`, swept collision
- **config** — `.env` loaders (`env_f32`, `env_i32`, `env_u32`, `env_color`)
- **grid** — `Grid<T>` with 4/8-neighbor iteration
- **input** — `Input` frame snapshot, decoupled from macroquad
- **ui** — `Button`, `CircleButton`, `Label`, `Panel`, `TimerDisplay`, `hit`
- **audio** — `AudioClip`, WAV loading and playback
- **fonts** — `load_and_set_default`, `draw_text_bold`, `measure_regular`
- **layout** — `centered_rect`
- **hash** — `hash_u32` (MurmurHash3 finalizer)
- **persistence** — `Persistence<T>` typed RON saving
- **time** — `Cooldown`, `TickTimer`
- **graphics** — `draw_centered`, `draw_centered_shadowed`, `draw_right`, `draw_left`

## Principles

- **Rule of Three** — don't extract to the engine until a pattern has appeared 3 times.
- **Data-driven** — game content lives in `.ron` files.
- **Clippy gate** — `cargo clippy --workspace --all-targets -- -D warnings` must stay green.
- **Tests required** — ~1661 tests across the workspace.
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
cargo run -p memory
cargo run -p freecell
cargo run -p air-hockey
cargo run -p zhuo-ji
cargo run -p ember-wars
cargo run -p jade-garden
```

## WASM

`jade-garden` and `bore-inc` can be compiled for the web. See `tools/build-web.sh` in each game's folder.

## See also

`version.txt` — the full project recap, phase-by-phase history, and known pitfalls.