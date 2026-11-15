# PAGODA (The Stacker) — Design Document

> A hyper-casual, one-button precision game where you build a tower of geometric blocks as high as possible.

Last updated: Session 17.9 (click-only mute & mobile/touch pass).
Status: **Milestone 5 complete**. Core loop, juice, menus, persisted best score, a verified WASM bundle, a bundled DejaVu Sans Mono font, a procedural SFX bank, a click/tap-only sound toggle, a touch-friendly web shell, and a full Chinese-inspired visual pass are in place, tested (112 tests), and clippy-clean. Music remains optional.

---

## 1. Pitch

**PAGODA** is a game of rhythm, precision, and escalation. A block slides horizontally across the top of the screen. With a single tap, you drop it onto the tower below. Any part of the block that hangs over the edge is sliced off, making the next block smaller. 

The goal is simple: build the highest tower possible. But as the tower grows, the camera pans up, the blocks move faster, and the margin for error shrinks to a few pixels. It is an addictive, "one more try" arcade experience.

---

## 2. Inspirations

- **Stack** (Ketchapp, 2016) — The definitive one-button tower builder.
- **Tower Bloxx** (Digital Chocolate, 2005) — The original swinging block stacking game.
- **Monument Valley** (ustwo, 2014) — Clean, isometric-like geometric aesthetic.
- **Tetris** — The satisfaction of perfectly fitting a piece into place.

---

## 3. Core Loop

1. **Spawn:** A new block appears at the top of the screen, sliding horizontally (or alternating direction each time).
2. **Drop:** The player presses a single button (Space, Left Click, or Touch). The block stops moving and falls instantly.
3. **Slice:** The block lands on the previous block. 
   - If the landing is **off-center**, the overhanging part is cut off. The next block will have the size of the intersecting area. The sliced part falls off the screen.
   - If the landing is **perfect** (within a few pixels of the center), the player gets a "Perfect" bonus, a visual reward, and the block width might even grow slightly (up to a maximum).
4. **Escalate:** The tower grows, the camera pans up, the horizontal sliding speed increases, and the blocks change color based on height.
5. **Game Over:** If the block misses entirely (zero intersection), the tower collapses, and the game ends. 

---

## 4. Mechanics

### 4.1 The Sliding Block
- The block moves back and forth on the X-axis.
- The speed increases slightly with each successful drop.
- The direction alternates (Left → Right → Left).
- The block's Y position is fixed relative to the camera (it sits at the top of the current view). When a block drops, the camera smoothly pans up to follow the new top of the tower.

### 4.2 The Intersection (Slicing)
- We use standard AABB collision (`ember_core::math::Aabb`) to calculate the overlap between the falling block and the top block of the tower.
- `overlap_min_x = max(falling.min.x, tower.min.x)`
- `overlap_max_x = min(falling.max.x, tower.max.x)`
- **Width**: `overlap_max_x - overlap_min_x`.
- If `width <= 0.0`, the player misses completely → Game Over.
- The new block's X position and width are set to this overlap.

### 4.3 "Perfect" Drops
- A drop is perfect when the distance between the centers of the falling
  block and the top block is inside a **speed-scaled window**:
  `max(speed / 60, 4 px)`. At 500 px/s that is ~8.3 px; at the 800 px/s
  cap it is ~13.3 px. The old fixed 4 px window was narrower than one
  frame of travel, which made many perfects a matter of frame luck
  rather than timing.
- **Live guide:** while the block slides, a plumb line follows its
  center and a gold target marker sits on the center of the tier below.
  Both turn gold (and grow short landing marks) as soon as the block is
  inside the perfect window, so the player can aim instead of guessing.
- **Effect:** the block keeps its full width (or grows by 2 px, capped at
  the base width). A combo counter increases, a particle burst and
  screen flash fire, and the newest tile carries a short gold landing
  outline (`land_pulse`). A perfect camera punch adds a small zoom.
- **HUD:** the combo reads `完美` (`PERFECT` when the CJK font is
  unavailable) and appends `xN` above one.

---

## 5. Visual Direction

The tower is a lacquered Chinese pagoda at night, not a neon arcade
stack. Everything stays flat-shaded and procedural so the web bundle
needs no texture atlas.

- **Background:** An ink-wash night sky. A 32-band gradient is tinted by
  the current tier colour, then layered with 52 deterministic stars, a
  warm paper moon with a smooth halo and drifting ink bands, three soft
  mist bands, and two jagged mountain ridges (blue ink behind,
  near-black in front) that drift down slowly as the tower climbs.
- **Blocks:** Lacquered timber tiles: a lit top bevel, a shadowed base,
  a thin gold seam and a small carved diamond boss on wide tiles. The
  palette advances through Chinese pigments — celadon 青瓷, turquoise
  碧, indigo 靛, cinnabar 朱砂 and gold 金 — over roughly 33 blocks
  instead of the old cyan→magenta rainbow.
- **Drop guide:** A plumb line and target diamond make alignment
  readable. The line, target and tile rim turn gold inside the perfect
  window. Any landing leaves a brief gold outline on the newest tile.
- **Camera:** Smooth vertical follow, with a short zoom punch on perfect
  drops.
- **Juice:**
  - **Slice:** Particles in the sliced tile's colour fall away.
  - **Land:** A small screen shake and the gold landing outline.
  - **Perfect:** A golden shockwave, floating `完美`/`PERFECT` text and
    an escalating pentatonic chime.
- **UI:** A lacquer signboard (匾额) with a red seal `塔` frames the
  menu; game over uses an ink panel and the seal `终`. Gold dividers,
  diamond corner studs and a small top-right seal carry the motif into
  gameplay. The HUD deliberately breathes: score baseline at `y=82`
  (62 px), a ruled divider at `y=100`, `BEST` at `y=140` (18 px), and
  the combo at `y=180` (24 px).
- **Fonts:** DejaVu Sans Mono remains the default and covers all Latin
  UI text. A 9 KB Noto Sans SC subset supplies the decorative glyphs
  `宝塔`, `完美`, `终` and `新纪录` (plus ASCII for mixed strings such as
  `完美 x3`). If it fails to load, Chinese accents are skipped and the
  English labels take over; gameplay is unaffected.

---

## 6. Audio Direction

All sounds are procedurally generated (no licensed audio) by
`tools/generate-sounds.py` and committed as **44.1 kHz mono 16-bit**
WAVs in `assets/sfx/`. `src/audio.rs` wraps them in a `SoundBank`;
missing files degrade to silence rather than breaking the game.

**Format is load-bearing:** quad-snd (below macroquad) resamples any
non-44.1 kHz clip with a nearest-neighbour filter, which crackles
audibly. A Rust test (`every_sfx_clip_uses_the_native_mix_format`)
fails if a future regeneration changes the rate, channel count, or bit
depth.

Every clip is high-passed at 25 Hz, low-passed between 4 and 8 kHz to
remove fizz, edge-faded, and normalized to -5…-8 dBFS so overlapping
voices never clip the mixer.

| File | Trigger | Design |
|------|---------|--------|
| `drop.wav` | Every landing | Short warm wooden tok (220 Hz, 180 ms) with a soft low-passed contact tick |
| `slice.wav` | Normal drop with overhang | Short soft "shk" through cascaded band-passes around 1.8 kHz; volume scales with the width of the cut, so near-perfect shaves stay quiet |
| `perfect_1..6.wav` | Perfect drop | Pentatonic bell ladder (C5 D5 E5 G5 A5 C6); the combo selects the note, so consecutive perfects audibly climb |
| `game_over.wav` | Miss | Two-note descending cadence (G3 → D3) with a filtered low rumble |
| `start.wav` | Menu start / restart | Quiet rising fifth |
| `new_best.wav` | Run beats the record | Bright arpeggio with 350 ms of leading silence, so it lands after the game-over thud |

- **Mix:** `audio.rs` holds per-event volumes; the perfect chime keeps
  a soft landing thud underneath so the block still feels physical.
- **Mute:** the top-left speaker icon is the only mute control; there
  is no keyboard shortcut. The icon shows a cinnabar slash and a
  persistent `SOUND OFF` label while muted, so touch users get feedback
  without hovering. Its invisible hit area is 96 virtual px (~47 CSS px
  on a 390 px wide phone), and mouse/touch taps on it are consumed, so
  they never start, drop or restart by accident. Muting stops any clip
  that is still ringing.
- **Future music:** a rhythmic synthwave/lo-fi loop whose tempo rises
  with the tower. Not implemented; the SFX pass comes first.
- **Original SFX brainstorm:** *Drop:* low thud. *Slice:* crisp swoosh.
  *Perfect:* rising chime (now realised as the note ladder).
  *Game Over:* descending gong/bass drop.

---

## 7. Architecture & Technical Constraints

This game is built using the `ember-workspace` foundation. It will be a native + WASM game.

- **Rendering:** `macroquad` 
- **Core Logic:** `ember-core` (Aabb, Rng, Cooldown)
- **Standard Library:** `ember-stdlib` (Input, UI, Persistence, Audio)

### 7.1 WASM / Web Constraints
- **No `wasm-bindgen`.**
- **No `Instant::now()`.** Use an `f32` accumulated from `dt`.
- **Async Preloading:** Audio and Fonts must be `.await`ed before the loop.
- **Persistence:** Web build will use an in-memory `thread_local` fallback for the high score (no `localStorage` bridge yet, unless we implement the miniquad plugin).
- **Keyboard focus:** macroquad attaches `keydown` to the canvas rather than the document. `web/index.html` focuses the canvas on load and pointer-down so `Space` and `Enter` work consistently. Mute is click/tap-only by design.
- **Mobile:** the canvas uses `100dvh` and `viewport-fit=cover`; text selection, tap highlight, callouts, overscroll, pinch zoom and the context menu are suppressed. A touch start maps to the same one-button action as a click, and a portrait hint suggests rotating without blocking play.
- **Build:** Uses the game's `tools/build-web.sh` (same template as jade-garden / accretion) for itch.io deployment.

### 7.2 Module Structure
```text
games/pagoda/
├── Cargo.toml
├── DESIGN.md
├── README.md
├── assets/
│   ├── DejaVuSansMono.ttf      (bundled UI font)
│   ├── NotoSansSC-Pagoda.otf   (9 KB CJK subset: 宝塔完美终新纪录)
│   └── sfx/                    (generated WAV sound effects)
├── web/ (index.html)
├── tools/
│   ├── build-web.sh
│   ├── generate-sounds.py      (regenerates `assets/sfx/`)
│   └── subset-font.sh          (regenerates the CJK subset)
└── src/
    ├── main.rs        (App loop, camera, scene/tile/guide rendering)
    ├── app.rs         (State Machine: Menu, Playing, GameOver + HUD)
    ├── art.rs         (Lacquered tiles, drop guide, colour helpers)
    ├── scene.rs       (Procedural ink-wash night backdrop)
    ├── ui.rs          (Signboard, seals, panels, gold dividers)
    ├── audio.rs       (SoundBank + GameEvent → sound mapping)
    ├── components.rs  (Block, Tower, Camera)
    ├── controls.rs    (Space / Enter / click action mapping)
    ├── fonts.rs       (DejaVu default + Noto Sans SC subset loader)
    ├── paths.rs       (Native disk / web relative asset paths)
    ├── systems.rs     (Physics, slicing, perfect window, GameEvent)
    ├── persistence.rs (Best score handle: `best_score.ron`)
    ├── config.rs      (GameContext, Chinese pigment palette)
    ├── render.rs      (letterbox math, virtual→window mapping)
    └── juice.rs       (Local particle system for the slices)
```

`render.rs` owns the 800×600 virtual canvas: `letterbox_rect` and
`window_to_virtual` (the latter returns `None` inside the bars). It is kept
separate from `main.rs` so the coordinate math is unit-tested without a
macroquad context.

A committed `dist/web/` bundle (wasm + `mq_js_bundle.js` + assets) is part of
the repository; rebuild it with `tools/build-web.sh` after any gameplay or
asset change.

---

## 8. Scope & Milestones

### Milestone 1: The Core Mechanic (Native)
- [x] Setup the game loop with a static camera.
- [x] Spawn a block that moves left/right.
- [x] Allow dropping the block (Space).
- [x] Implement AABB slicing (drop the block, cut the width).
- [x] Game Over if width <= 0.
- [x] Placeholder visuals (white boxes).

### Milestone 2: The Camera & Juice
- [x] Implement the smooth camera pan up.
- [x] Add the local `juice.rs` (particles falling when sliced, screen shake).
- [x] Add color gradient based on height.
- [x] Perfect drops: snapped placement, width growth, combo tracking.
- [x] Perfect feedback: gold burst, shockwave, floating text, flash, camera punch.
- [x] HUD shows the score and the current perfect combo.

### Milestone 3: Persistence & Menus
- [x] Main Menu (Press Space to Start).
- [x] Game Over Screen (Show Score + High Score).
- [x] Save High Score using `ember-stdlib::persistence` (native) / memory (web).
- [x] Show the loaded best score on the menu and HUD.
- [x] "NEW BEST!" callout when a run beats the record.

### Milestone 4: Web Deployment
- [x] Configure `Cargo.toml` for `wasm32`.
- [x] Add the `web/index.html` shell.
- [x] Run `tools/build-web.sh` and test locally.
- [x] Add generated sound effects (land, slice, perfect ladder, game over, start, new best).
- [ ] (Optional) Add music.

### Milestone 5: Visual Identity & Perfect-Drop Fairness
- [x] Replace the fixed 4 px perfect test with a speed-scaled window
      (one frame of travel, minimum 4 px).
- [x] Add a live plumb-line/target alignment guide that turns gold in
      the perfect window.
- [x] Add a short gold landing outline on the newest tile.
- [x] Render blocks as lacquered tiles with bevels, gold seams and
      carved diamond bosses.
- [x] Replace the synthwave background with a procedural Chinese night:
      paper moon, stars, ink mist, layered mountains and slow parallax.
- [x] Restyle the UI with a lacquer signboard, red seals, ink panels and
      gold dividers; add vertical breathing room to the HUD.
- [x] Add the Noto Sans SC subset for `宝塔`/`完美`/`终`/`新纪录`, with
      an ASCII-only fallback and a regeneration script.
- [x] Add a clickable speaker toggle with a cinnabar muted slash;
      icon clicks are consumed and no longer start/drop/restart.
- [x] Remove the keyboard mute shortcut; the speaker is click/tap-only
      and keeps a persistent `SOUND OFF` label plus a 96 px touch target.
- [x] Make the web shell mobile-friendly (`100dvh`, safe areas, no
      overscroll/zoom/selection/context menu) and add a non-blocking
      portrait rotate hint.
- [x] Focus the web canvas on load and pointer-down for reliable
      keyboard input; muting also stops any ringing clip.
- [x] Rebuild and verify the WASM bundle; 112 tests, clippy and fmt clean.

---

## 9. Post-Release / Future Ideas

- **Modes:** 
  - *Classic:* Endless.
  - *Time Attack:* Build as high as possible in 60 seconds.
- **Customization:** Unlock different block styles (wood, glass, neon) with the points earned.
- **Advanced Juice:** 3D isometric rendering for a more modern look.
- **Web Save:** Implement a custom miniquad plugin to access `localStorage`.