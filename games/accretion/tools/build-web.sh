#!/usr/bin/env bash
# Builds the WebAssembly bundle for itch.io (HTML5 play).
#
# Output:
#   dist/web/
#     ├── index.html
#     ├── mq_js_bundle.js
#     ├── localStorage_plugin.js
#     ├── accretion.wasm
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
WASM_SRC="$WORKSPACE_DIR/target/wasm32-unknown-unknown/release/accretion.wasm"

# ─── 1. Ensure the wasm32 target is installed ───
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "→ Installing wasm32-unknown-unknown target…"
    rustup target add wasm32-unknown-unknown
fi

# ─── 2. Build the release binary for wasm ───
echo "→ Building accretion for wasm32…"
(cd "$WORKSPACE_DIR" && cargo build --release --target wasm32-unknown-unknown -p accretion)

if [ ! -f "$WASM_SRC" ]; then
    echo "ERROR: wasm binary not found at $WASM_SRC"
    exit 1
fi

# ─── 3. Fresh output directory ───
rm -rf "$OUT"
mkdir -p "$OUT"
cp "$WASM_SRC" "$OUT/accretion.wasm"

# ─── 4. Resolve mq_js_bundle.js ─── (see BROWSER_BUILD.md §3.1)
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
    cp "$REGISTRY_BUNDLE" "$BUNDLE"
elif [ -f "$CACHE" ]; then
    echo "→ mq_js_bundle.js ← cache (macroquad $MQ_VERSION)"
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
        exit 1
    fi
fi

# ─── 5. HTML shell + localStorage plugin ───
cp "$CRATE_DIR/web/index.html" "$OUT/index.html"
cp "$CRATE_DIR/tools/localStorage_plugin.js" "$OUT/localStorage_plugin.js"

# ─── 6. Assets ───
if [ -d "$CRATE_DIR/assets" ]; then
    cp -R "$CRATE_DIR/assets" "$OUT/assets"
    rm -f "$OUT/assets/.env"
    rm -f "$OUT/assets/progress.ron"
fi

# ─── 7. wasm-opt (optional) ───
if command -v wasm-opt >/dev/null 2>&1; then
    echo "→ Running wasm-opt…"
    wasm-opt -Oz "$OUT/accretion.wasm" -o "$OUT/accretion.wasm"
else
    echo "ℹ️  wasm-opt not found. Skipping optimization."
fi

# ─── 8. Report ───
echo ""
echo "✅ Web bundle ready: $OUT"
du -sh "$OUT"
ls -la "$OUT"