//! Phase 0 exit criterion (`40-roadmap.md` §3.3): a headless loop verifies
//! the `init` / `process` / `draw` / `finalize` order, plus the fixed-step
//! and `MAX_FRAME_DT` rules of `14-main-loop.md` §3–4.

use ember_core::main_loop::{
    FIXED_DT, FrameStepper, HeadlessRun, MAX_FRAME_DT, MainLoop, run_headless,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Call {
    Init,
    Physics,
    Process,
    Draw,
    Finalize,
}

/// Records every callback. `quit_after` makes the n-th `process` return
/// `false`; `refuse_init` makes `init` return `false`.
#[derive(Default)]
struct Recorder {
    calls: Vec<Call>,
    process_dts: Vec<f32>,
    quit_after: Option<usize>,
    refuse_init: bool,
}

impl Recorder {
    fn count(&self, c: Call) -> usize {
        self.calls.iter().filter(|x| **x == c).count()
    }
}

impl MainLoop for Recorder {
    fn init(&mut self) -> bool {
        self.calls.push(Call::Init);
        !self.refuse_init
    }
    fn process(&mut self, dt: f32) -> bool {
        self.calls.push(Call::Process);
        self.process_dts.push(dt);
        self.quit_after != Some(self.process_dts.len())
    }
    fn physics_process(&mut self, fixed_dt: f32) {
        assert_eq!(fixed_dt, FIXED_DT);
        self.calls.push(Call::Physics);
    }
    fn draw(&mut self) {
        self.calls.push(Call::Draw);
    }
    fn finalize(&mut self) {
        self.calls.push(Call::Finalize);
    }
}

#[test]
fn lifecycle_order_init_frames_finalize() {
    use Call::*;
    let mut g = Recorder::default();
    // Frame 1: no physics yet. Frame 2: one physics step.
    let run = run_headless(&mut g, [FIXED_DT * 0.5, FIXED_DT * 0.75]);
    assert_eq!(
        run,
        HeadlessRun {
            frames: 2,
            quit: false,
            started: true
        }
    );
    assert_eq!(
        g.calls,
        vec![Init, Process, Draw, Physics, Process, Draw, Finalize]
    );
}

#[test]
fn physics_runs_before_process_within_a_frame() {
    use Call::*;
    let mut g = Recorder::default();
    run_headless(&mut g, [FIXED_DT * 2.5]);
    assert_eq!(
        g.calls,
        vec![Init, Physics, Physics, Process, Draw, Finalize]
    );
}

#[test]
fn quitting_skips_draw_and_still_finalizes() {
    use Call::*;
    let mut g = Recorder {
        quit_after: Some(2),
        ..Default::default()
    };
    let run = run_headless(&mut g, [0.001, 0.001, 0.001, 0.001]);
    assert_eq!(
        run,
        HeadlessRun {
            frames: 2,
            quit: true,
            started: true
        }
    );
    assert_eq!(g.calls, vec![Init, Process, Draw, Process, Finalize]);
}

#[test]
fn refused_init_calls_nothing_else() {
    let mut g = Recorder {
        refuse_init: true,
        ..Default::default()
    };
    let run = run_headless(&mut g, [0.1; 10]);
    assert!(!run.started);
    assert_eq!(run.frames, 0);
    assert_eq!(g.calls, vec![Call::Init]);
}

#[test]
fn long_frame_is_clamped_to_max_frame_dt() {
    let mut g = Recorder::default();
    run_headless(&mut g, [5.0]);
    assert_eq!(g.process_dts, vec![MAX_FRAME_DT]);
    // 0.25 s at 60 Hz is 15 steps; allow one step of float slack.
    let steps = g.count(Call::Physics);
    assert!((14..=15).contains(&steps), "steps = {steps}");
}

#[test]
fn physics_rate_is_independent_of_frame_rate() {
    for fps in [30.0_f32, 60.0, 144.0] {
        let mut g = Recorder::default();
        let frames = fps as usize * 2; // two seconds
        run_headless(&mut g, std::iter::repeat_n(1.0 / fps, frames));
        let steps = g.count(Call::Physics);
        assert!((119..=120).contains(&steps), "{fps} fps: {steps} steps");
        assert_eq!(g.count(Call::Process), frames);
        assert_eq!(g.count(Call::Draw), frames);
    }
}

#[test]
fn stepper_is_reusable_across_frames() {
    let mut g = Recorder::default();
    let mut s = FrameStepper::new();
    for _ in 0..3 {
        assert!(s.frame(&mut g, FIXED_DT * 0.5));
    }
    assert_eq!(g.count(Call::Physics), 1);
}
