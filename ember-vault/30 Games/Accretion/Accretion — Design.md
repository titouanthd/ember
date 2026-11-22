---
title: "Accretion — Design"
tags: ["ember/game", "game/accretion"]
game: "Accretion"
former-path: "games/accretion/DESIGN.md"
imported: "2026-11-22"
---

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

Last updated: Session 17.9 (implementation status reconciled with the code).
Status: **implemented (V1.3)** — native + browser (WASM). 127 tests passing,
1 ignored.
Engine: **Ember**.

> **Implementation status (read this first).** The code shipped in
> `games/accretion/` diverges from the original plan below. The design text is
> kept as the intent; this table is the truth.
>
> | Area | Design (below) | Implemented |
> |---|---|---|
> | Paradigms | 6 + endgame | 6 (`Dust` → `Star`) + endgame, identical |
> | Resources | 8 | 8, identical |
> | Buildings | 12 + passive effects | 12, hardcoded in `economy.rs`; passive multipliers in `compute_multipliers` |
> | Upgrades | ~40, `assets/upgrades.ron` | **not implemented** — no `upgrades.rs`, no asset file |
> | Prestige | at Planets | at **Gas Giant** (`prestige::MIN_PARADIGM`) |
> | Endgame | 50,000 Light | **10,000 Light** (`ENDGAME_LIGHT_THRESHOLD`) |
> | Narrative | `assets/narrative.ron`, ~30 fragments | hardcoded in `narrative.rs`, intro + one entry per paradigm |
> | Audio | ambience + SFX | **not implemented** — no `audio.rs` |
> | Juice | `juice.rs`, particles/shake/flash | **no `juice.rs`**; visual feedback lives in `main.rs` / `disk.rs` |
> | UI | `ui/` module tree | drawn directly in `main.rs` |
> | Save | `Progress` + `RunState`, JSON web | whole `Game` serialized; **RON** on both targets |
> | Native save | `progress.ron` in user folder | `save.ron` in the working directory |
> | Web save | localStorage | localStorage, key `accretion:save` |
> | Tests | ~150–180, 5 integration files | **127 passing + 1 ignored**, `tests/balance.rs` + `tests/save.rs` |
> | Modules | 17+ files incl. `upgrades`, `juice`, `ui/`, `render/` | 13 flat modules (see §8) |
>
> Rendering is procedural but concentrated in `main.rs` and `disk.rs`; there is
> no `render/` directory. The localStorage bridge (§17) is implemented exactly
> as described and is the first real web save in the workspace.

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
| **Upgrades** | **Not implemented** (was ~40 planned) | +upgrade tree |
| **Resources** | 8 (Dust, Pebbles, Planetesimals, Metals, Water, Gas, Light, Dark Matter) | +Organics, +Antimatter |
| **Prestige** | 1 layer (Gravitational Constant), unlocked at Gas Giant | +2nd layer (Cosmological Constant) |
| **Narration** | Intro + 1 fragment per paradigm (hardcoded) | +conditional fragments, +asset file |
| **Save** | localStorage (web) + RON (native) | +cloud sync |
| **Audio** | **Not implemented** | +ambience, +SFX, +adaptive music |
| **Visuals** | Procedural (circles, gradients, particles) | +textures |
| **Screens** | Title + WebStart (wasm) + Game + Ending | +credits, +gallery |
| **Web** | Build via `tools/build-web.sh` (`BROWSER_BUILD.md` template) | — |

**Actual tests**: 127 passing, 1 ignored (`tests/balance.rs::click_rate_sensitivity`).
The original target was ~150–180.

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
    ↓ condition: 5 Dust Collectors built + 500 Pebbles
Planetesimals (paradigm 3)
    ↓ condition: 1 Resonance Engine built + 1500 Pebbles
Planets (paradigm 4)
    ↓ condition: 5 Metal Foundries + 3 Water Condensers + 500 Water
Gas Giant (paradigm 5)
    ↓ condition: 10 Gas Scoops + 10,000 Gas
Star (paradigm 6)
    ↓ condition: 10,000 Light
Endgame choice: Neutron Star OR Black Hole
    ↓
Ending screen → auto-prestige → Title
```

### 4.3 Prestige

Available **from the Gas Giant paradigm** (raised from Planets in Session 13:
Planets runs were ~2.4 min with optimal play, too short to justify a reset).
The player clicks `Collapse`:

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

> **Not implemented in V1.3.** There is no `upgrades.rs` and no
> `assets/upgrades.ron`; the 12-building chain and passive multipliers are the
> entire economy. The table below is the original intent.

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

The whole `Game` is serialized as-is — there is no `Progress`/`RunState`
split in V1.3.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub paradigm: Paradigm,
    pub resources: HashMap<ResourceId, f64>,
    pub buildings: Vec<BuildingInstance>,
    pub g_multiplier: f64,
    pub g_total: u64,
    #[serde(default)]
    pub endings_seen: Vec<Ending>,
    pub elapsed: f64,
    pub log: Vec<String>,
    pub dust_collected_total: f64,
}
```

**Native**: `save.ron` in the working directory (not `progress.ron` in a user
folder).

**Web (WASM)**: `localStorage["accretion:save"]` via the miniquad plugin
described in §17. The serialized payload is RON on **both** targets; the
original "JSON on web" idea was dropped because RON round-trips the same
struct without a second schema.

**Save tests**: `tests/save.rs` is a native-only placeholder (0 tests); the
round-trip is covered by unit tests in `save.rs` / `systems.rs`. Web
persistence must be verified manually in a browser (§18.2).

---

## 6. Systems (as implemented)

Each system is a flat module in `src/`. Execution order ≈ §4.1. There is no
`upgrades.rs`, `juice.rs`, `input.rs`, `audio.rs`, `ui/`, or `render/`
directory.

### 6.1 `components.rs` — Base types

`ResourceId` (8 variants + `ALL`), `BuildingId` (12 + `ALL`), `Paradigm`
(6 + `ALL`), `BodyKind`, `Ending`. **Pure**, no `macroquad`, headless
testable.

### 6.2 `config.rs` — `GameContext` and palette

Palette per paradigm, layout constants, hit-test helpers. `from_env` is
native-only; colors fall back to hardcoded defaults.

### 6.3 `disk.rs` — Disk state and rendering

`Disk`, `Orbit`, `Body`. Deterministic visual generation from a seed,
dissipation tick, re-seed on paradigm. Also owns the orbit/body drawing
helpers; there is no separate `render/` module.

### 6.4 `economy.rs` — Production and resources

- `BUILDINGS: &[BuildingDef]` — the 12 definitions, **hardcoded**.
- `tick_buildings(state, dt)`: tick cooldowns, produce (capped at
  `MAX_FIRINGS_PER_FRAME = 10_000` to survive huge `dt`).
- `compute_multipliers(state)`: passive building effects + `g_multiplier`.
- `cost_of(def, count)`: `base * 1.15^count`.
- `can_afford`, `buy`, `id_to_def`.
- `BuildingInstance`: id + count + `Cooldown`.

### 6.5 `paradigm.rs` — Paradigm progression

`CONDITIONS` (5 transitions), `next_condition`, `condition_met`,
`check_advance`, `progress_lines`. Advance is explicit (player clicks
`Advance`), never automatic. No going back.

### 6.6 `prestige.rs` — Collapse

- `MIN_PARADIGM = GasGiant` (not Planets).
- `total_mass_run` = held resources + Σ(building count × base cost).
- `compute_g_gain` = `floor(log10(mass + 1) * (1 + 0.1 × paradigms))`,
  minimum 1.
- `prestige`: reset the run, add G, recompute `g_multiplier =
  1 + 0.05 × G_total`, preserve `endings_seen`.

### 6.7 `endgame.rs` — Endings

- `ENDGAME_LIGHT_THRESHOLD = 10_000` (not 50,000).
- `can_endgame`: `Star` + Light ≥ threshold.
- `choose_ending`: records the ending, then **auto-prestiges** the run.

### 6.8 `narrative.rs` — Textual fragments

Hardcoded `intro()` (2 lines) and `paradigm_entry(p)` (one line per
paradigm). There is no `assets/narrative.ron` and no conditional fragment
queue.

### 6.9 `save.rs` — Persistence

Native `save.ron` / web `localStorage` (§17). Serializes `Game` directly.

### 6.10 `format.rs` — Number formatting

Compact suffixes (`K`, `M`, `B`, …) for the HUD and cost labels.

### 6.11 `fonts.rs` — Font loading

Loads `assets/fonts/DejaVuSansMono.ttf` through
`ember_stdlib::fonts::load_and_set_default` (path resolved by `paths.rs`).

### 6.12 `paths.rs` — Path resolution

`assets` relative to `index.html` on wasm; `CARGO_MANIFEST_DIR`-relative on
native.

### 6.13 `main.rs` — Loop and UI

State machine (`WebStart` on wasm, `Title`, `Game`, `Ending`), all drawing
(HUD, building list, log, buttons), input handling, and the single
`next_frame().await`. `systems.rs` holds `Game` and its pure tick/input
methods, so the loop stays thin and testable.

---

## 7. State struct & Phases

### 7.1 State machine

```
AppScreen::Title
    ↓ "New Run" / "Continue"
AppScreen::Game
    ↓ prestige clicked (paradigm ≥ Gas Giant)
AppScreen::PrestigeConfirm → confirm → AppScreen::Title (G updated)
    ↓ endgame condition met
AppScreen::EndgameChoice
    ↓ choice
AppScreen::Ending(NeutronStar | BlackHole)
    ↓ Enter
AppScreen::Title (auto-prestiged)
```

There is no `WebStart` screen: accretion's audio is not implemented, so the
first-click audio unlock is unnecessary. The title screen still requires a
click to start, which satisfies browser gesture rules.

### 7.2 State struct `Game`

```rust
pub struct Game {
    pub paradigm: Paradigm,
    pub resources: HashMap<ResourceId, f64>,
    pub buildings: Vec<BuildingInstance>,
    pub g_multiplier: f64,
    pub g_total: u64,
    pub endings_seen: Vec<Ending>,
    pub elapsed: f64,
    pub log: Vec<String>,
    pub dust_collected_total: f64,
}
```

UI-only state lives in `main.rs`: `AppScreen`, `TimeSpeed` (×1–×4),
`RightTab` (Buildings / Log), hover/drag helpers.

### 7.3 Timers

- **Building ticks**: `Cooldown` (Phase 26). `BuildingInstance::new` uses
  `Cooldown::new(d)` — i.e. a newly bought building is **running** and
  produces for the first time after one full cooldown. If instant first
  production is wanted, use `Cooldown::with_duration(d)` (ready) instead.
- **Dissipation**: continuous, no cooldown.
  `disk.mass -= rate * dt`.
- **Narrative fragments**: only the hardcoded intro and one entry per
  paradigm (pushed to the log); there is no timed queue or conditional
  fragment system.
- **Paradigm flash**: **not implemented** (no juice module).
- **No game timer** — the game is real-time but the player can let
  it run. No pressure.

---

## 8. File structure

```
games/accretion/
├── DESIGN.md                (this file)
├── README.md                controls, run, build, save
├── Cargo.toml
├── assets/
│   ├── .gitkeep
│   └── fonts/DejaVuSansMono.ttf
├── web/
│   └── index.html           HTML5 shell (loads the localStorage plugin)
├── tools/
│   ├── build-web.sh         WASM bundle for itch.io
│   └── localStorage_plugin.js  miniquad plugin (§17)
├── dist/web/                committed bundle (index.html, wasm, assets, plugin)
├── src/
│   ├── main.rs              loop + all screens + UI drawing
│   ├── lib.rs               public module exports
│   ├── paths.rs             runtime asset resolution
│   ├── config.rs            GameContext + palette
│   ├── components.rs        ResourceId, BuildingId, Paradigm, Ending
│   ├── disk.rs              visual disk state + rendering
│   ├── economy.rs           building table, production, costs
│   ├── paradigm.rs          transition conditions
│   ├── prestige.rs          collapse + G formula
│   ├── endgame.rs           ending choice + auto-prestige
│   ├── narrative.rs         hardcoded fragments
│   ├── save.rs              RON native / localStorage web
│   ├── format.rs            compact number formatting
│   ├── fonts.rs             font loading
│   └── systems.rs           Game state + pure tick/input methods
└── tests/
    ├── balance.rs           greedy-AI balance report (1 ignored)
    └── save.rs              native placeholder (0 tests)
```

The original plan had `assets/*.ron`, `upgrades.rs`, `juice.rs`, `input.rs`,
`audio.rs`, `ui/`, and `render/`. None of those exist; see the status table at
the top.

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

**Entry condition**: 1 Resonance Engine + 1500 Pebbles (lowered from 2000).
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

**Entry condition**: 5 Metal Foundries + 3 Water Condensers + 500 Water.
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

**Entry condition**: Star paradigm + 10,000 Light (lowered from 50,000 for
V1.3 balance).
**Mechanic**: the player clicks *Neutron Star* or *Black Hole*.
**Final cutscene**: 3–5 fragments depending on the choice, then
ending screen.

---

## 10. Prestige — Detail

**Unlocked**: paradigm ≥ **Gas Giant** (raised from Planets after Session 13
balance showed Planets runs were too short).

**Button**: in the HUD, `Collapse` (with tooltip: "Reset your run.
Gain permanent Gravitational Constant.").

**Confirmation**: intermediate screen with `Confirm` / `Cancel`.

**Formula** (as implemented):
```
total_mass_run = held resources + Σ(building count × base cost)
paradigms = current paradigm index + 1
G_gain = max(1, floor(log10(total_mass_run + 1) * (1 + 0.1 * paradigms)))
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

**Actual**: 127 passing, 1 ignored (125 unit + 2 integration + 1 ignored).
The original target was ~150–180.

### 13.1 Unit (per module)

| Module | Tests |
|---|---|
| `components.rs` | 14 |
| `config.rs` | 2 |
| `economy.rs` | 35 |
| `endgame.rs` | 9 |
| `narrative.rs` | 2 |
| `paradigm.rs` | 14 |
| `prestige.rs` | 16 |
| `save.rs` | 6 |
| `systems.rs` | 27 |
| **Unit total** | **125** |

`disk.rs`, `format.rs`, and `fonts.rs` have no tests (rendering / thin
wrappers).

### 13.2 Integration

| File | Passing | Ignored |
|---|---|---|
| `tests/balance.rs` | 2 | 1 (`click_rate_sensitivity`) |
| `tests/save.rs` | 0 | 0 |
| **Total** | **2** | **1** |

### 13.3 What the tests actually cover

**`economy.rs`**:
- Base cost and ×1.15 scaling.
- Cooldown-gated production (`Cooldown::new` starts running).
- Passive multipliers + `g_multiplier`.
- Affordability, purchase, no partial purchase.
- `MAX_FIRINGS_PER_FRAME` clamp for huge `dt`.

**`paradigm.rs`**:
- The 5 conditions, lower-bound semantics.
- Explicit advance only; no advance from Star.
- `progress_lines` formatting.

**`prestige.rs`**:
- Gas Giant threshold, no prestige at Planets.
- G formula bounds and minimum 1.
- Reset preserves G / endings; multiplier recomputed.
- Prestige panics below the threshold.

**`endgame.rs`**:
- Both endings reachable, idempotent `endings_seen`.
- Auto-prestige after a choice.
- 10,000 Light threshold.

**`tests/balance.rs`**:
- Greedy-AI simulated run reaches each paradigm within an expected time.
- Prestige loop yields a measurable advantage.
- Click-rate sensitivity is ignored (slow, exploratory).

### 13.4 What we do NOT test

- Visual rendering (no snapshots).
- Web `localStorage` (manual browser check).
- Feel of the palettes (validated in play).
- Audio (not implemented).

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
- **Clippy is not green.** `components::Paradigm::next` triggers
  `skip_while_next`; `systems::Game::new` triggers `vec_init_then_push`.
  `main.rs` also has unnecessary `unsafe` blocks and an
  `unwrap_or_else` default. Fix these before running `make check`.

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
| **WASM build via BROWSER_BUILD.md template** | 2 occurrences (jade-garden, bore-inc) → accretion was the 3rd, pagoda the 4th |
| **Async preload on wasm** | 2 occurrences (jade-garden, bore-inc) → proven on 4 ports |

**Likely upcoming extractions**:
- `ember_stdlib::economy` (if a 2nd incremental appears).
- `ember_stdlib::upgrades` (2nd occurrence — ember-wars has one).
- `ember_stdlib::prestige` (if a 2nd incremental).
- `ember_stdlib::localStorage` (dedicated crate — used by all
  future WASM games).

---

## 20. Open questions (status after V1.3)

1. **Dissipation balancing.** Still placeholder-ish; `tests/balance.rs` checks
   paradigm timings, but the click-rate sensitivity test is ignored.
2. **Number of upgrades.** Resolved for V1.3 by **not shipping upgrades**.
   The 12-building chain is the whole economy.
3. **Is prestige at Planets too early?** Resolved: moved to **Gas Giant**.
4. **Is a 2nd prestige layer needed?** No; one layer shipped.
5. **localStorage on itch.io:** implemented and bundled; still needs a manual
   check inside the itch.io iframe. No in-memory fallback yet.
6. **Where should the localStorage bridge live?** Still local to accretion.
   Port it to pagoda (currently in-memory) before extracting.
7. **Is a "Zen" mode needed?** Not shipped. Time speed ×1–×4 is the only
   pacing control.
8. **Are achievements needed?** Not shipped; only `endings_seen` is tracked.
9. **Should the narration be more present?** Not shipped beyond intro + one
   line per paradigm.
10. **Is the name "Accretion" distinctive enough?** Unresolved; check itch.io
    before publishing.

---

## 21. Current state

**Implemented and playable** on native and browser (WASM). 127 tests pass,
1 is ignored. The committed `dist/web/` bundle is the reference build.

**What shipped**:
- 6 paradigms + 2 endings, 8 resources, 12 buildings, passive multipliers.
- Prestige at Gas Giant; endgame at 10,000 Light; auto-prestige on ending.
- Native `save.ron`; web `localStorage["accretion:save"]`.
- Procedural disk rendering, palette per paradigm, HUD/building/log UI.
- Greedy-AI balance report.

**What did not ship** (kept as future work):
- The upgrade tree (~40 planned) and `assets/upgrades.ron`.
- Audio/ambience and the `juice.rs` extraction.
- The `ui/` and `render/` module split (drawing lives in `main.rs`/`disk.rs`).
- `WebStart` screen, achievements, Zen mode, cloud sync.
- A real web-save integration test.

**Next steps**:
1. Green clippy: `skip_while_next` in `components::Paradigm::next` and
   `vec_init_then_push` in `systems::Game::new`.
2. Decide whether upgrades are worth adding; if not, delete the references
   from this doc rather than leaving a phantom feature.
3. Port the localStorage bridge to pagoda, then extract it.
4. Verify the save inside the itch.io iframe.
5. Extract the shared `mq_js_bundle.js` resolver (4 copies now).

**Estimate**: shipped in ~5 sessions (design + implementation + balance).

---

*This design assumes the player wants to watch something without a
shape grow — a consciousness, a disk, a star. The real subject is
not optimization, it's the fall. We are not building an empire; we
are falling toward ourselves. The game is a meditation on
inevitability, disguised as an incremental.*
