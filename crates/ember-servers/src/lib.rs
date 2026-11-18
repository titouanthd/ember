//! Trait definitions for Ember's server subsystems.
//!
//! Three traits, one shared error type, one shared set of opaque handles.
//! Backends (`ember-render-macroquad`, `ember-audio-macroquad`, future
//! `ember-render-wgpu`) implement these; `ember_scene` and `ember_ui`
//! consume them through `&mut dyn Trait`.
//!
//! ## Testing
//!
//! `MockRenderer` and `MockAudio` are recording backends for integration
//! tests. They live in the public API because every crate above this one
//! uses them. See `tests/contract.rs` for the trait-shape verification.

pub mod audio;
pub mod error;
pub mod mock;
pub mod physics;
pub mod rendering;
pub mod types;

pub use audio::AudioServer;
pub use error::ServerError;
pub use mock::{MockAudio, MockRenderer, RecordedAudioCall, RecordedCall};
pub use physics::{CollisionMask, LinearPhysics, PhysicsServer, RayHit, Shape as PhysicsShape};
pub use rendering::RenderingServer;
pub use types::{
    AudioClipId, AudioFeature, FontId, RenderFeature, SoundId, TextMetrics, TextureId,
};
