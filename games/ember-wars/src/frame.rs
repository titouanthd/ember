//! This frame's window size, pointer, clock and frame time, as
//! macroquad's global `screen_width` / `screen_height` /
//! `mouse_position` / `get_time` / `get_frame_time` gave them. The root node sets them once per frame in `process`
//! (`frame::begin`); the layout helpers (shared by hit tests and
//! drawing) and the hover effects read them.

use std::cell::Cell;

use glam::Vec2;

#[derive(Clone, Copy)]
struct Frame {
    size: Vec2,
    mouse: Vec2,
    time: f64,
    dt: f32,
}

thread_local! {
    static FRAME: Cell<Frame> = const {
        Cell::new(Frame {
            size: Vec2::new(1600.0, 900.0),
            mouse: Vec2::ZERO,
            time: 0.0,
            dt: 0.0,
        })
    };
}

/// Start a frame: the window `size`, the pointer, and `dt` more seconds.
pub fn begin(size: Vec2, mouse: Vec2, dt: f32) {
    FRAME.with(|f| {
        let time = f.get().time + dt as f64;
        f.set(Frame {
            size,
            mouse,
            time,
            dt,
        });
    });
}

pub fn screen_width() -> f32 {
    FRAME.with(|f| f.get().size.x)
}

pub fn screen_height() -> f32 {
    FRAME.with(|f| f.get().size.y)
}

/// The pointer as `(x, y)`, as macroquad returned it.
pub fn mouse_position() -> (f32, f32) {
    FRAME.with(|f| {
        let m = f.get().mouse;
        (m.x, m.y)
    })
}

/// This frame's duration in seconds (uncapped).
pub fn get_frame_time() -> f32 {
    FRAME.with(|f| f.get().dt)
}

/// Seconds since start.
pub fn get_time() -> f64 {
    FRAME.with(|f| f.get().time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_sets_the_frame_and_advances_the_clock() {
        let t0 = get_time();
        begin(Vec2::new(1000.0, 700.0), Vec2::new(3.0, 4.0), 0.5);
        assert_eq!((screen_width(), screen_height()), (1000.0, 700.0));
        assert_eq!(mouse_position(), (3.0, 4.0));
        assert!((get_time() - t0 - 0.5).abs() < 1e-9);
    }
}
