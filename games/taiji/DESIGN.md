# TÀIJÍ (太极) — Design Document

> A contemplative Pong where players absorb and release energy (Qi) in a minimalist, Yin-Yang-inspired duel.

Last updated: Session 15 (Initial Concept & Mechanic Pitch).
Status: **Pre-production**. Core mechanic defined. Background is currently a placeholder (white) pending final art direction.

---

## 1. Pitch

**TÀIJÍ** is a meditative reimagining of Pong. Set in the philosophy of Yin and Yang, the game strips away the frantic, reflex-based nature of the classic arcade game and replaces it with a rhythm of breathing, timing, and energy management.

You play as the **Yang** (the active, the light), facing the **Yin** (the passive, the shadow), controlled by an AI Guardian. The ball is not a solid object, but a pearl of **Qi** (energy). 
Instead of simply bouncing the ball back, you must **absorb (Yin)** and **project (Yang)**. You hold the button to attract the Qi to your paddle, charging it, and release it to send it flying. 

The objective is not to overwhelm your opponent with speed, but to find perfect balance. 

---

## 2. Inspirations

- **Pong** (Atari, 1972) — The foundational duel of two paddles and a ball.
- **Monument Valley** (ustwo, 2014) — Contemplative pacing and minimalist aesthetic.
- **Journey** (thatgamecompany, 2012) — Environmental, non-verbal storytelling and emotional resonance.
- **Tai Chi / Qi Gong** — The actual martial arts philosophy of absorbing force and redirecting energy.
- **Chinese Ink Wash Painting (Shui-mo)** — High contrast, minimalist visual language (planned for later iterations).

---

## 3. Core Concept: The Breath of Qi

The defining mechanic is the **Charge & Release** system, replacing the traditional "velocity transfer" (English) of classic Pong.

### 3.1 Yin (Absorption)
- **Action:** The player holds a button (Space, or Right/Left Click).
- **Visual:** The paddle turns dark blue/black, and a swirling vortex of particles begins pulling towards its center.
- **Effect:** If the Qi ball touches the paddle while it is absorbing, it is **captured**. It orbits the paddle, held in stasis. The player can hold it for a few seconds.
- **Risk:** If the player holds the Qi for too long, it becomes unstable (turns red, vibrates violently) and automatically escapes in a random direction.

### 3.2 Yang (Projection)
- **Action:** The player releases the button while holding the Qi.
- **Visual:** The paddle flashes gold. A massive shockwave and radial burst of particles erupt.
- **Effect:** The Qi ball is launched. The speed and size of the ball are proportional to how long the player held the charge.
- **Strategy:** You can charge for a soft, slow lob to disrupt the AI's timing, or charge fully for a fast, aggressive strike.

### 3.3 The AI Guardian
The AI opponent uses the exact same mechanic.
- It must "breathe" to play.
- It telegraphs its intentions: if its paddle turns black, it is about to absorb. If it flashes gold, it is about to strike.
- The player can bait the AI by holding a slow, uncharged orb, or overwhelm it with a heavily charged fast orb.

---

## 4. Gameplay Loop

1. **Start of Round:** The Qi ball spawns in the center, drifting slowly towards one side.
2. **The Duel:** Players manipulate their paddles vertically. 
3. **Capture:** When the ball approaches, the player decides: absorb it (hold) or just let it bounce off the paddle (basic parry).
4. **Charge:** The player holds the ball, building up a visual and auditory hum. The AI moves to predict the trajectory.
5. **Release:** The player launches the ball. The opponent must now absorb or parry.
6. **Scoring:** A point is scored when the Qi passes the edge of the screen. The first to 5 points wins.
7. **End of Match:** The game returns to the main menu, saving the player's high score (longest rally, or matches won).

---

## 5. Visual Direction (MVP)

- **Background:** **Pure white** (Placeholder). The final iteration will feature a minimalist ink-wash aesthetic, but gameplay must be readable on a blank canvas first.
- **Paddles:** Floating circles (or pill shapes) representing the Yin/Yang duality. 
  - Player (Yang): White with a gold outline.
  - AI (Yin): Black with a silver outline.
- **The Qi Ball:** A glowing orb. Its color shifts from calm white (neutral) to dark blue (Yin/Absorbing) to bright gold/red (Yang/Charged).
- **UI:** Minimalist. No flashy numbers. Two minimal bars at the top representing the balance of the match.
- **Juice:** Shockwaves (on release), particle vortexes (on absorb), screen shake (on heavy impacts), floating text (for feedback like "Perfect Parry").

---

## 6. Audio Direction (MVP)

- **Music:** Ambient, low-tempo. Drone synthesizers or a single sustained guqin (Chinese zither) note.
- **SFX:**
  - *Absorb:* A rising, reversed whoosh sound.
  - *Holding Charge:* A low, pulsing hum that increases in pitch as the charge grows.
  - *Release:* A deep gong or a bass drop.
  - *Scoring:* A soft bell or chime.

---

## 7. Architecture & Technical Constraints

This game will be built using the `ember-workspace` foundation.
- **Rendering:** `macroquad` (native and `wasm32-unknown-unknown`).
- **Core Logic:** `ember-core` (Math, RNG, Aabb, Cooldown).
- **Standard Library:** `ember-stdlib` (UI, Audio, Input, Transform).

### 7.1 WASM / Web Constraints (Crucial)
- **No `wasm-bindgen`.** Use `miniquad`'s native JS imports.
- **No `Instant::now()`.** Use an `f32` accumulated from `delta_time`.
- **Async Preloading:** All audio and fonts must be `.await`ed before the game loop starts.
- **Persistence:** Web build will use an in-memory `thread_local` fallback for high scores (no `localStorage` bridge yet).
- **Build:** Will use the workspace `tools/build-web.sh` script for itch.io deployment.

### 7.2 Module Structure (Planned)
```text
games/taiji/
├── Cargo.toml
├── DESIGN.md
├── README.md
├── assets/
├── web/ (index.html)
├── tools/ (build-web.sh)
└── src/
    ├── main.rs        (App loop, states)
    ├── app.rs         (State Machine: Menu, Playing, Paused, GameOver)
    ├── components.rs  (Paddle, QiBall, ChargeState)
    ├── systems.rs     (Physics, Capture, Release, AI logic)
    ├── config.rs      (GameContext, palette)
    ├── render.rs      (Drawing the board, UI)
    ├── audio.rs       (Audio wrapper)
    └── progress.rs    (High score persistence)