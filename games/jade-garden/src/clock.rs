//! Seconds since the game started, for drawing-only animation phases
//! (pulses, glows). It replaces macroquad's global `get_time`: the root
//! node advances it once per frame in `process`, and drawing reads it.
//! Game logic never reads it; it keeps its own timers.

use std::cell::Cell;

thread_local! {
    static NOW: Cell<f64> = const { Cell::new(0.0) };
}

/// Seconds since start (was `macroquad::time::get_time`).
pub fn time() -> f64 {
    NOW.with(Cell::get)
}

/// Advance the clock by one frame.
pub fn advance(dt: f32) {
    NOW.with(|now| now.set(now.get() + dt as f64));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_accumulates() {
        let start = time();
        advance(0.5);
        advance(0.25);
        assert!((time() - start - 0.75).abs() < 1e-9);
    }
}
