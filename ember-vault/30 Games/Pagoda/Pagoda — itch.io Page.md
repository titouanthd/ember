---
title: "Pagoda — itch.io Page"
tags: ["ember/game", "game/pagoda", "ember/release"]
game: "Pagoda"
created: "2026-11-26"
status: "ready to upload"
---

# Pagoda — itch.io Page

Everything needed to create Pagoda's itch.io page, prepared with the
[[itch.io Release Media]] workflow (the one [[Jade Garden]] used). Nothing has
been uploaded yet.

## 1. Files to upload

All build output, git-ignored, regenerable from `games/pagoda/`:

| File | Use | Regenerate |
|---|---|---|
| `dist/pagoda-web.zip` (~850 KB) | HTML5 build, `index.html` at the zip root | `./tools/build-web.sh`, then `cd dist/web && zip -r ../pagoda-web.zip index.html pagoda.wasm mq_js_bundle.js assets` |
| `dist/itch-media/cover-630x500.png` | Cover image (browse grid) | `./tools/itch-media/build-media.sh` |
| `dist/itch-media/cover-1260x1000.png` | Same cover @2×, upload this one if itch accepts it | same |
| `dist/itch-media/banner-960x400.png` | Page header banner | same |
| `dist/itch-media/screenshots/01…05-*.png` | Screenshots, 1200×900 | `node tools/itch-media/capture-screenshots.mjs` |

Sources committed under `games/pagoda/tools/itch-media/`:

- `cover.svg`: the night register. Lacquer signboard with the `塔` seal, a
  tower climbing celadon → turquoise → indigo → cinnabar, a sliced overhang
  falling, the sliding tile inside the gold perfect window, floating `完美 ×3`.
- `banner.svg`: the rice-paper register. Ink-outlined tower, vermilion sun,
  shan-shui ridges, the title on the left.
- `build-media.sh`: copied unchanged from Jade Garden.
- `capture-screenshots.mjs`: new. It serves `dist/web/`, runs headless Chrome
  and lets an in-page bot play the real build. The bot reads one WebGL pixel
  row through the slide and one through the top tile, predicts the next frame
  and drops on the centre, so the screenshots show a real 30+ perfect run.
  Shots: menu, perfect combo, 30-tier tower, slice, game over / new best.

## 2. Project settings

| Field | Value |
|---|---|
| Title | **Pagoda** |
| Project URL | `ember-workspace.itch.io/pagoda` |
| Short description / tagline | Drop lacquered tiles, slice the overhang, chase the perfect landing. A one-button stacker under an ink-wash moon. |
| Classification | Games |
| Kind of project | HTML |
| Release status | Released |
| Pricing | *Your call*: no payments, or donations accepted |
| Uploads | `pagoda-web.zip`, tick **This file will be played in the browser** |
| Embed options | **Click to launch in fullscreen** (unlocks audio on the first gesture) |
| Viewport | leave blank; macroquad letterboxes the 800×600 canvas |
| Mobile friendly | ✅, orientation **Landscape** (portrait works, with a rotate hint) |
| Frame options | ✅ Fullscreen button |
| Genre | Arcade |
| Tags (max 10) | `arcade`, `one-button`, `casual`, `stacking`, `high-score`, `minimalist`, `chinese`, `touch`, `short`, `score-attack` |
| App store links | none |
| Custom noun | leave empty |
| Made with | Rust, macroquad |
| Average session | A few minutes |
| Languages | English (Chinese decorative glyphs only) |
| Inputs | Keyboard, Mouse, Touchscreen |
| Accessibility | One button; no colour-only information (the guide shape also changes) |
| Community | Comments |
| Visibility | Draft → Public once the embed is tested |

Page theme (Edit theme), from `config.rs`:

| Slot | Colour |
|---|---|
| Background | `#0B0712` (the `web/index.html` theme colour) |
| Inner background | `#161D28` (far ridge ink) |
| Text | `#F2E8CC` (ivory) |
| Links | `#FFD140` (perfect gold) |
| Buttons | `#DB3D2B` (cinnabar) |
| Header | banner image, title hidden (the banner carries it) |

## 3. Description (paste into the page body)

```markdown
**Drop. Slice. Climb.**

A lacquered tile slides across the night sky. Tap once to drop it onto the
pagoda. Whatever hangs over the edge is sliced away, and the next tile is
only as wide as what's left.

Land dead centre for a **完美 / PERFECT**: the tile keeps its full width, the
combo climbs and a pentatonic chime rises with it. Miss completely and the
tower rests.

The higher you build, the faster the tiles slide, and the pagoda's lacquer
turns from celadon to turquoise, indigo, cinnabar and gold.

### Controls

| Input | Action |
|---|---|
| Click / tap / Space / Enter | Drop the tile · start · rebuild |
| Speaker icon (top-left) | Mute / unmute |

A plumb line and target diamond follow the sliding tile. They turn **gold**
when a drop would be perfect.

### Notes

- Plays on desktop and mobile browsers. Landscape is best on phones.
- Your best score lasts for the session. It resets when the tab is closed.
- All sound effects are generated procedurally. No samples, no music.

### Credits

Made with Rust and [macroquad](https://macroquad.rs) as part of the Ember
workspace.
Fonts: DejaVu Sans Mono (permissive licence) and a subset of Noto Sans SC
(SIL Open Font License 1.1).
```

## 4. Upload checklist

1. [ ] `./tools/build-web.sh` on the commit you're releasing; re-zip.
2. [ ] `./tools/itch-media/build-media.sh` and
       `node tools/itch-media/capture-screenshots.mjs`; eyeball the PNGs.
3. [ ] Create the project with §2's settings (start as **Draft**).
4. [ ] Upload the zip, mark it playable in the browser, set the embed options.
5. [ ] Cover, banner, the 5 screenshots; paste §3; set the theme.
6. [ ] Test the draft page on desktop (Chrome + Safari) and on a phone:
       first tap starts audio, taps never scroll or zoom the page.
7. [ ] Publish, then put the URL in [[Pagoda]], `games/pagoda/README.md`, and
       §8 of [[Browser Build (WASM)]].

## 5. Open decisions

- **One page or two?** Jade Garden used two projects (native `jade-garden` +
  `jade-garden-web`). For Pagoda, one HTML page is recommended: it is
  mobile-first and has no native-only features. A macOS zip can be added
  later as an extra download on the same page.
- **Pricing.** Free vs. donations, left to you.
- **Web save.** The best score is in memory only. The page says so. A
  `localStorage` bridge (pattern in `games/accretion`) would be a better
  first impression but isn't needed for the release.
