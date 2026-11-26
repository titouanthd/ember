//! The `MainLoop` trait and the backend-agnostic frame driver.
//! See `14-main-loop.md`.
//!
//! - [`MainLoop`] is the contract a game (or `ember_scene::SceneLoop`)
//!   implements.
//! - [`FrameStepper`] is the one implementation of the frame algorithm
//!   (`14-main-loop.md §3–4`): clamp `dt`, run fixed-step
//!   `physics_process` 0..N times, then `process`, then `draw`. Backend
//!   runners (`ember::run`) call it once per displayed frame.
//! - [`run_headless`] drives a whole game lifecycle from a list of frame
//!   times, with no window. Used by tests and tools.

/// Fixed physics tick, in seconds.
pub const FIXED_DT: f32 = 1.0 / 60.0;

/// Upper bound on a frame's `dt`; avoids the spiral of death.
pub const MAX_FRAME_DT: f32 = 0.25;

pub trait MainLoop {
    /// Initialise the game. Return `false` to abort startup.
    fn init(&mut self) -> bool {
        true
    }

    /// Called every frame. Return `false` to quit.
    fn process(&mut self, dt: f32) -> bool;

    /// Called 0..N times per frame at a fixed `FIXED_DT`.
    fn physics_process(&mut self, _fixed_dt: f32) {}

    /// Called once per frame after `process`. Takes `&mut self` because
    /// rendering backends are stateful.
    fn draw(&mut self) {}

    /// Called once before shutdown. Best-effort.
    fn finalize(&mut self) {}

    /// The window's size in pixels: once before the first `process`,
    /// then whenever it changes (ADR-041). Headless runs never call it.
    fn resize(&mut self, _width: f32, _height: f32) {}
}

/// Runs one frame of the fixed-step algorithm. Keeps the physics
/// accumulator between frames, so one stepper lives for a whole run.
#[derive(Debug, Clone, Default)]
pub struct FrameStepper {
    accumulator: f32,
}

impl FrameStepper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Time carried over to the next frame's physics steps, in seconds.
    /// Always in `[0, FIXED_DT)` after a frame.
    pub fn accumulator(&self) -> f32 {
        self.accumulator
    }

    /// Run one frame: `physics_process` 0..N times, then `process`, then
    /// `draw` (skipped when `process` asks to quit).
    ///
    /// `frame_dt` is clamped to `[0, MAX_FRAME_DT]`; a negative or NaN
    /// time counts as `0`. Returns `false` when the game asked to quit.
    pub fn frame<L: MainLoop + ?Sized>(&mut self, game: &mut L, frame_dt: f32) -> bool {
        let dt = if frame_dt > 0.0 {
            frame_dt.min(MAX_FRAME_DT)
        } else {
            0.0
        };
        self.accumulator += dt;
        while self.accumulator >= FIXED_DT {
            game.physics_process(FIXED_DT);
            self.accumulator -= FIXED_DT;
        }
        if !game.process(dt) {
            return false;
        }
        game.draw();
        true
    }
}

/// What a [`run_headless`] call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadlessRun {
    /// Frames started, including the one whose `process` quit.
    pub frames: usize,
    /// `true` if `process` returned `false`; `false` if the frame list ran
    /// out first or `init` refused to start.
    pub quit: bool,
    /// `false` if `init` returned `false` (nothing else was called).
    pub started: bool,
}

/// Drive `game` through a full lifecycle without a window: `init`, one
/// [`FrameStepper::frame`] per entry of `frame_dts`, then `finalize`.
///
/// Stops early when `process` returns `false`. If `init` returns `false`,
/// nothing else is called (not even `finalize`), matching the windowed
/// runner.
pub fn run_headless<L, I>(game: &mut L, frame_dts: I) -> HeadlessRun
where
    L: MainLoop + ?Sized,
    I: IntoIterator<Item = f32>,
{
    if !game.init() {
        return HeadlessRun {
            frames: 0,
            quit: false,
            started: false,
        };
    }
    let mut stepper = FrameStepper::new();
    let mut frames = 0;
    let mut quit = false;
    for dt in frame_dts {
        frames += 1;
        if !stepper.frame(game, dt) {
            quit = true;
            break;
        }
    }
    game.finalize();
    HeadlessRun {
        frames,
        quit,
        started: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Minimal;
    impl MainLoop for Minimal {
        fn process(&mut self, _dt: f32) -> bool {
            false
        }
    }

    #[test]
    fn only_process_is_required() {
        let mut m = Minimal;
        assert!(m.init());
        assert!(!m.process(FIXED_DT));
    }

    #[derive(Default)]
    struct Steps(u32);
    impl MainLoop for Steps {
        fn process(&mut self, _dt: f32) -> bool {
            true
        }
        fn physics_process(&mut self, _fixed_dt: f32) {
            self.0 += 1;
        }
    }

    #[test]
    fn accumulator_carries_between_frames() {
        let mut g = Steps::default();
        let mut s = FrameStepper::new();
        s.frame(&mut g, FIXED_DT * 0.5);
        assert_eq!(g.0, 0);
        s.frame(&mut g, FIXED_DT * 0.6);
        assert_eq!(g.0, 1);
        assert!(s.accumulator() >= 0.0 && s.accumulator() < FIXED_DT);
    }

    #[test]
    fn bad_dt_counts_as_zero() {
        let mut g = Steps::default();
        let mut s = FrameStepper::new();
        for dt in [-1.0, f32::NAN, 0.0] {
            assert!(s.frame(&mut g, dt));
        }
        assert_eq!(g.0, 0);
        assert_eq!(s.accumulator(), 0.0);
    }
}
