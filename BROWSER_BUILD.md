# Ember — Browser (WASM) Build Guide

This document is the definitive guide for compiling and publishing any Ember game to the browser (`wasm32-unknown-unknown`), based on the reference implementation in `jade-garden`. 

**Target Audience:** The next agent / developer working on this workspace.
**Constraint:** This workspace targets WASM without `wasm-bindgen` (using `macroquad`'s native `miniquad` bindings).

---

## 1. Prerequisites

Before attempting a WASM build, ensure the target is installed:

```bash
rustup target add wasm32-unknown-unknown
```

### Crucial Dependency Rule
**NEVER** add `wasm-bindgen`, `web-sys`, or `wasm-bindgen-futures` to a game's `Cargo.toml`. 
Macroquad relies on `miniquad`, which has its own custom WASM import system (`env.console_log`, etc.). `wasm-bindgen` generates an incompatible import namespace (`__wbindgen_placeholder__`) and will cause the WASM module to fail loading in the browser.

---

## 2. Code Architecture Requirements

When writing a game destined for the browser, the following modules/patterns are mandatory:

### A. Paths Resolution (`paths.rs`)
Browsers do not have a filesystem. Assets must be fetched relative to the `index.html` file.
```rust
#[cfg(target_arch = "wasm32")]
pub fn assets_dir() -> PathBuf {
    PathBuf::from("assets")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn assets_dir() -> PathBuf {
    // ... native filesystem logic ...
}
```

### B. Time Tracking
`std::time::Instant::now()` is unavailable and will panic (`unreachable`) on `wasm32-unknown-unknown`. 
Use a manually accumulated `f32` from `dt` (see `jade-garden/src/audio.rs`).

### C. Async Asset Preloading
On the web, `macroquad::audio::load_sound` and `macroquad::text::load_ttf_font` perform HTTP fetches. They **must** be awaited before the game loop starts.
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

### D. Progression / Save Files
`localStorage` is inaccessible without a `wasm-bindgen` bridge or a miniquad plugin. 
Fallback to an in-memory `thread_local` for now (see `jade-garden/src/progress.rs`).
```rust
#[cfg(target_arch = "wasm32")]
thread_local! {
    static IN_MEMORY_PROGRESS: RefCell<Option<String>> = const { RefCell::new(None) };
}
```

---

## 3. The Build Script (`tools/build-web.sh`)

Every browser-targeted game must have a `tools/build-web.sh` script. Here is the exact, battle-tested template from `jade-garden`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKSPACE_DIR="$(cd "$CRATE_DIR/../.." && pwd)"

GAME_NAME="jade-garden" # Replace with your game's crate name
OUT="$CRATE_DIR/dist/web"
WASM_SRC="$WORKSPACE_DIR/target/wasm32-unknown-unknown/release/$GAME_NAME.wasm"

# 1. Build the release binary for wasm
echo "→ Building $GAME_NAME for wasm32…"
(cd "$WORKSPACE_DIR" && cargo build --release --target wasm32-unknown-unknown -p "$GAME_NAME")

# 2. Fresh output directory
rm -rf "$OUT"
mkdir -p "$OUT"
cp "$WASM_SRC" "$OUT/$GAME_NAME.wasm"

# 3. Get mq_js_bundle.js
# CRITICAL: Copy from the local cargo registry to guarantee the exact version
# aligned with the compiled macroquad binary.
BUNDLE="$OUT/mq_js_bundle.js"
REGISTRY_BUNDLE="$(find "${HOME}/.cargo/registry/src" \
    -path '*macroquad-*/js/mq_js_bundle.js' -type f 2>/dev/null | head -n 1 || true)"

if [ -n "$REGISTRY_BUNDLE" ] && [ -f "$REGISTRY_BUNDLE" ]; then
    echo "→ Using mq_js_bundle.js from cargo registry"
    cp "$REGISTRY_BUNDLE" "$BUNDLE"
else
    echo "ERROR: mq_js_bundle.js not found in cargo registry. Run 'cargo build' on native first."
    exit 1
fi

# 4. HTML shell
cp "$CRATE_DIR/web/index.html" "$OUT/index.html"

# 5. Assets (clean of dev files)
if [ -d "$CRATE_DIR/assets" ]; then
    cp -R "$CRATE_DIR/assets" "$OUT/assets"
    rm -f "$OUT/assets/.env"
    rm -f "$OUT/assets/progress.ron"
fi

# 6. wasm-opt (optional but highly recommended for size reduction)
if command -v wasm-opt >/dev/null 2>&1; then
    echo "→ Running wasm-opt…"
    wasm-opt -Oz "$OUT/$GAME_NAME.wasm" -o "$OUT/$GAME_NAME.wasm"
else
    echo "ℹ️  wasm-opt not found. Skipping optimization. (brew install binaryen)"
fi

echo "✅ Web bundle ready: $OUT"
```

---

## 4. The HTML Shell (`web/index.html`)

Macroquad requires a specific setup to initialize WebGL and map input events. Use this exact template:

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

---

## 5. Local Testing

Do not open `index.html` directly via `file://` (CORS will block the WASM and assets). Use a local server:

```bash
cd dist/web
python3 -m http.server 8080
# Open http://localhost:8080 in your browser
```
Check the browser console. You should see successful asset fetches, and no missing import errors.

---

## 6. Publishing to Itch.io

1. **Zip the contents, not the folder**. 
   Ensure `index.html` is at the **root** of the `.zip` file, not inside a subfolder.
   ```bash
   cd dist/web
   zip -r ../../my-game-web.zip index.html game.wasm mq_js_bundle.js assets
   ```
2. **Upload to Itch.io**.
3. **Kind of Project**: Select **HTML**.
4. **Embed options**: Choose **"Click to launch in fullscreen"**. This bypasses browser autoplay restrictions for audio and gives the game the whole screen.
5. **Viewport**: Leave dimensions blank to let the game fill the screen (macroquad handles the canvas resizing).

---

## 7. Common Browser Pitfalls (Recap)

*   **Missing imports**: `wasm-bindgen` is in your dependency tree. Remove it.
*   **Panic on load**: You called `Instant::now()` or `SystemTime::now()`. Use `dt` accumulation.
*   **Assets 404**: Your `paths.rs` returned a native OS path instead of `PathBuf::from("assets")` on wasm.
*   **Audio won't play**: Browser autoplay policy. You need a "Click to start" screen (like `AppScreen::WebStart`) that starts the audio context on the first user click.
*   **Version mismatch error in console**: `mq_js_bundle.js` does not match the compiled `macroquad` version. Re-copy it from the `~/.cargo/registry` folder.
*   **Black screen / Half-rendered**: You called `next_frame().await` twice per loop iteration.
*   **Wasm binary too big**: Install `binaryen` (`brew install binaryen`) to allow `build-web.sh` to run `wasm-opt -Oz`.