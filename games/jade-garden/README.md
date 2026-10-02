# jade-garden

A contemplative match-3 puzzle set in an abandoned imperial garden.
Align the jades, wake the garden, recompose a Tang poem.

Inspired by *Bejeweled* (PopCap, 2001) for its match-3 core and
*Monument Valley* (ustwo, 2014) for its contemplative pacing.

**▶ Play in your browser:** [ember-workspace.itch.io/jade-garden-web](https://ember-workspace.itch.io/jade-garden-web)
— no download, no install. A native macOS build is also available
at [ember-workspace.itch.io/jade-garden](https://ember-workspace.itch.io/jade-garden).

## Running

### Native

```sh
cargo run --release -p jade-garden
```

For a standalone build, the binary must sit next to an `assets/`
folder:

```
jade-garden          ← the executable
assets/
├── levels.ron
├── chapitres.ron
├── DejaVuSansMono.ttf
├── NotoSansSC-JadeGarden.otf
└── sonican-*.mp3
```

In dev (`cargo run`), assets are resolved from the source tree
automatically.

### Web (WASM)

```sh
./tools/build-web.sh
```

Produces `dist/web/`:

```
dist/web/
├── index.html
├── jade-garden.wasm
├── mq_js_bundle.js
└── assets/
```

Serve it locally with any static server:

```sh
cd dist/web && python3 -m http.server 8080
# → http://localhost:8080
```

On the web, the first click unlocks audio (browser autoplay policy);
after that, everything behaves normally.

## Controls

| Input | Action |
|---|---|
| Click | Select / swap a jade |
| H | Show a hint (3 per game) |
| Tab / F1 | Open / close the in-game help |
| Enter | Continue / confirm |
| R | Replay (from a won or lost screen) |
| Esc | Back to menu |
| Wheel / ↑↓ | Scroll the menu |
| Ctrl (held) | Reveal debug buttons (dev builds only) |

## The six jades

Each type has a distinct silhouette so it reads at a glance even
without colour:

| Shape | Name | Chinese | Meaning |
|---|---|---|---|
| Round | Bai | 白玉 | Purity, mourning |
| Hexagon | Bi | 碧玉 | Harmony, growth |
| Diamond | Qing | 青玉 | Spirit, immortality |
| Octagon | Hong | 红玉 | Joy, celebration |
| Pentagon | Huang | 黄玉 | Earth, prosperity |
| Square | Mo | 墨玉 | Mystery, ink |

## Objectives

Three objective types appear across the campaign:

- **Score N** — reach N points.
- **Clear N jade** — remove N jades of a given type.
- **Trigger N cascades** — chain N cascade reactions. The first
  match after a swap is free; only the cascades that follow count.

## Story

Twelve levels, four chapters, one Tang poem — 《春晓》
(*Spring Dawn*) by Meng Haoran — revealed one character at a time.
After the final level: an epilogue. Between September and November,
a Halloween teaser plays once, the first time you finish the
campaign in that window.

### Speedrunning

Every level is deterministic — the board is generated from a fixed
seed, so two players see the same jades in the same order. The game
tracks your fastest time to each star tier (1★, 2★, 3★) and the
moment of your peak score, and shows them in the level select menu.
The HUD also has a live timer (top-right) for score runs and time
runs.

## Audio backends

### Native

Music is played by trying, in order:

1. **soloud** (bundled, no external runtime).
2. **OS-native player**: `afplay` on macOS, `ffplay` or `mpv` on
   Linux/Windows.

If neither works, the game continues silently. To install a
fallback on Linux: `apt install ffmpeg` (or `mpv`).

### Web

Music is played via `macroquad::audio` (backed by quad-snd →
WebAudio). Tracks are preloaded asynchronously before the main
loop, then played synchronously. MP3 and WAV work in every modern
browser; OGG is not supported by Safari.

## Save file

### Native

Player progress lives in `progress.ron`, in the platform user-data
directory:

- macOS: `~/Library/Application Support/jade-garden/progress.ron`
- Linux: `$XDG_DATA_HOME/jade-garden/` or
  `~/.local/share/jade-garden/`
- Windows: `%APPDATA%\jade-garden\progress.ron`

Override with `$JADE_GARDEN_PROGRESS` if you need a portable setup.

### Web

Progress is **kept in memory** for the current browser tab. Best
scores, stars, and poem fragments reset when the tab is closed.
A `localStorage` bridge is planned but not yet shipped.

## Development

```sh
# Unit + integration tests
cargo test -p jade-garden

# Balance report (slow — run in release)
cargo test --release -p jade-garden --test balance -- --nocapture

# Objective feasibility
cargo test --release -p jade-garden --test objectives -- --nocapture

# Seed scanner (ignored by default, can take a while)
cargo test --release -p jade-garden --test objectives -- --ignored --nocapture

# Web bundle
./tools/build-web.sh
```

### Regenerating the CJK font subset

The game ships with a subset of Noto Sans SC containing only the 20
poem characters, the title, the author name and the six jade names.
To regenerate it:

```sh
./subset_font.sh
```

Requires `fonttools` (`pip install fonttools`) and `curl`.

### Regenerating the itch.io media

Cover (630×500) and banner (960×400) live as SVG under
`tools/itch-media/`. Regenerate the PNGs with:

```sh
./tools/itch-media/build-media.sh
```

The script uses a Chromium-based browser in headless mode (Chrome,
Brave, Edge, Chromium, Arc) for exact-size rendering, with
`rsvg-convert` and `cairosvg` as fallbacks.

### Colour overrides

In dev builds, the `.env` file can override the palette via
`COLOR_*` variables (e.g. `COLOR_GOLD_R=0.9`). This is a dev tool
only — the release build always uses the compiled-in defaults.

## Credits

See [CREDITS.md](CREDITS.md) for music, fonts, and third-party
attributions.

## License

Same as the surrounding `ember-workspace`. Third-party assets have
their own terms — see `CREDITS.md`.