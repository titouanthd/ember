//! The editor as a library (ADR-039): a game opens its own scenes by
//! passing its node classes.
//!
//! ```ignore
//! // games/<game>/examples/edit.rs
//! fn main() {
//!     let config = ember_editor::EditorConfig::from_args(my_game::scene::classes());
//!     ember_editor::run(config.with_viewport(800.0, 600.0));
//! }
//! ```
//!
//! `ember-editor` cannot depend on games (layers), so the dependency
//! points the other way: the game's example depends on the editor.

use crate::app::{EditorApp, WINDOW_H, WINDOW_W, build_app_tree};
use crate::{Document, value};
use ember_audio_macroquad::MacroquadAudio;
use ember_core::ClassDB;
use ember_core::math::Rect;
use ember_render_macroquad::{MacroquadRenderer, run_window};
use ember_scene::SceneLoop;
use glam::Vec2;
use std::path::PathBuf;

/// What the editor opens, and with which classes.
pub struct EditorConfig {
    /// Every class the scene uses.
    pub classes: ClassDB,
    /// The `.escene` file to open and save.
    pub scene: PathBuf,
    /// The game's resolution; the preview frames it.
    pub viewport: Option<Vec2>,
    /// Print the outline and properties instead of opening a window.
    pub print: bool,
}

pub const USAGE: &str = "usage: <editor> <scene.escene> [--print]";

impl EditorConfig {
    pub fn new(classes: ClassDB, scene: impl Into<PathBuf>) -> Self {
        Self {
            classes,
            scene: scene.into(),
            viewport: None,
            print: false,
        }
    }

    /// From command-line arguments (without the program name):
    /// the scene path and an optional `--print`.
    pub fn parse<I, S>(classes: ClassDB, args: I) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let args: Vec<String> = args.into_iter().map(Into::into).collect();
        let print = args.iter().any(|a| a == "--print");
        let scene = args.iter().find(|a| !a.starts_with("--")).ok_or(USAGE)?;
        Ok(Self {
            print,
            ..Self::new(classes, scene)
        })
    }

    /// From `std::env::args()`; prints the usage and exits on error.
    pub fn from_args(classes: ClassDB) -> Self {
        Self::parse(classes, std::env::args().skip(1)).unwrap_or_else(|usage| {
            eprintln!("{usage}");
            std::process::exit(2);
        })
    }

    pub fn with_viewport(mut self, width: f32, height: f32) -> Self {
        self.viewport = Some(Vec2::new(width, height));
        self
    }
}

/// Opens `config.scene` in the editor window (or prints it with
/// `print`). Exits the process if the scene cannot be opened.
pub fn run(config: EditorConfig) {
    let path = config.scene.display().to_string();
    let doc = Document::open(&config.scene, config.classes).unwrap_or_else(|e| {
        eprintln!("ember-editor: {path}: {e}");
        std::process::exit(1);
    });
    if config.print {
        print_outline(&doc);
        return;
    }
    let app = EditorApp::new(doc).with_viewport(config.viewport);
    let editor = SceneLoop::new(
        build_app_tree(app),
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

/// The outline with each node's properties, as `--print` shows it.
pub fn outline_text(doc: &Document) -> String {
    let mut out = String::new();
    for row in doc.outline() {
        let indent = "  ".repeat(row.depth);
        out += &format!("{indent}{} ({})\n", row.name, row.class);
        for (key, v) in doc.properties(row.id).unwrap_or_default() {
            out += &format!("{indent}  · {key} = {}\n", value::format(&v));
        }
    }
    out
}

fn print_outline(doc: &Document) {
    print!("{}", outline_text(doc));
}
