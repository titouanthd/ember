# 23 — `ember_render_macroquad`

> **Status:** 🟡 partial — reviewed 2026-11-19; renderer, fonts, camera offset, `run`/`run_window`, input polling and audio backend work; z-sorted batching, shaders and draw-call counting are not implemented.
> **Audience:** implementers
> **Prerequisites:** [22-ember-servers.md](22-ember-servers.md)

## 1. Purpose

Specify the reference `RenderingServer` implementation on
macroquad/miniquad. This is the crate that actually talks to the GPU.
It is the **only** crate in the workspace allowed to import
`macroquad::prelude` (see [02-principles I2](02-principles.md#i2--only-the-_macroquad-crates-depend-on-macroquad)).

## 2. Module layout

```
ember-render/macroquad/          (crate `ember-render-macroquad`)
├── Cargo.toml
├── src/
│   ├── lib.rs        re-exports (MacroquadRenderer, poll_input, run, run_window, servers, macroquad)
│   ├── renderer.rs   MacroquadRenderer (RenderingServer impl)
│   ├── convert.rs    Color/Rect <-> macroquad conversions
│   ├── input.rs      poll_input() -> ember_input::Input
│   └── run.rs        run / run_window: the native/wasm main loop driver
└── tests/headless.rs
```

*Not yet implemented:* `textures.rs`, `batching.rs`, `shaders.rs`, `error.rs`
(there is no `RenderError`; failures use `ServerError`). The audio
counterpart is `ember-audio/macroquad` (`MacroquadAudio`, §14).

## 3. `MacroquadRenderer`

### 3.1 Struct

Private fields: viewport, camera offset, current z, clear colour, a
`HashMap<TextureId, Texture2D>` and `HashMap<FontId, Font>` with
monotonic ids starting at 1, and the optional default font.

### 3.2 Constructor

```rust
impl MacroquadRenderer {
    pub fn new() -> Self;                    // also `Default`
    /// Register a font already loaded elsewhere (wasm path, ADR-020).
    pub fn adopt_font(&mut self, font: Font) -> FontId;
}
```

The constructor does no GPU work; it is a plain struct, not a boxed
trait object. Create it inside the macroquad context (`run_window`
does this for you).

### 3.3 Trait impl strategy

Every method maps to a small macroquad call. The renderer adds the
camera offset (§4) and texture/font lookup (§6, §9).

## 4. Camera and coordinates

`set_camera(offset)` stores the offset; every `draw_*` call **adds** it
to its position (`rect.x + offset.x`, …). `draw_rect_rotated` rotates
around the rect centre (pivot 0.5, 0.5).

```rust
fn draw_rect(&mut self, rect: Rect, color: Color) {
    let o = self.camera_offset;
    macroquad::shapes::draw_rectangle(rect.x + o.x, rect.y + o.y, rect.w, rect.h, color_to_mq(color));
}
```

### 4.2 Why offset (not matrix)

macroquad's immediate-mode API uses screen coordinates. Applying an
offset per call is cheaper than setting up a camera matrix on every
draw. v0.1 is translate-only; zoom/rotation of the whole view is not
supported.

## 5. Z-ordered batching

*Not yet implemented.* The planned renderer buffers draw commands between
`begin_frame` and `end_frame`, stable-sorts them by `(z, sequence)` and
coalesces adjacent sprites/rects. In v0.1 the renderer draws
**immediately in call order**: `begin_frame`/`end_frame` are markers and
`set_z` only stores the value (readable via `current_z`). Visual layering
therefore follows tree order, not `z`.

## 6. Texture management

### 6.1 `create_texture`

Uploads an `ember_core::Image` via `Texture2D::from_rgba8`. An empty image
returns `ServerError::Backend`. Ids start at 1.

### 6.2 `destroy_texture`

Removes the texture from the map (the GPU texture is freed when the
`Texture2D` drops). The lifecycle is explicit: nothing destroys textures
automatically (see [25-ember-assets.md](25-ember-assets.md)).

### 6.3 `texture_size`

*Not yet implemented* (not in the trait). Use `Texture::{width,height}`
from `ember-assets`.

### 6.4 Unknown ids

`draw_sprite` with an unknown or destroyed texture is a silent no-op.
Each sprite is one macroquad `draw_texture_ex` call (no batching).

## 7. Shaders

*Not yet implemented.* There is no `register_shader`/`use_shader` and no
built-in shader set. `RenderFeature::CustomShaders` is reported by
`supports` as a capability flag only.

## 8. Draw-call counting

*Not yet implemented* (`draw_call_count` is not in the trait).

## 9. Text rendering

`draw_text` uses the default font (macroquad's built-in, or the one set
via `set_default_font`). `draw_text_with_font(font, …)` uses a font from
`create_font` / `adopt_font`; an unknown font is a silent no-op.
`measure_text(_with_font)` returns `TextMetrics { width, height,
offset_y }`; an unknown font falls back to the default. `destroy_font`
resets the default font if it was that font.

`create_font` is blocking on native and returns `ServerError::NotSupported`
on wasm, where games load the font asynchronously and call `adopt_font`
(ADR-020).

## 10. Color conversion

`convert.rs` has internal helpers `color_to_mq`, `rect_to_mq` and their
inverses; there are no public `From` impls. `ember_core::Color` is
field-identical to macroquad's.

## 11. Feature support

| Feature | Supported | Notes |
|---|---|---|
| `AlphaBlending` | ✅ | Default in macroquad |
| `CustomShaders` | ✅ (flag) | No shader API exists yet |
| `TextureArray` | ❌ | |
| `Instancing` | ❌ | |

`RenderToTexture` is not a `RenderFeature` variant yet.

## 12. Frame lifecycle and `run`

```rust
pub async fn run<L: MainLoop + InputSink>(game: L);
pub fn run_window<L: MainLoop + InputSink + 'static>(title: &str, width: i32, height: i32, game: L);
```

`run_window` builds the macroquad `Window` and calls `run`. Per frame,
`run` does:

1. `poll_input()` → `game.set_input(..)`.
2. A fixed-step accumulator: frame dt is capped at `MAX_FRAME_DT` (0.25)
   and `process(dt)` / `physics_process(FIXED_DT = 1/60)` run as needed
   (`process` returning `false` quits).
3. `game.draw()` (for `SceneLoop`: `begin_frame`, `tree.draw`, `end_frame`,
   `audio.flush`), then `next_frame().await`.
4. On exit, `game.finalize()`.

`input.rs::poll_input` builds an `ember_input::Input` from macroquad
(keys A–Z, 0–9, F1–F12, arrows, Space, Enter, Escape, Tab, Backspace;
mouse L/R/M; touches; **no gamepad**).

## 13. Error handling

The fallible operations are `create_texture` (empty image) and
`create_font` (bad bytes, or `NotSupported` on wasm). Everything else is
infallible (draw calls with unknown ids are silently skipped, matching
[02-principles §5.4](02-principles.md#54-what-is-never-an-error)).

## 14. `MacroquadAudio` (`ember-audio/macroquad`)

Implements `AudioServer`: `new()`, `adopt_clip(Sound) -> AudioClipId`,
`load_clip` (blocking on native, `NotSupported` on wasm, ADR-020) and
`destroy_clip`. `play` clamps volume to 0..1, multiplies by the master
volume, **ignores pitch** and never loops; an unknown clip still returns
a `SoundId`. `stop_all` stops every clip, `flush` is a no-op,
`supports` is always `false`.

## 15. Design notes

### 15.1 Why not use macroquad's camera

macroquad's `set_camera2d` is a full camera transform. Most games use
translate-only; managing the offset in the renderer keeps the trait's
`set_camera(offset: Vec2)` simple.

### 15.2 Why copy images instead of zero-copy

`Texture2D::from_rgba8` copies the bytes; negligible for v0.1 asset sizes.

## 16. Open questions

- **Q1:** Should the renderer support a simple "scissor rectangle" for
  UI clipping? *Provisional:* yes — see [24-ember-ui.md](24-ember-ui.md).
  Would add `push_clip_rect` / `pop_clip_rect` (not in the trait yet).
- **Q2:** Should the renderer cache text layout? *Provisional:* no in
  v0.1. Text layout is cheap and caching risks invalidation bugs.
- **Q3:** Should the batcher use a radix sort? *Provisional:* only if
  profiling shows sort cost. v0.1 does no sorting (§5).
- **Q4:** Should we expose `draw_call_count` per frame or cumulative?
  *Provisional:* per-frame, reset at `begin_frame` (not implemented).

## 17. Cross-references

- Server trait being implemented → [22-ember-servers.md §4](22-ember-servers.md#4-renderingserver)
- Why batched draws → [12-servers.md](12-servers.md)
- Texture lifecycle → [25-ember-assets.md](25-ember-assets.md)
- The UI layer that uses clipping → [24-ember-ui.md](24-ember-ui.md)