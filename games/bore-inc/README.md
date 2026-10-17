# bore-inc

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

See `DESIGN.md` §19 for the technical stack, §22 for scope.

```
src/
  main.rs        — entry point, preload, main loop
  lib.rs         — module list + reexports
  paths.rs       — runtime asset / progress path resolution
  config.rs      — GameContext, palette
  components.rs  — TileKind, ResourceKind, Tile
  systems.rs     — AppState (the single state struct)
  audio.rs       — MusicPlayer (native + wasm)
  progress.rs    — save / load (RON native, in-memory wasm)
  fonts.rs       — font loading
```

## Status

Starter kit. Compiles, runs, opens a window, switches between screens.
**The gameplay (mining, truck, robots, HQ, map) is not implemented
yet** — Session 2 onward.