# Ember

A 2D game engine in Rust, built incrementally through 17 games (16 playable, 1 abandoned jam project).

- **Language**: Rust, edition 2024, resolver 3
- **Rendering / input / window**: macroquad 0.4
- **Math**: glam 0.27
- **Serialization**: serde + RON
- **Native config**: dotenvy (`.env`)
- **Targets**: native (macOS, Linux, Windows) + `wasm32-unknown-unknown`
- **Tests**: 1941 executed — **1909 passing, 32 failing** (31 in `ember-wars`, 1 balance test in `jade-garden`)
- **Quality gate**: `cargo clippy --workspace --all-targets -- -D warnings` — currently **not green**: 2 lints in `accretion`, 1 in `taiji`

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
| **Zhuo Ji** | 108 tiles, 4-player turn-based, shanten, claim window, Dou/Ji scoring, custom CJK tiles | Native |
| **ember-wars** | Catapult, auto-combat, 28-node upgrade tree, procedural textures, juice | Native |
| **jade-garden** | Match-3, cascade, Tween/Easing, CJK subset, parallax, WASM | Native + WASM |
| **bore-inc** | Incremental mining, pixel-perfect rendering, procedural sprites | Abandoned |
| **Accretion** | Incremental production chain, 6 paradigm shifts, prestige, localStorage bridge, WASM | Native + WASM |
| **Taiji** | Time-dilation Pong (Yin slow-mo / Yang burst), letterboxed virtual canvas | Native (prototype) |
| **Pagoda** | One-button stacker, speed-scaled perfect window, procedural ink-wash art, SFX bank, mobile web shell | Native + WASM |

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
- **hash** — `hash_u32`, `hash_u32_2`, `hash_u32_3` (MurmurHash3 finalizer)
- **math** — `reflect(v, n)` for bounce/reflection
- **persistence** — `Persistence<T>` typed RON saving
- **time** — `Cooldown` (`new` = running, `with_duration` = ready), `TickTimer`
- **graphics** — `draw_centered`, `draw_centered_shadowed`, `draw_right`, `draw_left`

## Principles

- **Rule of Three** — don't extract to the engine until a pattern has appeared 3 times.
- **Data-driven** — game content lives in `.ron` files where practical (`accretion` currently hardcodes its tables).
- **Clippy gate** — `cargo clippy --workspace --all-targets -- -D warnings` must return to green.
- **Tests required** — 1941 tests across the workspace, currently 1909 green.
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
cargo run -p bore-inc      # abandoned prototype
cargo run -p accretion
cargo run -p taiji         # playable prototype
cargo run -p pagoda
```

## WASM

`jade-garden`, `bore-inc`, `accretion`, and `pagoda` can be compiled for the web. See `BROWSER_BUILD.md` and each game's `tools/build-web.sh`.

- `jade-garden` — shipped; progress in memory only.
- `bore-inc` — abandoned; kept as a WASM reference.
- `accretion` — localStorage save bridge implemented.
- `pagoda` — committed `dist/web` bundle, mobile-friendly shell.

## See also

`version.txt` — the full project recap, phase-by-phase history, and known pitfalls.
