//! Textures procédurales — ciel, parallax layers, sol, tours.
//!
//! Tout est dessiné en code à partir d'une palette et d'un seed. Aucun
//! asset externe. Les palettes sont chargées depuis `assets/palettes.ron`.

use std::path::Path;
use std::sync::OnceLock;

use ember_core::io::load_from_file;
use glam::Vec2;
use macroquad::prelude::*;
use serde::Deserialize;

// ---------- Palette ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SceneStyle {
    Urban,
    Mountain,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Palette {
    pub id: String,
    pub style: SceneStyle,
    pub sky_top: (f32, f32, f32, f32),
    pub sky_bottom: (f32, f32, f32, f32),
    pub far_layer: (f32, f32, f32, f32),
    pub mid_layer: (f32, f32, f32, f32),
    pub ground_base: (f32, f32, f32, f32),
    pub ground_accent: (f32, f32, f32, f32),
    pub tower_player: (f32, f32, f32, f32),
    pub tower_enemy: (f32, f32, f32, f32),
    pub window_color: (f32, f32, f32, f32),
    pub accent_light: (f32, f32, f32, f32),
    pub neon_1: (f32, f32, f32, f32),
    pub neon_2: (f32, f32, f32, f32),
    pub mist_color: (f32, f32, f32, f32),
}

fn col(c: (f32, f32, f32, f32)) -> Color {
    Color::new(c.0, c.1, c.2, c.3)
}

impl Palette {
    pub fn sky_top_c(&self) -> Color {
        col(self.sky_top)
    }
    pub fn sky_bottom_c(&self) -> Color {
        col(self.sky_bottom)
    }
    pub fn far_layer_c(&self) -> Color {
        col(self.far_layer)
    }
    pub fn mid_layer_c(&self) -> Color {
        col(self.mid_layer)
    }
    pub fn ground_base_c(&self) -> Color {
        col(self.ground_base)
    }
    pub fn ground_accent_c(&self) -> Color {
        col(self.ground_accent)
    }
    pub fn tower_player_c(&self) -> Color {
        col(self.tower_player)
    }
    pub fn tower_enemy_c(&self) -> Color {
        col(self.tower_enemy)
    }
    pub fn window_c(&self) -> Color {
        col(self.window_color)
    }
    pub fn accent_light_c(&self) -> Color {
        col(self.accent_light)
    }
    pub fn neon_1_c(&self) -> Color {
        col(self.neon_1)
    }
    pub fn neon_2_c(&self) -> Color {
        col(self.neon_2)
    }
    pub fn mist_c(&self) -> Color {
        col(self.mist_color)
    }
}

static PALETTES: OnceLock<Vec<Palette>> = OnceLock::new();

pub fn all_palettes() -> &'static [Palette] {
    PALETTES
        .get_or_init(|| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("palettes.ron");
            load_from_file(&path).expect("assets/palettes.ron should load and parse")
        })
        .as_slice()
}

pub fn palette_by_id(id: &str) -> Option<&'static Palette> {
    all_palettes().iter().find(|p| p.id == id)
}

// ---------- Hash déterministe ----------

pub fn hash_u32(seed: u32, x: u32) -> u32 {
    let mut h = seed ^ x.wrapping_mul(0x9E37_79B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

// ---------- Générateurs de hauteur ----------

pub fn far_urban_height(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed, chunk as u32);
    80.0 + (h % 220) as f32
}

pub fn mid_urban_height(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed.wrapping_add(0x9E37_79B9), chunk as u32);
    120.0 + (h % 160) as f32
}

pub fn far_mountain_height(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed.wrapping_add(0x1234_5678), chunk as u32);
    200.0 + (h % 180) as f32
}

pub fn mid_mountain_height(seed: u32, chunk: i32) -> f32 {
    let h = hash_u32(seed.wrapping_add(0xCAFE_BABE), chunk as u32);
    100.0 + (h % 120) as f32
}

// ---------- Constantes ----------

pub const SKY_STRIPS: usize = 24;
pub const FAR_CHUNK_W: f32 = 90.0;
pub const MID_CHUNK_W: f32 = 70.0;
pub const FAR_PARALLAX: f32 = 0.25;
pub const MID_PARALLAX: f32 = 0.55;
pub const NEON_PARALLAX: f32 = 0.75;

// ---------- Dessin : ciel ----------

pub fn draw_sky(palette: &Palette, ground_y: f32) {
    let vw = screen_width();
    let strip_h = (ground_y / SKY_STRIPS as f32).max(1.0);
    let top = palette.sky_top_c();
    let bottom = palette.sky_bottom_c();

    for i in 0..SKY_STRIPS {
        let t = i as f32 / (SKY_STRIPS - 1) as f32;
        let c = Color::new(
            top.r * (1.0 - t) + bottom.r * t,
            top.g * (1.0 - t) + bottom.g * t,
            top.b * (1.0 - t) + bottom.b * t,
            1.0,
        );
        draw_rectangle(0.0, i as f32 * strip_h, vw, strip_h + 1.0, c);
    }

    // Étoiles en haut du ciel (Urban uniquement).
    if palette.style == SceneStyle::Urban {
        draw_stars(palette, ground_y);
    }
}

fn draw_stars(palette: &Palette, ground_y: f32) {
    let vw = screen_width();
    let star_zone_h = ground_y * 0.35;
    let window_c = palette.window_c();

    // 40 étoiles réparties sur la zone haute.
    for i in 0..40u32 {
        let h1 = hash_u32(0x51A1_5EED, i);
        let h2 = hash_u32(0xBEEF_CAFE, i);
        let sx = (h1 % 10000) as f32 / 10000.0 * vw;
        let sy = (h2 % 10000) as f32 / 10000.0 * star_zone_h;

        // Clignotement subtil basé sur le temps.
        let phase = (get_time() as f32 * 0.8 + i as f32 * 0.7).sin() * 0.5 + 0.5;
        let alpha = 0.35 + phase * 0.5;

        let c = Color::new(window_c.r, window_c.g, window_c.b, alpha);
        let size = if h1.is_multiple_of(7) { 2.0 } else { 1.0 };
        draw_rectangle(sx, sy, size, size, c);
    }
}

// ---------- Dessin : parallax layers ----------

pub fn draw_far_layer(palette: &Palette, cam_x: f32, ground_y: f32, seed: u32) {
    match palette.style {
        SceneStyle::Urban => draw_far_urban(palette, cam_x, ground_y, seed),
        SceneStyle::Mountain => draw_far_mountain(palette, cam_x, ground_y, seed),
    }
}

pub fn draw_mid_layer(palette: &Palette, cam_x: f32, ground_y: f32, seed: u32) {
    match palette.style {
        SceneStyle::Urban => draw_mid_urban(palette, cam_x, ground_y, seed),
        SceneStyle::Mountain => draw_mid_mountain(palette, cam_x, ground_y, seed),
    }
}

fn draw_far_urban(palette: &Palette, cam_x: f32, ground_y: f32, seed: u32) {
    let vw = screen_width();
    let layer_x = cam_x * FAR_PARALLAX;
    let first = (layer_x / FAR_CHUNK_W).floor() as i32;
    let last = ((layer_x + vw) / FAR_CHUNK_W).ceil() as i32;
    let color = palette.far_layer_c();

    for i in first..=last {
        let h = far_urban_height(seed, i);
        let x = i as f32 * FAR_CHUNK_W - layer_x;
        let top = ground_y - h;
        draw_rectangle(x, top, FAR_CHUNK_W - 4.0, h, color);
    }
}

fn draw_mid_urban(palette: &Palette, cam_x: f32, ground_y: f32, seed: u32) {
    let vw = screen_width();
    let layer_x = cam_x * MID_PARALLAX;
    let first = (layer_x / MID_CHUNK_W).floor() as i32;
    let last = ((layer_x + vw) / MID_CHUNK_W).ceil() as i32;
    let color = palette.mid_layer_c();
    let window_c = palette.window_c();
    let neon_1 = palette.neon_1_c();
    let neon_2 = palette.neon_2_c();

    for i in first..=last {
        let h = mid_urban_height(seed, i);
        let x = i as f32 * MID_CHUNK_W - layer_x;
        let top = ground_y - h;
        let w = MID_CHUNK_W - 8.0;

        draw_rectangle(x, top, w, h, color);
        draw_rectangle_lines(x, top, w, h, 1.0, Color::new(0.0, 0.0, 0.0, 0.4));

        // Fenêtres allumées.
        let win_cols = 3;
        let win_rows = (h / 22.0) as i32;
        for wx in 0..win_cols {
            for wy in 0..win_rows {
                let r = hash_u32(
                    seed.wrapping_add((wx as u32) * 17 + (wy as u32) * 31),
                    i as u32,
                );
                if !r.is_multiple_of(3) {
                    continue;
                }
                let sx = x + 6.0 + wx as f32 * 14.0;
                let sy = top + 10.0 + wy as f32 * 22.0;
                draw_rectangle(sx, sy, 6.0, 8.0, window_c);
            }
        }

        // Enseigne néon verticale sur certaines tours.
            let neon_seed = hash_u32(seed.wrapping_add(0x0E0F), i as u32);
        if neon_seed.is_multiple_of(3) {
            let neon_x = x + w - 8.0;
            let neon_top = top + 20.0;
            let neon_h = (h - 40.0).clamp(20.0, 80.0);
            let neon_c = if neon_seed.is_multiple_of(2) {
                neon_1
            } else {
                neon_2
            };
            // Rectangle lumineux + halo.
            draw_rectangle(neon_x - 1.0, neon_top - 1.0, 6.0, neon_h + 2.0, Color::new(neon_c.r, neon_c.g, neon_c.b, 0.25));
            draw_rectangle(neon_x, neon_top, 4.0, neon_h, neon_c);
        }
    }
}

fn draw_far_mountain(palette: &Palette, cam_x: f32, ground_y: f32, seed: u32) {
    let vw = screen_width();
    let layer_x = cam_x * FAR_PARALLAX;
    let step = 200.0;
    let first = (layer_x / step).floor() as i32;
    let last = ((layer_x + vw) / step).ceil() as i32;
    let color = palette.far_layer_c();

    for i in first..=last {
        let h = far_mountain_height(seed, i);
        let base_x = i as f32 * step - layer_x;
        let peak_x = base_x + step * 0.5;
        let top_y = ground_y - h;
        let p1 = Vec2::new(base_x - step * 0.4, ground_y);
        let p2 = Vec2::new(peak_x, top_y);
        let p3 = Vec2::new(base_x + step * 1.4, ground_y);
        draw_triangle(p1, p2, p3, color);
    }
}

fn draw_mid_mountain(palette: &Palette, cam_x: f32, ground_y: f32, seed: u32) {
    let vw = screen_width();
    let layer_x = cam_x * MID_PARALLAX;
    let step = 180.0;
    let first = (layer_x / step).floor() as i32;
    let last = ((layer_x + vw) / step).ceil() as i32;
    let color = palette.mid_layer_c();
    let accent = palette.accent_light_c();

    for i in first..=last {
        let h = mid_mountain_height(seed, i);
        let base_x = i as f32 * step - layer_x;
        let peak_x = base_x + step * 0.5;
        let top_y = ground_y - h;

        let p1 = Vec2::new(base_x - step * 0.3, ground_y);
        let p2 = Vec2::new(peak_x, top_y);
        let p3 = Vec2::new(base_x + step * 1.3, ground_y);
        draw_triangle(p1, p2, p3, color);

        // Pagode simple sur certaines collines.
        if hash_u32(seed.wrapping_add(0xABCD), i as u32).is_multiple_of(4) {
            let pw = 22.0;
            let ph = 34.0;
            let px = peak_x;
            let py = top_y - ph;
            draw_rectangle(px - pw * 0.5, py, pw, ph, accent);
            // Étage supérieur plus fin.
            draw_rectangle(px - pw * 0.35, py - 12.0, pw * 0.7, 12.0, accent);
            // Toits triangulaires.
            let t1 = Vec2::new(px - pw, py);
            let t2 = Vec2::new(px, py - 10.0);
            let t3 = Vec2::new(px + pw, py);
            draw_triangle(t1, t2, t3, accent);
            let t1b = Vec2::new(px - pw * 0.5, py - 12.0);
            let t2b = Vec2::new(px, py - 22.0);
            let t3b = Vec2::new(px + pw * 0.5, py - 12.0);
            draw_triangle(t1b, t2b, t3b, accent);
            // Pointe dorée en haut.
            draw_circle(px, py - 24.0, 2.0, accent);
        }
    }
}

// ---------- Dessin : sol ----------

pub fn draw_ground(palette: &Palette, cam_x: f32, ground_y: f32, vh: f32) {
    let vw = screen_width();
    let base = palette.ground_base_c();
    let accent = palette.ground_accent_c();

    draw_rectangle(0.0, ground_y, vw, vh - ground_y, base);

    match palette.style {
        SceneStyle::Urban => {
            // Néons au sol (bandes horizontales colorées façon Blade Runner).
            draw_urban_neons(palette, cam_x, ground_y, vw);

            // Ligne de trottoir accentuée.
            draw_line(0.0, ground_y, vw, ground_y, 3.0, accent);

            // Pavés verticaux.
            let tile_w = 60.0;
            let first = (cam_x / tile_w).floor() as i32;
            let last = ((cam_x + vw) / tile_w).ceil() as i32;
            let faint = Color::new(accent.r, accent.g, accent.b, 0.18);
            for i in first..=last {
                let x = i as f32 * tile_w - cam_x;
                draw_line(x, ground_y + 12.0, x, vh, 1.0, faint);
            }
        }
        SceneStyle::Mountain => {
            // Ligne de sol.
            draw_line(0.0, ground_y, vw, ground_y, 3.0, accent);

            // Touffes d'herbe.
            let tuft_w = 40.0;
            let first = (cam_x / tuft_w).floor() as i32;
            let last = ((cam_x + vw) / tuft_w).ceil() as i32;
            for i in first..=last {
                let x = i as f32 * tuft_w - cam_x;
                let r = hash_u32(0xC0FFEE, i as u32);
                if r.is_multiple_of(3) {
                    let h = 6.0 + (r % 5) as f32;
                    draw_line(x, ground_y + 6.0, x - 3.0, ground_y + 6.0 - h, 2.0, accent);
                    draw_line(x, ground_y + 6.0, x + 3.0, ground_y + 6.0 - h, 2.0, accent);
                }
            }

            // Petites pierres.
            for i in first..=last {
                let x = i as f32 * tuft_w - cam_x + 12.0;
                let r = hash_u32(0x5EED, i as u32);
                if r.is_multiple_of(5) {
                    let size = 2.0 + (r % 3) as f32;
                    draw_rectangle(x, ground_y + 20.0, size, size, accent);
                }
            }
        }
    }
}

fn draw_urban_neons(palette: &Palette, cam_x: f32, ground_y: f32, vw: f32) {
    let layer_x = cam_x * NEON_PARALLAX;
    let stripe_w = 140.0;
    let first = (layer_x / stripe_w).floor() as i32;
    let last = ((layer_x + vw) / stripe_w).ceil() as i32;
    let neon_1 = palette.neon_1_c();
    let neon_2 = palette.neon_2_c();

    for i in first..=last {
        let r = hash_u32(0x3E0F_1E50, i as u32);
        if !r.is_multiple_of(2) {
            continue;
        }
        let x = i as f32 * stripe_w - layer_x;
        let c = if r.is_multiple_of(4) { neon_1 } else { neon_2 };

        // Halo semi-transparent.
        let halo = Color::new(c.r, c.g, c.b, 0.18);
        draw_rectangle(x, ground_y + 2.0, stripe_w * 0.7, 4.0, halo);
        // Trait plein.
        draw_rectangle(x, ground_y + 3.0, stripe_w * 0.7, 2.0, c);
    }
}

// ---------- Dessin : brume ----------

pub fn draw_mist(palette: &Palette, ground_y: f32) {
    let m = palette.mist_c();
    if m.a <= 0.01 {
        return;
    }
    let vw = screen_width();
    // Bande de brume juste au-dessus du sol.
    let mist_h = 90.0;
    let mist_y = ground_y - mist_h + 20.0;
    draw_rectangle(0.0, mist_y, vw, mist_h, m);
}

// ---------- Dessin : tours ----------

pub fn draw_tower(
    player: bool,
    palette: &Palette,
    world_x: f32,
    cam_x: f32,
    ground_y: f32,
    shake: Vec2,
) {
    let sx = world_x - cam_x + shake.x;
    let sy_off = shake.y;
    let w = 56.0;
    let h = 140.0;
    let base = if player {
        palette.tower_player_c()
    } else {
        palette.tower_enemy_c()
    };
    let accent = palette.accent_light_c();
    let window_c = palette.window_c();
    let top = ground_y - h + sy_off;

    // Corps.
    draw_rectangle(sx - w * 0.5, top, w, h, base);
    draw_rectangle_lines(sx - w * 0.5, top, w, h, 2.0, accent);

    // Fenêtres verticales (3 colonnes).
    let win_cols = 3;
    let win_rows = 5;
    for wx in 0..win_cols {
        for wy in 0..win_rows {
            let fx = sx - w * 0.5 + 8.0 + wx as f32 * 14.0;
            let fy = top + 24.0 + wy as f32 * 22.0;
            draw_rectangle(fx, fy, 6.0, 10.0, window_c);
        }
    }

    // Créneaux en haut.
    let slots = 3;
    for i in 0..slots {
        let slot_x = sx - w * 0.5 + 6.0 + i as f32 * (w - 12.0) / (slots as f32 - 1.0) - 5.0;
        draw_rectangle(slot_x, top - 8.0, 10.0, 8.0, base);
        draw_rectangle_lines(slot_x, top - 8.0, 10.0, 8.0, 1.0, accent);
    }

    // Porte.
    let door_w = 16.0;
    let door_h = 22.0;
    let door_x = sx - door_w * 0.5;
    let door_y = top + h - door_h - 4.0;
    draw_rectangle(door_x, door_y, door_w, door_h, Color::new(0.02, 0.02, 0.04, 1.0));
    draw_rectangle_lines(door_x, door_y, door_w, door_h, 1.0, accent);

    // Antenne + voyant.
    let antenna_top = top - 26.0;
    draw_line(sx, top - 8.0, sx, antenna_top, 3.0, base);
    let light = if player {
        Color::new(0.35, 1.0, 0.45, 1.0)
    } else {
        Color::new(1.0, 0.35, 0.35, 1.0)
    };
    draw_circle(sx, antenna_top, 5.0, light);
    // Halo du voyant.
    draw_circle(sx, antenna_top, 9.0, Color::new(light.r, light.g, light.b, 0.3));
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_ron_loads_two_entries() {
        assert_eq!(all_palettes().len(), 2);
    }

    #[test]
    fn palette_ids_are_unique() {
        let mut ids: Vec<&str> = all_palettes().iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before);
    }

    #[test]
    fn palette_by_id_finds_known() {
        assert!(palette_by_id("shanghai").is_some());
        assert!(palette_by_id("guiyang").is_some());
        assert!(palette_by_id("nope").is_none());
    }

    #[test]
    fn all_palettes_have_full_alpha() {
        for p in all_palettes() {
            assert_eq!(p.sky_top.3, 1.0);
            assert_eq!(p.ground_base.3, 1.0);
            assert_eq!(p.tower_player.3, 1.0);
        }
    }

    #[test]
    fn shanghai_has_no_mist_guiyang_has_mist() {
        let sh = palette_by_id("shanghai").unwrap();
        let gy = palette_by_id("guiyang").unwrap();
        assert!(sh.mist_color.3 < 0.01);
        assert!(gy.mist_color.3 > 0.1);
    }

    #[test]
    fn shanghai_is_urban_and_guiyang_is_mountain() {
        assert_eq!(palette_by_id("shanghai").unwrap().style, SceneStyle::Urban);
        assert_eq!(palette_by_id("guiyang").unwrap().style, SceneStyle::Mountain);
    }

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_u32(42, 100), hash_u32(42, 100));
        assert_ne!(hash_u32(42, 100), hash_u32(43, 100));
        assert_ne!(hash_u32(42, 100), hash_u32(42, 101));
    }

    #[test]
    fn far_urban_height_in_range() {
        for chunk in -10..10 {
            let h = far_urban_height(1, chunk);
            assert!((80.0..300.0).contains(&h), "h = {h}");
        }
    }

    #[test]
    fn mid_urban_height_in_range() {
        for chunk in -10..10 {
            let h = mid_urban_height(1, chunk);
            assert!((120.0..280.0).contains(&h), "h = {h}");
        }
    }

    #[test]
    fn far_mountain_height_in_range() {
        for chunk in -10..10 {
            let h = far_mountain_height(1, chunk);
            assert!((200.0..380.0).contains(&h), "h = {h}");
        }
    }

    #[test]
    fn mid_mountain_height_in_range() {
        for chunk in -10..10 {
            let h = mid_mountain_height(1, chunk);
            assert!((100.0..220.0).contains(&h), "h = {h}");
        }
    }

    #[test]
    fn heights_vary_by_chunk() {
        let mut heights = Vec::new();
        for chunk in 0..20 {
            heights.push(far_urban_height(1, chunk));
        }
        heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
        heights.dedup_by(|a, b| (*a - *b).abs() < 0.1);
        assert!(heights.len() >= 5, "only {} unique heights", heights.len());
    }
}