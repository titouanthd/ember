//! The runner: drives a `MainLoop` on macroquad. See `14-main-loop.md §3`.

use crate::input::poll_input;
use ember_core::MainLoop;
use ember_core::main_loop::{FIXED_DT, MAX_FRAME_DT};
use ember_input::InputSink;
use macroquad::Window;
use macroquad::time::get_frame_time;
use macroquad::window::{Conf, next_frame};

/// Run `game` inside an already-open macroquad context.
pub async fn run<L: MainLoop + InputSink>(mut game: L) {
    if !game.init() {
        return;
    }
    let mut accumulator = 0.0;
    loop {
        let dt = get_frame_time().min(MAX_FRAME_DT);
        game.set_input(poll_input());

        accumulator += dt;
        while accumulator >= FIXED_DT {
            game.physics_process(FIXED_DT);
            accumulator -= FIXED_DT;
        }
        if !game.process(dt) {
            break;
        }
        game.draw();
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
