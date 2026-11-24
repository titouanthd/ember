//! A dev-only scene editor (Phase 6 POC, roadmap §9).
//!
//! - [`Document`]: an opened `.escene` as a live `NodeTree` you can
//!   inspect, edit through exported properties and save (no window).
//! - [`nodes`]: the built-in editable classes (`Player`, `Sprite`).
//! - [`value`]: properties shown and edited as text.

pub mod document;
pub mod nodes;
pub mod value;

pub use document::{Document, EditorError, OutlineRow};
