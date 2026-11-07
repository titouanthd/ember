# Accretion — Design Document

> Inspired by **Universal Paperclips** (Frank Lantz, 2017) for its
> paradigm shifts and cosmic arc, by **Factorio** (Wube, 2020) for its
> visible production chain and ratios, by **Cookie Clicker** (Orteil,
> 2013) and **Antimatter Dimensions** (Hevipelle, 2016) for the
> exponential incremental loop, by **Dyson Sphere Program** (Youthcat,
> 2021) for cosmic scale and megastructures, by **Kittens Game**
> (bloodrizer, 2014) for interlocking resource chains and prestige, by
> **A Dark Room** (Doublespeak, 2013) for abrupt narrative transitions,
> and by **Outer Wilds** (Mobius, 2019) for its contemplative tone and
> orbital physics as scenery.
>
> A gravitational incremental where the player is an emergent
> consciousness in a protoplanetary disk, and where growth is not a
> choice — it is a fall.

Last updated: Session 12 (first complete design doc).
Status: **in design**. Target: **browser (WASM) + optional native**.
Engine: **Ember**.

---

## 1. Pitch

**Accretion** is a cosmic incremental game about gravity.

The player is a **nascent consciousness** formed in the disk of dust
and gas surrounding a newborn star. This consciousness has no body of
its own — it *is* a local disturbance in the disk. To grow, it must
**accrete**: pull matter toward itself, collapse it into increasingly
massive structures, and eventually become a star.

**Key mechanics:**

- **Passive accretion**: the more mass the consciousness has, the
  stronger its gravitational field, the more matter it attracts. The
  loop is *structurally* exponential: mass → gravity → mass.
- **Paradigm shifts**: six major stages (Dust → Pebbles →
  Planetesimals → Planets → Gas Giant → Star) that *replace* the
  previous mechanics instead of simply multiplying them. Each stage
  has its own language, its own buildings, its own game window.
- **Production chain**: matter flows between tiers. Dust → Pebbles →
  Planetesimals → Planets → Gas → Energy. Each tier needs the
  previous one as *support*, but the new tier is the *only* thing
  that matters for progression.
- **Soft dissipation**: the disk naturally dissipates. Each paradigm
  reached *re-seeds* the disk (stellar wind). The rhythm is: growth →
  peak → dissipation → new paradigm → new growth. No fail state.
- **Two endings**: at the Star stage, the player chooses between
  *neutron star* (peace) and *black hole* (transcendence). Both are
  valid.

**Visual theme**: an accretion disk seen from above. Concentric
orbits, particles spiraling toward the center, bodies growing and
brightening. The palette evolves: brown-ochre (dust) → amber
(pebbles) → orange (planetesimals) → yellow (planets) → blue-green
(gas giant) → white-blue (star) → deep black (black hole). Starfield
background. Everything is procedural: circles, gradients, particles,
halos. No textures.

**Narration**: textual fragments between paradigms — no dialogue, no
characters. An inner voice that builds as the consciousness grows.
"I am small." → "I eat." → "I fall." → "I burn." → "I am." The text
is the only dialogue in the game.

**Platform**: browser (WebAssembly) prioritized. Optional native
desktop. Default window 1280×800, resizable.

---

## 2. Inspirations

- **Universal Paperclips** (Frank Lantz, 2017) — main model: paradigm
  shifts, cosmic arc, mechanics that *replace* themselves.
- **Factorio** (Wube, 2020) — visible production chain, ratios,
  complexity growing by layers.
- **Cookie Clicker** (Orteil, 2013) — passive production,
  multipliers, prestige.
- **Antimatter Dimensions** (Hevipelle, 2016) — exponential scaling,
  infinity mechanics, deep prestige.
- **Dyson Sphere Program** (Youthcat, 2021) — cosmic scale,
  megastructures, visual satisfaction of growth.
- **Kittens Game** (bloodrizer, 2014) — interlocking resource chains,
  prestige, seasons.
- **A Dark Room** (Doublespeak, 2013) — abrupt narrative transitions,
  sense of scale.
- **Outer Wilds** (Mobius, 2019) — orbital physics as scenery,
  contemplative tone.
- **Sandustry** (2024) — visual satisfaction of pixel-perfect
  destruction.
- **The Powder Toy** (cellular simulation) — immediate reading of
  matter.

---

## 3. Scope — current state (V1)

| Element | Delivered | Post-release (bonus) |
|---|---|---|
| **Disk** | 1 disk, 8 visual orbits | +multi-disks, +binary systems |
| **Paradigms** | 6 stages (Dust, Pebbles, Planetesimals, Planets, Gas Giant, Star) + endgame | — |
| **Endings** | 2 (neutron star, black hole) | +secret 3rd ending |
| **Buildings** | ~12 types across tiers | +megastructures (Dyson sphere, Matrioshka brain) |
| **Upgrades** | ~40 multiplicative upgrades | +prestige upgrades |
| **Resources** | 8 (Dust, Pebbles, Planetesimals, Metals, Water, Gas, Light, Dark Matter) | +Organics, +Antimatter |
| **Prestige** | 1 layer (Gravitational Constant) | +2nd layer (Cosmological Constant) |
| **Narration** | ~30 textual fragments | +conditional fragments |
| **Save** | localStorage (web) + RON (native) | +cloud sync |
| **Audio** | Ambience + SFX (minimal) | +adaptive music |
| **Visuals** | Procedural (circles, gradients, particles) | +textures |
| **Screens** | Title + WebStart (wasm) + Game + Ending | +credits, +gallery |
| **Web** | Build via `tools/build-web.sh` (`BROWSER_BUILD.md` template) | — |

**Target tests**: ~150–180 tests (unit + integration).

**Target playtime**: 2–3 hours for the first ending. Replayability
via prestige (×3–5 runs for completionists).

---

## 4. Core loop

### 4.1 Frame by frame (in-game)

1. Read input (mouse → hover + click, keyboard → shortcuts).
2. Tick cooldowns (passives, building ticks, juice timers).
3. Tick the **disk simulation**:
   - Visual particles spiraling toward the center.
   - Bodies on orbits growing / moving.
   - No real physics: visual interpolation.
4. Tick **buildings**:
   - Each building has a `Cooldown` (already extracted, Phase 26).
   - When the cooldown is ready, it produces its resource and re-arms.
   - Upgrade multipliers apply to production.
5. Tick **dissipation**:
   - The disk loses mass at a rate depending on the paradigm.
   - When the disk reaches 0, passive production stops.
   - Automatic re-seed on paradigm advance.
6. Check **paradigm conditions**:
   - Each paradigm has a condition (total mass, specific resource,
     building constructed).
   - If reached → paradigm transition (textual cutscene + flash +
     re-seed).
7. Check **end conditions**:
   - At the Star stage, endgame is accessible.
   - The player clicks *Neutron Star* or *Black Hole* →
     cinematic + ending screen.
8. Tick **juice**: particles, floating text, shake, flash.
9. `next_frame().await` — **exactly once** per iteration.

### 4.2 Game by game

```
Title → WebStart (wasm) → Game
    ↓
Dust (paradigm 1)
    ↓ condition: 100 Dust collected manually
Pebbles (paradigm 2)
    ↓ condition: 5 Pebble Collectors built + 500 Pebbles
Planetesimals (paradigm 3)
    ↓ condition: 1 Resonance Engine built + 2000 Pebbles
Planets (paradigm 4)
    ↓ condition: 3 Planets formed + 5 Metal Foundries
Gas Giant (paradigm 5)
    ↓ condition: 10 Gas Scoops + 10,000 Gas
Star (paradigm 6)
    ↓
Endgame choice: Neutron Star OR Black Hole
    ↓
Ending screen → Title (prestige available)
```

### 4.3 Prestige

Available **at any time after the Planets paradigm** (to avoid
forcing a full run). The player clicks `Collapse`:

- The disk collapses into a new seed.
- Gain of **Gravitational Constant (G)** proportional to the total
  mass accreted during the run.
- `G` multiplies *all* production for subsequent runs.
- All buildings and upgrades reset.
- Paradigm returns to Dust.
- Endings already seen are remembered (gallery unlocked).

**G formula** (first pass, to be balanced):
```
G_gain = floor( log10(total_mass) * (1 + 0.1 * paradigms_reached) )
G_total = G_total + G_gain
production_multiplier = 1 + 0.05 * G_total
```

### 4.4 Endings

**Neutron Star** (peace):
- The star collapses into a neutron star. Mass mechanics are replaced
  by a static display: "I shine. I have always shone. I will always
  shine."
- The game stops. Prestige is available from the title.

**Black Hole** (transcendence):
- The star crosses the Chandrasekhar limit. Progressive black screen.
  The accretion disk eventually disappears. "I no longer shine. I
  curve. I am curvature."
- The game stops. Same for prestige.

Both endings unlock the *final* prestige (both count for 100 %
completion).

---

## 5. Data model

### 5.1 The 8 resources

| ID | Name | Introducing paradigm | Usage |
|---|---|---|---|
| `dust` | Dust | Dust | Base resource, fuels everything |
| `pebbles` | Pebbles | Pebbles | Agglomerated dust, support |
| `planetesimals` | Planetesimals | Planetesimals | Agglomerated pebbles |
| `metals` | Metals | Planetesimals | Construction resource |
| `water` | Water | Planets | Required for habitable planets |
| `gas` | Gas | Gas Giant | Fuels fusion |
| `light` | Light | Star | Fusion energy, endgame |
| `dark_matter` | Dark Matter | Endgame | Black hole ending resource |

**Production chain**:
```
Dust → Pebbles → Planetesimals ─┬→ Metals
                                └→ (Planets) → Water
                                        ↓
                                   Gas Giant → Gas → (Star) → Light
                                                                ↓
                                                        Black Hole → Dark Matter
```

### 5.2 Buildings

Each building has: `id`, `name`, `paradigm` (appearance tier),
`base_cost` (in resource), `cost_scaling` (×1.15 by default,
Cookie Clicker style), `production` (resource + amount),
`cooldown` (seconds), `description`.

| Building | Paradigm | Base cost | Production | Cooldown |
|---|---|---|---|---|
| `dust_collector` | Pebbles | 10 Pebbles | 1 Dust / tick | 1.0 s |
| `accretion_ring` | Pebbles | 50 Pebbles | ×1.10 to Dust collection | passive |
| `pebble_foundry` | Planetesimals | 100 Pebbles | 1 Pebble / tick | 1.0 s |
| `resonance_engine` | Planetesimals | 500 Pebbles | 5 Planetesimals / tick | 2.0 s |
| `metal_foundry` | Planets | 200 Planetesimals | 1 Metal / tick | 2.0 s |
| `water_condenser` | Planets | 300 Planetesimals | 1 Water / tick | 2.5 s |
| `orbital_anchor` | Planets | 1000 Planetesimals | ×1.20 to everything | passive |
| `gas_scoop` | Gas Giant | 500 Metals | 1 Gas / tick | 2.0 s |
| `magnetosphere_coil` | Gas Giant | 2000 Metals + 500 Water | ×1.30 to Gas | passive |
| `fusion_catalyst` | Star | 5000 Gas + 2000 Metals | 1 Light / tick | 3.0 s |
| `jet_injector` | Star | 10,000 Light | ×1.50 to Light | passive |
| `void_weaver` | Endgame | 50,000 Light | 1 Dark Matter / tick | 5.0 s |

**Passive buildings rule**: no cooldown, continuous effect applied
at the start of the tick. Active buildings have a `Cooldown` and
produce via discrete ticks.

### 5.3 Upgrades

Multiplicative upgrades, ~40 total, spread across paradigms.
Each upgrade: `id`, `name`, `effect` (enum or closure), `cost`,
`requires` (paradigm or building prerequisite).

Examples:
- `dust_dance`: +20 % Dust production (cost 100 Pebbles).
- `orbital_harmony`: +15 % to all Pebbles-tier buildings.
- `tidal_lock`: Resonance Engines produce ×2.
- `hydrogen_shell`: +50 % Gas.
- `photon_echo`: +25 % Light.
- `singularity_edge`: unlocks Dark Matter.
- etc.

### 5.4 Disk structure

```rust
pub struct Disk {
    /// Visual orbits (inside to outside).
    pub orbits: Vec<Orbit>,
    /// Visual particles (non-logical) — spiral inward.
    pub particles: Vec<Particle>,
    /// Total disk mass (decreases via dissipation).
    pub mass: f64,
    /// Max disk mass (re-seeded at paradigm).
    pub mass_max: f64,
    /// Dissipation rate per second (depends on paradigm).
    pub dissipation_rate: f64,
    /// Deterministic seed for visual generation.
    pub seed: u32,
}

pub struct Orbit {
    pub radius: f32,          // in pixels
    pub bodies: Vec<Body>,    // orbiting bodies
    pub angular_speed: f32,   // rad/s
}

pub struct Body {
    pub kind: BodyKind,       // Dust, Pebble, Planetesimal, Planet, GasGiant, Star
    pub mass: f64,            // logical mass
    pub angle: f32,           // angular position (rad)
    pub visual_radius: f32,   // visual radius (log)
    pub color: Color,         // depends on kind
}
```

**Note**: the disk is *visual* as much as *logical*. Bodies are
abstractions — no true orbital simulation (no N-body, no body-body
interactions). Orbits are fixed circles, bodies rotate at constant
angular speed, and the only "physics" is the mass accumulation by
the player. It's a visual metaphor, not a simulation.

### 5.5 Persistence

```rust
pub struct Progress {
    pub g_total: u64,                          // Gravitational Constant
    pub paradigms_reached: u8,                 // historical max
    pub endings_seen: Vec<Ending>,             // neutron, black hole
    pub run_state: Option<RunState>,           // current run (None = title)
    pub achievements: HashMap<String, bool>,
}

pub struct RunState {
    pub paradigm: Paradigm,
    pub resources: HashMap<ResourceId, f64>,
    pub buildings: HashMap<BuildingId, u32>,   // id → count
    pub upgrades: HashSet<UpgradeId>,
    pub disk_mass: f64,
    pub total_mass: f64,                       // for G_gain
    pub elapsed_seconds: f64,
    pub seed: u32,
}

pub enum Ending {
    NeutronStar,
    BlackHole,
}
```

**Native**: `progress.ron` in the user folder (see
`jade-garden/src/paths.rs` as a model).

**Web (WASM)**: **localStorage** via a custom miniquad plugin
(~30 lines of JS + Rust wrapper). This is **the first time** Ember
uses `localStorage` — it's the resolution point for item 27 in
`version.txt`. The plugin is documented separately (see §17).

Serialization: JSON (simpler to parse in JS) for the web, RON for
native. The choice is `#[cfg]`-gated.

---

## 6. Systems

Each system is a module in `src/`. Execution order ≈ §4.1.

### 6.1 `components.rs` — Base types

`ResourceId`, `BuildingId`, `UpgradeId`, `Paradigm`, `BodyKind`,
constants. **Pure** module, no `macroquad`. Headless testable.

### 6.2 `config.rs` — `GameContext`

Palette (colors per paradigm), layout, hit-test helpers. Loaded from
`.env` in dev (`COLOR_*`), with default values hardcoded.
`from_env` is `#[cfg(not(target_arch = "wasm32"))]`.

### 6.3 `disk.rs` — Disk state

`Disk`, `Orbit`, `Body`. Deterministic visual generation from `seed`.
Dissipation tick. Re-seed at paradigm. No production logic — purely
visual (but mass is read by `economy.rs` for G computation).

### 6.4 `economy.rs` — Production and resources

- `tick_buildings(state, dt)`: iterate buildings, tick cooldowns,
  produce.
- `apply_upgrades(state)`: recompute multipliers.
- `cost_of(building_id, count)`: cost of the next building (Cookie
  Clicker scaling: `base * 1.15^count`).
- `can_afford(state, building_id)`: check resources.
- `buy(state, building_id)`: decrement resources, increment count.

### 6.5 `paradigm.rs` — Paradigm progression

- `current_paradigm(state) -> Paradigm`.
- `next_paradigm(state) -> Option<Paradigm>`.
- `check_paradigm_advance(state) -> Option<Paradigm>`: check
  conditions, return new paradigm if reached.
- `advance_paradigm(state, new_paradigm)`: apply the transition
  (disk re-seed, add new buildings, update dissipation, trigger
  textual cutscene).

**Important**: paradigm advance is *not* automatic — the player
must click an "Advance" button when the condition is met. This
materializes the choice (the player may want to stay at a paradigm
to farm a resource before advancing).

### 6.6 `upgrades.rs` — Upgrade tree

- `available_upgrades(state) -> Vec<UpgradeId>`.
- `buy_upgrade(state, id)`.
- Upgrade table loaded from `assets/upgrades.ron`.

### 6.7 `prestige.rs` — Collapse

- `can_prestige(state) -> bool` (paradigm ≥ Planets).
- `compute_g_gain(state) -> u64`.
- `prestige(state)`: reset the run, add G, update records.

### 6.8 `narrative.rs` — Textual fragments

- Loads `assets/narrative.ron` (fragments per paradigm +
  conditional fragments).
- `next_fragment(state) -> Option<String>`: return the next
  fragment to display (only once per run).
- Fragment queue, displayed in the log at the top right.

### 6.9 `juice.rs` — Visual feedback

Primitives (reused from `ember_stdlib::juice`, to extract):
- `Particle`: dust particles, streaming inward.
- `FloatingText`: "+1 Pebble", "+5 Gas".
- `Shockwave`: ring at body formation.
- `Flash`: paradigm transition.
- `ScreenShake`: light, on big purchases.

### 6.10 `ui/` — Interface

- `ui/hud.rs`: resources at the top, prestige button, paradigm
  indicator, Advance button.
- `ui/buildings.rs`: list of purchasable buildings, costs, counters,
  tooltips.
- `ui/upgrades.rs`: scrollable upgrade tree.
- `ui/log.rs`: textual fragments.
- `ui/endgame.rs`: ending choice (Neutron Star / Black Hole).

**No fancy UI widgets** — standard `ember_stdlib::ui` buttons,
labels, panels.

### 6.11 `render/` — Disk rendering

- `render/disk.rs`: drawing orbits, bodies, particles.
- `render/star.rs`: central star, radiation.
- `render/palette.rs`: colors per paradigm.
- `render/bodies.rs`: drawing bodies (circles with gradients).

Everything is procedural: no textures, no sprites. Circles,
gradients, particles. Minimalist style.

### 6.12 `paths.rs` — Path resolution

Model: `jade-garden/src/paths.rs`. Assets as
`PathBuf::from("assets")` on wasm, standard native resolution
otherwise.

### 6.13 `audio.rs` — Ambience + SFX

**Native**: standard `AudioClip` (macroquad).
**Web**: `macroquad::audio` (WebAudio).

SFX:
- Purchase click.
- Paradigm advance (swoosh).
- Body formation (thump).
- Ending flash.
- Continuous ambient (drone, mixed low).

### 6.14 `save.rs` — Persistence

Wrapper around `Persistence<T>` (native) and the localStorage plugin
(web). Serializes `Progress`.

### 6.15 `input.rs` — Input

Keyboard + mouse snapshot via `Input::from_macroquad`. Shortcuts:
- `Space`: advance paradigm (if condition met).
- `Tab`: open upgrades tab.
- `Esc`: pause / menu.
- `R`: reset (with confirmation, dev only).

### 6.16 `main.rs` — Loop

State machine with 5 screens: `WebStart` (wasm), `Title`, `Game`,
`PrestigeConfirm`, `Ending`.

---

## 7. State struct & Phases

### 7.1 State machine

```
AppScreen::WebStart (wasm only)
    ↓ click
AppScreen::Title
    ↓ click "New Run"
AppScreen::Game (Phase = Playing)
    ↓ endgame condition met
AppScreen::Game (Phase = EndgameChoice)
    ↓ choice
AppScreen::Ending(Ending::NeutronStar | Ending::BlackHole)
    ↓ Enter
AppScreen::Title

    ↓ click "Prestige" (if paradigm ≥ Planets)
AppScreen::PrestigeConfirm
    ↓ confirmation
AppScreen::Title (G updated, run reset)
```

### 7.2 State struct `Game`

```rust
pub struct Game {
    pub screen: AppScreen,
    pub progress: Progress,
    pub run: RunState,
    pub disk: Disk,
    pub buildings: Vec<BuildingInstance>,   // runtime instances
    pub upgrades: HashSet<UpgradeId>,
    pub paradigm: Paradigm,
    pub resources: HashMap<ResourceId, f64>,
    pub log: VecDeque<String>,              // displayed fragments
    pub juice: Juice,
    pub rng: Rng,                           // deterministic
    pub elapsed: f64,
    pub g_multiplier: f64,                  // 1 + 0.05 * G
}

pub enum AppScreen {
    WebStart,
    Title,
    Game,
    PrestigeConfirm,
    Ending(Ending),
}

pub enum Phase {
    Playing,
    EndgameChoice,
    Won(Ending),
}
```

### 7.3 Timers

- **Building ticks**: `Cooldown` (already extracted, Phase 26).
  Prefer `Cooldown::new(d)` to simulate an active tick.
- **Dissipation**: continuous, no cooldown.
  `disk.mass -= rate * dt`.
- **Narrative fragments**: queue, one fragment displayed at a time,
  new one every 2 s.
- **Paradigm flash**: 0.5 s.
- **No game timer** — the game is real-time but the player can let
  it run. No pressure.

---

## 8. File structure

```
games/accretion/
├── DESIGN.md                (this file)
├── README.md                controls, install, dev
├── Cargo.toml
├── .env                     color overrides (dev)
├── .gitignore               (progress.ron, .env, dist/, tools/.cache/)
├── assets/
│   ├── buildings.ron        12 buildings
│   ├── upgrades.ron         ~40 upgrades
│   ├── narrative.ron        ~30 fragments
│   └── paradigms.ron        6 paradigms + conditions
├── web/
│   └── index.html           HTML5 shell for the WASM build
├── tools/
│   ├── build-web.sh         WASM bundle for itch.io (BROWSER_BUILD.md template)
│   └── localStorage_plugin.js  (miniquad plugin — see §17)
├── src/
│   ├── main.rs              loop + screens
│   ├── lib.rs               public exports
│   ├── paths.rs             runtime resolution
│   ├── config.rs            GameContext + palette
│   ├── components.rs        ResourceId, BuildingId, Paradigm
│   ├── disk.rs              visual disk state
│   ├── economy.rs           production, costs
│   ├── paradigm.rs          paradigm progression
│   ├── upgrades.rs          upgrade tree
│   ├── prestige.rs          collapse
│   ├── narrative.rs         textual fragments
│   ├── juice.rs             visual feedback
│   ├── save.rs              persistence (RON native / localStorage web)
│   ├── input.rs             shortcuts
│   ├── audio.rs             SFX + ambience
│   ├── systems.rs           Game state + tick
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── hud.rs
│   │   ├── buildings.rs
│   │   ├── upgrades.rs
│   │   ├── log.rs
│   │   └── endgame.rs
│   └── render/
│       ├── mod.rs
│       ├── disk.rs
│       ├── star.rs
│       ├── bodies.rs
│       └── palette.rs
└── tests/
    ├── economy.rs           cost calculations, production
    ├── paradigm.rs          paradigm conditions
    ├── prestige.rs          G formula
    ├── endings.rs           both endings reachable
    └── scenarios.rs         full simulated run (greedy AI)
```

---

## 9. Paradigm shifts — detail

Each paradigm has: an entry condition, a textual cutscene (1–3
fragments), a disk re-seed, and new mechanics that *partially
replace* older ones.

### 9.1 Dust (start)

**Entry condition**: start of the game.
**Mechanic**: click to attract dust. +1 Dust per click.
**Buildings**: none.
**Dissipation**: none (the disk is small, stable).
**Cutscene**: "I am. I do not know what I am. I eat the dust."

### 9.2 Pebbles

**Entry condition**: 100 Dust collected manually.
**Mechanic**: clicks become *optional*. Passive production begins.
Dust Collectors attract dust.
**New buildings**: `dust_collector`, `accretion_ring`.
**Replaced mechanic**: manual clicking becomes inefficient (10:1
ratio compared to a Dust Collector). The player is *encouraged* to
build, not click.
**Dissipation**: 0.5 %/s.
**Cutscene**: "The wind carries me. I no longer fall, I am carried.
I grow."

### 9.3 Planetesimals

**Entry condition**: 5 Dust Collectors + 500 Pebbles.
**Mechanic**: Pebbles agglomerate into Planetesimals. Resonance
Engines force orbital collisions.
**New buildings**: `pebble_foundry`, `resonance_engine`.
**Replaced mechanic**: Dust Collectors become *support* (they feed
Pebble Foundries) instead of being the main source.
**Dissipation**: 1 %/s.
**Cutscene**: "I am no longer alone. I am a chorus of pebbles. We
fall together."

### 9.4 Planets

**Entry condition**: 1 Resonance Engine + 2000 Pebbles.
**Mechanic**: Planetesimals agglomerate into Planets. Metal Foundries
extract metals from Planetesimals. Water Condensers produce water.
**New buildings**: `metal_foundry`, `water_condenser`,
`orbital_anchor`.
**Replaced mechanic**: Resonance Engines become *support* (they
supply Planetesimals to Metal Foundries).
**Dissipation**: 2 %/s.
**Cutscene**: "I have worlds. I am no longer a chorus, I am a
system."

### 9.5 Gas Giant

**Entry condition**: 3 Planets + 5 Metal Foundries.
**Mechanic**: Gas Scoops capture hydrogen and helium. Magnetosphere
Coils multiply the capture.
**New buildings**: `gas_scoop`, `magnetosphere_coil`.
**Replaced mechanic**: Metal Foundries become *support* (they supply
metals for Gas Scoops). Planets become *support* (they supply water
for Coils).
**Dissipation**: 3 %/s.
**Cutscene**: "I breathe. I am an atmosphere. I am heavy."

### 9.6 Star

**Entry condition**: 10 Gas Scoops + 10,000 Gas.
**Mechanic**: fusion ignites. Mass converts to *Light*. Fusion
Catalysts produce Light. Jet Injectors multiply it.
**New buildings**: `fusion_catalyst`, `jet_injector`.
**Replaced mechanic**: the entire mass chain (Dust → Pebbles →
Planetesimals → Planets → Gas) becomes *support* for fusion. Gas
Scoops feed the Catalysts; the rest is *vestigial* but still
functional.
**Dissipation**: 5 %/s — **and the disk no longer re-seeds**. This
is the final stretch. The player must reach endgame before the disk
empties.
**Cutscene**: "I burn. I am light. I am the end of the fall."

### 9.7 Endgame

**Entry condition**: Star paradigm + 50,000 Light.
**Mechanic**: the player clicks *Neutron Star* or *Black Hole*.
**Final cutscene**: 3–5 fragments depending on the choice, then
ending screen.

---

## 10. Prestige — Detail

**Unlocked**: paradigm ≥ Planets (to avoid forcing a full run before
being able to prestige).

**Button**: in the HUD, `Collapse` (with tooltip: "Reset your run.
Gain permanent Gravitational Constant.").

**Confirmation**: intermediate screen with `Confirm` / `Cancel`.

**Formula**:
```
total_mass_run = sum of accumulated resources (weighted)
paradigms = number of paradigms reached in this run
G_gain = floor( log10(total_mass_run + 1) * (1 + 0.1 * paradigms) )
G_total += G_gain
g_multiplier = 1 + 0.05 * G_total
```

**Effect**: `g_multiplier` applies to *all* production (each tick
is multiplied by `g_multiplier`). The effect is immediate in the
next run.

**Design decision**: no *multiple* prestige (no 2nd layer). A single
prestige is enough for a 2–3h playtime. If the game is well
received, a 2nd layer can be added post-release.

---

## 11. Endgame — Detail

### 11.1 Neutron Star (peace)

- The star collapses. The screen stabilizes to white-blue.
- Fragments: "I no longer fall. I shine. I have always shone. I
  will always shine."
- Ending screen: a slowly pulsing neutron star.
- Unlocks `Ending::NeutronStar` in `Progress`.

### 11.2 Black Hole (transcendence)

- The star crosses the Chandrasekhar limit. The screen progressively
  darkens.
- Fragments: "I no longer shine. I curve. I am curvature. I am the
  space that remembers me."
- Ending screen: accretion disk around a black hole, light that can
  no longer escape.
- Unlocks `Ending::BlackHole`.

### 11.3 Completion

`Progress::endings_seen` lists endings seen. Both endings unlock an
"achievement" and a line on the title screen ("2 / 2 endings").

---

## 12. Juice — Detailed plan

### 12.1 Passive production

- Floating text `+1 Dust`, `+5 Gas`, etc. above buildings (or at
  the center if no physical position).
- Dust particles spiraling toward the center of the disk
  continuously (visual).
- Bodies that visibly grow with each mass building purchase.

### 12.2 Building purchase

- Floating text `-100 Pebbles`, `+1 Dust Collector`.
- Light screen shake.
- The building "pops" in the UI (scale bounce).

### 12.3 Paradigm advance

- Full-screen white flash (0.5 s).
- Screen shake.
- The disk visually re-seeds (bodies reform).
- Textual cutscene: 1–3 fragments appearing one by one.
- Palette change (smooth 1 s transition).

### 12.4 Body formation

- Shockwave (expanding ring) around the new body.
- Small audio thump.
- The body "pops" (scale bounce).

### 12.5 Prestige

- The disk collapses to a point (2 s animation).
- Black flash.
- Title screen with the new G displayed.

### 12.6 Ending

- Neutron Star: slow pulsing, stable light.
- Black Hole: progressive darkening, visual distortion (a circle
  that twists, simple shader or manual drawing).
- Final fade to black.

### 12.7 General ambience

- Static starfield background (procedural, deterministic seed).
- Orbits drawn with thin lines.
- Bodies with halos.
- Palette that evolves with the paradigm.
- No "over-juice" — it's a contemplative game, not a frenetic one.

---

## 13. Tests

**Target**: ~150–180 tests.

### 13.1 Unit (per module)

| Module | Tests |
|---|---|
| `components.rs` | 8 |
| `config.rs` | 6 |
| `disk.rs` | 12 |
| `economy.rs` | 25 |
| `paradigm.rs` | 18 |
| `upgrades.rs` | 15 |
| `prestige.rs` | 10 |
| `narrative.rs` | 8 |
| `juice.rs` | 10 |
| `save.rs` | 12 |
| `systems.rs` | 20 |
| `render/palette.rs` | 6 |
| **Unit total** | **~150** |

### 13.2 Integration

| File | Tests |
|---|---|
| `economy.rs` | 5 |
| `paradigm.rs` | 5 |
| `prestige.rs` | 4 |
| `endings.rs` | 3 |
| `scenarios.rs` | 3 |
| **Total** | **~20** |

### 13.3 Priority tests

**`economy.rs`**:
- Base cost correct.
- ×1.15 scaling per purchase.
- Production respects cooldown.
- Upgrade multipliers applied.
- `g_multiplier` applied to everything.
- No production if insufficient resources for the cost (should not
  happen — checked before purchase).
- No capacity overflow (or overflow handled).

**`paradigm.rs`**:
- Paradigm conditions correct.
- No automatic advance.
- Advance applies re-seed.
- Advance unlocks new buildings.
- Advance changes dissipation.
- No going back.

**`prestige.rs`**:
- G formula correct at bounds.
- G_gain ≥ 1 even for a minimal run.
- Reset clears the run but preserves G.
- Prestige impossible before Planets.

**`endings.rs`**:
- Neutron Star reachable from a complete run.
- Black Hole reachable from a complete run.
- Both endings are independent.
- An ending already seen is not re-unlocked (idempotent).

**`scenarios.rs`**:
- Full simulated run (greedy AI) reaches Star in < 3h of simulated
  play.
- No softlocks.
- Prestige grants a measurable advantage.

### 13.4 What we do NOT test

- Visual rendering (no snapshots).
- Feel of the palettes (validated in play).
- The cutscenes (animated in real time).

---

## 14. UI/UX

### 14.1 Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  Resources: Dust 1234 | Pebbles 567 | ...       G: 12   [Collapse]│  ← top bar
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│                       ┌──────────────────┐                       │
│                       │                  │                       │
│                       │                  │                       │
│  Buildings            │     DISK         │         Log           │
│  ┌────────────────┐   │                  │   ┌──────────────┐    │
│  │ Dust Collector │   │   (visual)       │   │ "I eat the   │    │
│  │ Cost: 10       │   │                  │   │  dust."      │    │
│  │ [Buy]          │   │                  │   │              │    │
│  ├────────────────┤   │                  │   │ "The wind    │    │
│  │ Pebble Foundry │   │                  │   │  carries me."│    │
│  │ Cost: 100      │   │                  │   └──────────────┘    │
│  │ [Buy]          │   │                  │                       │
│  └────────────────┘   └──────────────────┘                       │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  [Upgrades]  [Advance Paradigm]  [Prestige]                      │  ← bottom bar
└──────────────────────────────────────────────────────────────────┘
```

### 14.2 Style

- Background: deep black, starfield.
- Panels: semi-transparent, thin border with palette color.
- Text: off-white, monospace.
- Buttons: rectangles with hover glow.
- No flourishes — it's an incremental game, information first.

### 14.3 Interactions

- Click a building → purchase if affordable, otherwise shake +
  tooltip.
- Click `Advance` → paradigm transition (if condition met).
- Click `Upgrades` → scrollable panel.
- Click `Prestige` → confirmation.
- `Tab` → toggle upgrades.
- `Esc` → menu (title + quit).
- `Space` → advance (if condition met).

### 14.4 Accessibility

- All actions accessible by mouse **and** keyboard.
- Tooltips on all buttons.
- No color-only info (icons + text).
- Adjustable font size (post-release).

---

## 15. Narration

Textual fragments, loaded from `assets/narrative.ron`. Structure:

```ron
[
    (id: "dust_1", paradigm: Dust, text: "I am."),
    (id: "dust_2", paradigm: Dust, text: "I do not know what I am."),
    (id: "dust_3", paradigm: Dust, text: "I eat the dust."),
    (id: "pebbles_1", paradigm: Pebbles, text: "The wind carries me."),
    (id: "pebbles_2", paradigm: Pebbles, text: "I no longer fall, I am carried."),
    (id: "pebbles_3", paradigm: Pebbles, text: "I grow."),
    // ...
]
```

**Rules**:
- One fragment displayed every 2 s, in order.
- Once per run (fragments already seen do not repeat within the
  same run).
- Some fragments are conditional (`requires: Some(...)`): for
  example, a special fragment when the player clicks `Prestige` for
  the first time.
- No dialogue, no characters. An inner voice.

**Themes**: solitude → growth → fall → shining → choice. Short,
poetic, never explanatory.

---

## 16. Technical

### 16.1 Architecture choices

- **Engine**: Ember (existing workspace).
- **Target**: WASM prioritized (`wasm32-unknown-unknown`), native
  optional.
- **Rendering**: macroquad, procedural (no textures).
- **Persistence**: RON (native) + localStorage (web).
- **Config**: `.env` (native only, `#[cfg]`-gated).

### 16.2 Ember reuse

- `ember_core::rng` — deterministic seed.
- `ember_core::hash` — visual hash (particles, stars).
- `ember_core::time::Cooldown` — building ticks.
- `ember_stdlib::persistence::Persistence<T>` — native save.
- `ember_stdlib::ui` — buttons, labels, panels.
- `ember_stdlib::fonts` — default font.
- `ember_stdlib::layout::centered_rect` — centering.

### 16.3 Anticipated Ember additions

- **localStorage bridge** (web) — see §17.
- **Cross-platform save** — `Persistence` + localStorage wrapper
  pattern.
- **Production chain** — `economy.rs` pattern (costs, multipliers,
  ticks) — extraction candidate if a 2nd incremental appears.
- **Upgrade tree** — `upgrades.rs` pattern — extraction candidate if
  a 2nd game needs it.
- **Prestige** — `prestige.rs` pattern — extraction candidate.

---

## 17. localStorage bridge (WASM) — detail

This is **the first time** Ember uses `localStorage`. This section
documents the implementation, as it will be reused by all future
WASM games.

### 17.1 Problem

`wasm-bindgen` is incompatible with miniquad (different import
systems). So we cannot call `window.localStorage` directly from
Rust. We must go through a **miniquad plugin**.

### 17.2 Solution

JS plugin (`tools/localStorage_plugin.js`) loaded in `index.html`
**after** `mq_js_bundle.js`:

```js
// tools/localStorage_plugin.js
(function () {
    if (typeof mergeInto !== 'function') {
        console.error('localStorage_plugin.js must be loaded AFTER mq_js_bundle.js');
        return;
    }

    mergeInto(LibraryManager.library, {
        ember_storage_get: function (key_ptr, out_buf_ptr, out_buf_len) {
            const key = UTF8ToString(key_ptr);
            try {
                const value = localStorage.getItem(key) || '';
                const bytes = new TextEncoder().encode(value);
                const n = Math.min(bytes.length, out_buf_len - 1);
                HEAPU8.set(bytes.subarray(0, n), out_buf_ptr);
                HEAPU8[out_buf_ptr + n] = 0;  // null terminator
                return n;
            } catch (e) {
                console.error('ember_storage_get failed:', e);
                return -1;
            }
        },
        ember_storage_set: function (key_ptr, val_ptr) {
            const key = UTF8ToString(key_ptr);
            const value = UTF8ToString(val_ptr);
            try {
                localStorage.setItem(key, value);
                return 0;
            } catch (e) {
                console.error('ember_storage_set failed:', e);
                return -1;
            }
        },
        ember_storage_remove: function (key_ptr) {
            const key = UTF8ToString(key_ptr);
            try {
                localStorage.removeItem(key);
                return 0;
            } catch (e) {
                return -1;
            }
        },
    });
})();
```

Rust wrapper (`src/save.rs`):

```rust
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn ember_storage_get(key: *const u8, out_buf: *mut u8, out_buf_len: usize) -> i32;
    fn ember_storage_set(key: *const u8, value: *const u8) -> i32;
    fn ember_storage_remove(key: *const u8) -> i32;
}

#[cfg(target_arch = "wasm32")]
pub fn save_web(key: &str, value: &str) -> bool {
    let key_c = std::ffi::CString::new(key).unwrap();
    let value_c = std::ffi::CString::new(value).unwrap();
    unsafe {
        ember_storage_set(key_c.as_ptr() as *const u8, value_c.as_ptr() as *const u8) == 0
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_web(key: &str, max_len: usize) -> Option<String> {
    let key_c = std::ffi::CString::new(key).ok()?;
    let mut buf = vec![0u8; max_len];
    let n = unsafe {
        ember_storage_get(key_c.as_ptr() as *const u8, buf.as_mut_ptr(), max_len)
    };
    if n < 0 {
        return None;
    }
    buf.truncate(n as usize);
    String::from_utf8(buf).ok()
}
```

### 17.3 index.html

```html
<script src="mq_js_bundle.js"></script>
<script src="localStorage_plugin.js"></script>
<script>
    if (typeof load !== 'function') { /* ... error ... */ }
    else { load("accretion.wasm"); }
</script>
```

### 17.4 build-web.sh

The script (`tools/build-web.sh`) must copy `localStorage_plugin.js`
into `dist/web/` next to `mq_js_bundle.js`. That's one extra line in
the `BROWSER_BUILD.md` §3.2 template.

### 17.5 Scope

This bridge is **generic** — it will be reused by all WASM games
that want persistence. It's the resolution point for item 27 in
`version.txt` ("jade-garden: no localStorage on web"). Once
validated by Accretion, it can be moved to `ember_stdlib` or a
dedicated crate.

---

## 18. Known pitfalls / delicate decisions

### 18.1 Inherited from the workspace

- `env_color`: `{PREFIX}_R/_G/_B/_A`, not a string.
- `clippy::collapsible_if`: let-chains.
- `clippy::too_many_arguments`: bundle in a tuple or `#[allow]`.
- `gen` reserved → `rng_state` / `id_gen`.
- `should_implement_trait` on custom `next()` → `next_id()`.
- `set_default_font` after `load_ttf_font`.
- `next_frame().await` unique.
- `Instant::now()` forbidden on `wasm32-unknown-unknown` — use an
  accumulated `f32`.
- `wasm-bindgen` forbidden — use `env.console_log` / `mergeInto`.
- Assets on wasm: URLs relative to `index.html`.
- Async preload (`load_sound`, `load_ttf_font`) before the loop.

### 18.2 Specific to Accretion

- **No true orbital simulation.** Orbits are fixed circles, bodies
  rotate at constant angular speed. All "physics" is visual. Do not
  attempt N-body: it would be slow, unstable, and useless for
  gameplay.
- **The disk is both visual and logical.** `disk.mass` is read by
  `economy.rs` (for dissipation) but bodies do not participate in
  production. Do not confuse the two layers.
- **Dissipation = no fail state.** When `disk.mass == 0`, passive
  production stops, but the player can keep clicking (at a reduced
  rate). They can also prestige to reset. **Never block the
  player** — this is a contemplative game, not a punishing one.
- **The re-seed at paradigm is the safety valve.** Each paradigm
  advance restores `disk.mass` to `mass_max`. It's the mechanism
  that guarantees you cannot get stuck.
- **Paradigm advance = explicit click.** Do not automate. The
  player must *choose* to advance (in case they want to farm).
- **No prestige before Planets.** Otherwise a player could prestige
  without having seen the game.
- **G_multiplier is cumulative and multiplicative.** Do not confuse
  it with an additive bonus.
- **Upgrades are permanent within a run.** No deactivation. The
  reset comes only from prestige.
- **localStorage: the key must be prefixed by the game.** For
  example `accretion:save`. Otherwise collision with other Ember
  games on the same itch.io domain.
- **localStorage: ~5 MB quota.** The complete save must be
  < 100 KB. No snapshots, no history — one state only.
- **No cloud sync in V1.** Local only.
- **The localStorage bridge must be tested on itch.io.** The local
  build is not enough — itch.io wraps the game in an iframe. Verify
  that `localStorage` is accessible in that context (normally yes,
  but some browsers in strict mode block it).
- **Narrative fragments are deterministic.** No randomization. The
  player must be able to replay and find the same texts.

### 18.3 Validated decisions

- ✅ Name: `accretion`.
- ✅ 6 paradigms + endgame.
- ✅ 2 endings (neutron star, black hole).
- ✅ 8 resources.
- ✅ ~12 buildings, ~40 upgrades.
- ✅ 1 prestige layer.
- ✅ localStorage for web persistence.
- ✅ No true orbital physics.
- ✅ No fail state.
- ✅ Evolving palette per paradigm.
- ✅ 100 % English code.

---

## 19. What this game adds to the engine

| New capability | Previous max |
|---|---|
| **localStorage bridge (WASM)** | None — **resolves item 27** |
| **Cross-platform save** (RON + localStorage) | 1 occurrence (jade-garden in-memory) |
| **Production chain** (costs, multipliers, ticks) | None |
| **Upgrade tree** | 1 occurrence (ember-wars) |
| **Prestige** | None |
| **Paradigm shifts** (mechanics that replace themselves) | None |
| **Abstract procedural rendering** (circles, gradients) | 1 occurrence (bore-inc pixel) |
| **Fragmentary narration** | 1 occurrence (jade-garden) |
| **Two endings** | None |
| **WASM build via BROWSER_BUILD.md template** | 2 occurrences (jade-garden, bore-inc) → **3rd → validation** |
| **Async preload on wasm** | 2 occurrences (jade-garden) → **3rd → validation** |

**Likely upcoming extractions**:
- `ember_stdlib::economy` (if a 2nd incremental appears).
- `ember_stdlib::upgrades` (2nd occurrence — ember-wars has one).
- `ember_stdlib::prestige` (if a 2nd incremental).
- `ember_stdlib::localStorage` (dedicated crate — used by all
  future WASM games).

---

## 20. Open questions

1. **Dissipation balancing.** The 0.5–5 %/s values are placeholders.
   To be tested with a simulator (greedy AI as in
   `jade-garden/tests/balance.rs`).
2. **Number of upgrades.** ~40 is an estimate. To be adjusted after
   the first playable run.
3. **Is prestige at Planets too early?** Maybe unlock at Gas Giant
   to avoid a too-short run.
4. **Is a 2nd prestige layer needed?** Not in V1. To be
   reconsidered if the game is well received.
5. **localStorage on itch.io:** test in real conditions. Provide an
   in-memory fallback if `localStorage` is blocked.
6. **Should the localStorage bridge live in `ember_stdlib` or in a
   dedicated `ember-wasm` crate?** To decide after Accretion
   (1 occurrence = local).
7. **Is a "Zen" mode needed?** (no dissipation) Maybe post-release,
   for players who just want to watch the disk turn.
8. **Are achievements needed?** (Steam-style) Post-release.
9. **Should the narration be more present?** Accretion is
   contemplative; too much text would break the rhythm. To be
   dosed.
10. **Is the name "Accretion" distinctive enough?** Check for major
    collisions on itch.io. (There may be niche games with this
    name, but nothing mainstream.)

---

## 21. Current state

**In design.** This document is the starting point. Code has not
been written yet.

**Next steps**:
1. Validate the design (this doc).
2. Scaffold `games/accretion/` (Cargo.toml, structure).
3. Implement `components.rs` + `economy.rs` + tests.
4. Implement the 6 paradigms + integration tests.
5. Implement the 2 endings + tests.
6. Implement the procedural disk rendering.
7. Implement the localStorage bridge + itch.io tests.
8. WASM build + publication.
9. Update `BROWSER_BUILD.md` (the template is validated by a 3rd
   use).
10. Update `version.txt` (new capability, 3rd WASM, resolution of
    item 27).

**Estimate**: 3–5 dev sessions (depending on prototype quality).

---

*This design assumes the player wants to watch something without a
shape grow — a consciousness, a disk, a star. The real subject is
not optimization, it's the fall. We are not building an empire; we
are falling toward ourselves. The game is a meditation on
inevitability, disguised as an incremental.*