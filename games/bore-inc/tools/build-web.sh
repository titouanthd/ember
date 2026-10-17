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

# ─── 4. Get mq_js_bundle.js ───
#
# Preferred: copy from the local cargo registry, guarantees the exact
# version aligned with the compiled macroquad binary. Fallback: download
# from several mirrors (macroquad's tag format changes between versions).
BUNDLE="$OUT/mq_js_bundle.js"

REGISTRY_BUNDLE="$(find "${HOME}/.cargo/registry/src" \
    -path '*macroquad-*/js/mq_js_bundle.js' -type f 2>/dev/null | head -n 1 || true)"

if [ -n "$REGISTRY_BUNDLE" ] && [ -f "$REGISTRY_BUNDLE" ]; then
    echo "→ Using mq_js_bundle.js from cargo registry:"
    echo "   $REGISTRY_BUNDLE"
    cp "$REGISTRY_BUNDLE" "$BUNDLE"
else
    echo "→ mq_js_bundle.js not found in cargo registry, downloading…"
    mkdir -p "$SCRIPT_DIR/.cache"

    BUNDLE_URLS=(
        "https://cdn.jsdelivr.net/gh/not-fl3/macroquad@0.4.16/js/mq_js_bundle.js"
        "https://raw.githubusercontent.com/not-fl3/macroquad/0.4.16/js/mq_js_bundle.js"
        "https://cdn.jsdelivr.net/gh/not-fl3/macroquad@master/js/mq_js_bundle.js"
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
        echo "ERROR: could not obtain mq_js_bundle.js."
        echo "Install macroquad via cargo once (cargo build), then re-run."
        exit 1
    fi
    cp "$SCRIPT_DIR/.cache/mq_js_bundle.js" "$BUNDLE"
fi

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