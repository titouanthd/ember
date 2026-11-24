//! `cargo run -p ember-editor -- path/to/scene.escene`

use ember_editor::{Document, nodes};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: ember-editor <scene.escene>");
        std::process::exit(2);
    };
    let doc = match Document::open(&path, nodes::classes()) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("ember-editor: {path}: {e}");
            std::process::exit(1);
        }
    };
    // The window (session 6.2) is not there yet: print the outline.
    for row in doc.outline() {
        println!("{}{} ({})", "  ".repeat(row.depth), row.name, row.class);
        for (key, v) in doc.properties(row.id).unwrap_or_default() {
            println!(
                "{}  · {key} = {}",
                "  ".repeat(row.depth),
                ember_editor::value::format(&v)
            );
        }
    }
}
