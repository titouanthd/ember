#!/usr/bin/env bash
# Generates the itch.io page media (cover + banner) as PNGs.
#
# Sources:  tools/itch-media/*.svg
# Outputs:  dist/itch-media/*.png
#
# Requires one of: rsvg-convert (brew install librsvg),
# cairosvg (pip install cairosvg), inkscape, or qlmanage (macOS).
#
# Usage:
#     chmod +x tools/itch-media/build-media.sh
#     ./tools/itch-media/build-media.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="$CRATE_DIR/dist/itch-media"
mkdir -p "$OUT_DIR"

# ─── Pick an SVG→PNG converter ───
convert_svg() {
    local svg="$1"
    local png="$2"
    local w="$3"
    local h="$4"

    if command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w "$w" -h "$h" "$svg" -o "$png"
    elif python3 -c "import cairosvg" 2>/dev/null; then
        python3 -c "import cairosvg; cairosvg.svg2png(url='$svg', write_to='$png', output_width=$w, output_height=$h)"
    elif command -v inkscape >/dev/null 2>&1; then
        inkscape "$svg" -w "$w" -h "$h" -o "$png"
    elif command -v qlmanage >/dev/null 2>&1; then
        # macOS Quick Look — slower, produces "<name>.svg.png" in outdir.
        local tmpdir
        tmpdir="$(mktemp -d)"
        qlmanage -t -s "${w}" -o "$tmpdir" "$svg" >/dev/null 2>&1
        mv "$tmpdir/$(basename "$svg").png" "$png"
        rm -rf "$tmpdir"
    else
        echo "ERROR: no SVG→PNG converter found."
        echo "Install one of: librsvg (rsvg-convert), cairosvg, inkscape."
        exit 1
    fi
}

# ─── Generate ───
echo "→ Cover (630×500)…"
convert_svg "$SCRIPT_DIR/cover.svg" "$OUT_DIR/cover-630x500.png" 630 500

echo "→ Banner (960×400)…"
convert_svg "$SCRIPT_DIR/banner.svg" "$OUT_DIR/banner-960x400.png" 960 400

# Also generate a 2× cover for retina-grade displays on itch.io.
echo "→ Cover @2× (1260×1000)…"
convert_svg "$SCRIPT_DIR/cover.svg" "$OUT_DIR/cover-1260x1000.png" 1260 1000

# ─── Report ───
echo ""
echo "✅ Media generated in: $OUT_DIR"
ls -lh "$OUT_DIR"