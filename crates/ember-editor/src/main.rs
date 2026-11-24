//! `cargo run -p ember-editor -- path/to/scene.escene [--print]`
//!
//! Opens the scene in the editor window. `--print` prints the outline
//! and properties instead (no window).

use ember_audio_macroquad::MacroquadAudio;
use ember_core::math::Rect;
use ember_editor::app::{WINDOW_H, WINDOW_W, build_tree};
use ember_editor::{Document, nodes, value};
use ember_render_macroquad::{MacroquadRenderer, run_window};
use ember_scene::SceneLoop;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let print = args.iter().any(|a| a == "--print");
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: ember-editor <scene.escene> [--print]");
        std::process::exit(2);
    };
    let doc = match Document::open(path, nodes::classes()) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("ember-editor: {path}: {e}");
            std::process::exit(1);
        }
    };
    if print {
        print_outline(&doc);
        return;
    }
    let editor = SceneLoop::new(
        build_tree(doc),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    );
    run_window(
        &format!("Ember Editor — {path}"),
        WINDOW_W as i32,
        WINDOW_H as i32,
        editor,
    );
}

fn print_outline(doc: &Document) {
    for row in doc.outline() {
        let indent = "  ".repeat(row.depth);
        println!("{indent}{} ({})", row.name, row.class);
        for (key, v) in doc.properties(row.id).unwrap_or_default() {
            println!("{indent}  · {key} = {}", value::format(&v));
        }
    }
}
