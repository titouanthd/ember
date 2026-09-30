#!/usr/bin/env bash
# Génère un subset Noto Sans SC contenant uniquement les glyphes utilisés
# par jade-garden : les 20 caractères du poème 《春晓》, les noms des
# jades, l'auteur, et l'ASCII de base.
#
# Prérequis : fonttools (pip install fonttools), curl.
#
# Usage :
#     chmod +x subset_font.sh
#     ./subset_font.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ASSETS="$SCRIPT_DIR/assets"
SRC="$ASSETS/NotoSansSC-Regular.otf"
DST="$ASSETS/NotoSansSC-JadeGarden.otf"

mkdir -p "$ASSETS"

# Retourne 0 si le fichier ressemble à une vraie police (taille > 500 KB
# + magic bytes OTTO / TTF / true). Détecte les pages HTML déguisées.
is_valid_font() {
    local f="$1"
    [ -f "$f" ] || return 1
    local size
    size=$(wc -c < "$f" 2>/dev/null || echo 0)
    [ "$size" -gt 500000 ] || return 1
    local magic
    magic=$(head -c 4 "$f" | od -An -c | tr -d ' \n')
    case "$magic" in
        OTTO*|true*) return 0 ;;
    esac
    # TrueType magic: 00 01 00 00
    if [ "$(head -c 4 "$f" | od -An -tx1 | tr -d ' \n')" = "00010000" ]; then
        return 0
    fi
    return 1
}

# Nettoie tout fichier cassé résiduel.
if [ -f "$SRC" ] && ! is_valid_font "$SRC"; then
    echo "Removing invalid cached $SRC"
    rm -f "$SRC"
fi

# Télécharge depuis une liste d'URLs, essaie chacune à son tour.
if [ ! -f "$SRC" ]; then
    URLS=(
        "https://github.com/notofonts/noto-cjk/raw/main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf"
        "https://cdn.jsdelivr.net/gh/notofonts/noto-cjk@main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf"
        "https://github.com/notofonts/noto-cjk/raw/main/Sans/OTF/SimplifiedChinese/NotoSansSC-Regular.otf"
    )
    for url in "${URLS[@]}"; do
        echo "Trying $url …"
        rm -f "$SRC"
        if curl -L --fail --silent --show-error -o "$SRC" "$url"; then
            if is_valid_font "$SRC"; then
                echo "  → OK"
                break
            else
                echo "  → not a valid font, trying next"
            fi
        else
            echo "  → download failed, trying next"
        fi
    done
fi

if ! is_valid_font "$SRC"; then
    rm -f "$SRC"
    echo ""
    echo "ERROR: could not obtain NotoSansSC-Regular.otf."
    echo ""
    echo "Please download manually:"
    echo "  1. Open https://fonts.google.com/noto/specimen/Noto+Sans+SC"
    echo "  2. Click 'Get font' → 'Download all'"
    echo "  3. Unzip, extract NotoSansSC-Regular.otf"
    echo "  4. Place it at: $SRC"
    echo "  5. Re-run this script"
    exit 1
fi

echo "Source font: $SRC ($(wc -c < "$SRC") bytes)"

if ! command -v pyftsubset >/dev/null 2>&1; then
    echo ""
    echo "ERROR: pyftsubset introuvable."
    echo "Installer fonttools : pip install fonttools"
    exit 1
fi

# Poème (20 chars) + titre 《春晓》 + auteur 唐孟浩然 + noms des jades.
GLYPHS="春眠不觉晓处处闻啼鸟夜来风雨声花落知多少"
GLYPHS+="《》唐孟浩然"
GLYPHS+="白玉碧玉青玉红玉黄玉墨玉"

echo ""
echo "Generating subset…"
pyftsubset "$SRC" \
    --text="$GLYPHS" \
    --unicodes=U+0020-007E,U+00A0,U+2013,U+2014 \
    --output-file="$DST" \
    --no-hinting \
    --desubroutinize \
    --layout-features='' \
    --drop-tables+=GSUB,GPOS,GDEF

echo ""
echo "Generated: $DST"
ls -lh "$DST"