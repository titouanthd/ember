#!/usr/bin/env bash
# Scaffold Phase-1 crates. Works on macOS /bin/bash 3.2 (no associative arrays).
# Idempotent: skips any crate whose Cargo.toml already exists.
set -euo pipefail
cd "$(dirname "$0")/.."

write_lib_rs() {
    local dir="$1"
    local desc="$2"
    cat > "$dir/src/lib.rs" <<EOF
//! ${desc}

// Intentionally empty in Phase 1. Public API lands with its phase.
// See ember-vault/10 Engine Spec v0.1/ for the spec.
EOF
}

scaffold() {
    local name="$1"
    local desc="$2"
    shift 2
    local dir="crates/$name"

    if [ -f "$dir/Cargo.toml" ]; then
        echo "  skip  $dir (already exists)"
        return
    fi

    mkdir -p "$dir/src"
    {
        echo '[package]'
        echo "name = \"$name\""
        echo 'version.workspace = true'
        echo 'edition.workspace = true'
        echo
        echo '[dependencies]'
        for dep in "$@"; do
            echo "${dep}.workspace = true"
        done
    } > "$dir/Cargo.toml"
    write_lib_rs "$dir" "$desc"
    echo "  create $dir"
}

scaffold_proc_macro() {
    local name="$1"
    local desc="$2"
    local dir="crates/$name"

    if [ -f "$dir/Cargo.toml" ]; then
        echo "  skip  $dir (already exists)"
        return
    fi

    mkdir -p "$dir/src"
    {
        echo '[package]'
        echo "name = \"$name\""
        echo 'version.workspace = true'
        echo 'edition.workspace = true'
        echo
        echo '[lib]'
        echo 'proc-macro = true'
        echo
        echo '[dependencies]'
        echo 'syn.workspace = true'
        echo 'quote.workspace = true'
        echo 'proc-macro2.workspace = true'
    } > "$dir/Cargo.toml"
    write_lib_rs "$dir" "$desc"
    echo "  create $dir"
}

# ---- The 9 crates that need to be created -----------------------------
# (ember-core, ember-editor, ember-stdlib already exist and are skipped.)

scaffold ember-servers \
    "Trait definitions for the rendering, audio, and physics subsystems. No implementations." \
    ember-core

scaffold ember-input \
    "Backend-agnostic input snapshot and action mapping (primary_action / secondary_action)." \
    ember-core

scaffold_proc_macro ember-macros \
    "Procedural macros: #[derive(Node)], #[signal], #[export]."

scaffold ember-scene \
    "Node, NodeId, NodeTree, NodeContext, PackedScene. The tree the game lives in." \
    ember-core ember-servers ember-assets ember-macros

scaffold ember-ui \
    "Retained-mode Control nodes, layout containers, focus management, theming." \
    ember-scene ember-servers

scaffold ember-assets \
    "AssetServer, Handle<T>, UID registry, hot-reload. Games never touch the filesystem directly." \
    ember-core ember-servers

scaffold ember-render-macroquad \
    "Reference RenderingServer implementation on macroquad." \
    ember-core ember-servers

scaffold ember-audio-macroquad \
    "Reference AudioServer implementation on macroquad." \
    ember-core ember-servers

scaffold ember-editor \
    "Scene inspector (dev-only). Never shipped in games." \
    ember-scene ember-assets ember-render-macroquad

scaffold ember \
    "Facade crate: prelude re-exports, MainLoop, run(). Games depend on this, not on the backends." \
    ember-core ember-scene ember-ui ember-assets ember-servers ember-input \
    ember-render-macroquad ember-audio-macroquad

echo
echo "Done. Next: verify workspace Cargo.toml includes all 12 members,"
echo "then run: cargo check --workspace"