# accretion

A cosmic incremental game about gravity. You are a nascent consciousness in a
protoplanetary disk; to grow, you accrete. Dust becomes pebbles, pebbles become
planetesimals, planets, a gas giant, a star — and then a choice.

Six paradigms, eight resources, twelve buildings, one prestige layer, two
endings. Inspired by *Universal Paperclips*, *Factorio*, and *Cookie Clicker*.

**Status:** implemented (V1.3), native + browser. 127 tests pass, 1 ignored.
The design intent and the divergences are documented in `ember-vault/30 Games/Accretion/Accretion — Design.md`.

## Running

### Native

```sh
cargo run --release -p accretion
```

The native save is `save.ron` in the working directory. Delete it to start
over.

### Web (WASM)

```sh
./tools/build-web.sh
cd dist/web && python3 -m http.server 8080
# → http://localhost:8080
```

The web build saves to `localStorage["accretion:save"]` (a RON string)
through `tools/localStorage_plugin.js`, a miniquad JS plugin
(`miniquad_add_plugin`) that `index.html` loads after `mq_js_bundle.js`.
Reloading the page offers Continue. The font is built into the wasm, so the
bundle needs no `assets/` folder.

## Controls

| Input | Action |
|---|---|
| Click the disk | Collect 1 Dust (always available; inefficient after Dust) |
| Click a building card | Buy the next one |
| `1` / `2` / `3` / `4` | Simulation speed ×1 / ×2 / ×3 / ×4 |
| `R` / `B` / `L` / `T` | Right panel: Resources / Buildings / Log / Progression |
| `Advance` button | Move to the next paradigm when its condition is met |
| `Collapse` button | Prestige (from Gas Giant onward) |
| `Esc` | Cancel the prestige/endgame confirmation |
| `Enter` | Leave an ending screen |
| Wheel over the building list | Scroll |

## The loop

1. Click dust until you can afford Dust Collectors.
2. Build the chain: Dust → Pebbles → Planetesimals → Metals / Water → Gas →
   Light.
3. Meet each paradigm's condition and click **Advance** (advance is never
   automatic).
4. At **Star**, reach 10,000 Light, then choose **Neutron Star** or **Black
   Hole**. The run auto-prestiges.
5. Spend **G** on the next run: every point gives +5 % production.

### Transition conditions

| From → To | Requirement |
|---|---|
| Dust → Pebbles | 100 Dust |
| Pebbles → Planetesimals | 5 Dust Collectors + 500 Pebbles |
| Planetesimals → Planets | 1 Resonance Engine + 1500 Pebbles |
| Planets → Gas Giant | 5 Metal Foundries + 3 Water Condensers + 500 Water |
| Gas Giant → Star | 10 Gas Scoops + 10,000 Gas |
| Star → Endgame | 10,000 Light |

Prestige unlocks at **Gas Giant** (not Planets, as the original design said).

## What is not implemented

- The planned ~40-upgrade tree (the 12 buildings and their passive
  multipliers are the whole economy).
- Audio and the `juice.rs`/`ui/`/`render/` module split.
- Achievements, Zen mode, cloud sync.
- Web save has no automated test; verify it in a browser.

## Tests

```sh
cargo test -p accretion
```

126 unit + 2 integration + 6 headless scene tests pass;
`tests/balance.rs::click_rate_sensitivity` is ignored. `tests/save.rs` is a
native placeholder with 0 tests. Clippy is clean (native and wasm).

## Layout

```text
games/accretion/
├── (design doc → ember-vault)              full design + implementation status
├── src/
│   ├── main.rs            5 lines: scene::run()
│   ├── scene.rs           the Accretion root node (engine node tree)
│   ├── app.rs             screens, input, UI drawing (was main.rs)
│   ├── frame.rs           this frame's window size, pointer, clock
│   ├── systems.rs         Game state + pure tick/input
│   ├── economy.rs         12-building table, production
│   ├── paradigm.rs        5 transition conditions
│   ├── prestige.rs        G formula + reset
│   ├── endgame.rs         2 endings + auto-prestige
│   ├── disk.rs            procedural disk + rendering
│   ├── save.rs            RON native / localStorage web
│   └── ...
├── tests/balance.rs       greedy-AI balance report
└── tools/                 build-web.sh + localStorage_plugin.js
```

## Documentation

Design notes, session logs and release notes live in the Obsidian vault: `ember-vault/30 Games/Accretion/` (hub note: `Accretion.md`).
