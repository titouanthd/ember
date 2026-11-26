//! `make edit GAME=pong`: opens a Pong scene in the editor (ADR-039).

fn main() {
    let config = ember_editor::EditorConfig::from_args(pong::scene::classes());
    ember_editor::run(config.with_viewport(800.0, 600.0));
}
