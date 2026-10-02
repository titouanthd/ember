#!/usr/bin/env bash
# Builds the WebAssembly bundle for itch.io (HTML5 play).
#
# Requires:
#   - rustup target add wasm32-unknown-unknown
#   - (optional) wasm-opt from binaryen for size optimization
#
# Output:
#   dist/web/  — ready to zip and upload as an HTML5 game on itch.io
#     ├── index.html
#     ├── mq_js_bundle.js
#     ├── jade-garden.wasm
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
WASM_SRC="$WORKSPACE_DIR/target/wasm32-unknown-unknown/release/jade-garden.wasm"
BUNDLE_JS_URL="https://raw.githubusercontent.com/not-fl3/macroquad/v0.4.16/js/mq_js_bundle.js"

# 1. Ensure the wasm32 target is installed.
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "→ Installing wasm32-unknown-unknown target…"
    rustup target add wasm32-unknown-unknown
fi

# 2. Build the release binary for wasm.
echo "→ Building jade-garden for wasm32…"
(cd "$WORKSPACE_DIR" && cargo build --release --target wasm32-unknown-unknown -p jade-garden)

# 3. Fresh output directory.
rm -rf "$OUT"
mkdir -p "$OUT"

# 4. Copy the wasm binary.
if [ ! -f "$WASM_SRC" ]; then
    echo "ERROR: wasm binary not found at $WASM_SRC"
    exit 1
fi
cp "$WASM_SRC" "$OUT/jade-garden.wasm"

# 5. Fetch mq_js_bundle.js (macroquad's JS glue).
if [ ! -f "$SCRIPT_DIR/.cache/mq_js_bundle.js" ]; then
    echo "→ Downloading mq_js_bundle.js…"
    mkdir -p "$SCRIPT_DIR/.cache"

    # Try several sources in order — the macroquad repo changes its
    # tag format between versions (v0.4.16 vs 0.4.16), and jsdelivr
    # is a CDN mirror that is more resilient than raw.githubusercontent.
    BUNDLE_URLS=(
        "https://cdn.jsdelivr.net/gh/not-fl3/macroquad@0.4.16/js/mq_js_bundle.js"
        "https://raw.githubusercontent.com/not-fl3/macroquad/0.4.16/js/mq_js_bundle.js"
        "https://cdn.jsdelivr.net/gh/not-fl3/macroquad@master/js/mq_js_bundle.js"
        "https://raw.githubusercontent.com/not-fl3/macroquad/master/js/mq_js_bundle.js"
    )

    downloaded=false
    for url in "${BUNDLE_URLS[@]}"; do
        echo "   trying: $url"
        if curl -L --fail --silent --show-error \
            -o "$SCRIPT_DIR/.cache/mq_js_bundle.js" "$url"; then
            downloaded=true
            echo "   → OK"
            break
        fi
        rm -f "$SCRIPT_DIR/.cache/mq_js_bundle.js"
    done

    if [ "$downloaded" = false ]; then
        echo "ERROR: could not download mq_js_bundle.js from any URL."
        echo ""
        echo "Download it manually:"
        echo "  1. Go to https://github.com/not-fl3/macroquad"
        echo "  2. Navigate to 'js/mq_js_bundle.js' on the matching branch/tag"
        echo "  3. Save it to: $SCRIPT_DIR/.cache/mq_js_bundle.js"
        echo "  4. Re-run this script"
        exit 1
    fi
fi

cp "$SCRIPT_DIR/.cache/mq_js_bundle.js" "$OUT/mq_js_bundle.js"

# 6. Copy the HTML shell.
cp "$CRATE_DIR/web/index.html" "$OUT/index.html"

# 7. Copy assets (clean of dev files).
cp -R "$CRATE_DIR/assets" "$OUT/assets"
rm -f "$OUT/assets/.env"
rm -f "$OUT/assets/progress.ron"
rm -f "$OUT/assets/NotoSansSC-Regular.otf"
rm -f "$OUT/assets/NotoSansSC-Regular.ttf"
rm -f "$OUT/assets/icon-256.png"

# 8. Optimize the wasm binary if wasm-opt is available.
if command -v wasm-opt >/dev/null 2>&1; then
    echo "→ Running wasm-opt…"
    wasm-opt -Oz "$OUT/jade-garden.wasm" -o "$OUT/jade-garden.wasm"
else
    echo "ℹ️  wasm-opt not found. Skipping optimization."
    echo "   Install with: brew install binaryen"
fi

# 9. Report.
echo ""
echo "✅ Web bundle ready: $OUT"
du -sh "$OUT"
ls -la "$OUT"