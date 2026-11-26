//! The runner: drives a `MainLoop` on macroquad. See `14-main-loop.md §3`.
//!
//! The frame algorithm itself lives in `ember_core::main_loop::FrameStepper`
//! (tested headlessly there); this file only adds the window, the input
//! snapshot and the frame pacing.

use crate::input::poll_input;
use ember_core::{FrameStepper, MainLoop};
use ember_input::InputSink;
use macroquad::Window;
use macroquad::time::get_frame_time;
use macroquad::window::{Conf, next_frame, screen_height, screen_width};

/// Run `game` inside an already-open macroquad context.
pub async fn run<L: MainLoop + InputSink>(mut game: L) {
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
        game.set_input(poll_input());
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
    let conf = Conf {
        window_title: title.to_owned(),
        window_width: width,
        window_height: height,
        ..Conf::default()
    };
    Window::from_config(conf, run(game));
}
