---
title: "itch.io Release Media"
aliases: ["itch.io media", "itch.io page assets", "Store page assets"]
tags: ["ember/workflow", "ember/release"]
created: "2026-11-26"
---

# itch.io Release Media

How the store-page media for an Ember game is produced, reverse-engineered
from the [[Jade Garden]] v0.1.0 release (commits `bd46c5a` and `7777cde`,
Oct 2026), and the checklist to reuse it. [[Pagoda]] is the second game to
follow it — see [[Pagoda — itch.io Page]].

## 1. What Jade Garden shipped

Two itch.io projects under the `ember-workspace` account:

| Project | Kind | Upload |
|---|---|---|
| `ember-workspace.itch.io/jade-garden` | Downloadable | `jade-garden-osx.zip` (binary + `assets/`) |
| `ember-workspace.itch.io/jade-garden-web` | HTML | `jade-garden-web.zip` (`dist/web/` contents) |

Page media:

| Asset | Size | Source | Look |
|---|---|---|---|
| Cover | 630×500 (+ 1260×1000 @2×) | `tools/itch-media/cover.svg` | Light rice paper, shan-shui ridges, one hexagonal jade hero, red seal, serif title |
| Banner | 960×400 | `tools/itch-media/banner.svg` | Night sky, moon gate, title left-aligned, minimalist |
| Screenshots | window size | captured by hand into `screenshots/` (git-ignored) | in-game |
| App icon | 1024 → `.icns` + `assets/icon-256.png` | `tools/icon.svg` via `tools/build-icon.sh` | native build only |

The cover and banner deliberately use **two registers** (light paper vs.
night) so the browse grid and the page header don't look like the same image.

## 2. How the media is made

Everything is **procedural SVG, hand-written, committed**. PNGs are build
output, never committed.

```text
games/<game>/tools/itch-media/
├── cover.svg        viewBox 0 0 630 500, no width/height attributes
├── banner.svg       viewBox 0 0 960 400, no width/height attributes
└── build-media.sh   SVG → PNG into dist/itch-media/ (git-ignored via /dist/)
```

`build-media.sh` renders each SVG at an exact size, trying in order:

1. **Chromium headless** (Chrome, Brave, Edge, Chromium, Arc on macOS):
   `--headless --screenshot=<png> --window-size=W,H file://<svg>`.
   Exact pixels, no padding, system fonts available.
2. `rsvg-convert -w W -h H` (librsvg).
3. `cairosvg` (Python).

It then prints each PNG's dimensions with `sips` as a sanity check.

Colours are taken from the game's palette (`config.rs`) so the page matches
the game; text uses system serif stacks (`'Playfair Display','Didot','Bodoni 72',Georgia,serif`
for Latin, `'Songti SC','STSong','PingFang SC',serif` for CJK).

## 3. Lessons (keep these)

- **Use Chrome headless, not cairosvg.** The first version (`bd46c5a`)
  tried rsvg → cairosvg → inkscape → qlmanage; cairosvg pads SVGs that carry
  explicit `width`/`height` with white. `7777cde` switched to Chromium first.
- **Omit `width`/`height` on the root `<svg>`**, keep only `viewBox`, so every
  converter scales to the requested size.
- **Don't capture screenshots with `get_screen_data()`** on macOS/Metal: it is
  unimplemented in miniquad and yields black PNGs. Use the OS screenshot tool
  or a browser screenshot of the web build.
- **Fonts in SVG are the system's.** Chrome on macOS resolves `Songti SC` /
  `PingFang SC`; on another OS the CJK falls back. Render on macOS.
- **Zip the contents, not the folder** — `index.html` at the zip root (see
  [[Browser Build (WASM)#6. Publishing to Itch.io|Publishing to Itch.io]]).
- HTML embed: **Click to launch in fullscreen**, viewport left blank. It also
  gives the browser the user gesture that unlocks audio.
- macOS downloads hit Gatekeeper; say so on the page (right-click → Open).

## 4. Checklist for the next game

1. Copy `games/jade-garden/tools/itch-media/build-media.sh` and write a
   `cover.svg` (630×500) + `banner.svg` (960×400) from the game's palette.
2. Make sure `/dist/` is in the game's `.gitignore`.
3. `./tools/itch-media/build-media.sh` → check `dist/itch-media/*.png`.
4. Capture 3–5 screenshots (menu, mid-run, a juicy moment, game over).
5. `./tools/build-web.sh`, zip `dist/web/` contents.
6. Write the page copy in a vault note `<Game> — itch.io Page` (the docs
   guard forbids `.md` outside the vault).
7. Create the itch project, upload, set embed options, attach media, publish.
8. Record the URLs in the game hub and the README.
