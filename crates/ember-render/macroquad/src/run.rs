//! The runner: drives a `MainLoop` on macroquad. See `14-main-loop.md §3`.
//!
//! The frame algorithm itself lives in `ember_core::main_loop::FrameStepper`
//! (tested headlessly there); this file only adds the window, the input
//! snapshot and the frame pacing.

use crate::input::poll_input;
use ember_core::{FrameStepper, Image, MainLoop};
use ember_input::InputSink;
use macroquad::Window;
use macroquad::input::{is_quit_requested, prevent_quit};
use macroquad::miniquad::conf::Icon;
use macroquad::time::get_frame_time;
use macroquad::window::{Conf, next_frame, screen_height, screen_width};

/// Run `game` inside an already-open macroquad context.
pub async fn run<L: MainLoop + InputSink>(game: L) {
    run_inner(game, false).await;
}

async fn run_inner<L: MainLoop + InputSink>(mut game: L, confirm_close: bool) {
    if confirm_close {
        prevent_quit();
    }
    if !game.init() {
        return;
    }
    let mut stepper = FrameStepper::new();
    let mut window = (0.0, 0.0);
    loop {
        let size = (screen_width(), screen_height());
        if size != window {
            window = size;
            game.resize(size.0, size.1);
        }
        let dt = get_frame_time();
        let mut input = poll_input();
        input.close_requested = confirm_close && is_quit_requested();
        game.set_input(input);
        if !stepper.frame(&mut game, dt) {
            break;
        }
        next_frame().await;
    }
    game.finalize();
}

/// Open a window and run `game` until `process` returns `false`.
pub fn run_window<L: MainLoop + InputSink + 'static>(
    title: &str,
    width: i32,
    height: i32,
    game: L,
) {
    run_window_with(WindowConfig::new(title, width, height), game);
}

/// The window's icon at the three sizes the platforms ask for: 16, 32
/// and 64 pixels square, RGBA.
#[derive(Debug, Clone)]
pub struct WindowIcon {
    pub small: Image,
    pub medium: Image,
    pub big: Image,
}

/// How `run_window_with` opens the window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: i32,
    pub height: i32,
    /// Defaults to `true`, as macroquad's own default.
    pub resizable: bool,
    /// Draw at the screen's full pixel density (Retina). Sizes, the
    /// viewport and the mouse stay in logical pixels either way.
    pub high_dpi: bool,
    pub icon: Option<WindowIcon>,
    /// Keep the window open when its close button is pressed and report
    /// it as `Input::close_requested`; the game quits itself (ADR-045).
    pub confirm_close: bool,
}

impl WindowConfig {
    pub fn new(title: &str, width: i32, height: i32) -> Self {
        Self {
            title: title.to_owned(),
            width,
            height,
            resizable: true,
            high_dpi: false,
            icon: None,
            confirm_close: false,
        }
    }

    pub fn confirm_close(mut self, on: bool) -> Self {
        self.confirm_close = on;
        self
    }

    pub fn high_dpi(mut self, on: bool) -> Self {
        self.high_dpi = on;
        self
    }

    pub fn resizable(mut self, on: bool) -> Self {
        self.resizable = on;
        self
    }

    pub fn with_icon(mut self, icon: WindowIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The macroquad configuration. An icon whose images are not 16, 32
    /// and 64 pixels square is left out, with a warning.
    pub fn to_conf(&self) -> Conf {
        Conf {
            window_title: self.title.clone(),
            window_width: self.width,
            window_height: self.height,
            window_resizable: self.resizable,
            high_dpi: self.high_dpi,
            icon: self.icon.as_ref().and_then(|icon| {
                let converted = to_icon(icon);
                if converted.is_none() {
                    eprintln!("⚠️  window icon ignored: images must be 16, 32 and 64 px square");
                }
                converted
            }),
            ..Conf::default()
        }
    }
}

fn to_icon(icon: &WindowIcon) -> Option<Icon> {
    fn copy<const N: usize>(image: &Image, side: u32, out: &mut [u8; N]) -> Option<()> {
        (image.width() == side && image.height() == side).then_some(())?;
        out.copy_from_slice(image.as_bytes());
        Some(())
    }
    let mut out = Icon {
        small: [0; 16 * 16 * 4],
        medium: [0; 32 * 32 * 4],
        big: [0; 64 * 64 * 4],
    };
    copy(&icon.small, 16, &mut out.small)?;
    copy(&icon.medium, 32, &mut out.medium)?;
    copy(&icon.big, 64, &mut out.big)?;
    Some(out)
}

/// `run_window` with a full `WindowConfig` (icon, high DPI, resizing).
pub fn run_window_with<L: MainLoop + InputSink + 'static>(config: WindowConfig, game: L) {
    let confirm = config.confirm_close;
    Window::from_config(config.to_conf(), run_inner(game, confirm));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(side: u32) -> Image {
        Image::new(side, side)
    }

    #[test]
    fn the_defaults_match_the_old_run_window() {
        let conf = WindowConfig::new("T", 800, 600).to_conf();
        assert_eq!(conf.window_title, "T");
        assert_eq!((conf.window_width, conf.window_height), (800, 600));
        assert!(conf.window_resizable);
        assert!(!conf.high_dpi);
        assert!(conf.icon.is_none());
    }

    #[test]
    fn options_reach_the_conf() {
        let mut small = square(16);
        small.set_pixel(0, 0, ember_core::Color::WHITE);
        let icon = WindowIcon {
            small,
            medium: square(32),
            big: square(64),
        };
        let conf = WindowConfig::new("T", 1, 1)
            .high_dpi(true)
            .resizable(false)
            .with_icon(icon)
            .to_conf();
        assert!(conf.high_dpi);
        assert!(!conf.window_resizable);
        let icon = conf.icon.expect("icon kept");
        assert_eq!(&icon.small[..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn a_wrongly_sized_icon_is_left_out() {
        let icon = WindowIcon {
            small: square(16),
            medium: square(16),
            big: square(64),
        };
        let conf = WindowConfig::new("T", 1, 1).with_icon(icon).to_conf();
        assert!(conf.icon.is_none());
    }
}
