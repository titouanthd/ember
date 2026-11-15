# Ember — Browser (WASM) Build Guide

Definitive guide for compiling and publishing any Ember game to the browser
(`wasm32-unknown-unknown`), based on the four working implementations:
`games/jade-garden`, `games/bore-inc`, `games/accretion`, and
`games/pagoda`.

**Target Audience:** the next agent / developer shipping a browser game from
this workspace.

**Constraint:** this workspace targets WASM **without** `wasm-bindgen`. We use
`macroquad` → `miniquad`'s native JS imports (`env.console_log`, etc.).

---

## 1. Prerequisites

```bash
rustup target add wasm32-unknown-unknown
```

The `tools/build-web.sh` scripts auto-install the target if it is missing,
but installing it once avoids a spurious first-run build failure.

### Dependency target-gating (mandatory)

Any dependency that cannot compile for `wasm32-unknown-unknown` **must** be
placed under a native-only section of the game's `Cargo.toml`. This is the
pattern all four browser-targeted games use:

```toml
[package]
name = "my-game"
version.workspace = true
edition.workspace = true

[dependencies]
ember-core.workspace = true
ember-stdlib.workspace = true
macroquad.workspace = true
glam.workspace = true
serde.workspace = true
ron.workspace = true

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
dotenvy.workspace = true
# soloud = "1.1"                                       # native C++ audio
# image = { version = "0.25", default-features = false, features = ["png"] }

[dev-dependencies]
pollster = "0.4"

[lib]
name = "my_game"               # underscores in code, dashes in the package name
path = "src/lib.rs"

[[bin]]
name = "my-game"
path = "src/main.rs"
```

Why this matters: `Cargo.lock` is workspace-wide, so native-only crates like
`soloud`, `dotenvy`, and `image` *will* appear in the lockfile when any other
crate uses them. That is **not** a problem — the `[target.'cfg(not(...))']`
section is what keeps them out of the `wasm32` dependency graph. Verify with:

```bash
cargo tree -p my-game --target wasm32-unknown-unknown | grep -E 'soloud|dotenvy|image'
# → must produce no output
```

`pollster` is `[dev-dependencies]` only (used by `#[test]` async harnesses),
so it never reaches the runtime binary on any target.

### Crucial dependency rule

**NEVER** add `wasm-bindgen`, `web-sys`, or `wasm-bindgen-futures` to a game's
`Cargo.toml`. `miniquad` has its own WASM import namespace; `wasm-bindgen`
generates `__wbindgen_placeholder__`, which miniquad does not define. The
module will fail to load in the browser with a `LinkError`.

---

## 2. Code architecture requirements

### A. Path resolution (`paths.rs`)

Browsers have no filesystem. Assets are fetched relative to `index.html`.

```rust
#[cfg(target_arch = "wasm32")]
pub fn assets_dir() -> PathBuf {
    PathBuf::from("assets")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn assets_dir() -> PathBuf {
    // native filesystem logic, e.g. CARGO_MANIFEST_DIR-relative
}
```

### B. Time tracking

`std::time::Instant::now()` is unavailable on `wasm32-unknown-unknown` and
will panic (`unreachable`). Accumulate an `f32` from `dt` instead
(see `games/jade-garden/src/audio.rs`).

### C. Async asset preloading

On the web, `macroquad::audio::load_sound` and `macroquad::text::load_ttf_font`
perform HTTP fetches. They **must** be `.await`ed before the game loop starts.

```rust
#[macroquad::main(window_conf)]
async fn main() {
    // 1. Preload fonts
    let font = load_ttf_font("assets/font.ttf").await.unwrap();

    // 2. Preload audio
    let sound = load_sound("assets/music.mp3").await.unwrap();

    // 3. Start loop
    loop {
        // ... game logic ...
        next_frame().await;
    }
}
```

### D. Progression / save files

There are two supported strategies. Choose deliberately:

1. **In-memory only** (jade-garden, pagoda): progress survives scene changes
   but not a page reload. Use a `thread_local` buffer on `wasm32`.

   ```rust
   #[cfg(target_arch = "wasm32")]
   thread_local! {
       static IN_MEMORY_PROGRESS: RefCell<Option<String>> = const { RefCell::new(None) };
   }
   ```

2. **Real `localStorage`** (accretion): a miniquad plugin exposes three C
   functions, and the Rust side calls them through `unsafe extern "C"`. This
   is the only option that survives reloads without `wasm-bindgen`.

   * JS side: `games/accretion/tools/localStorage_plugin.js`, loaded **after**
     `mq_js_bundle.js` (it extends `LibraryManager.library`).
   * Rust side: `games/accretion/src/save.rs`, with `#[cfg(target_arch =
     "wasm32")]` FFI declarations and a native `save.ron` fallback.
   * Key names must be prefixed per game (`accretion:save`) because several
     Ember games share the same itch.io origin.
   * Read buffers are bounded (`MAX_WEB_SAVE_BYTES = 100 KB`) and
     null-terminated; truncate to the returned length before parsing RON.

   ```rust
   #[cfg(target_arch = "wasm32")]
   unsafe extern "C" {
       fn ember_storage_get(key: *const u8, out_buf: *mut u8, out_buf_len: usize) -> i32;
       fn ember_storage_set(key: *const u8, value: *const u8) -> i32;
       fn ember_storage_remove(key: *const u8) -> i32;
   }
   ```

The plugin is not yet extracted into a shared helper — copy
`tools/localStorage_plugin.js` and `save.rs` from accretion when porting the
pattern to another game (see §8).

### E. Procedural audio (pagoda)

`macroquad::audio::load_sound` performs an HTTP fetch on web and must be
preloaded before the loop (§2.C). Pagoda generates its clips offline with
`tools/generate-sounds.py` and ships them as WAV files:

* 44.1 kHz, mono, 16-bit PCM — `quad-snd` resamples anything else and short
  slices audibly detune.
* Every clip is optional at runtime: a missing or malformed WAV disables that
  event instead of panicking.
* The test suite parses the `fmt ` chunk and asserts the format, so a bad
  regeneration fails CI rather than shipping silently.
* Browser autoplay policy still applies: the first user gesture must unlock
  the audio context (§7).

---

## 3. The build script (`games/<game>/tools/build-web.sh`)

Every browser-targeted game has **its own** `tools/build-web.sh` at
`games/<game>/tools/build-web.sh` (not at the workspace root). The script
derives `CRATE_DIR` and `WORKSPACE_DIR` from its own location, so it works
for any game without modification.

The section that resolves `mq_js_bundle.js` is the **only** part that must
match byte-for-byte across games — it is what guarantees the JS glue matches
the compiled `macroquad` binary.

### 3.1 The shared `mq_js_bundle.js` resolution block

```bash
# ─── Resolve mq_js_bundle.js for the EXACT locked macroquad version ───
#
# The bundle must match the compiled macroquad binary byte-for-byte.
# We resolve the version from Cargo.lock (never hardcoded), and try:
#   1. the cargo registry (canonical source of truth),
#   2. a version-keyed cache (cannot go stale across a macroquad bump),
#   3. a CDN download of that exact version.
BUNDLE="$OUT/mq_js_bundle.js"

MQ_VERSION="$(awk '/^name = "macroquad"$/{getline; gsub(/[^0-9.]/,"",$3); print $3; exit}' \
    "$WORKSPACE_DIR/Cargo.lock")"

if [ -z "$MQ_VERSION" ]; then
    echo "ERROR: could not determine macroquad version from Cargo.lock"
    exit 1
fi

CACHE="$SCRIPT_DIR/.cache/mq_js_bundle-${MQ_VERSION}.js"
REGISTRY_BUNDLE="$(find "${HOME}/.cargo/registry/src" \
    -path "*macroquad-${MQ_VERSION}/js/mq_js_bundle.js" -type f 2>/dev/null | head -n 1 || true)"

if [ -n "$REGISTRY_BUNDLE" ] && [ -f "$REGISTRY_BUNDLE" ]; then
    echo "→ mq_js_bundle.js ← cargo registry (macroquad $MQ_VERSION)"
    echo "   $REGISTRY_BUNDLE"
    cp "$REGISTRY_BUNDLE" "$BUNDLE"
elif [ -f "$CACHE" ]; then
    echo "→ mq_js_bundle.js ← cache (macroquad $MQ_VERSION)"
    echo "   $CACHE"
    cp "$CACHE" "$BUNDLE"
else
    echo "→ mq_js_bundle.js ← download (macroquad $MQ_VERSION)"
    mkdir -p "$SCRIPT_DIR/.cache"
    if curl -L --fail --silent --show-error \
        -o "$CACHE" \
        "https://cdn.jsdelivr.net/gh/not-fl3/macroquad@${MQ_VERSION}/js/mq_js_bundle.js"; then
        cp "$CACHE" "$BUNDLE"
    else
        echo "ERROR: could not fetch mq_js_bundle.js for macroquad $MQ_VERSION."
        echo ""
        echo "Download it manually:"
        echo "  1. Go to https://github.com/not-fl3/macroquad"
        echo "  2. Navigate to 'js/mq_js_bundle.js' at tag $MQ_VERSION"
        echo "  3. Save it to: $CACHE"
        echo "  4. Re-run this script"
        exit 1
    fi
fi
```

**Do not** revert this to a hardcoded version or an unscoped
`find … '*macroquad-*' …`. The old form silently picked whichever macroquad
version happened to be first in `~/.cargo/registry/src/`, which is the exact
failure mode this guide's pitfall list used to warn about. The block above
makes that failure impossible (cache is version-keyed, registry path is
version-scoped) and diagnosable (every message names the resolved version).

### 3.2 Full template (`bore-inc`)

```bash
#!/usr/bin/env bash
# Builds the WebAssembly bundle for itch.io (HTML5 play).
#
# Output:
#   dist/web/
#     ├── index.html
#     ├── mq_js_bundle.js
#     ├── bore-inc.wasm
#     └── assets/
#
# Usage:
#     chmod +x tools/build-web.sh
#     ./tools/build-web.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKSPACE_DIR="$(cd "$CRATE_DIR/../.." && pwd)"

OUT="$CRATE_DIR/dist/web"
WASM_SRC="$WORKSPACE_DIR/target/wasm32-unknown-unknown/release/bore-inc.wasm"

# ─── 1. Ensure the wasm32 target is installed ───
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "→ Installing wasm32-unknown-unknown target…"
    rustup target add wasm32-unknown-unknown
fi

# ─── 2. Build the release binary for wasm ───
echo "→ Building bore-inc for wasm32…"
(cd "$WORKSPACE_DIR" && cargo build --release --target wasm32-unknown-unknown -p bore-inc)

if [ ! -f "$WASM_SRC" ]; then
    echo "ERROR: wasm binary not found at $WASM_SRC"
    exit 1
fi

# ─── 3. Fresh output directory ───
rm -rf "$OUT"
mkdir -p "$OUT"
cp "$WASM_SRC" "$OUT/bore-inc.wasm"

# ─── 4. Resolve mq_js_bundle.js ───  (see §3.1)

# ─── 5. HTML shell ───
cp "$CRATE_DIR/web/index.html" "$OUT/index.html"

# ─── 6. Assets (clean of dev files) ───
if [ -d "$CRATE_DIR/assets" ]; then
    cp -R "$CRATE_DIR/assets" "$OUT/assets"
    rm -f "$OUT/assets/.env"
    rm -f "$OUT/assets/progress.ron"
fi

# ─── 7. wasm-opt (optional) ───
if command -v wasm-opt >/dev/null 2>&1; then
    echo "→ Running wasm-opt…"
    wasm-opt -Oz "$OUT/bore-inc.wasm" -o "$OUT/bore-inc.wasm"
else
    echo "ℹ️  wasm-opt not found. Skipping optimization."
    echo "   Install with: brew install binaryen"
fi

# ─── 8. Report ───
echo ""
echo "✅ Web bundle ready: $OUT"
du -sh "$OUT"
ls -la "$OUT"
```

### 3.3 Per-game divergence (allowed)

Only the following differ between games. **Nothing else should.**

| Aspect | `jade-garden` | `bore-inc` | `accretion` | `pagoda` |
|---|---|---|---|---|
| Wasm binary name | `jade-garden.wasm` | `bore-inc.wasm` | `accretion.wasm` | `pagoda.wasm` |
| Crate name in `cargo build -p` | `jade-garden` | `bore-inc` | `accretion` | `pagoda` |
| Extra copied file | — | — | `localStorage_plugin.js` | — |
| Extra asset stripping | `NotoSansSC-Regular.{otf,ttf}`, `icon-256.png` | — | — | — |
| Audio assets | `music.mp3` + SFX | — | — | procedural `sfx/*.wav` |
| Save strategy | in-memory | none | `localStorage` | in-memory |

When porting a new game, copy the closest script and change only those
columns. Do **not** re-derive `mq_js_bundle.js` resolution by hand.

Four copies of the resolver now exist. Extracting it into
`tools/fetch-mq-bundle.sh` is overdue (Rule of Three: 4x), but was deferred
until the ports stopped changing. Until it is extracted, any fix to §3.1 must
be applied to all four scripts.

---

## 4. The HTML shell (`web/index.html`)

Macroquad requires a specific setup to initialize WebGL and map input events.

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Your Game</title>
    <style>
        html, body { margin: 0; padding: 0; height: 100%; background: #000; overflow: hidden; }
        #glcanvas { display: block; width: 100vw; height: 100vh; outline: none; }
        #error { display: none; padding: 2rem; text-align: center; color: white; font-family: sans-serif; }
    </style>
</head>
<body>
    <canvas id="glcanvas" tabindex="1"></canvas>
    <div id="error">
        <h2>Sorry — this browser cannot run the game.</h2>
        <p>It requires WebGL2. Try a recent version of Chrome, Firefox, or Safari.</p>
    </div>

    <script src="mq_js_bundle.js"></script>
    <script>
        if (typeof load !== 'function') {
            document.getElementById('glcanvas').style.display = 'none';
            document.getElementById('error').style.display = 'block';
        } else {
            load("your-game-name.wasm");
        }
    </script>
</body>
</html>
```

The `.wasm` filename in `load(...)` **must** match the name produced by
`build-web.sh`.

### 4.1 Mobile / touch shell (pagoda)

Pagoda adds a mobile-oriented shell on top of the template:

* `viewport-fit=cover, maximum-scale=1.0, user-scalable=no` — prevents iOS
  Safari from zooming when the canvas is tapped.
* `touch-action: none`, `-webkit-touch-callout: none`, `user-select: none` —
  no scrolling, long-press menu, or text selection over the canvas.
* A `touchstart` listener focuses the canvas so the first tap also unlocks
  audio.
* The game is click/tap-only. Browsers synthesize mouse events from touches,
  so the Rust input layer does not need a separate touch API. Coordinates are
  converted from window space to the virtual canvas with
  `render::window_to_virtual`, which returns `None` in the letterbox bars.

### 4.2 localStorage plugin shell (accretion)

Load the plugin **after** `mq_js_bundle.js` and **before** `load(...)`:

```html
<script src="mq_js_bundle.js"></script>
<script src="localStorage_plugin.js"></script>
<script>
    load("accretion.wasm");
</script>
```

If the plugin is loaded first, `mergeInto` / `LibraryManager` is undefined and
the wasm module fails to link (`ember_storage_get` missing).

---

## 5. Local testing

Do not open `index.html` via `file://` — CORS blocks the WASM and assets.
Serve it:

```bash
cd dist/web
python3 -m http.server 8080
# Open http://localhost:8080
```

Check the browser console: asset fetches should succeed and there should be
no missing-import errors. The first load downloads a few hundred KB; hard
reload (Cmd+Shift+R / Ctrl+Shift+R) after every rebuild.

---

## 6. Publishing to Itch.io

1. **Zip the contents, not the folder.** `index.html` must be at the root
   of the `.zip`.
   ```bash
   cd dist/web
   zip -r ../../my-game-web.zip index.html my-game.wasm mq_js_bundle.js assets
   ```
2. **Upload** to Itch.io.
3. **Kind of Project:** HTML.
4. **Embed options:** "Click to launch in fullscreen" (bypasses browser
   autoplay restrictions for audio, and gives the game the whole screen).
5. **Viewport:** leave dimensions blank — macroquad handles canvas resizing.

---

## 7. Common browser pitfalls

*   **`LinkError` on load / missing imports:** `wasm-bindgen` snuck into the
    dependency tree. Remove it. If the missing import is `ember_storage_get`,
    the localStorage plugin is absent or loaded before `mq_js_bundle.js`.
*   **Panic on load:** you called `Instant::now()` or `SystemTime::now()`.
    Accumulate `f32` from `dt` (see §2.B).
*   **Assets 404:** `paths.rs` returned a native OS path on `wasm32`. Return
    `PathBuf::from("assets")` (see §2.A).
*   **Audio won't play:** browser autoplay policy. You need a "Click to
    start" screen (`AppScreen::WebStart` in `jade-garden`) that opens the
    audio context on the first user gesture.
*   **Audio detuned / chipmunked:** a WAV is not 44.1 kHz mono 16-bit, or the
    `fmt ` chunk declares a different rate than the samples. Regenerate with
    `tools/generate-sounds.py` and run the header tests (see §2.E).
*   **Clicks outside the canvas trigger gameplay:** the game ignores the
    letterbox. Convert through `render::window_to_virtual` and drop `None`.
*   **iOS zoom / scroll on tap:** missing `touch-action: none` or the
    mobile viewport meta tag (see §4.1).
*   **Version mismatch / stale JS glue:** should not happen — §3.1 keys the
    cache on the exact `Cargo.lock` version and scopes the registry lookup.
    If you see it, someone reverted the script to a hardcoded version or
    `find '*macroquad-*'`. Fix the script, not the symptom.
*   **Black screen / half-rendered:** `next_frame().await` called twice per
    loop iteration. It must be exactly once.
*   **Wasm too big:** install `binaryen` (`brew install binaryen`); the
    script picks up `wasm-opt -Oz` automatically.

---

## 8. Status of the four reference ports

*   `games/jade-garden` — **shipped** to itch.io. Full feature parity except
    for web save (in-memory only, §2.D), web SFX (silent), and the Halloween
    teaser (needs `SystemTime`).
*   `games/bore-inc` — **abandoned jam project**, kept as a WASM reference.
    It still compiles for wasm and is a member of `cargo test --workspace`
    (223 tests), so a macroquad / glam bump will break it first. Do not delete
    it while it remains a useful example; do expect to fix it on bumps.
*   `games/accretion` — **playable WASM port** with the first real
    `localStorage` save (§2.D). Committed `dist/web` bundle. This is the
    reference for persistence and for the plugin loading order (§4.2).
*   `games/pagoda` — **playable mobile-first WASM port** with procedural SFX
    (§2.E), a touch shell (§4.1), a Noto Sans SC subset, and a committed
    `dist/web` bundle. This is the reference for mobile and audio. Its web
    save is still in-memory.

The shared `mq_js_bundle.js` resolver has now been copied four times. Under
the Rule of Three it should already have been extracted; do it in the next
session that touches a build script, before porting a fifth game. Likewise,
the localStorage bridge is proven but still local to accretion — port it to
pagoda (replacing the in-memory save) and then extract it.