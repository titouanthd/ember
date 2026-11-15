# bore-inc

**Status: abandoned prototype** — playable mining/truck/HQ loop, 223 tests,
native + WASM. Robots, map/intro screens, and the level-complete objective
were never finished. See **Status** below.

An incremental mining game. Archie the cat drills a tunnel through a
mountain with robot employees, so the Bore Inc. truck can cross.

Inspired by *Sandustry* (rendering), *Motherload* (mining economy),
*Cookie Clicker* / *Adventure Capitalist* (incremental loop),
*SteamWorld Dig* (digging), *Factorio* (automation), *Overcooked*
(top-down map).

**Restriction**: movement input only. No menus, no clicking, no mouse.
See `DESIGN.md` §2.

## Running

### Native

```sh
cargo run --release -p bore-inc
```

### Web (WASM)

```sh
./tools/build-web.sh
cd dist/web && python3 -m http.server 8080
# → http://localhost:8080
```

## Controls

Movement only.

| Input | Action |
|---|---|
| Left / Right | Move Archie |
| Space | Jump |
| Push against a solid | Mine it (hold to keep mining) |
| Push against a button / door | Activate (deposit bag, enter truck) |
| Left / Right inside the truck | Drive |
| Jump on an upgrade block | Buy it |

Any movement key starts the game from the title screen.

## Assets

Drop a `DejaVuSansMono.ttf` into `assets/` (copy it from another
Ember game). The game falls back to macroquad's default font if
missing — no crash, just a warning.

## Structure

See `DESIGN.md` for the design intent; the code has grown well beyond the
Session 2 starter kit.

```
src/
  main.rs        — entry point, preload, main loop
  lib.rs         — module list + reexports
  paths.rs       — runtime asset / progress path resolution
  config.rs      — GameContext, palette
  components.rs  — TileKind, ResourceKind, Tile
  systems.rs     — AppState + Game (app shell + world simulation)
  world.rs       — tile grid, collision, mining, delta persistence
  levelgen.rs    — procedural mountain generation
  player.rs      — Archie movement, mining, jump
  bag.rs         — bag capacity + deposits
  truck.rs       — drivable truck, storage, tunnel constraints
  drop.rs        — resource drops
  hq.rs          — HQ / upgrade blocks / cargo
  upgrade.rs     — upgrade definitions + effects
  hud.rs         — in-game HUD
  input.rs       — movement-only input mapping
  atlas.rs       — procedural RGBA atlas
  pixel.rs       — pixel-perfect camera/sprite helpers
  background.rs  — parallax backdrop
  sprites/       — procedural Archie / truck sprite generators
  audio.rs       — MusicPlayer (native + wasm)
  progress.rs    — save / load (RON native, in-memory wasm)
  rng.rs         — deterministic level seeds
  fonts.rs       — font loading
```

## Status

**Playable prototype, abandoned as a jam project.** Implemented: mining by
movement, resource drops, bag capacity, the drivable truck and its storage,
HQ selling and upgrades, procedural levels with deterministic seeds, delta
persistence, pixel-perfect sprites, and a web/WASM build. 223 tests pass
(202 unit + 21 integration).

**Incomplete:** robot employees (only capsule tiles and unlock flags exist),
the map screen (`MAP` is a placeholder), the intro screen (`INTRO` is a
placeholder), and the full level-completion loop. The design doc's "get the
truck to the finish line" objective is not yet a win condition.

The game still compiles for wasm and remains a useful second reference port
(see `BROWSER_BUILD.md`), so do not delete it while it is a workspace member
— but expect it to break first on macroquad/glam bumps.