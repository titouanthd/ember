//! `TileSet` and the `TileMap` node (ADR-053).
//!
//! A tile is named by **one character**, its *symbol*: the same legend a
//! level is written in (`#` wall, `.` floor). A `TileMap` stores a
//! symbol per cell and saves its cells in `.escene` as one string per
//! row; `' '` (a space) is an empty cell. A `TileSet` says how each
//! symbol draws: a cell of an atlas texture, tinted, or a plain colour
//! when there is no texture (procedural art, tests).

use crate::{DrawContext, Node, NodeName, PropertyHint, SceneError};
use ember_assets::{Asset, AssetError, AssetLoadContext, AssetServer};
use ember_core::math::Rect;
use ember_core::{Color, Object, Variant};
use ember_servers::{RenderingServer, TextureId};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;

/// The symbol of an empty cell.
pub const EMPTY: char = ' ';

/// One kind of tile.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    /// The tile's id: one printable, non-space character.
    pub symbol: char,
    /// A readable name (an editor palette shows it).
    pub name: String,
    /// The atlas cell `(col, row)` it draws from; `None` draws a
    /// `color` rectangle instead.
    pub atlas: Option<(u32, u32)>,
    /// Sprite tint, or the fill colour without an atlas cell or texture.
    pub color: Color,
}

impl Tile {
    /// A tile drawn as a plain `color` rectangle.
    pub fn solid(symbol: char, name: &str, color: Color) -> Self {
        Self {
            symbol,
            name: name.into(),
            atlas: None,
            color,
        }
    }

    /// A tile drawn from atlas cell `(col, row)`, untinted.
    pub fn atlas(symbol: char, name: &str, col: u32, row: u32) -> Self {
        Self {
            symbol,
            name: name.into(),
            atlas: Some((col, row)),
            color: Color::WHITE,
        }
    }
}

/// The tiles a map can use, and the atlas they draw from.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TileSet {
    /// The atlas image, as an asset path (`"tiles.png"`); empty for none.
    pub texture: String,
    /// Size of one atlas cell, in texture pixels.
    pub tile_size: Vec2,
    /// Gap between atlas cells, and around them, in texture pixels.
    pub spacing: f32,
    pub margin: f32,
    pub tiles: Vec<Tile>,
    /// The loaded atlas: set by [`Self::load_texture`] or
    /// [`Self::with_texture_id`]. Never saved.
    pub texture_id: Option<TextureId>,
}

// --- File format: plain tuples, as `glam` / `Color` have no serde. ---

fn white() -> (f32, f32, f32, f32) {
    (1.0, 1.0, 1.0, 1.0)
}
fn is_white(c: &(f32, f32, f32, f32)) -> bool {
    *c == white()
}
fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

#[derive(Serialize, Deserialize)]
struct TileWire {
    symbol: char,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    atlas: Option<(u32, u32)>,
    #[serde(default = "white", skip_serializing_if = "is_white")]
    color: (f32, f32, f32, f32),
}

#[derive(Serialize, Deserialize)]
struct TileSetWire {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    texture: String,
    #[serde(default)]
    tile_size: (f32, f32),
    #[serde(default, skip_serializing_if = "is_zero")]
    spacing: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    margin: f32,
    tiles: Vec<TileWire>,
}

impl TileSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder: an atlas `texture` with `tile_size`-pixel cells.
    pub fn with_atlas(mut self, texture: &str, tile_size: Vec2) -> Self {
        self.texture = texture.into();
        self.tile_size = tile_size;
        self
    }

    pub fn with_tile(mut self, tile: Tile) -> Self {
        self.tiles.push(tile);
        self
    }

    pub fn with_texture_id(mut self, id: TextureId) -> Self {
        self.texture_id = Some(id);
        self
    }

    /// The tile drawn for `symbol`.
    pub fn tile(&self, symbol: char) -> Option<&Tile> {
        self.tiles.iter().find(|t| t.symbol == symbol)
    }

    /// Where atlas cell `(col, row)` sits in the texture, in pixels.
    pub fn region(&self, col: u32, row: u32) -> Rect {
        let step = self.tile_size + Vec2::splat(self.spacing);
        Rect::new(
            self.margin + col as f32 * step.x,
            self.margin + row as f32 * step.y,
            self.tile_size.x,
            self.tile_size.y,
        )
    }

    /// Symbols are printable, not a space, and unique.
    pub fn validate(&self) -> Result<(), SceneError> {
        for (i, t) in self.tiles.iter().enumerate() {
            if t.symbol == EMPTY || t.symbol.is_control() {
                return Err(SceneError::Parse(format!(
                    "tile `{}` has a blank or control symbol",
                    t.name
                )));
            }
            if self.tiles[..i].iter().any(|u| u.symbol == t.symbol) {
                return Err(SceneError::Parse(format!(
                    "tile symbol `{}` is used twice",
                    t.symbol
                )));
            }
        }
        Ok(())
    }

    /// Parse a `.etileset` file:
    /// `(texture: "tiles.png", tile_size: (16.0, 16.0), tiles: [(symbol: '#', name: "wall", atlas: Some((0, 0)))])`.
    pub fn from_ron(text: &str) -> Result<Self, SceneError> {
        let w: TileSetWire = ron::from_str(text).map_err(|e| SceneError::Parse(e.to_string()))?;
        let set = Self {
            texture: w.texture,
            tile_size: Vec2::new(w.tile_size.0, w.tile_size.1),
            spacing: w.spacing,
            margin: w.margin,
            tiles: w
                .tiles
                .into_iter()
                .map(|t| Tile {
                    symbol: t.symbol,
                    name: t.name,
                    atlas: t.atlas,
                    color: Color::new(t.color.0, t.color.1, t.color.2, t.color.3),
                })
                .collect(),
            texture_id: None,
        };
        set.validate()?;
        Ok(set)
    }

    pub fn to_ron(&self) -> String {
        let w = TileSetWire {
            texture: self.texture.clone(),
            tile_size: (self.tile_size.x, self.tile_size.y),
            spacing: self.spacing,
            margin: self.margin,
            tiles: self
                .tiles
                .iter()
                .map(|t| TileWire {
                    symbol: t.symbol,
                    name: t.name.clone(),
                    atlas: t.atlas,
                    color: (t.color.r, t.color.g, t.color.b, t.color.a),
                })
                .collect(),
        };
        ron::ser::to_string_pretty(&w, ron::ser::PrettyConfig::default())
            .expect("a tile set always serializes")
    }

    /// Load `texture` through `assets` and keep its id. A set with no
    /// texture is left as is.
    pub fn load_texture(
        &mut self,
        assets: &AssetServer,
        rendering: &mut dyn RenderingServer,
    ) -> Result<(), AssetError> {
        if !self.texture.is_empty() {
            self.texture_id = Some(assets.texture(&self.texture, rendering)?.get().id);
        }
        Ok(())
    }
}

impl Asset for TileSet {
    fn extensions() -> &'static [&'static str] {
        &["etileset"]
    }

    /// The data only; call [`TileSet::load_texture`] for the atlas.
    fn load(bytes: &[u8], _ctx: &mut AssetLoadContext) -> Result<Self, AssetError> {
        let text = std::str::from_utf8(bytes).map_err(|e| AssetError::Parse(e.to_string()))?;
        Self::from_ron(text).map_err(|e| AssetError::Parse(e.to_string()))
    }
}

// --- TileMap ---------------------------------------------------------

/// A grid of tile symbols drawn through a [`TileSet`]. Cell `(0, 0)` is
/// the top-left one, at `position` in world space; each cell is
/// `cell_size` world units.
///
/// Saved properties: `position`, `cell_size`, `tile_set` (the set's
/// asset path) and `rows` (one string per row, `' '` for empty), plus
/// `visible` and `z` as the scene entry's own fields.
#[derive(Debug, Clone)]
pub struct TileMap {
    name: String,
    pub position: Vec2,
    pub cell_size: Vec2,
    pub z: f32,
    pub visible: bool,
    /// Asset path of the tile set, saved with the scene; the loaded set
    /// is given with [`Self::set_tile_set`].
    pub tile_set_path: String,
    tile_set: Option<Arc<TileSet>>,
    width: usize,
    height: usize,
    cells: Vec<char>,
}

impl Default for TileMap {
    fn default() -> Self {
        Self {
            name: "TileMap".into(),
            position: Vec2::ZERO,
            cell_size: Vec2::splat(32.0),
            z: 0.0,
            visible: true,
            tile_set_path: String::new(),
            tile_set: None,
            width: 0,
            height: 0,
            cells: Vec::new(),
        }
    }
}

impl TileMap {
    /// An empty `width × height` map with `cell_size` cells.
    pub fn new(width: usize, height: usize, cell_size: Vec2) -> Self {
        Self {
            cell_size,
            width,
            height,
            cells: vec![EMPTY; width * height],
            ..Self::default()
        }
    }

    /// A map read from `rows` (see [`Self::set_rows`]).
    pub fn from_rows(rows: &[&str], cell_size: Vec2) -> Self {
        let mut map = Self::new(0, 0, cell_size);
        map.set_rows(rows);
        map
    }

    pub fn named(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_tile_set(mut self, set: Arc<TileSet>) -> Self {
        self.tile_set = Some(set);
        self
    }

    pub fn at(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    // --- Cells ---

    /// The symbol at `(col, row)`; `None` when empty or out of bounds.
    pub fn cell(&self, col: usize, row: usize) -> Option<char> {
        self.index(col, row)
            .map(|i| self.cells[i])
            .filter(|c| *c != EMPTY)
    }

    /// Put `symbol` at `(col, row)` (`None` empties it). Returns `false`
    /// out of bounds or for a control character.
    pub fn set_cell(&mut self, col: usize, row: usize, symbol: Option<char>) -> bool {
        let symbol = symbol.unwrap_or(EMPTY);
        if symbol.is_control() {
            return false;
        }
        match self.index(col, row) {
            Some(i) => {
                self.cells[i] = symbol;
                true
            }
            None => false,
        }
    }

    /// Set every cell to `symbol`.
    pub fn fill(&mut self, symbol: Option<char>) {
        let s = symbol.filter(|c| !c.is_control()).unwrap_or(EMPTY);
        self.cells.fill(s);
    }

    /// Change the size, keeping what fits from the top-left; new cells
    /// are empty.
    pub fn resize(&mut self, width: usize, height: usize) {
        let mut cells = vec![EMPTY; width * height];
        for row in 0..height.min(self.height) {
            for col in 0..width.min(self.width) {
                cells[row * width + col] = self.cells[row * self.width + col];
            }
        }
        self.width = width;
        self.height = height;
        self.cells = cells;
    }

    /// Every non-empty cell, row by row: `(col, row, symbol)`.
    pub fn used_cells(&self) -> impl Iterator<Item = (usize, usize, char)> + '_ {
        let w = self.width.max(1);
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, c)| **c != EMPTY)
            .map(move |(i, c)| (i % w, i / w, *c))
    }

    /// One string per row, trailing empty cells kept (every row is
    /// `width` characters).
    pub fn rows(&self) -> Vec<String> {
        self.cells
            .chunks(self.width.max(1))
            .take(self.height)
            .map(|r| r.iter().collect())
            .collect()
    }

    /// Replace the cells with `rows`: the map becomes as wide as the
    /// longest row (shorter rows are padded with empty cells) and as tall
    /// as `rows`. Control characters read as empty.
    pub fn set_rows<S: AsRef<str>>(&mut self, rows: &[S]) {
        let width = rows
            .iter()
            .map(|r| r.as_ref().chars().count())
            .max()
            .unwrap_or(0);
        let mut cells = vec![EMPTY; width * rows.len()];
        for (y, r) in rows.iter().enumerate() {
            for (x, c) in r.as_ref().chars().enumerate() {
                if !c.is_control() {
                    cells[y * width + x] = c;
                }
            }
        }
        self.width = width;
        self.height = if width == 0 { 0 } else { rows.len() };
        if width == 0 {
            cells.clear();
        }
        self.cells = cells;
    }

    // --- Tile set ---

    pub fn set_tile_set(&mut self, set: Arc<TileSet>) {
        self.tile_set = Some(set);
    }

    pub fn tile_set(&self) -> Option<&TileSet> {
        self.tile_set.as_deref()
    }

    /// The tile drawn at `(col, row)`, when the cell is used and the set
    /// knows its symbol.
    pub fn tile_at(&self, col: usize, row: usize) -> Option<&Tile> {
        self.tile_set()?.tile(self.cell(col, row)?)
    }

    // --- Cell <-> world ---

    /// The world rectangle of `(col, row)` (any cell, in bounds or not).
    pub fn cell_rect(&self, col: usize, row: usize) -> Rect {
        let p = self.cell_to_world(col, row);
        Rect::new(p.x, p.y, self.cell_size.x, self.cell_size.y)
    }

    /// The world position of `(col, row)`'s top-left corner.
    pub fn cell_to_world(&self, col: usize, row: usize) -> Vec2 {
        self.position + Vec2::new(col as f32, row as f32) * self.cell_size
    }

    /// The world position of `(col, row)`'s centre.
    pub fn cell_center(&self, col: usize, row: usize) -> Vec2 {
        self.cell_to_world(col, row) + self.cell_size * 0.5
    }

    /// The cell under `world`, or `None` outside the map (or with a
    /// non-positive `cell_size`). A point on a shared edge belongs to
    /// the cell right / below it.
    pub fn world_to_cell(&self, world: Vec2) -> Option<(usize, usize)> {
        if self.cell_size.x <= 0.0 || self.cell_size.y <= 0.0 {
            return None;
        }
        let local = (world - self.position) / self.cell_size;
        if local.x < 0.0 || local.y < 0.0 {
            return None;
        }
        let (col, row) = (local.x.floor() as usize, local.y.floor() as usize);
        (col < self.width && row < self.height).then_some((col, row))
    }

    /// The whole map's world rectangle.
    pub fn bounds(&self) -> Rect {
        Rect::new(
            self.position.x,
            self.position.y,
            self.width as f32 * self.cell_size.x,
            self.height as f32 * self.cell_size.y,
        )
    }

    fn index(&self, col: usize, row: usize) -> Option<usize> {
        (col < self.width && row < self.height).then(|| row * self.width + col)
    }
}

impl Object for TileMap {
    fn class_name() -> &'static str {
        "TileMap"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

const PROPERTIES: &[&str] = &["position", "cell_size", "tile_set", "rows", "visible", "z"];

impl NodeName for TileMap {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn property_list(&self) -> &'static [&'static str] {
        PROPERTIES
    }

    fn get_property(&self, key: &str) -> Option<Variant> {
        Some(match key {
            "position" => Variant::Vec2(self.position),
            "cell_size" => Variant::Vec2(self.cell_size),
            "tile_set" => Variant::String(self.tile_set_path.clone()),
            "rows" => Variant::Array(self.rows().into_iter().map(Variant::String).collect()),
            "visible" => Variant::Bool(self.visible),
            "z" => Variant::Float(f64::from(self.z)),
            _ => return None,
        })
    }

    fn set_property(&mut self, key: &str, value: &Variant) -> bool {
        match (key, value) {
            ("position", Variant::Vec2(v)) => self.position = *v,
            ("cell_size", Variant::Vec2(v)) => self.cell_size = *v,
            ("tile_set", Variant::String(s)) => self.tile_set_path = s.clone(),
            ("rows", Variant::Array(items)) => {
                let mut rows = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Variant::String(s) => rows.push(s.as_str()),
                        _ => return false,
                    }
                }
                self.set_rows(&rows);
            }
            ("visible", Variant::Bool(b)) => self.visible = *b,
            ("z", Variant::Float(f)) => self.z = *f as f32,
            ("z", Variant::Int(i)) => self.z = *i as f32,
            _ => return false,
        }
        true
    }

    fn property_hint(&self, _key: &str) -> PropertyHint {
        PropertyHint::None
    }
}

impl Node for TileMap {
    fn z(&self) -> f32 {
        self.z
    }

    fn visible(&self) -> bool {
        self.visible
    }

    /// Row by row, top-left first. A cell whose symbol the set does not
    /// know, or every cell without a set, draws nothing.
    fn draw(&self, ctx: &mut DrawContext) {
        let Some(set) = self.tile_set() else { return };
        for (col, row, symbol) in self.used_cells() {
            let Some(tile) = set.tile(symbol) else {
                continue;
            };
            let dst = self.cell_rect(col, row);
            match (set.texture_id, tile.atlas) {
                (Some(tex), Some((c, r))) => {
                    ctx.render()
                        .draw_sprite(tex, dst, Some(set.region(c, r)), tile.color)
                }
                _ => ctx.render().draw_rect(dst, tile.color),
            }
        }
    }
}
