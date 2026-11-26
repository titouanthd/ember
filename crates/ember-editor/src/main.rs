//! `cargo run -p ember-editor -- path/to/scene.escene [--print]`
//!
//! The editor with its built-in demo classes (`Player`, `Sprite`).
//! A game opens its own scenes through `ember_editor::run` (ADR-039).

fn main() {
    ember_editor::run(ember_editor::EditorConfig::from_args(
        ember_editor::nodes::classes(),
    ));
}
