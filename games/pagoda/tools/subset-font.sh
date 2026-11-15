#!/usr/bin/env bash
# Generates the tiny Noto Sans SC subset used for Pagoda's Chinese
# accents: the signboard (宝塔), the perfect combo (完美), the game-over
# seal (终) and the new-record line (新纪录), plus ASCII for mixed
# strings such as "完美 x3".
#
# The full Noto Sans SC file is ~8 MB; this subset is a few dozen KB.
#
# Requirements: fonttools (pip install fonttools).
#
# Usage:
#     ./tools/subset-font.sh
#
# Source resolution:
#   1. $NOTO_SANS_SC, if set.
#   2. games/jade-garden/assets/NotoSansSC-Regular.otf (already vendored).
#   3. A download into tools/.cache/.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DST="$CRATE_DIR/assets/NotoSansSC-Pagoda.otf"

SRC="${NOTO_SANS_SC:-$CRATE_DIR/../jade-garden/assets/NotoSansSC-Regular.otf}"
if [ ! -f "$SRC" ]; then
    CACHE="$SCRIPT_DIR/.cache/NotoSansSC-Regular.otf"
    if [ ! -f "$CACHE" ]; then
        mkdir -p "$SCRIPT_DIR/.cache"
        echo "→ Downloading Noto Sans SC to $CACHE…"
        curl -L --fail --silent --show-error \
            -o "$CACHE" \
            "https://github.com/notofonts/noto-cjk/raw/main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf"
    fi
    SRC="$CACHE"
fi

if [ ! -f "$SRC" ]; then
    echo "ERROR: Noto Sans SC source font not found."
    echo "Set NOTO_SANS_SC=/path/to/NotoSansSC-Regular.otf and re-run."
    exit 1
fi

if ! command -v pyftsubset >/dev/null 2>&1; then
    echo "ERROR: pyftsubset not found. Install fonttools: pip install fonttools"
    exit 1
fi

GLYPHS="宝塔完美终新纪录"

echo "Source: $SRC"
pyftsubset "$SRC" \
    --text="$GLYPHS" \
    --unicodes=U+0020-007E,U+00A0,U+00B7,U+2013,U+2014 \
    --output-file="$DST" \
    --no-hinting \
    --desubroutinize \
    --layout-features='' \
    --drop-tables+=GSUB,GPOS,GDEF

echo "Generated: $DST"
ls -lh "$DST"
