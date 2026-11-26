// games/pagoda/src/scene.rs
//! Pagoda as a node tree: one root node, `Pagoda`, one-button input.
//!
//! It runs the menus (`App`) and the match (`systems`), plays the
//! sound bank's cues, saves the best score, and draws on an 800 × 600
//! canvas that `SceneLoop` letterboxes into the window (ADR-041). That
//! replaces the old macroquad render target, `Camera2D` and blit.

use crate::app::{App, AppState};
use crate::audio::SoundBank;
use crate::config::{GameContext, background_color};
use crate::fonts::{self, CjkFont};
use crate::persistence::{self, BestScore};
use crate::systems::{self, MatchState};
use crate::ui::MuteButton;
use crate::{art, backdrop};
use ember::prelude::*;
use ember_input::TouchPhase;

/// Extra camera zoom at full punch.
const CAMERA_PUNCH_ZOOM: f32 = 0.03;

/// Frame time cap, as before.
const MAX_FRAME_DT: f32 = 1.0 / 60.0;

pub struct Pagoda {
    name: String,
    pub ctx: GameContext,
    pub app: App,
    pub match_state: MatchState,
    best: BestScore,
    /// Loaded in `ready` (it needs the audio server).
    pub sounds: Option<SoundBank>,
    cjk: CjkFont,
    mute_button: MuteButton,
    /// Seconds since start, for animations (was macroquad's `get_time`).
    pub time: f32,
    /// This frame's screen-shake offset, taken once per frame.
    shake: Vec2,
    hovered: bool,
}

impl Pagoda {
    /// A game that loads and saves its best score through `best`.
    pub fn new(best: BestScore) -> Self {
        let ctx = GameContext::default();
        Self {
            name: "Pagoda".into(),
            app: App::with_high_score(best.load_or(0)),
            match_state: MatchState::new(&ctx),
            ctx,
            best,
            sounds: None,
            cjk: CjkFont::hermetic(),
            mute_button: MuteButton::default(),
            time: 0.0,
            shake: Vec2::ZERO,
            hovered: false,
        }
    }

    pub fn best_score(&self) -> &BestScore {
        &self.best
    }

    pub fn is_muted(&self) -> bool {
        self.sounds.as_ref().is_some_and(SoundBank::is_muted)
    }
}

impl Object for Pagoda {
    fn class_name() -> &'static str {
        "Pagoda"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Pagoda {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Pagoda {
    fn ready(&mut self, ctx: &mut NodeContext) {
        // Before any text is drawn.
        fonts::load_default_font(ctx.render());
        self.cjk = CjkFont::load(ctx.render());
        self.sounds = Some(SoundBank::load(ctx.audio()));
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        self.time += dt;
        let dt = dt.min(MAX_FRAME_DT);
        let mut input = ctx.input().clone();

        // Positions arrive in canvas coordinates; a point in the
        // letterbox bars lies outside the canvas, so it never hits.
        self.hovered = self.mute_button.contains(input.mouse_pos);
        let mut mute_clicked = self.hovered && input.mouse_left_pressed;

        // Touch arrives separately from the mouse. A fresh tap is the
        // game's one-button input, but a tap on the speaker must toggle
        // sound instead of dropping a block.
        for touch in &input.touch_points {
            if touch.phase == TouchPhase::Started {
                if self.mute_button.contains(touch.position) {
                    mute_clicked = true;
                } else {
                    input.mouse_left_pressed = true;
                }
            }
        }

        if mute_clicked {
            if let Some(sounds) = &mut self.sounds
                && sounds.toggle_mute()
            {
                ctx.audio().stop_all();
            }
            // Consume the click so it cannot start, drop or restart too.
            input.mouse_left_pressed = false;
            input.mouse_left_down = false;
        }

        let prev_state = self.app.state;
        self.app.update(&input);

        // A new game resets the match and skips this frame's logic, so
        // the starting press is not also a drop.
        if matches!(prev_state, AppState::MainMenu | AppState::GameOver)
            && self.app.state == AppState::Playing
        {
            self.match_state = MatchState::new(&self.ctx);
            if let Some(cue) = self.sounds.as_ref().and_then(SoundBank::start) {
                cue.play(ctx.audio());
            }
        } else {
            let events =
                systems::update(&mut self.match_state, &mut self.app, &self.ctx, dt, &input);
            if let Some(sounds) = &self.sounds {
                for cue in events.into_iter().flat_map(|e| sounds.on_event(e)) {
                    cue.play(ctx.audio());
                }
            }
        }

        // Persist a new best the moment a run ends. On WASM `save` fails
        // silently, leaving the record in memory for the session.
        if prev_state == AppState::Playing && self.app.state == AppState::GameOver {
            if self.app.record_high_score() {
                self.best.save(&self.app.high_score);
            }
            if self.app.new_best
                && let Some(cue) = self.sounds.as_ref().and_then(SoundBank::new_best)
            {
                cue.play(ctx.audio());
            }
        }

        self.shake = self.match_state.juice.shake_offset();
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let (w, h) = (self.ctx.screen_w, self.ctx.screen_h);
        let state = &self.match_state;
        let shake = self.shake;
        let r = ctx.render();

        // The perfect-drop punch zooms the whole canvas about its centre,
        // as the old `Camera2D` zoom did.
        let letterbox = r.camera();
        let punch = 1.0 + state.camera_punch * CAMERA_PUNCH_ZOOM;
        r.set_camera(letterbox.zoomed_about(Vec2::new(w / 2.0, h / 2.0), punch));

        // The canvas background (was `clear_background`, which would now
        // paint over the letterbox bars).
        let tower_height = state.tower.len() as u32;
        r.draw_rect(Rect::new(0.0, 0.0, w, h), background_color(tower_height));
        backdrop::draw(r, state.camera_y, tower_height, w, h, self.time);

        if matches!(self.app.state, AppState::Playing | AppState::GameOver) {
            // The tower. The newest block carries a short gold landing
            // outline.
            let last_index = state.tower.len().saturating_sub(1);
            for (index, block) in state.tower.iter().enumerate() {
                let glow = if index == last_index {
                    state.land_pulse
                } else {
                    0.0
                };
                art::draw_block(r, block, state.camera_y, shake, glow);
            }

            // The sliding block, while playing.
            if self.app.state == AppState::Playing
                && let Some(top_block) = state.tower.last()
            {
                let ctx = &self.ctx;
                let slide_y =
                    top_block.transform.position.y - ctx.block_height + state.camera_y + shake.y;
                let slide_center = state.sliding.x + state.sliding.width / 2.0;
                let top_center = top_block.transform.position.x + top_block.transform.scale.x / 2.0;
                let window = systems::perfect_window(state.sliding.speed, ctx.perfect_threshold);
                let aligned = (slide_center - top_center).abs() <= window;

                art::draw_sliding_block(
                    r,
                    &state.sliding,
                    slide_y,
                    ctx.block_height,
                    shake,
                    aligned,
                );
                art::draw_drop_guide(
                    r,
                    slide_center + shake.x,
                    top_center + shake.x,
                    top_block.transform.position.y + state.camera_y + shake.y,
                    ctx.block_height,
                    aligned,
                    self.time,
                );
            }

            state.juice.draw_world(r, state.camera_y, shake);
        }

        state.juice.draw_flash(r, w, h);
        self.app
            .draw(r, &self.ctx, state.combo, &self.cjk, self.time);
        self.mute_button.draw(r, self.is_muted(), self.hovered);

        r.set_camera(letterbox);
    }
}

/// The whole game: a single `Pagoda` root saving to `best`.
pub fn build_tree(best: BestScore) -> NodeTree {
    NodeTree::with_root(Pagoda::new(best))
}

/// Opens the window (resizable, letterboxed) and plays.
pub fn run() {
    let ctx = GameContext::default();
    let (w, h) = (ctx.screen_w, ctx.screen_h);
    let game = SceneLoop::new(
        build_tree(persistence::default()),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    )
    .with_virtual_resolution(w, h);
    run_window("PAGODA", w as i32, h as i32, game);
}
