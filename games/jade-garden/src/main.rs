//! jade-garden — entry point.

use glam::Vec2;
use macroquad::prelude::*;

use jade_garden::audio::{default_playlist, MusicPlayer};
use jade_garden::components::{GRID_H, GRID_W};
use jade_garden::config::GameContext;
use jade_garden::fonts::PoemFont;
use jade_garden::hud;
use jade_garden::juice;
use jade_garden::level;
use jade_garden::menu::{Menu, MenuAction};
use jade_garden::poem;
use jade_garden::progress;
use jade_garden::scroll_painting::{self, DRIFT_SPEED};
use jade_garden::scoring::compute_stars;
use jade_garden::systems::{Game, Phase};
use jade_garden::tile_render::{
    draw_selection_corners, draw_tile, draw_tile_glow, jade_shape_params,
    TILE_GAP, TILE_SIZE, TILE_STRIDE,
};
use jade_garden::Easing;

const EPILOGUE_FADE_IN: f32 = 1.2;
const EPILOGUE_CHAR_DELAY: f32 = 0.22;
const EPILOGUE_LINE_PAUSE: f32 = 0.55;
const EPILOGUE_CHAR_FADE: f32 = 0.4;
const EPILOGUE_PROSE_DELAY: f32 = 1.8;

const HALLOWEEN_FLASH_WHITE_END: f32 = 0.15;
const HALLOWEEN_FLASH_RED_END: f32 = 0.35;
const HALLOWEEN_ZOOM_DURATION: f32 = 5.0;
const HALLOWEEN_ZOOM_END: f32 =
    HALLOWEEN_FLASH_RED_END + HALLOWEEN_ZOOM_DURATION;
const HALLOWEEN_FADE_END: f32 = HALLOWEEN_ZOOM_END + 0.4;
const HALLOWEEN_MESSAGE_START: f32 = HALLOWEEN_FADE_END;

const HORIZON_FRAC: f32 = 0.78;
const INTRO_MIN_DURATION: f32 = 0.8;
const CHAR_REVEAL_DURATION: f32 = 0.35;
const REVEAL_TAIL_PAUSE: f32 = 0.9;

fn window_conf() -> Conf {
    Conf {
        window_title: "jade-garden".to_owned(),
        window_width: 1400,
        window_height: 900,
        window_resizable: true,
        icon: load_window_icon(),
        ..Default::default()
    }
}

fn load_window_icon() -> Option<macroquad::miniquad::conf::Icon> {
    let bytes = include_bytes!("../assets/icon-256.png");
    let img = image::load_from_memory_with_format(
        bytes,
        image::ImageFormat::Png,
    )
    .ok()?;

    let small = image::imageops::resize(
        &img, 16, 16, image::imageops::FilterType::Lanczos3,
    );
    let medium = image::imageops::resize(
        &img, 32, 32, image::imageops::FilterType::Lanczos3,
    );
    let big = image::imageops::resize(
        &img, 64, 64, image::imageops::FilterType::Lanczos3,
    );

    let mut icon = macroquad::miniquad::conf::Icon {
        small: [0; 16 * 16 * 4],
        medium: [0; 32 * 32 * 4],
        big: [0; 64 * 64 * 4],
    };
    icon.small.copy_from_slice(small.as_raw());
    icon.medium.copy_from_slice(medium.as_raw());
    icon.big.copy_from_slice(big.as_raw());
    Some(icon)
}

#[derive(Clone, Copy)]
enum AppScreen {
    Menu,
    Intro(usize),
    Game(usize),
    Epilogue,
    Halloween,
}

struct WinReveal {
    revealed_from: u8,
    revealed_to: u8,
    elapsed: f32,
    run_time: f32,
    new_best: bool,
}

struct AppState {
    screen: AppScreen,
    menu: Menu,
    game: Option<Game>,
    drift: f32,
    intro_elapsed: f32,
    last_chapter_shown: Option<u32>,
    win_reveal: Option<WinReveal>,
    poem_font: PoemFont,
    audio: MusicPlayer,
    help_open: bool,
    help_scroll: f32,
    epilogue_elapsed: f32,
    epilogue_from_debug: bool,
    halloween_elapsed: f32,
    halloween_from_debug: bool,
    halloween_entered_final: bool,
    halloween_final_elapsed: f32,
    halloween_scream_played: bool,
}

impl AppState {
    fn new(poem_font: PoemFont, audio: MusicPlayer) -> Self {
        let chapters = level::load_chapters().unwrap_or_default();
        let levels = level::load_levels().unwrap_or_default();
        let progress = progress::load_progress();
        Self {
            screen: AppScreen::Menu,
            menu: Menu::new(chapters, levels, progress),
            game: None,
            drift: 0.0,
            intro_elapsed: 0.0,
            last_chapter_shown: None,
            win_reveal: None,
            poem_font,
            audio,
            help_open: false,
            help_scroll: 0.0,
            epilogue_elapsed: 0.0,
            epilogue_from_debug: false,
            halloween_elapsed: 0.0,
            halloween_from_debug: false,
            halloween_entered_final: false,
            halloween_final_elapsed: 0.0,
            halloween_scream_played: false,
        }
    }

    fn start_level(&mut self, idx: usize) {
        let chapter = self.menu.levels[idx].chapter;
        if self.last_chapter_shown != Some(chapter) {
            self.screen = AppScreen::Intro(idx);
            self.intro_elapsed = 0.0;
            self.last_chapter_shown = Some(chapter);
            self.win_reveal = None;
        } else {
            self.begin_level(idx);
        }
    }

    fn begin_level(&mut self, idx: usize) {
        let level = self.menu.levels[idx].clone();
        let already_completed =
            self.menu.progress.level_completed(&level.id);
        let stop_on_objective = !already_completed;
        self.game = Some(Game::with_level_and_progress(level, stop_on_objective));
        self.screen = AppScreen::Game(idx);
        self.win_reveal = None;
    }

    fn back_to_menu(&mut self) {
        self.game = None;
        self.screen = AppScreen::Menu;
        self.win_reveal = None;
    }

    /// Returns the next reveal screen given what's been seen **and
    /// which level was just completed**. Reveals only trigger after
    /// the final level of the campaign.
    fn next_reveal(&self, completed_idx: usize) -> AppScreen {
        let is_last_level = completed_idx + 1 >= self.menu.levels.len();
        match next_reveal_step(
            is_last_level,
            self.menu.progress.epilogue_seen,
            self.menu.progress.halloween_seen,
            is_spooky_season(),
        ) {
            RevealStep::Menu => AppScreen::Menu,
            RevealStep::Epilogue => AppScreen::Epilogue,
            RevealStep::Halloween => AppScreen::Halloween,
        }
    }

    fn start_epilogue(&mut self, from_debug: bool) {
        self.epilogue_elapsed = 0.0;
        self.epilogue_from_debug = from_debug;
        self.screen = AppScreen::Epilogue;
        if !from_debug {
            self.menu.progress.epilogue_seen = true;
            let _ = progress::save_progress(&self.menu.progress);
        }
    }

    fn start_halloween(&mut self, from_debug: bool) {
        self.halloween_elapsed = 0.0;
        self.halloween_from_debug = from_debug;
        self.halloween_entered_final = false;
        self.halloween_final_elapsed = 0.0;
        self.halloween_scream_played = false;
        self.screen = AppScreen::Halloween;
        self.audio.stop();
        jade_garden::audio::play_scare();
        if !from_debug {
            self.menu.progress.halloween_seen = true;
            let _ = progress::save_progress(&self.menu.progress);
        }
    }

    fn end_halloween(&mut self) {
        self.screen = AppScreen::Menu;
        self.halloween_from_debug = false;
        self.audio.start();
    }

    /// Advances to the next reveal screen after the final level.
    fn go_to_next_reveal(&mut self, completed_idx: usize) {
        match self.next_reveal(completed_idx) {
            AppScreen::Epilogue => self.start_epilogue(false),
            AppScreen::Halloween => self.start_halloween(false),
            _ => self.back_to_menu(),
        }
    }

    /// Called after the epilogue finishes. We already know the player
    /// completed the last level, so we don't need a level index —
    /// just check whether the Halloween teaser should play next.
    fn advance_past_epilogue(&mut self) {
        if !self.menu.progress.halloween_seen && is_spooky_season() {
            self.start_halloween(false);
        } else {
            self.back_to_menu();
        }
    }
}

/// True if we're in the Halloween window: **September 1 through
/// November 30**, every year. The date is approximated from the Unix
/// epoch (±1-2 days of drift), which is fine for a seasonal trigger.
fn is_spooky_season() -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    is_spooky_month(month_from_unix(secs))
}

/// Pure predicate: is the given 1-indexed month inside the window?
fn is_spooky_month(month: u64) -> bool {
    matches!(month, 9..=11)
}

/// Exact calendar month (1..=12) for a Unix timestamp, in UTC.
///
/// Handles the Gregorian leap-year rules (÷4, except ÷100, except
/// ÷400). Uses a 400-year cycle jump to keep the loop bounded even
/// for far-future timestamps.
fn month_from_unix(secs: u64) -> u64 {
    const SECS_PER_DAY: u64 = 86_400;
    const DAYS_PER_400Y: i64 = 146_097; // 400 * 365 + 97 leap days

    let mut days = (secs / SECS_PER_DAY) as i64;
    let mut year = 1970i64;

    // Skip whole 400-year cycles in one go.
    let cycles = days.div_euclid(DAYS_PER_400Y);
    days = days.rem_euclid(DAYS_PER_400Y);
    year += cycles * 400;

    // Skip the remaining years one at a time (at most 400 iterations).
    loop {
        let year_len = if is_leap_year(year) { 366 } else { 365 };
        if days < year_len {
            break;
        }
        days -= year_len;
        year += 1;
    }

    // `days` is now day-of-year, 0-indexed.
    let month_lengths: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for len in month_lengths {
        if days < len {
            break;
        }
        days -= len;
        month += 1;
    }
    month
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Which post-campaign screen to show next, given progression state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RevealStep {
    Menu,
    Epilogue,
    Halloween,
}

/// Pure decision function — extracted so it can be unit-tested
/// without constructing an `AppState`.
fn next_reveal_step(
    is_last_level: bool,
    epilogue_seen: bool,
    halloween_seen: bool,
    in_spooky_season: bool,
) -> RevealStep {
    if !is_last_level {
        return RevealStep::Menu;
    }
    if !epilogue_seen {
        return RevealStep::Epilogue;
    }
    if !halloween_seen && in_spooky_season {
        return RevealStep::Halloween;
    }
    RevealStep::Menu
}

#[macroquad::main(window_conf)]
async fn main() {
    jade_garden::fonts::load_default_font().await;
    let poem_font = PoemFont::load().await;

    let mut audio = MusicPlayer::load(default_playlist());
    audio.start();

    let mut ctx = GameContext::from_env();
    ctx.viewport_w = screen_width();
    ctx.viewport_h = screen_height();

    let mut state = AppState::new(poem_font, audio);

    loop {
        let dt = get_frame_time().min(0.05);

        ctx.viewport_w = screen_width();
        ctx.viewport_h = screen_height();

        let mouse = Vec2::new(mouse_position().0, mouse_position().1);
        let click = is_mouse_button_pressed(MouseButton::Left);

        state.drift += dt;
        state.audio.tick(dt);

        // Help toggle — works in Menu, Intro, and Game.
        // Disabled during cinematics (Epilogue, Halloween).
        let help_allowed = matches!(
            state.screen,
            AppScreen::Menu | AppScreen::Intro(_) | AppScreen::Game(_)
        );

        if help_allowed {
            if is_key_pressed(KeyCode::F1) || is_key_pressed(KeyCode::Tab) {
                state.help_open = !state.help_open;
                if state.help_open {
                    state.help_scroll = 0.0; // reset on open
                }
            } else if state.help_open && is_key_pressed(KeyCode::Escape) {
                state.help_open = false;
            }
        } else {
            state.help_open = false;
        }

        if !state.help_open {
            match state.screen {
                AppScreen::Menu => {
                    if let Some(action) = state.menu.tick(dt, mouse, click) {
                        match action {
                            MenuAction::StartLevel(idx) => state.start_level(idx),
                            MenuAction::Quit => break,
                            MenuAction::DebugEpilogue => state.start_epilogue(true),
                            MenuAction::DebugHalloween => state.start_halloween(true),
                        }
                    }
                    if is_key_pressed(KeyCode::Escape) {
                        break;
                    }
                }
                AppScreen::Intro(idx) => {
                    state.intro_elapsed += dt;
                    let ready = state.intro_elapsed >= INTRO_MIN_DURATION;
                    let advance = click
                        || is_key_pressed(KeyCode::Enter)
                        || is_key_pressed(KeyCode::Space);

                    if is_key_pressed(KeyCode::Escape) {
                        state.back_to_menu();
                    } else if ready && advance {
                        state.begin_level(idx);
                    }
                }
                AppScreen::Epilogue => {
                    state.epilogue_elapsed += dt;
                    let advance = is_key_pressed(KeyCode::Enter)
                        || is_key_pressed(KeyCode::Escape)
                        || is_key_pressed(KeyCode::Space);
                    if advance {
                        if state.epilogue_from_debug {
                            state.screen = AppScreen::Menu;
                            state.epilogue_from_debug = false;
                        } else {
                            state.advance_past_epilogue();
                        }
                    }
                }
                AppScreen::Halloween => {
                    state.halloween_elapsed += dt;
                    if state.halloween_entered_final {
                        // ─── Final phase: silence, then jumpscare ───
                        state.halloween_final_elapsed += dt;

                        if !state.halloween_scream_played
                            && state.halloween_final_elapsed >= 0.7
                        {
                            jade_garden::audio::play_scream();
                            state.halloween_scream_played = true;
                        }

                        if state.halloween_final_elapsed >= 1.9 {
                            state.end_halloween();
                        }
                    } else {
                        // ─── Message phase ───
                        let escape = is_key_pressed(KeyCode::Escape);
                        let advance =
                            state.halloween_elapsed >= HALLOWEEN_MESSAGE_START
                                && (is_key_pressed(KeyCode::Enter)
                                    || is_key_pressed(KeyCode::Space));

                        if escape {
                            state.end_halloween();
                        } else if advance {
                            state.halloween_entered_final = true;
                            state.halloween_final_elapsed = 0.0;
                        }
                    }
                }
                AppScreen::Game(idx) => {
                    let phase = {
                        let game = state.game.as_mut().expect("Game should exist");
                        game.tick(&ctx, dt, mouse, click);
                        game.phase
                    };

                    match phase {
                        Phase::Playing => {
                            if is_key_pressed(KeyCode::Escape) {
                                state.back_to_menu();
                            }
                            if is_key_pressed(KeyCode::H)
                                && let Some(g) = state.game.as_mut()
                            {
                                g.request_hint();
                            }
                        }
                        Phase::Won => handle_won(&mut state, idx, dt),
                        Phase::Lost => {
                            if is_key_pressed(KeyCode::Enter)
                                || is_key_pressed(KeyCode::Escape)
                            {
                                state.back_to_menu();
                            } else if is_key_pressed(KeyCode::R) {
                                state.start_level(idx);
                            }
                        }
                    }
                }
            }
        }

        // ─── Rendering ───
        clear_background(ctx.colors.paper_bottom);

        match state.screen {
            AppScreen::Menu => state.menu.draw(&ctx, &state.poem_font),
            AppScreen::Intro(idx) => draw_intro(&ctx, &state, idx),
            AppScreen::Game(_) => {
                let chapter = state.game.as_ref().map(|g| g.level.chapter).unwrap_or(0);
                draw_game_background(&state, chapter);

                let shake = state
                    .game
                    .as_mut()
                    .map(|g| g.juice.screen_shake.offset())
                    .unwrap_or(Vec2::ZERO);

                if let Some(game) = state.game.as_ref() {
                    draw_game(&ctx, game, shake);

                    if game.phase == Phase::Won {
                        draw_won_overlay(&ctx, &state, game);
                    }
                }
            }
            AppScreen::Epilogue => draw_epilogue(&ctx, &state),
            AppScreen::Halloween => draw_halloween(&state),
        }

        // Help overlay, drawn on top of everything.
        if state.help_open {
            jade_garden::help::draw(&ctx, &state.poem_font, &mut state.help_scroll);
        }

        next_frame().await;
    }
}

fn handle_won(state: &mut AppState, idx: usize, dt: f32) {
    if state.win_reveal.is_none() {
        // Extract what we need from the game before touching `state`
        // mutably — avoids holding an immutable borrow across the
        // progress mutations below.
        let game_data = state.game.as_ref().map(|g| {
            let times = progress::RunTimes {
                to_1_star: g.crossed_1_star,
                to_2_star: g.crossed_2_star,
                to_3_star: g.crossed_3_star,
                peak_time: g.peak_score_time,
                peak_score: g.peak_score,
                total: g.elapsed,
            };
            (g.score, g.elapsed, times)
        });

        let Some((score, run_time, times)) = game_data else {
            // Invariant broken: we're in the Won phase with no active
            // game. Bail to the menu instead of panicking.
            state.back_to_menu();
            return;
        };

        let lvl = state.menu.levels[idx].clone();
        let before = state.menu.progress.poem_fragments;

        let prev_best = state.menu.progress.level_best_score(&lvl.id);
        let new_best = score > prev_best;
        let stars = compute_stars(score, lvl.star_target).max(1);

        state
            .menu
            .progress
            .record_win_with_times(&lvl.id, score, stars, lvl.poem_reveal, times);
        let _ = progress::save_progress(&state.menu.progress);

        let after = state.menu.progress.poem_fragments;
        state.win_reveal = Some(WinReveal {
            revealed_from: before,
            revealed_to: after,
            elapsed: 0.0,
            run_time,
            new_best,
        });
    }

    let (from, to, elapsed) = {
        let reveal = state.win_reveal.as_mut().unwrap();
        reveal.elapsed += dt;
        (reveal.revealed_from, reveal.revealed_to, reveal.elapsed)
    };

    let new_count = (to - from) as f32;
    let reveal_done_at = new_count * CHAR_REVEAL_DURATION + REVEAL_TAIL_PAUSE;
    let ready = elapsed >= reveal_done_at;

    if ready {
        if is_key_pressed(KeyCode::Enter) {
            state.go_to_next_reveal(idx);
        } else if is_key_pressed(KeyCode::R) {
            state.start_level(idx);
        } else if is_key_pressed(KeyCode::Escape) {
            state.back_to_menu();
        }
    }
}

// ─── Epilogue ──────────────────────────────────────────────────────

fn draw_epilogue(_ctx: &GameContext, state: &AppState) {
    let vw = screen_width();
    let vh = screen_height();
    let t_abs = state.epilogue_elapsed;

    let fade = (t_abs / EPILOGUE_FADE_IN).clamp(0.0, 1.0);
    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.02, 0.015, 0.01, 1.0));

    if fade < 0.5 {
        return;
    }

    let t = t_abs - EPILOGUE_FADE_IN;
    if t < 0.0 {
        return;
    }

    let poem_light = Color::new(0.96, 0.92, 0.82, 1.0);
    let poem_gold = Color::new(0.94, 0.82, 0.50, 1.0);
    let dim = Color::new(0.72, 0.68, 0.58, 1.0);
    let dim_more = Color::new(0.55, 0.50, 0.42, 1.0);

    let char_size = 56.0;
    let line_spacing = 82.0;
    let poem_x = vw * 0.5 - 5.0 * char_size * 0.5;
    let poem_y = vh * 0.18;

    let last_char_time =
        19.0 * EPILOGUE_CHAR_DELAY + 3.0 * EPILOGUE_LINE_PAUSE;
    let all_revealed_at = last_char_time + EPILOGUE_CHAR_FADE;

    let pulse_start = all_revealed_at + 0.6;
    let pulsing = t > pulse_start;

    for i in 0..20u8 {
        let line = (i / 5) as usize;
        let col = (i % 5) as usize;

        let char_start =
            i as f32 * EPILOGUE_CHAR_DELAY + line as f32 * EPILOGUE_LINE_PAUSE;
        let local = ((t - char_start) / EPILOGUE_CHAR_FADE).clamp(0.0, 1.0);
        let alpha = Easing::EaseOut.apply(local);
        if alpha <= 0.01 {
            continue;
        }

        let x = poem_x + col as f32 * char_size;
        let y = poem_y + line as f32 * line_spacing;

        let pulse = if pulsing {
            let phase = i as f32 * 0.18;
            let s = (t * 1.4 + phase).sin() * 0.5 + 0.5;
            0.72 + 0.28 * s
        } else {
            1.0
        };

        let color = if pulsing {
            Color::new(poem_gold.r, poem_gold.g, poem_gold.b, alpha * pulse)
        } else {
            Color::new(poem_light.r, poem_light.g, poem_light.b, alpha)
        };

        if let Some(c) = poem::char_at(i) {
            state.poem_font.draw(
                &c.to_string(),
                x + char_size * 0.10,
                y + char_size * 0.90,
                char_size * 0.88,
                color,
            );
        }

        let pin_start = char_start + 0.30;
        let pin_local = ((t - pin_start) / 0.5).clamp(0.0, 1.0);
        let pin_alpha = Easing::EaseOut.apply(pin_local);
        if pin_alpha > 0.01
            && let Some(p) = poem::pinyin_at(i)
        {
            let d = measure_text(p, None, 12, 1.0);
            draw_text(
                p,
                x + (char_size - d.width) * 0.5,
                y + char_size + 8.0,
                12.0,
                Color::new(0.85, 0.78, 0.60, pin_alpha * 0.90),
            );
        }
    }

    let prose_start = all_revealed_at + EPILOGUE_PROSE_DELAY;
    let prose_alpha = Easing::EaseOut
        .apply(((t - prose_start) / 0.8).clamp(0.0, 1.0));

    if prose_alpha > 0.01 {
        let footer_y = poem_y + 4.0 * line_spacing + 48.0;

        state.poem_font.draw_centered(
            poem::POEM_TITLE,
            vw * 0.5,
            footer_y,
            16.0,
            Color::new(poem_light.r, poem_light.g, poem_light.b, prose_alpha),
        );
        state.poem_font.draw_centered(
            poem::POEM_AUTHOR,
            vw * 0.5,
            footer_y + 24.0,
            13.0,
            Color::new(dim.r, dim.g, dim.b, prose_alpha),
        );

        let prose_lines = [
            "Xiao Lin sits down.",
            "He does not know if the garden is alive,",
            "or if he is the one who woke up.",
            "It does not matter.",
        ];
        let prose_y = footer_y + 64.0;
        for (i, line) in prose_lines.iter().enumerate() {
            let line_start = prose_start + i as f32 * 0.45;
            let line_local = ((t - line_start) / 0.6).clamp(0.0, 1.0);
            let a = Easing::EaseOut.apply(line_local);
            if a <= 0.01 {
                continue;
            }
            let d = measure_text(line, None, 16, 1.0);
            draw_text(
                line,
                vw * 0.5 - d.width * 0.5,
                prose_y + i as f32 * 26.0,
                16.0,
                Color::new(0.94, 0.90, 0.80, a * prose_alpha),
            );
        }
    }

    let prompt_start = prose_start + 4.0 * 0.45 + 1.8;
    let prompt_alpha = Easing::EaseOut
        .apply(((t - prompt_start) / 0.8).clamp(0.0, 1.0));

    if prompt_alpha > 0.01 {
        let hint = "[Enter] return to menu";
        let d = measure_text(hint, None, 14, 1.0);
        let pulse = 0.70 + 0.30 * ((t * 2.2).sin() * 0.5 + 0.5);
        draw_text(
            hint,
            vw * 0.5 - d.width * 0.5,
            vh - 40.0,
            14.0,
            Color::new(dim_more.r, dim_more.g, dim_more.b, prompt_alpha * pulse),
        );
    }

    if state.epilogue_from_debug {
        let txt = "DEBUG VIEW — progress not affected";
        let d = measure_text(txt, None, 12, 1.0);
        draw_text(
            txt,
            vw * 0.5 - d.width * 0.5,
            30.0,
            12.0,
            Color::new(0.85, 0.35, 0.25, 0.75),
        );
    }
}

// ─── Halloween teaser ──────────────────────────────────────────────

fn draw_halloween(state: &AppState) {
    let vw = screen_width();
    let vh = screen_height();
    let t = state.halloween_elapsed;

        // ─── Final jumpscare phase ───
    if state.halloween_entered_final {
        let ft = state.halloween_final_elapsed;

        if ft < 0.7 {
            // Silence: black screen.
            draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.0, 0.0, 0.0, 1.0));
        } else {
            // Alien fills the screen, zoomed way past comfort.
            let ft_scare = ft - 0.7;
            // Small shake for the first 0.15 s.
            let shake = if ft_scare < 0.15 {
                ((0.15 - ft_scare) / 0.15) * 12.0
            } else {
                0.0
            };
            let sx = (rand_shake_seed(ft_scare) - 0.5) * shake;
            let sy = (rand_shake_seed(ft_scare + 7.3) - 0.5) * shake;

            draw_alien(
                vw * 0.5 + sx,
                vh * 0.5 + sy,
                7.0,
                1.0,
                1.0,
                vw,
                vh,
            );
        }
        return;
    }

    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.0, 0.0, 0.0, 1.0));

    // Flash: white → dark red.
    if t < HALLOWEEN_FLASH_WHITE_END {
        let alpha = 1.0 - (t / HALLOWEEN_FLASH_WHITE_END);
        draw_rectangle(0.0, 0.0, vw, vh, Color::new(1.0, 1.0, 1.0, alpha));
        return;
    }
    if t < HALLOWEEN_FLASH_RED_END {
        let tt = (t - HALLOWEEN_FLASH_WHITE_END)
            / (HALLOWEEN_FLASH_RED_END - HALLOWEEN_FLASH_WHITE_END);
        let alpha = 1.0 - tt;
        draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.55, 0.05, 0.02, alpha));
        return;
    }

    // Zoom phase.
    let zoom_t =
        ((t - HALLOWEEN_FLASH_RED_END) / HALLOWEEN_ZOOM_DURATION).clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - zoom_t).powi(2);
    let zoom = 0.15 + ease * 3.35;

    let alien_alpha = if t < HALLOWEEN_ZOOM_END {
        ((t - HALLOWEEN_FLASH_RED_END) / 0.4).clamp(0.0, 1.0)
    } else {
        1.0 - ((t - HALLOWEEN_ZOOM_END) / 0.4).clamp(0.0, 1.0)
    };

    if alien_alpha > 0.01 {
        draw_alien(vw * 0.5, vh * 0.5, zoom, alien_alpha, zoom_t, vw, vh);
    }

    // Message phase.
    if t > HALLOWEEN_MESSAGE_START {
        draw_halloween_message(vw, vh, t - HALLOWEEN_MESSAGE_START);
    }

    if state.halloween_from_debug {
        let txt = "DEBUG VIEW — progress not affected";
        let d = measure_text(txt, None, 12, 1.0);
        draw_text(
            txt,
            vw * 0.5 - d.width * 0.5,
            26.0,
            12.0,
            Color::new(0.85, 0.35, 0.25, 0.75),
        );
    }
}

/// Draws the whole alien — head, eyes, neck, everything.
fn draw_alien(cx: f32, cy: f32, zoom: f32, alpha: f32, zoom_t: f32, vw: f32, vh: f32) {
    let skin_deep = Color::new(0.045, 0.055, 0.048, alpha);
    let skin_mid = Color::new(0.115, 0.135, 0.115, alpha);
    let skin_light = Color::new(0.20, 0.22, 0.19, alpha * 0.75);
    let vein = Color::new(0.19, 0.20, 0.17, alpha * 0.55);
    let eye_void = Color::new(0.005, 0.005, 0.005, alpha);

    let head_w = 380.0 * zoom;
    let head_h = 520.0 * zoom;

    // Mouse-tracking offset — the eyes follow the cursor.
    let mouse = mouse_position();
    let dx = ((mouse.0 - vw * 0.5) / (vw * 0.5)).clamp(-1.0, 1.0);
    let dy = ((mouse.1 - vh * 0.5) / (vh * 0.5)).clamp(-1.0, 1.0);
    let look_x = dx * head_w * 0.06;
    let look_y = dy * head_h * 0.045;

    // Neck.
    let neck_top_y = cy + head_h * 0.40;
    let neck_bottom_y = cy + head_h * 1.15;
    let neck_top_w = head_w * 0.30;
    let neck_bottom_w = head_w * 0.55;
    draw_tapered_column(
        cx,
        neck_top_y,
        neck_bottom_y,
        neck_top_w,
        neck_bottom_w,
        skin_deep,
    );

    // Head — nested eggs.
    draw_egg(cx, cy, head_w, head_h, skin_deep);
    draw_egg(
        cx,
        cy - head_h * 0.015,
        head_w * 0.96,
        head_h * 0.97,
        skin_mid,
    );
    draw_egg(
        cx - head_w * 0.06,
        cy - head_h * 0.16,
        head_w * 0.55,
        head_h * 0.36,
        skin_light,
    );

    // Forehead veins.
    draw_forehead_veins(cx, cy - head_h * 0.20, head_w, head_h, vein);

    // Eyes — two slanted almonds.
    let eye_dx = head_w * 0.235;
    let eye_dy = -head_h * 0.04;
    let eye_w = 130.0 * zoom;
    let eye_h = 58.0 * zoom;

    draw_slanted_almond(cx - eye_dx, cy + eye_dy, eye_w, eye_h, 0.55, eye_void);
    draw_slanted_almond(cx + eye_dx, cy + eye_dy, eye_w, eye_h, -0.55, eye_void);

    // Eye highlights fade out as the zoom progresses.
    let hl_alpha = (1.0 - zoom_t).powf(2.0) * 0.85 * alpha;
    if hl_alpha > 0.01 {
        let hl_r = eye_w * 0.09;
        draw_circle(
            cx - eye_dx - eye_w * 0.28 + look_x,
            cy + eye_dy - eye_h * 0.18 + look_y,
            hl_r,
            Color::new(0.88, 0.88, 0.88, hl_alpha),
        );
        draw_circle(
            cx + eye_dx + eye_w * 0.28 + look_x,
            cy + eye_dy - eye_h * 0.18 + look_y,
            hl_r,
            Color::new(0.88, 0.88, 0.88, hl_alpha),
        );
    }

    // Nostrils.
    let nose_y = cy + head_h * 0.13;
    draw_circle(
        cx - head_w * 0.028,
        nose_y,
        zoom * 3.0,
        Color::new(0.02, 0.02, 0.02, alpha),
    );
    draw_circle(
        cx + head_w * 0.028,
        nose_y,
        zoom * 3.0,
        Color::new(0.02, 0.02, 0.02, alpha),
    );

    // Mouth slit.
    let mouth_y = cy + head_h * 0.30;
    let mouth_w = head_w * 0.11;
    draw_rectangle(
        cx - mouth_w * 0.5,
        mouth_y,
        mouth_w,
        zoom * 2.0,
        Color::new(0.02, 0.02, 0.02, alpha),
    );
}

fn draw_egg(cx: f32, cy: f32, w: f32, h: f32, color: Color) {
    const STRIPS: usize = 60;
    let strip_h = h / STRIPS as f32;
    let top_y = cy - h * 0.5;
    for i in 0..STRIPS {
        let t = i as f32 / (STRIPS - 1) as f32;
        let y_norm = t * 2.0 - 1.0;
        let egg_factor = 1.0 + 0.18 * y_norm;
        let half_w =
            (1.0 - y_norm * y_norm).max(0.0).sqrt() * w * 0.5 * egg_factor;
        draw_rectangle(
            cx - half_w,
            top_y + i as f32 * strip_h,
            half_w * 2.0,
            strip_h + 0.6,
            color,
        );
    }
}

fn draw_slanted_almond(
    cx: f32,
    cy: f32,
    w: f32,
    h: f32,
    tilt: f32,
    color: Color,
) {
    const STRIPS: usize = 40;
    let strip_h = h / STRIPS as f32;
    let top_y = cy - h * 0.5;
    for i in 0..STRIPS {
        let t = i as f32 / (STRIPS - 1) as f32;
        let y_norm = t * 2.0 - 1.0;
        let half_w = (1.0 - y_norm * y_norm).max(0.0).sqrt() * w * 0.5;
        let x_shift = tilt * y_norm * w * 0.35;
        draw_rectangle(
            cx + x_shift - half_w,
            top_y + i as f32 * strip_h,
            half_w * 2.0,
            strip_h + 0.6,
            color,
        );
    }
}

fn draw_tapered_column(
    cx: f32,
    top_y: f32,
    bottom_y: f32,
    top_w: f32,
    bottom_w: f32,
    color: Color,
) {
    const STRIPS: usize = 30;
    let total_h = bottom_y - top_y;
    let strip_h = total_h / STRIPS as f32;
    for i in 0..STRIPS {
        let t = i as f32 / (STRIPS - 1) as f32;
        let w = top_w * (1.0 - t) + bottom_w * t;
        draw_rectangle(
            cx - w * 0.5,
            top_y + i as f32 * strip_h,
            w,
            strip_h + 0.6,
            color,
        );
    }
}

fn draw_forehead_veins(
    cx: f32,
    top_cy: f32,
    head_w: f32,
    head_h: f32,
    color: Color,
) {
    for i in 0..3 {
        let offset = (i as f32 - 1.0) * 0.18;
        let start_x = cx + head_w * offset * 0.6;
        let start_y = top_cy;
        const SEGS: usize = 10;
        let mut prev = (start_x, start_y);
        for s in 1..=SEGS {
            let t = s as f32 / SEGS as f32;
            let x =
                start_x + (t * 8.0 + i as f32 * 2.0).sin() * head_w * 0.05;
            let y = start_y + t * head_h * 0.22;
            draw_line(prev.0, prev.1, x, y, 1.2, color);
            prev = (x, y);
        }
    }
}

fn draw_halloween_message(vw: f32, vh: f32, t: f32) {
    let poem_light = Color::new(0.96, 0.92, 0.82, 1.0);
    let poem_gold = Color::new(0.94, 0.78, 0.40, 1.0);
    let dim = Color::new(0.62, 0.56, 0.46, 1.0);
    let cinnabar = Color::new(0.82, 0.30, 0.20, 1.0);

    // Fixed 300 px block, vertically centered on the screen.
    let base = vh * 0.5 - 150.0;
    let y_eyebrow = base;
    let y_div1 = base + 28.0;
    let y_line1 = base + 92.0;
    let y_line2 = base + 134.0;
    let y_accent = base + 200.0;
    let y_div2 = base + 240.0;
    let y_ember = base + 278.0;
    let y_year = base + 306.0;

    let fade = |start: f32| -> f32 {
        Easing::EaseOut.apply(((t - start) / 0.9).clamp(0.0, 1.0))
    };

    let centered = |text: &str, y: f32, size: f32, color: Color, alpha: f32| {
        if alpha <= 0.01 {
            return;
        }
        let d = measure_text(text, None, size as u16, 1.0);
        let c = Color::new(color.r, color.g, color.b, color.a * alpha);
        draw_text(text, vw * 0.5 - d.width * 0.5, y, size, c);
    };

    // Eyebrow.
    centered(
        "S I G N A L   H I J A C K E D",
        y_eyebrow,
        12.0,
        poem_gold,
        fade(0.0),
    );

    // Top divider.
    let a1 = fade(0.4);
    if a1 > 0.01 {
        let w = 380.0;
        draw_line(
            vw * 0.5 - w * 0.5,
            y_div1,
            vw * 0.5 + w * 0.5,
            y_div1,
            1.0,
            Color::new(poem_gold.r, poem_gold.g, poem_gold.b, 0.55 * a1),
        );
    }

    // Addressee — her name, big and warm.
    centered("Wang Yi,", y_line1, 32.0, poem_light, fade(0.9));

    // Ominous follow-up, smaller, same warm tone.
    centered(
        "you weren't supposed to see this.",
        y_line2,
        20.0,
        poem_light,
        fade(1.7),
    );

    // Accent line.
    centered(
        "Something darker waits there.",
        y_accent,
        22.0,
        cinnabar,
        fade(3.2),
    );

    // Bottom divider.
    let a2 = fade(3.9);
    if a2 > 0.01 {
        let w = 200.0;
        draw_line(
            vw * 0.5 - w * 0.5,
            y_div2,
            vw * 0.5 + w * 0.5,
            y_div2,
            1.0,
            Color::new(cinnabar.r, cinnabar.g, cinnabar.b, 0.45 * a2),
        );
    }

    // Signature.
    centered(
        "E M B E R   W O R K S P A C E",
        y_ember,
        11.0,
        dim,
        fade(4.4),
    );
    centered("Halloween 2026", y_year, 18.0, poem_gold, fade(4.9));

    // Blinking prompt.
    let prompt_alpha = Easing::EaseOut.apply(((t - 5.7) / 0.8).clamp(0.0, 1.0));
    if prompt_alpha > 0.01 {
        let prompt = "[Enter]  return to menu";
        let d = measure_text(prompt, None, 13, 1.0);
        let pulse = 0.55 + 0.45 * ((t * 2.2).sin() * 0.5 + 0.5);
        draw_text(
            prompt,
            vw * 0.5 - d.width * 0.5,
            vh - 44.0,
            13.0,
            Color::new(dim.r, dim.g, dim.b, prompt_alpha * pulse),
        );
    }
}

// ─── Game background & board ───────────────────────────────────────

fn draw_game_background(state: &AppState, chapter: u32) {
    let vh = screen_height();
    let ground_y = vh * HORIZON_FRAC;
    let palette = scroll_painting::Palette::for_chapter(chapter);
    scroll_painting::draw_sky(ground_y, palette);
    let cam_x = state.drift * DRIFT_SPEED;
    scroll_painting::draw_mountains(cam_x, ground_y, 0xBEEF, palette);
}

fn draw_game(ctx: &GameContext, game: &Game, shake: Vec2) {
    draw_grid(ctx, game, shake);
    juice::draw_shockwaves(&game.juice, shake);
    juice::draw_radial_rays(&game.juice, shake);
    juice::draw_particles(&game.juice, shake);
    juice::draw_floating_texts(&game.juice, shake);
    hud::draw(ctx, game);
    juice::draw_vignette(&game.juice);
    juice::draw_flashes(&game.juice);
}

#[allow(clippy::too_many_arguments)]
fn draw_wooden_board(
    bx: f32,
    by: f32,
    bw: f32,
    bh: f32,
    grid_x: f32,
    grid_y: f32,
    grid_w: f32,
    grid_h: f32,
    gold: Color,
) {
    let wood_dark = Color::new(0.075, 0.050, 0.030, 1.0);
    let wood_light = Color::new(0.26, 0.18, 0.10, 1.0);
    let wood_highlight = Color::new(0.36, 0.26, 0.14, 1.0);
    let wood_shadow = Color::new(0.04, 0.025, 0.015, 1.0);

    for i in 0..4 {
        let f = i as f32;
        let off = 3.0 + f * 3.0;
        let a = 0.28 * (1.0 - f * 0.22);
        draw_rectangle(
            bx - off,
            by - off + 4.0,
            bw + off * 2.0,
            bh + off * 2.0,
            Color::new(0.0, 0.0, 0.0, a),
        );
    }

    draw_rectangle(bx, by, bw, bh, gold);
    draw_rectangle_lines(
        bx + 2.5,
        by + 2.5,
        bw - 5.0,
        bh - 5.0,
        1.0,
        Color::new(gold.r * 0.55, gold.g * 0.55, gold.b * 0.55, 1.0),
    );

    let wx = bx + 4.0;
    let wy = by + 4.0;
    let ww = bw - 8.0;
    let wh = bh - 8.0;

    const STRIPS: usize = 24;
    let strip_h = wh / STRIPS as f32;
    for i in 0..STRIPS {
        let t = i as f32 / (STRIPS - 1) as f32;
        let c = Color::new(
            wood_light.r * (1.0 - t) + wood_dark.r * t,
            wood_light.g * (1.0 - t) + wood_dark.g * t,
            wood_light.b * (1.0 - t) + wood_dark.b * t,
            1.0,
        );
        draw_rectangle(wx, wy + i as f32 * strip_h, ww, strip_h + 0.5, c);
    }

    let grain_lines = 22;
    for i in 0..grain_lines {
        let t = (i as f32 + 0.5) / grain_lines as f32;
        let gy_line = wy + 6.0 + t * (wh - 12.0);
        let wave =
            ((gy_line * 0.09).sin() + (gy_line * 0.21).cos() * 0.5) * 3.5;
        let a = 0.05 + 0.04 * (gy_line * 0.13).sin().abs();
        draw_line(
            wx + 6.0,
            gy_line + wave,
            wx + ww - 6.0,
            gy_line + wave,
            1.0,
            Color::new(wood_shadow.r, wood_shadow.g, wood_shadow.b, a),
        );
    }

    for i in 0..2 {
        let t = if i == 0 { 0.28 } else { 0.68 };
        let gy_line = wy + t * wh;
        let wave = (gy_line * 0.12).sin() * 2.0;
        draw_line(
            wx + 4.0,
            gy_line + wave,
            wx + ww - 4.0,
            gy_line + wave,
            1.0,
            Color::new(
                wood_highlight.r,
                wood_highlight.g,
                wood_highlight.b,
                0.18,
            ),
        );
    }

    draw_rectangle(
        wx,
        wy,
        ww,
        2.0,
        Color::new(wood_highlight.r, wood_highlight.g, wood_highlight.b, 0.55),
    );
    draw_rectangle(
        wx,
        wy,
        1.5,
        wh,
        Color::new(wood_highlight.r, wood_highlight.g, wood_highlight.b, 0.35),
    );
    draw_rectangle(
        wx,
        wy + wh - 2.5,
        ww,
        2.5,
        Color::new(wood_shadow.r, wood_shadow.g, wood_shadow.b, 0.75),
    );
    draw_rectangle(
        wx + ww - 2.0,
        wy,
        2.0,
        wh,
        Color::new(wood_shadow.r, wood_shadow.g, wood_shadow.b, 0.55),
    );

    let rx = grid_x - 8.0;
    let ry = grid_y - 8.0;
    let rw = grid_w + 16.0;
    let rh = grid_h + 16.0;

    draw_rectangle(rx, ry, rw, rh, Color::new(0.025, 0.018, 0.012, 1.0));

    const SHADOW_BANDS: usize = 6;
    let band_h = 14.0 / SHADOW_BANDS as f32;
    for i in 0..SHADOW_BANDS {
        let t = i as f32 / SHADOW_BANDS as f32;
        let a = 0.35 * (1.0 - t);
        draw_rectangle(
            rx,
            ry + i as f32 * band_h,
            rw,
            band_h + 0.5,
            Color::new(0.0, 0.0, 0.0, a),
        );
    }
    for i in 0..4 {
        let t = i as f32 / 4.0;
        let a = 0.20 * (1.0 - t);
        draw_rectangle(
            rx + i as f32 * 3.0,
            ry,
            3.0,
            rh,
            Color::new(0.0, 0.0, 0.0, a),
        );
    }

    draw_rectangle_lines(rx, ry, rw, rh, 1.5, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_line(
        rx + 1.0,
        ry + rh - 1.5,
        rx + rw - 1.5,
        ry + rh - 1.5,
        1.0,
        Color::new(wood_highlight.r, wood_highlight.g, wood_highlight.b, 0.20),
    );

    draw_rectangle_lines(
        rx - 3.0,
        ry - 3.0,
        rw + 6.0,
        rh + 6.0,
        1.0,
        Color::new(gold.r * 0.75, gold.g * 0.75, gold.b * 0.75, 0.55),
    );
}

fn draw_grid(ctx: &GameContext, game: &Game, shake: Vec2) {
    let (gx, gy) = ctx.grid_origin();
    let grid_w = GRID_W as f32 * TILE_STRIDE - TILE_GAP;
    let grid_h = GRID_H as f32 * TILE_STRIDE - TILE_GAP;

    draw_wooden_board(
        gx - 20.0 + shake.x,
        gy - 20.0 + shake.y,
        grid_w + 40.0,
        grid_h + 40.0,
        gx + shake.x,
        gy + shake.y,
        grid_w,
        grid_h,
        ctx.colors.panel_border,
    );

    for row in 0..GRID_H {
        for col in 0..GRID_W {
            let t = match game.grid.get(row, col) {
                Some(t) => t,
                None => continue,
            };
            let x = gx + t.visual_col * TILE_STRIDE + shake.x;
            let y = gy + t.visual_row * TILE_STRIDE + shake.y;
            let scale = game.tile_scale(row, col);
            if scale < 0.01 {
                continue;
            }
            if game.selected == Some((row, col)) {
                draw_tile_glow(t.jade, x, y, TILE_SIZE, 0.9);
            }
            draw_tile(t.jade, x, y, TILE_SIZE, scale, 1.0, t.vein_seed);
        }
    }

    if let Some((r, c)) = game.selected
        && let Some(t) = game.grid.get(r, c)
    {
        let x = gx + t.visual_col * TILE_STRIDE + shake.x;
        let y = gy + t.visual_row * TILE_STRIDE + shake.y;
        let pulse = 0.65 + 0.35 * (game.time * 5.0).sin();
        draw_selection_corners(x, y, TILE_SIZE, pulse);
    }

    if let Some((r, c)) = game.hover
        && game.selected != Some((r, c))
        && game.phase == Phase::Playing
        && let Some(t) = game.grid.get(r, c)
    {
        let x = gx + t.visual_col * TILE_STRIDE + shake.x;
        let y = gy + t.visual_row * TILE_STRIDE + shake.y;
        let cx = x + TILE_SIZE * 0.5;
        let cy = y + TILE_SIZE * 0.5;
        let (sides, rot) = jade_shape_params(t.jade);

        let pulse = 0.80 + 0.20 * (game.time * 4.0).sin();
        draw_poly_lines(
            cx,
            cy,
            sides,
            TILE_SIZE * 0.58,
            rot,
            1.6,
            Color::new(1.0, 0.95, 0.65, 0.60 * pulse),
        );
        draw_poly_lines(
            cx,
            cy,
            sides,
            TILE_SIZE * 0.63,
            rot,
            1.0,
            Color::new(1.0, 0.95, 0.65, 0.25 * pulse),
        );
    }

    // ─── Active hint ───
    // Pulsing golden ring around the two suggested tiles.
    if let Some(h) = game.hint.as_ref() {
        let fade = 1.0 - (h.elapsed / h.ttl).clamp(0.0, 1.0);
        let pulse = 0.60 + 0.40 * (game.time * 5.0).sin();
        let alpha = fade * pulse;

        for (r, c) in [h.a, h.b] {
            if let Some(t) = game.grid.get(r, c) {
                let x = gx + t.visual_col * TILE_STRIDE + shake.x;
                let y = gy + t.visual_row * TILE_STRIDE + shake.y;
                let cx = x + TILE_SIZE * 0.5;
                let cy = y + TILE_SIZE * 0.5;
                let (sides, rot) = jade_shape_params(t.jade);

                draw_poly_lines(
                    cx,
                    cy,
                    sides,
                    TILE_SIZE * 0.60,
                    rot,
                    2.0,
                    Color::new(0.98, 0.80, 0.35, 0.85 * alpha),
                );
                draw_poly_lines(
                    cx,
                    cy,
                    sides,
                    TILE_SIZE * 0.66,
                    rot,
                    1.0,
                    Color::new(0.98, 0.80, 0.35, 0.40 * alpha),
                );
            }
        }
    }
}

fn draw_intro(ctx: &GameContext, state: &AppState, level_idx: usize) {
    let vw = screen_width();
    let vh = screen_height();

    let chapter_idx = state.menu.levels[level_idx].chapter;
    draw_game_background(state, chapter_idx);

    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.02, 0.03, 0.02, 0.72));

    let lvl = &state.menu.levels[level_idx];
    let chapter = &state.menu.chapters[chapter_idx as usize];

    let title = Color::new(0.94, 0.90, 0.80, 1.0);
    let dim = Color::new(0.72, 0.68, 0.58, 1.0);

    let d = measure_text(&chapter.subtitle, None, 22, 1.0);
    draw_text(
        &chapter.subtitle,
        vw * 0.5 - d.width * 0.5,
        vh * 0.26,
        22.0,
        dim,
    );

    let d = measure_text(&chapter.name, None, 44, 1.0);
    draw_text(
        &chapter.name,
        vw * 0.5 - d.width * 0.5,
        vh * 0.26 + 50.0,
        44.0,
        ctx.colors.gold,
    );

    let lines = wrap_text(&chapter.intro, 58);
    let line_h = 26.0;
    let text_y0 = vh * 0.48;
    for (i, line) in lines.iter().enumerate() {
        let d = measure_text(line, None, 18, 1.0);
        draw_text(
            line,
            vw * 0.5 - d.width * 0.5,
            text_y0 + i as f32 * line_h,
            18.0,
            title,
        );
    }

    let lvl_name = format!("— {} —", lvl.name);
    let d = measure_text(&lvl_name, None, 20, 1.0);
    draw_text(
        &lvl_name,
        vw * 0.5 - d.width * 0.5,
        vh * 0.72,
        20.0,
        ctx.colors.gold,
    );

    let obj_txt = level::objective_label(lvl.objective);
    let d = measure_text(&obj_txt, None, 14, 1.0);
    draw_text(
        &obj_txt,
        vw * 0.5 - d.width * 0.5,
        vh * 0.72 + 26.0,
        14.0,
        dim,
    );

    let ready = state.intro_elapsed >= INTRO_MIN_DURATION;
    let hint = if ready {
        "Click to start    [Esc] back"
    } else {
        "…"
    };
    let d = measure_text(hint, None, 16, 1.0);
    draw_text(
        hint,
        vw * 0.5 - d.width * 0.5,
        vh * 0.88,
        16.0,
        dim,
    );
}

fn draw_won_overlay(ctx: &GameContext, state: &AppState, game: &Game) {
    let vw = screen_width();
    let vh = screen_height();

    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.02, 0.03, 0.02, 0.96));

    let (from, to, elapsed) = match state.win_reveal.as_ref() {
        Some(r) => (r.revealed_from, r.revealed_to, r.elapsed),
        None => return,
    };

    let poem_light = Color::new(0.96, 0.92, 0.82, 1.0);
    let poem_gold = Color::new(0.94, 0.82, 0.50, 1.0);
    let dim = Color::new(0.72, 0.68, 0.58, 1.0);
    let dim_more = Color::new(0.55, 0.50, 0.42, 1.0);
    let empty_box = Color::new(0.32, 0.32, 0.28, 0.55);

    let title = "The garden awakens";
    let d = measure_text(title, None, 42, 1.0);
    draw_text(
        title,
        vw * 0.5 - d.width * 0.5,
        vh * 0.11,
        42.0,
        ctx.colors.gold,
    );

    let run_time = state.win_reveal.as_ref().map(|r| r.run_time).unwrap_or(0.0);
    let new_best = state.win_reveal.as_ref().map(|r| r.new_best).unwrap_or(false);
    let score_line = if new_best {
        format!("Score {}   ·   {}   ·   NEW BEST", game.score, fmt_time(run_time))
    } else {
        format!("Score {}   ·   {}", game.score, fmt_time(run_time))
    };
    let d = measure_text(&score_line, None, 18, 1.0);
    draw_text(
        &score_line,
        vw * 0.5 - d.width * 0.5,
        vh * 0.11 + 30.0,
        18.0,
        if new_best { ctx.colors.gold } else { poem_light },
    );

    let char_size = 48.0;
    let line_spacing = 68.0;
    let poem_x = vw * 0.5 - 5.0 * char_size * 0.5;
    let poem_y = vh * 0.26;

    for line in 0..4usize {
        for col in 0..5usize {
            let global = (line * 5 + col) as u8;
            let x = poem_x + col as f32 * char_size;
            let y = poem_y + line as f32 * line_spacing;

            if global >= to {
                draw_rectangle_lines(
                    x + 4.0,
                    y + 4.0,
                    char_size - 12.0,
                    char_size - 12.0,
                    1.0,
                    empty_box,
                );
            } else if global < from {
                if let Some(c) = poem::char_at(global) {
                    state.poem_font.draw(
                        &c.to_string(),
                        x + char_size * 0.08,
                        y + char_size * 0.88,
                        char_size * 0.88,
                        poem_light,
                    );
                }
            } else {
                let offset = (global - from) as f32;
                let local_t = ((elapsed - offset * CHAR_REVEAL_DURATION)
                    / CHAR_REVEAL_DURATION)
                    .clamp(0.0, 1.0);
                let alpha = Easing::EaseOut.apply(local_t);
                if alpha > 0.01
                    && let Some(c) = poem::char_at(global)
                {
                    let color =
                        Color::new(poem_gold.r, poem_gold.g, poem_gold.b, alpha);
                    state.poem_font.draw(
                        &c.to_string(),
                        x + char_size * 0.08,
                        y + char_size * 0.88,
                        char_size * 0.88,
                        color,
                    );
                }
            }
        }
    }

    for i in 0..(to - from) {
        let global = from + i;
        let local_t = ((elapsed - i as f32 * CHAR_REVEAL_DURATION)
            / CHAR_REVEAL_DURATION)
            .clamp(0.0, 1.0);
        let alpha = Easing::EaseOut.apply(local_t);
        if alpha <= 0.01 {
            continue;
        }
        let (line, col) = match poem::position(global) {
            Some(p) => p,
            None => continue,
        };
        let x = poem_x + col as f32 * char_size;
        let y = poem_y + line as f32 * line_spacing;
        if let Some(p) = poem::pinyin_at(global) {
            let d = measure_text(p, None, 12, 1.0);
            let color = Color::new(0.85, 0.78, 0.60, alpha);
            draw_text(
                p,
                x + (char_size - d.width) * 0.5,
                y + char_size + 8.0,
                12.0,
                color,
            );
        }
    }

    let footer_y = poem_y + 4.0 * line_spacing + 30.0;

    state.poem_font.draw_centered(
        poem::POEM_TITLE,
        vw * 0.5,
        footer_y,
        16.0,
        poem_light,
    );
    state.poem_font.draw_centered(
        poem::POEM_AUTHOR,
        vw * 0.5,
        footer_y + 22.0,
        13.0,
        dim,
    );

    let progress = format!("{} / {}", to, poem::TOTAL_CHARS);
    let d = measure_text(&progress, None, 22, 1.0);
    draw_text(
        &progress,
        vw * 0.5 - d.width * 0.5,
        footer_y + 56.0,
        22.0,
        ctx.colors.gold,
    );

    let new_count = (to - from) as f32;
    let reveal_done_at = new_count * CHAR_REVEAL_DURATION + REVEAL_TAIL_PAUSE;
    if elapsed >= reveal_done_at {
        let hint = "[Enter] continue    [R] replay    [Esc] menu";
        let d = measure_text(hint, None, 16, 1.0);
        draw_text(
            hint,
            vw * 0.5 - d.width * 0.5,
            vh - 36.0,
            16.0,
            dim_more,
        );
    }
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Formats a duration in seconds as `M:SS`.
fn fmt_time(secs: f32) -> String {
    let total = secs.round() as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

/// Cheap pseudo-random [0, 1] from a seed — used for the final shake.
fn rand_shake_seed(x: f32) -> f32 {
    let n = (x * 1234.567).sin() * 43_758.547;
    n - n.floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── next_reveal_step ───

    #[test]
    fn non_final_levels_never_trigger_reveals() {
        assert_eq!(next_reveal_step(false, false, false, true), RevealStep::Menu);
        assert_eq!(next_reveal_step(false, true, false, true), RevealStep::Menu);
        assert_eq!(next_reveal_step(false, false, true, true), RevealStep::Menu);
        assert_eq!(next_reveal_step(false, true, true, false), RevealStep::Menu);
    }

    #[test]
    fn epilogue_plays_first_after_final_level() {
        assert_eq!(next_reveal_step(true, false, false, true), RevealStep::Epilogue);
        assert_eq!(next_reveal_step(true, false, false, false), RevealStep::Epilogue);
        assert_eq!(next_reveal_step(true, false, true, true), RevealStep::Epilogue);
    }

    #[test]
    fn halloween_plays_after_epilogue_when_in_season() {
        assert_eq!(next_reveal_step(true, true, false, true), RevealStep::Halloween);
    }

    #[test]
    fn halloween_skipped_out_of_season() {
        assert_eq!(next_reveal_step(true, true, false, false), RevealStep::Menu);
    }

    #[test]
    fn halloween_never_replays_once_seen() {
        assert_eq!(next_reveal_step(true, true, true, true), RevealStep::Menu);
    }

    // ─── is_spooky_month ───

    #[test]
    fn is_spooky_month_only_covers_sept_oct_nov() {
        for m in 1..=12u64 {
            let expected = (9..=11).contains(&m);
            assert_eq!(is_spooky_month(m), expected, "month {m}");
        }
    }

    // ─── month_from_unix ───

    #[test]
    fn month_from_unix_always_returns_1_to_12() {
        for secs in [0u64, 1, 86400, 1_000_000_000, 2_000_000_000, u32::MAX as u64] {
            let m = month_from_unix(secs);
            assert!((1..=12).contains(&m), "secs {secs} → month {m}");
        }
    }

    #[test]
    fn month_from_unix_on_known_dates() {
        // All timestamps are 00:00:00 UTC on the 1st of the month.
        let cases = [
            (0u64, 1u64),            // 1970-01-01
            (1704067200u64, 1u64),   // 2024-01-01
            (1717200000u64, 6u64),   // 2024-06-01
            (1727740800u64, 10u64),  // 2024-10-01
            (1733011200u64, 12u64),  // 2024-12-01
            (1735689600u64, 1u64),   // 2025-01-01
            (1751328000u64, 7u64),   // 2025-07-01
        ];
        for (secs, expected) in cases {
            assert_eq!(
                month_from_unix(secs),
                expected,
                "unix {secs} should map to month {expected}"
            );
        }
    }

    #[test]
    fn month_from_unix_handles_leap_day() {
        // 2024-02-29 is a leap day → still month 2.
        assert_eq!(month_from_unix(1709164800), 2);
    }

    #[test]
    fn month_from_unix_handles_century_non_leap() {
        // 2100 is not a leap year (÷100, not ÷400). Sanity check that
        // the algorithm doesn't drift on the century boundary.
        // 2100-03-01 00:00:00 UTC.
        assert_eq!(month_from_unix(4107542400), 3);
    }
}