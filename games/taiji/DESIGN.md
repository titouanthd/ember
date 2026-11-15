# TÀIJÍ (太极) — Design Document

> A contemplative Pong where the player bends time instead of charging a shot:
> hold to become Yin (slow), release to become Yang (fast).

Last updated: Session 17.9 (implementation reconciled with the code).
Status: **implemented prototype** (native only). 2 tests. No audio, no
persistence, no web build.
Engine: **Ember**.

---

## 1. Pitch

**TÀIJÍ** is a meditative reimagining of Pong. Set in the philosophy of Yin and
Yang, it strips away the frantic reflex game and replaces it with breath,
timing, and tempo control.

You play the **Yang** paddle (left), facing the **Yin** AI Guardian (right).
The ball is a pearl of **Qi**. You do not charge a shot; you **bend the flow of
time**:

- **Hold Space → Yin.** Time slows to 0.25×. The screen tints blue, the ball
  crawls, and you get room to reposition.
- **Release Space → Yang.** A 0.4 s burst at 2× speed. The ball snaps forward
  and the screen flashes gold.
- **Neither → normal.** 1× speed.

The objective is not to overwhelm the opponent with speed, but to find the
moment between stillness and motion.

### What changed from the original concept

The first draft (Session 15) described a **Charge & Release** mechanic: hold
to capture the ball, release to launch it with charge-proportional speed. That
prototype was **not implemented**. The shipped mechanic is **time dilation**:
the ball is never captured, and holding Space affects the whole simulation.
The old charge/capture sections are kept only as rejected design history; see
§9.

---

## 2. Inspirations

- **Pong** (Atari, 1972) — the foundational duel of two paddles and a ball.
- **Monument Valley** (ustwo, 2014) — contemplative pacing, minimal aesthetic.
- **Journey** (thatgamecompany, 2012) — environmental, non-verbal storytelling.
- **Tai Chi / Qi Gong** — absorbing force and redirecting energy.
- **Chinese ink-wash painting (Shui-mo)** — high contrast, minimal visual
  language (still a future direction; the prototype is flat white/black).

---

## 3. Core mechanic: The Breath of Time

### 3.1 Yin (slow)

- **Input:** hold `Space`.
- **Effect:** `time_scale` eases toward **0.25×** at 15 units/s.
- **Visual:** pale blue full-screen tint whose alpha scales with the slowdown,
  a small "YIN" label, and the ball/paddle colors shift toward blue.
- **Purpose:** defensive recovery, precise repositioning, reading the AI.

### 3.2 Yang (burst)

- **Input:** release `Space` after holding it.
- **Effect:** `yang_burst_timer = 0.4 s` (real time). While it lasts,
  `time_scale` eases toward **2×**. Re-entering Yin cancels the burst.
- **Visual:** gold tint/flash on the ball and paddles.
- **Purpose:** a short offensive window; the ball crosses the court fast, but
  the player also has less reaction time.

### 3.3 Time-scale rules

```text
target = 0.25 if Space held
       = 2.0  if Yang burst active
       = 1.0  otherwise

time_scale += (target - time_scale) * 15.0 * real_dt
dt = real_dt * time_scale
```

- `real_dt` drives input, the Yang timer, UI, and the time-scale easing.
- `dt` drives paddle movement, AI movement, ball movement, and collisions.
- The easing prevents a hard snap and makes the transition feel like breathing.

### 3.4 The AI Guardian

`ai.rs` implements a **predictive tracker**, not the same time mechanic:

- It predicts the ball's intercept Y with wall bounces (`rem_euclid` folding).
- It moves toward that target with a maximum speed (500 px/s) and a small
  dead zone.
- It does **not** slow or speed time; only the player controls the clock.
- Difficulty comes from the AI's reaction speed and the player's time budget.

This is a deliberate divergence from the "AI uses the exact same mechanic"
idea in the original pitch: giving the AI time control made the duel
unreadable in the prototype.

---

## 4. Gameplay loop

1. **Main menu:** `Space` starts, `Esc` quits.
2. **Round start:** the Qi ball spawns centered and drifts toward one side
   (`BASE_SPEED = 400`).
3. **Duel:** move the paddle with `W`/`S` or `↑`/`↓`; hold/release `Space` to
   bend time.
4. **Walls:** top/bottom bounce; paddles reflect with an angle based on the
   contact offset (`±60°`) plus a small "english" from paddle velocity.
5. **Scoring:** a point when the ball passes the left or right edge.
6. **Round reset:** ball recentered, direction toward the scorer's opponent,
   time scale reset to 1×.
7. **Match:** first to **5** points (`GameContext::win_score`). The winner is
   shown as `YANG WINS` or `YIN WINS`.
8. **Game over:** `Space` returns to the menu.

`Esc` pauses during play and resumes from pause. There is no quit-to-menu from
pause; `Esc` on the main menu exits the process.

---

## 5. Visual direction (as implemented)

- **Background:** pure white (`Color::WHITE`). Ink-wash art is not implemented.
- **Virtual canvas:** 800×600, rendered to a `render_target` and letterboxed
  into the window by `render.rs`.
- **Paddles:** rounded rectangles.
  - Player (Yang): black by default, blue while Yin, gold while Yang.
  - AI (Yin): dark gray, same tint logic.
- **Ball:** a glowing circle; color shifts blue (Yin) / gold (Yang) / black
  (neutral).
- **UI:** score `player | ai` centered at the top, plus the state screens.
- **Juice:** screen tint, a "YIN" label, and color shifts. No particles,
  shockwaves, floating text, or screen shake.

## 6. Audio direction

**Not implemented.** The original plan (ambient drone, absorb whoosh, charge
hum, release gong, score chime) remains aspirational. There is no `audio.rs`,
no `assets/`, and no `WebStart` screen.

---

## 7. Architecture

Built on `ember-workspace`:

- **Rendering:** `macroquad`, virtual 800×600 canvas + letterbox.
- **Core:** `ember-core` (`Rng` seeded at 42, `Aabb`, `Transform`).
- **Stdlib:** `ember-stdlib` (`Input`, `Transform`, `graphics::text`).
- **No `wasm-bindgen`, no `Instant::now()`** — the code is wasm-compatible in
  principle, but no web shell or build script exists yet.

### 7.1 Actual module structure

```text
games/taiji/
├── Cargo.toml
├── DESIGN.md
├── README.md
└── src/
    ├── main.rs        loop, render target, scoring, tint
    ├── lib.rs         exports App/AppState, modules
    ├── app.rs         AppState: MainMenu / Playing / Paused / GameOver
    ├── components.rs  Paddle, QiBall, Side
    ├── systems.rs     time dilation, movement, collisions, scoring
    ├── ai.rs          predictive AI tracker
    ├── config.rs      GameContext (800×600, win_score 5)
    └── render.rs      letterbox math + virtual/window mapping
```

There is **no** `audio.rs`, `progress.rs`, `assets/`, `web/`, or `tools/`.
The planned `README.md` now exists.

### 7.2 Constants

| Constant | Value |
|---|---|
| Window / virtual canvas | 800×600, resizable, letterboxed |
| Paddle | 20×120, player speed 600, AI speed 500 |
| Ball radius | 12 |
| Base speed / max speed | 400 / 1200 |
| Yin scale | 0.25× |
| Yang scale / duration | 2.0× / 0.4 s real time |
| Easing | 15.0 units/s |
| Win score | 5 |

---

## 8. Tests

**Actual: 2 tests**, both in `render.rs` (letterbox aspect ratio and
centering). There are no integration tests and no tests for the time mechanic,
scoring, AI, or collisions.

The two tests cover only the letterbox math. The following are **not**
tested: AI prediction, wall bounces, paddle collision angles, time dilation,
scoring/reset, state transitions, rendering.

Clippy reports:
- missing `Default for App` (`app.rs`);
- a collapsible `if` in `main.rs`.

There is no `Default` impl, no audio, and no persistence to test.

---

## 9. Rejected design history (Charge & Release)

Kept for context; **do not implement without a new design pass**.

- Hold to **capture** the ball into an orbit around the paddle.
- Hold too long → instability → random escape.
- Release → launch with speed/size proportional to charge.
- AI uses the same capture/release mechanic and telegraphs it.

Why it was dropped:
- Capture removed the ball from play, which killed the Pong rhythm.
- Charge-proportional speed made the duel about waiting, not reading.
- An AI with the same mechanic was opaque: the player could not tell whether
  the AI was charging, holding, or about to release.
- Time dilation delivered the same Yin/Yang fantasy with one input and no
  hidden state.

---

## 10. Known pitfalls

- **Scale `dt` exactly once.** `real_dt` for input/UI/timers; `dt =
  real_dt * time_scale` for physics. Never multiply twice.
- **The Yang timer is real time.** Tick `yang_burst_timer` with `real_dt`,
  otherwise slow motion stretches the 0.4 s burst.
- **Re-entering Yin cancels Yang.** Set `yang_burst_timer = 0.0` on hold.
- **`was_yin` edge detection.** The burst triggers only on the release edge,
  not every frame the key is up.
- **Clamp `dt`.** `get_frame_time().min(1.0 / 60.0)` prevents tunneling after
  a stall.
- **Letterbox mapping.** Use `render::window_to_virtual`; clicks in the bars
  must be ignored (currently only keyboard is used, but future mouse input
  must respect it).

---

## 11. Current state and next steps

**Implemented:** playable native prototype with the time-dilation duel, AI,
scoring to 5, pause, game-over, letterboxed 800×600 rendering, and 2 tests.

**Not implemented:** audio, persistence/high scores, web/WASM build, particles
or juice beyond tint, ink-wash art, gamepad/mouse input, difficulty settings,
tests for AI/collisions.

**Next steps:**
1. Add `Default for App` and fix the collapsible `if` so clippy is clean.
2. Add tests for AI prediction, wall bounces, paddle angle, and the Yang timer.
3. Decide whether to keep the prototype native-only or port it to WASM
   (the code is already wasm-safe; it needs `web/index.html` +
   `tools/build-web.sh`).
4. If ported, add a `WebStart` screen and procedural or preloaded SFX.
5. Revisit the visual direction: ink-wash background, brush-stroke paddles.
6. Consider a best-of-N match structure and a high-score save.

---

*The game is about the pause before the strike. The ball is not the subject;
the breath is.*
