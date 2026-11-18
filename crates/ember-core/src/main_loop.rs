//! The `MainLoop` trait. See `14-main-loop.md §2`.
//!
//! This is only the contract. The runner (`ember::run`) lives in the
//! facade crate because it drives a backend.

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
}
