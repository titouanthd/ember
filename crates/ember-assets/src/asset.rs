//! The `Asset` trait, its load context and the built-in asset types.

use crate::{AssetError, Uid};
use ember_core::Image;
use ember_servers::{RenderingServer, TextureId};
use serde::de::DeserializeOwned;
use std::path::Path;

pub struct AssetLoadContext<'a> {
    pub path: &'a Path,
    pub uid: &'a Uid,
    /// `Some` only when loaded through `load_with` (ADR-026).
    pub rendering: Option<&'a mut dyn RenderingServer>,
}

pub trait Asset: Send + Sync + Sized + 'static {
    /// Lowercase file extensions this type loads from.
    fn extensions() -> &'static [&'static str];

    fn load(bytes: &[u8], ctx: &mut AssetLoadContext) -> Result<Self, AssetError>;

    /// Hot-reload hook. The default replaces the value.
    fn reload(
        current: &mut Self,
        bytes: &[u8],
        ctx: &mut AssetLoadContext,
    ) -> Result<(), AssetError> {
        *current = Self::load(bytes, ctx)?;
        Ok(())
    }
}

/// A GPU texture. The CPU pixels are not kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub id: TextureId,
    pub width: u32,
    pub height: u32,
}

fn decode_png(bytes: &[u8]) -> Result<Image, AssetError> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| AssetError::Parse(e.to_string()))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| AssetError::Parse(e.to_string()))?;
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px
            .chunks(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => {
            return Err(AssetError::Parse("unexpanded indexed png".into()));
        }
    };
    Image::from_rgba8(info.width, info.height, rgba)
        .ok_or_else(|| AssetError::Parse("png size mismatch".into()))
}

impl Asset for Texture {
    fn extensions() -> &'static [&'static str] {
        &["png"]
    }

    fn load(bytes: &[u8], ctx: &mut AssetLoadContext) -> Result<Self, AssetError> {
        let image = decode_png(bytes)?;
        let path = ctx.path.display().to_string();
        let rendering = ctx
            .rendering
            .as_deref_mut()
            .ok_or(AssetError::NeedsRenderer(path))?;
        let id = rendering
            .create_texture(&image)
            .map_err(|e| AssetError::Server(e.to_string()))?;
        Ok(Self {
            id,
            width: image.width(),
            height: image.height(),
        })
    }

    fn reload(
        current: &mut Self,
        bytes: &[u8],
        ctx: &mut AssetLoadContext,
    ) -> Result<(), AssetError> {
        let fresh = Self::load(bytes, ctx)?;
        if let Some(r) = ctx.rendering.as_deref_mut() {
            r.destroy_texture(current.id);
        }
        *current = fresh;
        Ok(())
    }
}

/// Any RON-deserializable value loaded from a `.ron` file.
#[derive(Debug, Clone, PartialEq)]
pub struct RonAsset<T>(pub T);

impl<T: DeserializeOwned + Send + Sync + 'static> Asset for RonAsset<T> {
    fn extensions() -> &'static [&'static str] {
        &["ron"]
    }

    fn load(bytes: &[u8], _ctx: &mut AssetLoadContext) -> Result<Self, AssetError> {
        let v = ron::de::from_bytes(bytes).map_err(|e| AssetError::Parse(e.to_string()))?;
        Ok(Self(v))
    }
}
