// games/pagoda/src/app.rs
use crate::art::with_alpha;
use crate::config::{CINNABAR, GameContext, IVORY, PERFECT_GOLD};
use crate::controls::confirm_pressed;
use crate::fonts::CjkFont;
use crate::ui::{self, draw_centered, draw_centered_shadowed};
use ember_core::Color;
use ember_input::Input;
use ember_servers::RenderingServer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    MainMenu,
    Playing,
    GameOver,
}

pub struct App {
    pub state: AppState,
    pub score: u32,
    pub high_score: u32,
    /// True when the finished run beat the previous best.
    pub new_best: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self::with_high_score(0)
    }

    /// Start in the menu with a best score already loaded from disk.
    pub fn with_high_score(high_score: u32) -> Self {
        Self {
            state: AppState::MainMenu,
            score: 0,
            high_score,
            new_best: false,
        }
    }

    /// Record the current run as the best score. Returns `true` when it is a
    /// new best, so the caller can persist it.
    pub fn record_high_score(&mut self) -> bool {
        if self.score > self.high_score {
            self.high_score = self.score;
            self.new_best = true;
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, input: &Input) {
        match self.state {
            AppState::MainMenu => {
                if confirm_pressed(input) {
                    self.state = AppState::Playing;
                    self.score = 0;
                    self.new_best = false;
                }
            }
            AppState::Playing => {
                // Logic is handled in systems.rs
            }
            AppState::GameOver => {
                self.record_high_score();
                if confirm_pressed(input) {
                    self.state = AppState::Playing;
                    self.score = 0;
                    self.new_best = false;
                }
            }
        }
    }

    /// `time`: seconds since the game started (was `get_time()`).
    pub fn draw(
        &self,
        r: &mut dyn RenderingServer,
        ctx: &GameContext,
        combo: u32,
        cjk: &CjkFont,
        time: f32,
    ) {
        let cx = ctx.screen_w / 2.0;
        let breath = 0.62 + 0.38 * (0.5 + 0.5 * (time * 2.2).sin());

        match self.state {
            AppState::MainMenu => {
                // Lacquer signboard with a red seal: 宝塔 (precious pagoda).
                ui::draw_signboard(r, cx, 112.0, 520.0, 112.0);
                ui::draw_seal(r, cx, 112.0, 40.0, "塔", cjk);

                if cjk.is_available() {
                    cjk.draw_centered(r, "宝 塔", cx, 150.0, 20.0, with_alpha(PERFECT_GOLD, 0.92));
                } else {
                    ui::draw_divider(r, cx, 145.0, 54.0, with_alpha(PERFECT_GOLD, 0.55));
                }

                draw_centered_shadowed(
                    r,
                    "PAGODA",
                    cx,
                    208.0,
                    64,
                    PERFECT_GOLD,
                    Color::new(0.20, 0.02, 0.01, 0.85),
                    3.0,
                );

                draw_centered(
                    r,
                    "STACK THE BLOCKS · CHASE THE PERFECT DROP",
                    cx,
                    272.0,
                    18,
                    with_alpha(IVORY, 0.72),
                );

                if self.high_score > 0 {
                    ui::draw_plaque(r, cx, 314.0, 250.0, 46.0);
                    draw_centered(
                        r,
                        &format!("BEST  {}", self.high_score),
                        cx,
                        344.0,
                        24,
                        PERFECT_GOLD,
                    );
                }

                draw_centered(
                    r,
                    "TAP / CLICK to start",
                    cx,
                    438.0,
                    28,
                    with_alpha(IVORY, breath),
                );
                draw_centered(
                    r,
                    "TAP / CLICK drops · SPEAKER mutes",
                    cx,
                    548.0,
                    15,
                    with_alpha(IVORY, 0.38),
                );
            }
            AppState::Playing => {
                let score_text = format!("{}", self.score);
                draw_centered_shadowed(
                    r,
                    &score_text,
                    cx,
                    82.0,
                    62,
                    IVORY,
                    Color::new(0.02, 0.02, 0.05, 0.75),
                    3.0,
                );

                // A breath of space between the score and the record,
                // marked with a small gold rule.
                ui::draw_divider(r, cx, 100.0, 78.0, with_alpha(PERFECT_GOLD, 0.55));
                draw_centered(
                    r,
                    &format!("BEST  {}", self.high_score),
                    cx,
                    140.0,
                    18,
                    with_alpha(IVORY, 0.68),
                );

                if combo > 0 {
                    if cjk.is_available() {
                        let label = if combo > 1 {
                            format!("完美 x{combo}")
                        } else {
                            "完美".to_owned()
                        };
                        cjk.draw_centered(
                            r,
                            &label,
                            cx + 1.5,
                            181.5,
                            24.0,
                            Color::new(0.30, 0.05, 0.02, 0.70),
                        );
                        cjk.draw_centered(r, &label, cx, 180.0, 24.0, PERFECT_GOLD);
                    } else {
                        let label = if combo > 1 {
                            format!("PERFECT x{combo}")
                        } else {
                            "PERFECT".to_owned()
                        };
                        draw_centered(r, &label, cx, 180.0, 24, PERFECT_GOLD);
                    }
                }

                ui::draw_seal(r, ctx.screen_w - 44.0, 44.0, 34.0, "塔", cjk);
            }
            AppState::GameOver => {
                ui::draw_panel(
                    r,
                    cx,
                    150.0,
                    470.0,
                    360.0,
                    Color::new(0.030, 0.030, 0.050, 0.84),
                    with_alpha(PERFECT_GOLD, 0.55),
                );
                ui::draw_seal(r, cx, 150.0, 42.0, "终", cjk);

                draw_centered_shadowed(
                    r,
                    "GAME OVER",
                    cx,
                    228.0,
                    54,
                    CINNABAR,
                    Color::new(0.05, 0.01, 0.01, 0.80),
                    3.0,
                );

                if self.new_best {
                    if cjk.is_available() {
                        cjk.draw_centered(
                            r,
                            "新 纪 录",
                            cx,
                            282.0,
                            24.0,
                            with_alpha(PERFECT_GOLD, 0.95),
                        );
                    }
                    draw_centered(r, "NEW BEST!", cx, 312.0, 19, PERFECT_GOLD);
                } else {
                    draw_centered(r, "THE TOWER RESTS", cx, 290.0, 17, with_alpha(IVORY, 0.55));
                }

                ui::draw_divider(r, cx, 342.0, 84.0, with_alpha(PERFECT_GOLD, 0.45));
                draw_centered(r, &format!("SCORE   {}", self.score), cx, 388.0, 30, IVORY);
                draw_centered(
                    r,
                    &format!("BEST    {}", self.high_score),
                    cx,
                    424.0,
                    19,
                    with_alpha(PERFECT_GOLD, 0.90),
                );
                draw_centered(
                    r,
                    "TAP / CLICK to rebuild",
                    cx,
                    478.0,
                    23,
                    with_alpha(IVORY, breath),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ember_input::Key;

    fn press(key: Key) -> Input {
        Input {
            keys_pressed: vec![key],
            ..Default::default()
        }
    }

    fn click() -> Input {
        Input {
            mouse_left_pressed: true,
            ..Default::default()
        }
    }

    #[test]
    fn default_matches_new() {
        let app = App::default();
        assert_eq!(app.state, AppState::MainMenu);
        assert_eq!(app.score, 0);
        assert_eq!(app.high_score, 0);
    }

    #[test]
    fn space_starts_from_the_menu() {
        let mut app = App::new();
        app.update(&press(Key::Space));
        assert_eq!(app.state, AppState::Playing);
        assert_eq!(app.score, 0);
    }

    #[test]
    fn click_starts_from_the_menu() {
        let mut app = App::new();
        app.update(&click());
        assert_eq!(app.state, AppState::Playing);
    }

    #[test]
    fn with_high_score_starts_in_the_menu_with_the_best() {
        let app = App::with_high_score(42);
        assert_eq!(app.state, AppState::MainMenu);
        assert_eq!(app.score, 0);
        assert_eq!(app.high_score, 42);
    }

    #[test]
    fn record_high_score_reports_only_new_bests() {
        let mut app = App::with_high_score(10);
        app.score = 12;

        assert!(app.record_high_score());
        assert!(app.new_best);
        assert_eq!(app.high_score, 12);
        assert!(!app.record_high_score());
        assert_eq!(app.high_score, 12);
    }

    #[test]
    fn record_high_score_keeps_a_better_previous_run() {
        let mut app = App::with_high_score(10);
        app.score = 7;

        assert!(!app.record_high_score());
        assert!(!app.new_best);
        assert_eq!(app.high_score, 10);
    }

    #[test]
    fn restarting_clears_the_new_best_flag() {
        let mut app = App {
            state: AppState::GameOver,
            score: 12,
            high_score: 10,
            new_best: false,
        };

        app.update(&Input::default());
        assert!(app.new_best);

        app.update(&press(Key::Space));
        assert_eq!(app.state, AppState::Playing);
        assert!(!app.new_best);
    }

    #[test]
    fn game_over_records_a_new_best() {
        let mut app = App {
            state: AppState::GameOver,
            score: 12,
            high_score: 10,
            new_best: false,
        };

        app.update(&Input::default());

        assert_eq!(app.high_score, 12);
        assert_eq!(app.state, AppState::GameOver);
    }

    #[test]
    fn game_over_does_not_lower_the_best() {
        let mut app = App {
            state: AppState::GameOver,
            score: 7,
            high_score: 10,
            new_best: false,
        };

        app.update(&Input::default());

        assert_eq!(app.high_score, 10);
    }

    #[test]
    fn game_over_restarts_and_resets_the_score() {
        let mut app = App {
            state: AppState::GameOver,
            score: 12,
            high_score: 10,
            new_best: false,
        };

        app.update(&press(Key::Space));

        assert_eq!(app.state, AppState::Playing);
        assert_eq!(app.score, 0);
        assert_eq!(app.high_score, 12);
    }
}
