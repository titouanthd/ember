//! Layered parallax background for the surface zone.
//!
//! A single zone for V1 — warm daylight sky, distant hills, three
//! layers of pine silhouettes at increasing depth, drifting clouds,
//! birds, and wind strokes. All generated procedurally from a zone
//! seed and baked into per-layer textures at startup.
//!
//! Layers are tiled horizontally with x-parallax. Their bottom edge
//! sits on the horizon line (world Y = `surface_row * TILE_SIZE`), so
//! the world tiles below it naturally occlude them.
//!
//! **Atmospheric perspective** is color + alpha: far layers are pale
//! blue-grey, and every layer's top edge fades to alpha 0 so trees
//! and hills dissolve into the sky instead of ending in a hard line.

use macroquad::prelude::*;

use crate::pixel::{Canvas, Rgba};
use crate::rng::Rng;

/// Width of every layer's canvas, in pixels. Tiles horizontally.
const LAYER_W: u16 = 512;
/// Height of every layer's canvas, in pixels. Tall enough for the
/// tallest pine (68 px).
const LAYER_H: u16 = 96;

/// How many pixels at the top of each column's content fade out.
/// Gives a "mist into sky" look at the tops of trees and hills.
const TOP_FADE_DEPTH: i32 = 10;

// ---------- Zone palette ----------

const FAR_HILL: Rgba = [0xA8, 0xB8, 0xCC, 0xFF];
const FAR_PINE: Rgba = [0x8A, 0x9D, 0xB0, 0xFF];
const MID_PINE: Rgba = [0x6A, 0x85, 0x70, 0xFF];
const NEAR_PINE: Rgba = [0x45, 0x6B, 0x48, 0xFF];

// ---------- Layer kinds ----------

enum LayerKind {
    Hills { count: u32, min_h: i32, max_h: i32, min_w: i32, max_w: i32 },
    Pines { count: u32, min_h: i32, max_h: i32, min_w: i32, max_w: i32 },
}

struct LayerSpec {
    kind: LayerKind,
    color: Rgba,
    parallax: f32,
    phase: f32,
}

const LAYER_SPECS: &[LayerSpec] = &[
    LayerSpec {
        kind: LayerKind::Hills {
            count: 4,
            min_h: 40,
            max_h: 66,
            min_w: 90,
            max_w: 170,
        },
        color: FAR_HILL,
        parallax: 0.05,
        phase: 0.0,
    },
    LayerSpec {
        kind: LayerKind::Pines {
            count: 20,
            min_h: 20,
            max_h: 34,
            min_w: 8,
            max_w: 14,
        },
        color: FAR_PINE,
        parallax: 0.12,
        phase: 64.0,
    },
    LayerSpec {
        kind: LayerKind::Pines {
            count: 15,
            min_h: 32,
            max_h: 52,
            min_w: 13,
            max_w: 20,
        },
        color: MID_PINE,
        parallax: 0.24,
        phase: 128.0,
    },
    LayerSpec {
        kind: LayerKind::Pines {
            count: 11,
            min_h: 44,
            max_h: 68,
            min_w: 16,
            max_w: 26,
        },
        color: NEAR_PINE,
        parallax: 0.42,
        phase: 192.0,
    },
];

// ---------- Pure data (pre-GPU) ----------

pub struct BackgroundData {
    layers: Vec<Canvas>,
    sun: Sun,
    clouds: Vec<Cloud>,
    birds: Vec<Bird>,
    winds: Vec<Wind>,
}

impl BackgroundData {
    pub fn from_seed(seed: u32) -> Self {
        let mut rng = Rng::new(seed ^ 0xB1_5D_1E_51);

        let mut layers = Vec::with_capacity(LAYER_SPECS.len());
        for spec in LAYER_SPECS {
            layers.push(render_layer(spec, &mut rng));
        }

        let sun = Sun {
            x: 340.0 + rng.next_f32() * 60.0,
            y: -70.0 - rng.next_f32() * 20.0,
            radius: 70.0 + rng.next_f32() * 20.0,
            color: Color::new(1.0, 0.94, 0.78, 1.0),
        };

        let mut clouds = Vec::with_capacity(5);
        for _ in 0..5 {
            clouds.push(Cloud {
                x: rng.next_f32() * 640.0,
                y: -110.0 - rng.next_f32() * 30.0,
                w: 40.0 + rng.next_f32() * 50.0,
                h: 8.0 + rng.next_f32() * 6.0,
                alpha: 0.42 + rng.next_f32() * 0.20,
                vx: 2.0 + rng.next_f32() * 3.0,
            });
        }

        let mut birds = Vec::with_capacity(6);
        for _ in 0..6 {
            birds.push(Bird {
                x: rng.next_f32() * LAYER_W as f32,
                y: -85.0 + rng.next_f32() * 40.0,
                vx: 5.0 + rng.next_f32() * 8.0,
                phase: rng.next_f32() * std::f32::consts::TAU,
                size: 2.0 + rng.next_f32() * 1.5,
            });
        }

        let mut winds = Vec::with_capacity(14);
        for _ in 0..14 {
            winds.push(Wind {
                x: rng.next_f32() * LAYER_W as f32,
                y: -90.0 + rng.next_f32() * 80.0,
                len: 10.0 + rng.next_f32() * 18.0,
                speed: 18.0 + rng.next_f32() * 26.0,
                alpha: 0.05 + rng.next_f32() * 0.07,
            });
        }

        Self { layers, sun, clouds, birds, winds }
    }

    pub fn upload(self) -> Background {
        let layers = self
            .layers
            .into_iter()
            .zip(LAYER_SPECS.iter())
            .map(|(canvas, spec)| {
                let texture =
                    Texture2D::from_rgba8(canvas.width, canvas.height, &canvas.pixels);
                texture.set_filter(FilterMode::Nearest);
                UploadedLayer {
                    texture,
                    parallax: spec.parallax,
                    phase: spec.phase,
                }
            })
            .collect();

        Background {
            layers,
            sun: self.sun,
            clouds: self.clouds,
            birds: self.birds,
            winds: self.winds,
        }
    }
}

// ---------- Runtime ----------

pub struct Background {
    layers: Vec<UploadedLayer>,
    sun: Sun,
    clouds: Vec<Cloud>,
    birds: Vec<Bird>,
    winds: Vec<Wind>,
}

struct UploadedLayer {
    texture: Texture2D,
    parallax: f32,
    phase: f32,
}

struct Sun {
    x: f32,
    y: f32,
    radius: f32,
    color: Color,
}

struct Cloud {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    alpha: f32,
    vx: f32,
}

struct Bird {
    x: f32,
    y: f32,
    vx: f32,
    phase: f32,
    size: f32,
}

struct Wind {
    x: f32,
    y: f32,
    len: f32,
    speed: f32,
    alpha: f32,
}

impl Background {
    pub fn tick(&mut self, dt: f32) {
        for c in &mut self.clouds {
            c.x += c.vx * dt;
            if c.x > 640.0 + c.w {
                c.x = -c.w;
            }
        }
        for b in &mut self.birds {
            b.x += b.vx * dt;
            if b.x > LAYER_W as f32 {
                b.x -= LAYER_W as f32;
            }
            b.phase += dt * 6.0;
        }
        for w in &mut self.winds {
            w.x += w.speed * dt;
            if w.x > LAYER_W as f32 + w.len {
                w.x = -w.len;
            }
        }
    }

    pub fn draw(
        &self,
        cam_x: f32,
        horizon_screen_y: f32,
        screen_ox: f32,
        vw: f32,
        scale: f32,
    ) {
        draw_sun_glow(&self.sun, horizon_screen_y, screen_ox, scale);

        for c in &self.clouds {
            let cx = screen_ox + (c.x - cam_x * 0.04) * scale;
            let cy = horizon_screen_y + c.y * scale;
            draw_rectangle(
                cx,
                cy,
                c.w * scale,
                c.h * scale,
                Color::new(1.0, 1.0, 1.0, c.alpha),
            );
        }

        for layer in &self.layers {
            self.draw_layer(layer, cam_x, horizon_screen_y, screen_ox, vw, scale);
        }

        let sxp = |wx: f32| -> f32 { (screen_ox + wx * scale).floor() };
        for b in &self.birds {
            let sky_x = (b.x - cam_x * 0.30).rem_euclid(LAYER_W as f32);
            let sky_y = horizon_screen_y + b.y * scale;
            draw_bird(sxp(sky_x), sky_y, (b.size * scale).max(2.0), b.phase);
        }

        for w in &self.winds {
            let range = LAYER_W as f32 + w.len;
            let sky_x = (w.x - cam_x * 0.55).rem_euclid(range) - w.len;
            let sky_y = horizon_screen_y + w.y * scale;
            let thickness = (0.5 * scale).max(1.0);
            draw_rectangle(
                sxp(sky_x),
                sky_y,
                w.len * scale,
                thickness,
                Color::new(1.0, 1.0, 1.0, w.alpha),
            );
        }
    }

    fn draw_layer(
        &self,
        layer: &UploadedLayer,
        cam_x: f32,
        horizon_screen_y: f32,
        screen_ox: f32,
        vw: f32,
        scale: f32,
    ) {
        let w_scaled = LAYER_W as f32 * scale;
        let h_scaled = LAYER_H as f32 * scale;
        let offset = -(cam_x * layer.parallax + layer.phase);
        let start_world = offset.rem_euclid(LAYER_W as f32) - LAYER_W as f32;
        let start_sx = screen_ox + start_world * scale;

        let mut x = start_sx;
        while x < screen_ox + vw {
            draw_texture_ex(
                &layer.texture,
                x,
                horizon_screen_y - h_scaled,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(w_scaled, h_scaled)),
                    ..Default::default()
                },
            );
            x += w_scaled;
        }
    }
}

// ---------- Layer rendering (pure) ----------

fn render_layer(spec: &LayerSpec, rng: &mut Rng) -> Canvas {
    let mut c = Canvas::new(LAYER_W, LAYER_H);
    let horizon_y = LAYER_H as i32 - 1;

    match &spec.kind {
        LayerKind::Hills { count, min_h, max_h, min_w, max_w } => {
            for _ in 0..*count {
                let w = rng.range(*min_w, *max_w + 1);
                let h = rng.range(*min_h, *max_h + 1);
                let cx = rng.range(w / 2, LAYER_W as i32 - w / 2);
                draw_hill_wrapped(&mut c, cx, horizon_y, w, h, spec.color);
            }
        }
        LayerKind::Pines { count, min_h, max_h, min_w, max_w } => {
            for _ in 0..*count {
                let w = rng.range(*min_w, *max_w + 1);
                let h = rng.range(*min_h, *max_h + 1);
                let cx = rng.range(w / 2, LAYER_W as i32 - w / 2);
                draw_pine_wrapped(&mut c, cx, horizon_y, w, h, spec.color);
            }
        }
    }

    // Soft fade at the top of every column's content, so trees and
    // hills dissolve into the sky instead of ending on a hard edge.
    apply_top_alpha_fade(&mut c, TOP_FADE_DEPTH);

    c
}

/// Per-column: find the topmost content pixel, then fade the `depth`
/// pixels below it from alpha 0 (top) to unchanged (bottom). Short
/// and tall shapes alike get their tops softened.
fn apply_top_alpha_fade(canvas: &mut Canvas, depth: i32) {
    if depth <= 0 {
        return;
    }
    let w = canvas.width as i32;
    let h = canvas.height as i32;
    for x in 0..w {
        let mut top = None;
        for y in 0..h {
            if canvas.get(x, y).is_some_and(|p| p[3] > 0) {
                top = Some(y);
                break;
            }
        }
        let Some(ty) = top else { continue };
        for dy in 0..depth {
            let y = ty + dy;
            if y >= h {
                break;
            }
            let t = dy as f32 / depth as f32;
            if let Some(p) = canvas.get(x, y) {
                let new_a = (p[3] as f32 * t) as u8;
                canvas.set(x, y, [p[0], p[1], p[2], new_a]);
            }
        }
    }
}

fn draw_hill_wrapped(c: &mut Canvas, cx: i32, base_y: i32, w: i32, h: i32, color: Rgba) {
    draw_hill(c, cx, base_y, w, h, color);
    draw_hill(c, cx - LAYER_W as i32, base_y, w, h, color);
}

fn draw_hill(c: &mut Canvas, cx: i32, base_y: i32, w: i32, h: i32, color: Rgba) {
    let half_w = w / 2;
    for dy in 0..h {
        let t = dy as f32 / h.max(1) as f32;
        let hw = ((t * std::f32::consts::FRAC_PI_2).sin() * half_w as f32) as i32;
        c.hline(cx - hw, cx + hw, base_y - dy, color);
    }
}

fn draw_pine_wrapped(c: &mut Canvas, cx: i32, base_y: i32, w: i32, h: i32, color: Rgba) {
    draw_pine(c, cx, base_y, w, h, color);
    draw_pine(c, cx - LAYER_W as i32, base_y, w, h, color);
}

fn draw_pine(c: &mut Canvas, cx: i32, base_y: i32, w: i32, h: i32, color: Rgba) {
    let trunk_h = (h / 5).max(2);
    let trunk_w = (w / 6).max(1);
    c.rect(cx - trunk_w / 2, base_y - trunk_h, trunk_w, trunk_h, color);

    let canopy_h = h - trunk_h;
    let half_w = w / 2;
    let notch = (canopy_h / 5).max(2);

    for dy in 0..canopy_h {
        let t = dy as f32 / canopy_h.max(1) as f32;
        let mut hw = (t * half_w as f32).round() as i32;
        if dy > 0 && dy % notch == 0 {
            hw = (hw - 1).max(0);
        }
        c.hline(cx - hw, cx + hw, base_y - canopy_h + dy, color);
    }
}

fn draw_sun_glow(sun: &Sun, horizon_screen_y: f32, screen_ox: f32, scale: f32) {
    let cx = screen_ox + sun.x * scale;
    let cy = horizon_screen_y + sun.y * scale;
    let r = sun.radius * scale;
    let steps = 10;
    for i in (0..steps).rev() {
        let rr = r * (i as f32 + 1.0) / steps as f32;
        let a = sun.color.a * (1.0 - i as f32 / steps as f32) * 0.12;
        draw_circle(
            cx,
            cy,
            rr,
            Color::new(sun.color.r, sun.color.g, sun.color.b, a),
        );
    }
}

fn draw_bird(cx: f32, cy: f32, size: f32, phase: f32) {
    let flap = phase.sin() * size * 0.35;
    let color = Color::new(0.12, 0.10, 0.09, 0.85);
    let thickness = (size / 2.0).max(1.0);
    draw_line(cx, cy, cx - size, cy - size * 0.6 - flap, thickness, color);
    draw_line(cx, cy, cx + size, cy - size * 0.6 - flap, thickness, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_seed_is_deterministic() {
        let a = BackgroundData::from_seed(42);
        let b = BackgroundData::from_seed(42);
        assert_eq!(a.layers.len(), b.layers.len());
        for (ca, cb) in a.layers.iter().zip(b.layers.iter()) {
            assert_eq!(ca.pixels, cb.pixels);
        }
    }

    #[test]
    fn different_seeds_differ() {
        let a = BackgroundData::from_seed(1);
        let b = BackgroundData::from_seed(2);
        let differs = a
            .layers
            .iter()
            .zip(b.layers.iter())
            .any(|(ca, cb)| ca.pixels != cb.pixels);
        assert!(differs);
    }

    #[test]
    fn all_layers_are_non_empty() {
        let data = BackgroundData::from_seed(42);
        for (i, layer) in data.layers.iter().enumerate() {
            assert!(!layer.is_empty(), "layer {i} is empty");
        }
    }

    #[test]
    fn layers_are_layer_w_by_layer_h() {
        let data = BackgroundData::from_seed(42);
        for layer in &data.layers {
            assert_eq!(layer.width, LAYER_W);
            assert_eq!(layer.height, LAYER_H);
        }
    }

    #[test]
    fn layer_count_matches_specs() {
        let data = BackgroundData::from_seed(42);
        assert_eq!(data.layers.len(), LAYER_SPECS.len());
    }

    #[test]
    fn clouds_and_birds_are_in_bounds() {
        let data = BackgroundData::from_seed(42);
        for c in &data.clouds {
            assert!(c.alpha > 0.0 && c.alpha < 1.0);
            assert!(c.w > 0.0 && c.h > 0.0);
        }
        for b in &data.birds {
            assert!(b.size > 0.0);
        }
    }

    #[test]
    fn top_fade_lowers_alpha_at_top() {
        // A fully opaque 4x20 block should have lower alpha at its
        // top after the fade, and unchanged alpha at the bottom.
        let mut c = Canvas::new(4, 20);
        for y in 0..20 {
            for x in 0..4 {
                c.set(x, y, [100, 100, 100, 255]);
            }
        }
        apply_top_alpha_fade(&mut c, 5);
        let top_alpha = c.get(0, 0).unwrap()[3];
        let bottom_alpha = c.get(0, 19).unwrap()[3];
        assert!(top_alpha < bottom_alpha, "top {top_alpha} bottom {bottom_alpha}");
        assert_eq!(bottom_alpha, 255);
    }

    #[test]
    fn top_fade_empty_canvas_noop() {
        let mut c = Canvas::new(4, 20);
        apply_top_alpha_fade(&mut c, 5);
        assert!(c.is_empty());
    }
}