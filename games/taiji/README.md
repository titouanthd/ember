# taiji

A contemplative Pong where you bend time instead of charging a shot. Hold
`Space` to become **Yin** (0.25× speed); release to become **Yang** (a 0.4 s
burst at 2×). First to 5 points wins.

**Status:** playable native prototype. 2 tests. No audio, no save, no web
build yet.

## Running

```sh
cargo run --release -p taiji
```

The game opens an 800×600 window. The canvas is letterboxed if the window is
resized (the engine's virtual resolution, ADR-041).

## Controls

| Input | Action |
|---|---|
| `Space` (hold) | Yin: slow time to 0.25× |
| `Space` (release) | Yang: 0.4 s burst at 2× |
| `W` / `S` or `↑` / `↓` | Move the left paddle |
| `Space` (menu) | Start a match |
| `Space` (game over) | Return to the menu |
| `Esc` (playing) | Pause |
| `Esc` (paused) | Resume |
| `Esc` (menu) | Quit |

## The duel

- The Qi ball starts centered at 400 px/s and bounces off the top and bottom
  walls.
- Paddle contact angle depends on where the ball hits (±60°), plus a small
  amount of "english" from paddle movement.
- The AI predicts the intercept point, including wall bounces, and tracks it
  at 500 px/s. It does **not** control time.
- A point is scored when the ball leaves the left or right edge. The ball
  recenters and serves toward the player who conceded.
- First to 5 points wins; the winner is announced as `YANG WINS` or
  `YIN WINS`.

## Design

See `ember-vault/30 Games/Taiji/Taiji — Design.md` for the time-dilation rules, the AI model, and the rejected
charge/capture mechanic from the original pitch.

## Tests

```sh
cargo test -p taiji
```

11 tests: 7 match scenarios (`tests/game_scenarios.rs`) and 4 headless
`SceneLoop` runs (`tests/scene.rs`).

## Layout

```text
games/taiji/
├── (design doc → ember-vault)
├── Cargo.toml
├── src/
│   ├── main.rs        calls scene::run()
│   ├── scene.rs       the Taiji root node: scoring, tint, drawing
│   ├── app.rs         MainMenu / Playing / Paused / GameOver
│   ├── components.rs  Paddle, QiBall, Side
│   ├── systems.rs     time dilation, physics, collisions
│   ├── ai.rs          predictive AI tracker
│   └── config.rs      GameContext + constants
└── tests/             game_scenarios.rs, scene.rs
```

## Documentation

Design notes, session logs and release notes live in the Obsidian vault: `ember-vault/30 Games/Taiji/` (hub note: `Taiji.md`).
