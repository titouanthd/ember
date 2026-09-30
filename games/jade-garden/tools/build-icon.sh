#!/usr/bin/env bash
# Generate PNG + .icns + in-game window icon from icon.svg.
#
# Requires one of: rsvg-convert (brew install librsvg),
# cairosvg (pip install cairosvg), inkscape, or qlmanage (macOS).
# .icns generation requires macOS's iconutil + sips.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SVG="$SCRIPT_DIR/icon.svg"
PNG_1024="$SCRIPT_DIR/icon-1024.png"
ASSETS="$SCRIPT_DIR/../assets"

if [ ! -f "$SVG" ]; then
    echo "ERROR: $SVG not found"
    exit 1
fi

# 1. SVG → PNG at 1024×1024
if command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w 1024 -h 1024 "$SVG" -o "$PNG_1024"
elif python3 -c "import cairosvg" 2>/dev/null; then
    python3 -c "import cairosvg; cairosvg.svg2png(url='$SVG', write_to='$PNG_1024', output_width=1024, output_height=1024)"
elif command -v inkscape >/dev/null 2>&1; then
    inkscape "$SVG" -w 1024 -h 1024 -o "$PNG_1024"
elif command -v qlmanage >/dev/null 2>&1; then
    qlmanage -t -s 1024 -o "$SCRIPT_DIR" "$SVG" >/dev/null 2>&1
    mv "$SCRIPT_DIR/icon.svg.png" "$PNG_1024"
else
    echo "ERROR: no SVG→PNG converter found."
    echo "Install one of: librsvg, cairosvg, inkscape."
    exit 1
fi

echo "✅ $PNG_1024 ($(wc -c < "$PNG_1024") bytes)"

# 2. .icns (macOS only)
if command -v iconutil >/dev/null 2>&1; then
    ICONSET="$SCRIPT_DIR/AppIcon.iconset"
    rm -rf "$ICONSET"
    mkdir -p "$ICONSET"

    for size in 16 32 128 256 512; do
        sips -z "$size" "$size" "$PNG_1024" \
             --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
        sips -z "$((size*2))" "$((size*2))" "$PNG_1024" \
             --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
    done

    iconutil -c icns "$ICONSET" -o "$SCRIPT_DIR/AppIcon.icns"
    rm -rf "$ICONSET"
    echo "✅ $SCRIPT_DIR/AppIcon.icns"
fi

# 3. Small PNG for in-game window icon
mkdir -p "$ASSETS"
sips -z 256 256 "$PNG_1024" --out "$ASSETS/icon-256.png" >/dev/null
echo "✅ $ASSETS/icon-256.png"