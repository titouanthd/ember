//! Air Hockey as a node tree (Phase 7 migration).
//!
//! One root node, `AirHockeyGame`, owns the untouched `Game`, feeds it the
//! frame's `Input`, plays the `GameEvent` sounds through the engine's
//! `AudioServer` and draws table, puck, paddles, particles, HUD and
//! overlays through the `RenderingServer`. Keyboard only, so there is no
//! UI subtree. Font (DejaVu Sans Mono) and sounds load in `ready`.

use std::path::PathBuf;

use crate::audio::SoundBank;
use crate::components::Side;
use crate::{Game, GameContext, GameEvent, Phase, font};
use ember::prelude::*;

/// Window size (fixed, as before).
pub const WINDOW_W: f32 = 1280.0;
pub const WINDOW_H: f32 = 720.0;
/// Duration of the visual goal flash (must match `systems::GOAL_FLASH_TIME`).
const GOAL_FLASH_TIME: f32 = 0.2;
const PUCK_SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.15);
const PUCK_HIGHLIGHT: Color = Color::new(1.0, 1.0, 1.0, 0.7);
const DIM: Color = Color::new(0.0, 0.0, 0.0, 0.55);

pub struct AirHockeyGame {
    name: String,
    pub ctx: GameContext,
    pub game: Game,
    /// Loaded in `ready` (needs the `AudioServer`).
    pub sounds: Option<SoundBank>,
    /// Whether DejaVu Sans Mono loaded (set in `ready`).
    pub font_loaded: bool,
    /// Events of the last `process`, for tests.
    pub last_events: Vec<GameEvent>,
}

impl AirHockeyGame {
    pub fn new(ctx: GameContext, game: Game) -> Self {
        Self {
            name: "AirHockey".into(),
            ctx,
            game,
            sounds: None,
            font_loaded: false,
            last_events: Vec::new(),
        }
    }

    fn draw_frame(&self, r: &mut dyn RenderingServer) {
        let shake = self.game.shake_offset;
        self.draw_table(r, shake);
        self.draw_goals(r, shake);
        self.draw_particles(r, shake);
        self.draw_puck(r, shake);
        self.draw_paddles(r, shake);
        self.draw_hud(r, shake);
        self.draw_overlays(r, shake);
    }

    fn draw_table(&self, r: &mut dyn RenderingServer, shake: Vec2) {
        let ctx = &self.ctx;
        let a = &ctx.arena;
        let (ox, oy) = (shake.x, shake.y);
        r.draw_rect(
            Rect::new(
                a.wall + ox,
                a.wall + oy,
                a.w - 2.0 * a.wall,
                a.h - 2.0 * a.wall,
            ),
            ctx.colors.table_bg,
        );

        let step = 80.0;
        let mut x = a.wall + step;
        while x < a.w - a.wall - 1.0 {
            r.draw_line(
                Vec2::new(x + ox, a.wall + oy),
                Vec2::new(x + ox, a.h - a.wall + oy),
                1.0,
                ctx.colors.grid,
            );
            x += step;
        }
        let mut y = a.wall + step;
        while y < a.h - a.wall - 1.0 {
            r.draw_line(
                Vec2::new(a.wall + ox, y + oy),
                Vec2::new(a.w - a.wall + ox, y + oy),
                1.0,
                ctx.colors.grid,
            );
            y += step;
        }

        r.draw_line(
            Vec2::new(a.mid_x() + ox, a.wall + oy),
            Vec2::new(a.mid_x() + ox, a.h - a.wall + oy),
            2.0,
            ctx.colors.midline,
        );
        r.draw_circle_outline(
            Vec2::new(a.mid_x() + ox, a.mid_y() + oy),
            60.0,
            2.0,
            ctx.colors.midline,
        );

        let b = ctx.colors.table_border;
        for rect in [
            Rect::new(ox, oy, a.w, a.wall),
            Rect::new(ox, a.h - a.wall + oy, a.w, a.wall),
            Rect::new(ox, oy, a.wall, a.goal_top()),
            Rect::new(ox, a.goal_bottom() + oy, a.wall, a.h - a.goal_bottom()),
            Rect::new(a.w - a.wall + ox, oy, a.wall, a.goal_top()),
            Rect::new(
                a.w - a.wall + ox,
                a.goal_bottom() + oy,
                a.wall,
                a.h - a.goal_bottom(),
            ),
        ] {
            r.draw_rect(rect, b);
        }
    }

    fn draw_goals(&self, r: &mut dyn RenderingServer, shake: Vec2) {
        let ctx = &self.ctx;
        let a = &ctx.arena;
        let left = Rect::new(shake.x, a.goal_top() + shake.y, a.wall, a.goal_h);
        let right = Rect::new(
            a.w - a.wall + shake.x,
            a.goal_top() + shake.y,
            a.wall,
            a.goal_h,
        );
        r.draw_rect(left, ctx.colors.goal_p1);
        r.draw_rect(right, ctx.colors.goal_p2);
        for (flash, rect) in [
            (self.game.goal_flash[0], left),
            (self.game.goal_flash[1], right),
        ] {
            if flash > 0.0 {
                let mut c = ctx.colors.goal_flash;
                c.a *= (flash / GOAL_FLASH_TIME).clamp(0.0, 1.0);
                r.draw_rect(rect, c);
            }
        }
    }

    fn draw_particles(&self, r: &mut dyn RenderingServer, shake: Vec2) {
        for p in self.game.particles.iter() {
            let alpha = p.alive_fraction();
            let mut c = self.ctx.colors.puck;
            c.a = alpha * 0.9;
            r.draw_circle(p.pos + shake, 2.0 + alpha * 3.0, c);
        }
    }

    fn draw_puck(&self, r: &mut dyn RenderingServer, shake: Vec2) {
        let ctx = &self.ctx;
        let p = &self.game.puck;
        for (pos, t) in self.game.trail.iter() {
            let mut c = ctx.colors.puck_trail;
            c.a *= 1.0 - t;
            r.draw_circle(pos + shake, p.radius * (1.0 - t * 0.7), c);
        }
        let centre = p.pos + shake;
        r.draw_circle(centre + Vec2::new(2.0, 3.0), p.radius, PUCK_SHADOW);
        r.draw_circle(centre, p.radius, ctx.colors.puck);
        let highlight = centre - Vec2::splat(p.radius * 0.35);
        r.draw_circle(highlight, p.radius * 0.35, PUCK_HIGHLIGHT);
    }

    fn draw_paddles(&self, r: &mut dyn RenderingServer, shake: Vec2) {
        for pad in &self.game.paddles {
            let base = self.ctx.colors.for_side(pad.side);
            let centre = pad.pos + shake;
            let radius = pad.radius;
            let mut halo = base;
            halo.a = 0.18;
            r.draw_circle(centre, radius * 1.35, halo);
            r.draw_circle(centre, radius, base);
            let core = Color::new(
                (base.r + 1.0) * 0.5,
                (base.g + 1.0) * 0.5,
                (base.b + 1.0) * 0.5,
                0.85,
            );
            r.draw_circle(centre, radius * 0.45, core);
        }
    }

    fn draw_hud(&self, r: &mut dyn RenderingServer, shake: Vec2) {
        let ctx = &self.ctx;
        let game = &self.game;
        let cx = ctx.arena.mid_x() + shake.x;
        let top = 60.0 + shake.y;
        let score = format!("J1   {}  —  {}   J2", game.score.p1, game.score.p2);
        let played = game.rounds.p1 + game.rounds.p2;
        let best_of = ctx.tuning.rounds_to_win_match * 2 - 1;
        let rounds = format!("Manche {} / {}", played + 1, best_of);
        let (text, shadow) = (ctx.colors.text, ctx.colors.text_shadow);
        centered(r, &score, cx + 2.0, top + 2.0, 48.0, shadow);
        centered(r, &score, cx, top, 48.0, text);
        centered(r, &rounds, cx + 1.0, top + 39.0, 22.0, shadow);
        centered(r, &rounds, cx, top + 38.0, 22.0, text);
    }

    fn draw_overlays(&self, r: &mut dyn RenderingServer, shake: Vec2) {
        let ctx = &self.ctx;
        let game = &self.game;
        let cx = ctx.arena.mid_x() + shake.x;
        let cy = ctx.arena.mid_y() + shake.y;
        let text = ctx.colors.text;

        // Dim the playfield behind any overlay that needs readable text.
        let dim = matches!(
            game.phase,
            Phase::Menu
                | Phase::Countdown { .. }
                | Phase::GoalPause { .. }
                | Phase::RoundOver { .. }
                | Phase::MatchOver { .. }
        ) || (matches!(game.phase, Phase::Playing) && game.paused);
        if dim {
            r.draw_rect(Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H), DIM);
        }

        match game.phase {
            Phase::Menu => {
                centered(r, "AIR HOCKEY", cx, cy - 140.0, 72.0, text);
                centered(r, "ESPACE : Joueur vs Joueur", cx, cy - 20.0, 32.0, text);
                centered(r, "ENTRÉE : Joueur vs IA", cx, cy + 30.0, 32.0, text);
                let keys = "J1 : W A S D     J2 : Flèches";
                centered(r, keys, cx, cy + 100.0, 24.0, text);
                let meta = "P : pause     Échap : quitter";
                centered(r, meta, cx, cy + 140.0, 20.0, text);
            }
            Phase::Countdown { t } => {
                let (label, scale) = countdown_label(ctx.tuning.countdown, t);
                centered(r, &label, cx, cy - 40.0, 120.0 * scale, text);
            }
            Phase::GoalPause { scorer, .. } => {
                let msg = format!("BUT  J{} !", player(scorer));
                centered(r, &msg, cx + 2.0, cy - 38.0, 72.0, ctx.colors.text_shadow);
                centered(r, &msg, cx, cy - 40.0, 72.0, text);
            }
            Phase::RoundOver { winner, .. } => {
                let msg = format!("Manche à J{}", player(winner));
                centered(r, &msg, cx, cy - 40.0, 64.0, text);
            }
            Phase::MatchOver { winner } => {
                let msg = format!("J{} GAGNE !", player(winner));
                centered(r, &msg, cx, cy - 80.0, 72.0, text);
                centered(r, "ESPACE pour rejouer", cx, cy + 40.0, 32.0, text);
            }
            Phase::Playing => {
                if game.paused {
                    centered(r, "PAUSE", cx, cy - 40.0, 72.0, text);
                    centered(r, "P pour reprendre", cx, cy + 40.0, 24.0, text);
                }
            }
        }
    }
}

fn player(side: Side) -> u8 {
    if side == Side::Left { 1 } else { 2 }
}

/// Countdown label and its "punch" scale: 3 → 2 → 1, then GO! in the
/// last 0.5 s; each label starts at 1.4× and shrinks to 1.0× (unchanged
/// maths).
pub fn countdown_label(total: f32, t: f32) -> (String, f32) {
    const GO_WINDOW: f32 = 0.5;
    let num_duration = (total - GO_WINDOW).max(0.0);
    let step_dur = (num_duration / 3.0).max(1e-3);
    let elapsed = (total - t).max(0.0);
    let (label, frac) = if elapsed < num_duration {
        let k = (elapsed / step_dur).floor().min(2.0) as i32;
        let frac = (elapsed - k as f32 * step_dur) / step_dur;
        (format!("{}", 3 - k), frac.clamp(0.0, 1.0))
    } else {
        let frac = (elapsed - num_duration) / GO_WINDOW.max(1e-3);
        ("GO!".to_string(), frac.clamp(0.0, 1.0))
    };
    (label, 1.4 - 0.4 * frac)
}

/// Text centred on `x`; `y` is the baseline.
fn centered(r: &mut dyn RenderingServer, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let w = r.measure_text(text, size).width;
    r.draw_text(text, Vec2::new(x - w * 0.5, y), size, color);
}

impl Object for AirHockeyGame {
    fn class_name() -> &'static str {
        "AirHockeyGame"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for AirHockeyGame {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for AirHockeyGame {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(self.ctx.colors.table_bg);
        self.font_loaded = font::load_default_font(ctx.render());
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        self.sounds = Some(SoundBank::load(&manifest_dir, ctx.audio()));
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input().clone();
        if input.is_key_pressed(Key::Escape) {
            ctx.quit();
            return;
        }
        let dt = dt.min(self.ctx.tuning.dt_max);
        self.last_events = self.game.update(&input, &self.ctx, dt);
        if let Some(sounds) = &self.sounds {
            for ev in &self.last_events {
                if let Some((clip, volume)) = sounds.on_event(*ev) {
                    ctx.audio().play(clip, volume, 1.0);
                }
            }
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        self.draw_frame(ctx.render());
    }
}

/// The game's tree: a single `AirHockeyGame` root.
pub fn build_tree(ctx: GameContext, game: Game) -> NodeTree {
    NodeTree::with_root(AirHockeyGame::new(ctx, game))
}

/// Open the window and play.
pub fn run() {
    let ctx = GameContext::load();
    let game = Game::new(&ctx);
    let scene = SceneLoop::new(
        build_tree(ctx, game),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    );
    run_window("Air Hockey", WINDOW_W as i32, WINDOW_H as i32, scene);
}
