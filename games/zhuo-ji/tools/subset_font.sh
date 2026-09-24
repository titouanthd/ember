#!/usr/bin/env bash
set -euo pipefail

ASSETS="$(cd "$(dirname "$0")/.." && pwd)/assets"
SRC="$ASSETS/NotoSansSC-Regular.otf"
DST="$ASSETS/NotoSansSC-ZhuoJi.otf"

mkdir -p "$ASSETS"

# Delete any previous bad download (HTML error page).
if [ -f "$SRC" ] && [ "$(stat -f%z "$SRC" 2>/dev/null || stat -c%s "$SRC" 2>/dev/null)" -lt 1000000 ]; then
    echo "Removing stale/broken $SRC"
    rm -f "$SRC"
fi

if [ ! -f "$SRC" ]; then
    echo "Downloading Noto Sans SC Regular from Google Fonts..."
    # Google Fonts static download URL — returns a zip.
    curl -L --fail -o /tmp/notosanssc.zip \
        "https://fonts.google.com/download?family=Noto%20Sans%20SC"

    echo "Unzipping..."
    unzip -o /tmp/notosanssc.zip -d /tmp/notosanssc
    # The zip contains static/NotoSansSC-Regular.otf and variable fonts.
    find /tmp/notosanssc -name "NotoSansSC-Regular.otf" -exec cp {} "$SRC" \;
    rm -rf /tmp/notosanssc /tmp/notosanssc.zip
fi

if [ ! -f "$SRC" ]; then
    echo "ERROR: could not obtain NotoSansSC-Regular.otf"
    exit 1
fi

echo "Source font size:"
ls -lh "$SRC"

GLYPHS="0123456789一二三四五六七八九万萬条條筒東南西北中發白贵阳捉鸡麻将豆和了"

echo ""
echo "Generating subset..."
pyftsubset "$SRC" \
    --text="$GLYPHS" \
    --output-file="$DST" \
    --no-hinting \
    --desubroutinize \
    --layout-features='' \
    --drop-tables+=GSUB,GPOS,GDEF

echo ""
echo "Generated: $DST"
ls -lh "$DST"