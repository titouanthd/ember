//! A dev-only scene editor (Phase 6 POC, roadmap §9).
//!
//! - [`Document`]: an opened `.escene` as a live `NodeTree` you can
//!   inspect, edit through exported properties and save (no window).
//! - [`nodes`]: the built-in editable classes (`Player`, `Sprite`).
//! - [`value`]: properties shown and edited as text.
//! - [`app`]: the window — outline, inspector, preview (an `ember_ui`
//!   node tree run by `SceneLoop`).
//! - [`run`](mod@run): the library entry point; a game passes its own
//!   classes (ADR-039).

pub mod app;
pub mod document;
pub mod nodes;
pub mod run;
pub mod value;

pub use document::{Document, EditorError, OutlineRow};
pub use run::{EditorConfig, run};
