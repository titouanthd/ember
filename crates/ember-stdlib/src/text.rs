//! Text helpers on a `RenderingServer`: aligned, shadowed and bold text
//! (v0.2 V1.4). They replace the `centered` / `shadowed` / `draw_bold`
//! functions games kept after the macroquad `fonts` / `graphics` modules
//! went away (M5.1).
//!
//! Positions are an **anchor x** and a **baseline y**, as
//! `RenderingServer::draw_text` takes them. The anchor is the left edge,
//! the centre or the right edge of the text, per [`Align`].
//!
//! ```ignore
//! use ember_stdlib::text::{self, TextStyle};
//! text::centered(r, "GAME OVER", cx, cy, 48.0, Color::WHITE);
//! let title = TextStyle::new(64.0, gold).centered().shadow(Vec2::splat(3.0), Color::BLACK).bold();
//! text::draw(r, "PAGODA", Vec2::new(cx, 120.0), &title);
//! ```

use ember_core::Color;
use ember_servers::{FontId, RenderingServer, TextMetrics};
use glam::Vec2;

/// How far apart the two passes of [`TextStyle::bold`] are drawn, in
/// pixels (as ember-wars' fake bold).
pub const BOLD_OFFSET: f32 = 0.6;

/// Where the anchor x sits on the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// The anchor is the left edge (plain `draw_text`).
    #[default]
    Left,
    Center,
    Right,
}

/// A drop shadow: the text drawn first, `offset` away, in `color`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub offset: Vec2,
    pub color: Color,
}

/// How to draw a piece of text. Built with chained calls from
/// [`TextStyle::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub color: Color,
    /// `None`: the renderer's default font.
    pub font: Option<FontId>,
    pub align: Align,
    pub shadow: Option<Shadow>,
    /// Fake bold: a second pass this many pixels to the right; `0.0` is
    /// off.
    pub bold: f32,
}

impl TextStyle {
    /// Left-aligned, default font, no shadow, not bold.
    pub fn new(size: f32, color: Color) -> Self {
        Self {
            size,
            color,
            font: None,
            align: Align::Left,
            shadow: None,
            bold: 0.0,
        }
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn centered(self) -> Self {
        self.align(Align::Center)
    }

    pub fn right(self) -> Self {
        self.align(Align::Right)
    }

    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn shadow(mut self, offset: Vec2, color: Color) -> Self {
        self.shadow = Some(Shadow { offset, color });
        self
    }

    /// Fake bold, [`BOLD_OFFSET`] apart.
    pub fn bold(self) -> Self {
        self.bold_by(BOLD_OFFSET)
    }

    pub fn bold_by(mut self, offset: f32) -> Self {
        self.bold = offset.max(0.0);
        self
    }
}

/// `text` measured in `style`'s font and size. A bold style is
/// `style.bold` wider; the shadow is not counted.
pub fn measure(r: &dyn RenderingServer, text: &str, style: &TextStyle) -> TextMetrics {
    let mut m = match style.font {
        Some(font) => r.measure_text_with_font(font, text, style.size),
        None => r.measure_text(text, style.size),
    };
    if !text.is_empty() {
        m.width += style.bold;
    }
    m
}

/// The left edge `text` starts at when anchored at `x` in `style`.
pub fn left_edge(r: &dyn RenderingServer, text: &str, x: f32, style: &TextStyle) -> f32 {
    let width = || measure(r, text, style).width;
    match style.align {
        Align::Left => x,
        Align::Center => x - width() * 0.5,
        Align::Right => x - width(),
    }
}

/// Draw `text` anchored at `pos` (anchor x, baseline y): the shadow
/// first (bold too, if the style is), then the text. Empty text draws
/// nothing.
pub fn draw(r: &mut dyn RenderingServer, text: &str, pos: Vec2, style: &TextStyle) {
    if text.is_empty() {
        return;
    }
    let at = Vec2::new(left_edge(r, text, pos.x, style), pos.y);
    if let Some(shadow) = style.shadow {
        pass(r, text, at + shadow.offset, style, shadow.color);
    }
    pass(r, text, at, style, style.color);
}

fn pass(r: &mut dyn RenderingServer, text: &str, at: Vec2, style: &TextStyle, color: Color) {
    let one = |r: &mut dyn RenderingServer, p: Vec2| match style.font {
        Some(font) => r.draw_text_with_font(font, text, p, style.size, color),
        None => r.draw_text(text, p, style.size, color),
    };
    one(r, at);
    if style.bold > 0.0 {
        one(r, at + Vec2::new(style.bold, 0.0));
    }
}

/// Default-font text centred on `cx`, baseline at `y`.
pub fn centered(r: &mut dyn RenderingServer, text: &str, cx: f32, y: f32, size: f32, color: Color) {
    draw(
        r,
        text,
        Vec2::new(cx, y),
        &TextStyle::new(size, color).centered(),
    );
}

/// Default-font text whose right edge sits on `rx`, baseline at `y`.
pub fn right(r: &mut dyn RenderingServer, text: &str, rx: f32, y: f32, size: f32, color: Color) {
    draw(
        r,
        text,
        Vec2::new(rx, y),
        &TextStyle::new(size, color).right(),
    );
}

/// [`centered`] over a black drop shadow `offset` px down-right.
pub fn centered_shadowed(
    r: &mut dyn RenderingServer,
    text: &str,
    cx: f32,
    y: f32,
    size: f32,
    color: Color,
    offset: f32,
) {
    let style = TextStyle::new(size, color)
        .centered()
        .shadow(Vec2::splat(offset), Color::BLACK);
    draw(r, text, Vec2::new(cx, y), &style);
}
