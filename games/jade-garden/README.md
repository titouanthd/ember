# jade-garden

A contemplative match-3 puzzle set in an abandoned imperial garden.
Align the jades, wake the garden, recompose a Tang poem.

Inspired by *Bejeweled* (PopCap, 2001) for its match-3 core and
*Monument Valley* (ustwo, 2014) for its contemplative pacing.

## Running

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
- **Clear N <jade>** — remove N jades of a given type.
- **Trigger N cascades** — chain N cascade reactions. The first
  match after a swap is free; only the cascades that follow count.

## Story

Twelve levels, four chapters, one Tang poem — 《春晓》
(*Spring Dawn*) by Meng Haoran — revealed one character at a time.
After the final level: an epilogue. Between September and November,
a Halloween teaser plays once, the first time you finish the
campaign in that window.

## Audio backends

Music is played by trying, in order:

1. **soloud** (bundled, no external runtime).
2. **OS-native player**: `afplay` on macOS, `ffplay` or `mpv` on
   Linux/Windows.

If neither works, the game continues silently. To install a
fallback on Linux: `apt install ffmpeg` (or `mpv`).

## Save file

Player progress lives in `progress.ron`, in the platform user-data
directory:

- macOS: `~/Library/Application Support/jade-garden/progress.ron`
- Linux: `$XDG_DATA_HOME/jade-garden/` or
  `~/.local/share/jade-garden/`
- Windows: `%APPDATA%\jade-garden\progress.ron`

Override with `$JADE_GARDEN_PROGRESS` if you need a portable setup.

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
```

### Regenerating the CJK font subset

The game ships with a subset of Noto Sans SC containing only the 20
poem characters, the title, the author name and the six jade names.
To regenerate it:

```sh
./subset_font.sh
```

Requires `fonttools` (`pip install fonttools`) and `curl`.

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