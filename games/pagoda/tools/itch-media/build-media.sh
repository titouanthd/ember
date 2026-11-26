#!/usr/bin/env bash
# Generates the itch.io page media (cover + banner) as PNGs.
#
# Sources:  tools/itch-media/*.svg
# Outputs:  dist/itch-media/*.png
#
# Uses, in order of preference:
#   1. A Chromium-based browser in headless mode (Chrome, Brave, Edge,
#      Chromium, Arc). Exact dimensions, no padding, no extra deps.
#   2. rsvg-convert (librsvg) if installed.
#   3. cairosvg (Python).
#
# Usage:
#     chmod +x tools/itch-media/build-media.sh
#     ./tools/itch-media/build-media.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="$CRATE_DIR/dist/itch-media"
mkdir -p "$OUT_DIR"

# ─── Chromium-based browser paths (macOS) ───
CHROME_PATHS=(
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
    "/Applications/Chromium.app/Contents/MacOS/Chromium"
    "/Applications/Arc.app/Contents/MacOS/Arc"
)

find_chrome() {
    for p in "${CHROME_PATHS[@]}"; do
        if [ -x "$p" ]; then
            echo "$p"
            return 0
        fi
    done
    return 1
}

# ─── SVG → PNG conversion ───
convert_svg() {
    local svg="$1"
    local png="$2"
    local w="$3"
    local h="$4"

    # 1. Chromium headless (preferred — exact size, no padding)
    local chrome
    if chrome="$(find_chrome)"; then
        rm -f "$png"
        "$chrome" \
            --headless \
            --disable-gpu \
            --hide-scrollbars \
            --no-sandbox \
            --screenshot="$png" \
            --window-size="$w,$h" \
            "file://$svg" >/dev/null 2>&1
        if [ -s "$png" ]; then
            return 0
        fi
        echo "  ⚠️  $chrome failed, trying next converter…"
    fi

    # 2. rsvg-convert
    if command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w "$w" -h "$h" "$svg" -o "$png"
        return 0
    fi

    # 3. cairosvg
    if python3 -c "import cairosvg" 2>/dev/null; then
        python3 -c "import cairosvg; cairosvg.svg2png(url='$svg', write_to='$png', output_width=$w, output_height=$h)"
        return 0
    fi

    echo "ERROR: no SVG→PNG converter available."
    echo "Install one of:"
    echo "  - A Chromium-based browser (Chrome, Brave, Edge, Chromium, Arc)"
    echo "  - librsvg (brew install librsvg)"
    echo "  - cairosvg (pip install cairosvg)"
    exit 1
}

# ─── Generate ───
echo "→ Cover (630×500)…"
convert_svg "$SCRIPT_DIR/cover.svg" "$OUT_DIR/cover-630x500.png" 630 500

echo "→ Banner (960×400)…"
convert_svg "$SCRIPT_DIR/banner.svg" "$OUT_DIR/banner-960x400.png" 960 400

echo "→ Cover @2× (1260×1000)…"
convert_svg "$SCRIPT_DIR/cover.svg" "$OUT_DIR/cover-1260x1000.png" 1260 1000

# ─── Report ───
echo ""
echo "✅ Media generated in: $OUT_DIR"
ls -lh "$OUT_DIR"

# Quick sanity check on dimensions.
echo ""
echo "Dimensions:"
for f in "$OUT_DIR"/*.png; do
    dims=$(sips -g pixelWidth -g pixelHeight "$f" 2>/dev/null | grep pixel | awk '{print $2}' | paste -sd'×' -)
    echo "  $(basename "$f")  →  $dims"
done