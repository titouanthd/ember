//! Visual disk: central star, concentric orbits, spiraling particles,
//! and one body per owned building. Purely cosmetic — no game logic
//! here. `Disk` lives in the outer loop, not in `Game`.

use macroquad::prelude::*;

use crate::components::Paradigm;
use crate::config as cfg;
use crate::economy::BUILDINGS;
use crate::systems::Game;

const MAX_PARTICLES: usize = 150;
const PARTICLE_SPAWN_RATE: f32 = 40.0;
const MIN_RADIUS: f32 = 25.0;
const MAX_RADIUS: f32 = 240.0;
const TAU: f32 = std::f32::consts::TAU;

pub struct Particle {
    pub radius: f32,
    pub angle: f32,
    pub radius_speed: f32,
    pub angle_speed: f32,
    pub size: f32,
    pub brightness: f32,
}

pub struct Disk {
    pub particles: Vec<Particle>,
    pub rotation: f32,
    pub time: f32,
    spawn_counter: u32,
}

impl Disk {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            rotation: 0.0,
            time: 0.0,
            spawn_counter: 0,
        }
    }

    /// Cheap deterministic noise. No `Rng` dependency (Rng API drift
    /// across sessions is a known trap; we only need "different enough").
    fn noise(counter: u32, salt: u32) -> f32 {
        let h = counter.wrapping_mul(2654435761).wrapping_add(salt.wrapping_mul(40503));
        ((h & 0xFFFF) as f32) / 65536.0
    }

    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        self.rotation += dt * 0.06;

        for p in &mut self.particles {
            p.radius -= p.radius_speed * dt;
            p.angle += p.angle_speed * dt;
            p.brightness = ((p.radius - MIN_RADIUS) / (MAX_RADIUS - MIN_RADIUS))
                .clamp(0.0, 1.0)
                * 0.65;
        }
        self.particles.retain(|p| p.radius > MIN_RADIUS);

        let mut n = (PARTICLE_SPAWN_RATE * dt) as u32;
        if Self::noise(self.spawn_counter, 0xBEEF) < PARTICLE_SPAWN_RATE * dt - n as f32 {
            n += 1;
        }
        n = n.min(20); // guard against huge dt (tab switch, breakpoint)

        for _ in 0..n {
            if self.particles.len() >= MAX_PARTICLES {
                break;
            }
            let c = self.spawn_counter;
            self.spawn_counter = self.spawn_counter.wrapping_add(1);
            self.particles.push(Particle {
                radius: MAX_RADIUS + Self::noise(c, 1) * 30.0,
                angle: Self::noise(c, 2) * TAU,
                radius_speed: 15.0 + Self::noise(c, 3) * 30.0,
                angle_speed: (Self::noise(c, 4) - 0.5) * 0.8,
                size: 0.8 + Self::noise(c, 5) * 1.2,
                brightness: 0.0,
            });
        }
    }

    /// Draw the whole disk around `center`.
    pub fn draw(&self, game: &Game, center: Vec2, max_radius: f32) {
        let accent = cfg::paradigm_color(game.paradigm);

        // Orbit rings
        for i in 0..6 {
            let r = max_radius * (0.30 + i as f32 * 0.14);
            let alpha = 0.06 + 0.03 * i as f32;
            draw_circle_lines(center.x, center.y, r, 1.0, accent.with_alpha(alpha));
        }

        // Star with layered glow
        let star_scale = 1.0 + game.paradigm as usize as f32 * 0.10;
        let pulse = 1.0 + (self.time * 2.0).sin() * 0.06;
        let star_r = 18.0 * star_scale * pulse;
        for i in (1..=5).rev() {
            let r = star_r * (1.0 + i as f32 * 0.45);
            let a = 0.10 * (1.0 - (i as f32 - 1.0) / 5.0);
            draw_circle(center.x, center.y, r, accent.with_alpha(a));
        }
        draw_circle(center.x, center.y, star_r, accent);
        draw_circle(center.x, center.y, star_r * 0.55, Color::new(1.0, 1.0, 1.0, 0.9));

        // Bodies: one dot per owned building, grouped per paradigm orbit.
        for (pidx, &paradigm) in Paradigm::ALL.iter().enumerate() {
            if paradigm == Paradigm::Dust {
                continue;
            }
            let owned: Vec<_> = BUILDINGS
                .iter()
                .filter(|d| d.paradigm == paradigm && game.building_count(d.id) > 0)
                .collect();
            if owned.is_empty() {
                continue;
            }
            let orbit_r = max_radius * (0.30 + pidx as f32 * 0.11);
            let n = owned.len();
            for (j, def) in owned.iter().enumerate() {
                let count = game.building_count(def.id) as usize;
                let arc_start = (j as f32 / n as f32) * TAU;
                let arc_size = TAU / n as f32;
                for i in 0..count {
                    let t = if count == 1 {
                        0.5
                    } else {
                        i as f32 / (count - 1) as f32
                    };
                    let angle = arc_start + arc_size * t + self.rotation;
                    let x = center.x + orbit_r * angle.cos();
                    let y = center.y + orbit_r * angle.sin();
                    let size = 2.0 + pidx as f32 * 0.5;
                    draw_circle(x, y, size + 1.5, accent.with_alpha(0.20));
                    draw_circle(x, y, size, accent);
                }
            }
        }

        // Particles last (on top of orbits/bodies, below star? — actually
        // particles should be behind the star to feel "consumed"). Redraw
        // star tip so it sits on top of the innermost particles.
        for p in &self.particles {
            let x = center.x + p.radius * p.angle.cos();
            let y = center.y + p.radius * p.angle.sin();
            draw_circle(x, y, p.size, accent.with_alpha(p.brightness));
        }
        draw_circle(center.x, center.y, star_r * 0.55, Color::new(1.0, 1.0, 1.0, 0.9));
    }
}

impl Default for Disk {
    fn default() -> Self {
        Self::new()
    }
}

/// Screen-space center of the disk, accounting for the right panel.
pub fn center() -> Vec2 {
    let game_w = screen_width() - cfg::RIGHT_PANEL_W;
    vec2(game_w * 0.50, screen_height() * 0.55)
}

/// Screen-space radius of the disk's outer edge.
pub fn radius() -> f32 {
    let game_w = screen_width() - cfg::RIGHT_PANEL_W;
    (game_w.min(screen_height()) * 0.30).min(MAX_RADIUS)
}