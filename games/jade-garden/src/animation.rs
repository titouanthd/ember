//! Tweens and easings for tile and HUD animations.
//!
//! Each `Tween` interpolates an `f32` value from a start point to an
//! end point over a fixed duration. It is updated via `tick(dt)`,
//! which returns `true` when the duration has elapsed.
//!
//! **Architecture note**: this module is local to jade-garden. It is a
//! candidate for extraction into `ember_stdlib::animation` once a 2nd
//! game uses it (Rule of Three).
//!
//! No `macroquad` dependency — everything is testable headless.

/// Interpolation curve applied to raw progress `t ∈ [0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    /// `f(t) = t`. Constant speed.
    Linear,
    /// `f(t) = t²` — starts slowly, accelerates.
    EaseIn,
    /// `f(t) = 1 - (1-t)²` — starts fast, decelerates. Ideal for falls
    /// and appearances.
    EaseOut,
    /// Smoothstep — `f(t) = t² * (3 - 2t)`. Starts slow, accelerates,
    /// then decelerates. The most natural for swaps.
    EaseInOut,
    /// Damped bounce — the value reaches its target then oscillates
    /// before settling. Used for jade falls.
    Bounce,
    /// Overshoot then return. Used for tile selection.
    Elastic,
}

impl Easing {
    /// Applies the easing to raw progress `t ∈ [0, 1]`.
    ///
    /// Guarantees `apply(0) == 0` and `apply(1) == 1` for all variants
    /// (tested property). Intermediate values may leave `[0, 1]` only
    /// for `Bounce` and `Elastic`.
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => t * t * (3.0 - 2.0 * t),
            Easing::Bounce => bounce_out(t),
            Easing::Elastic => elastic_out(t),
        }
    }
}

/// Classic damped bounce (Robert Penner).
fn bounce_out(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Overshoot then return (Robert Penner).
fn elastic_out(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

/// A tween interpolates `from` → `to` over `duration` seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: Easing,
}

impl Tween {
    /// Creates a tween starting at `from`, ending at `to`, over `duration`.
    pub fn new(from: f32, to: f32, duration: f32, easing: Easing) -> Self {
        Self {
            from,
            to,
            duration: duration.max(1e-4),
            elapsed: 0.0,
            easing,
        }
    }

    /// Immediately completed tween (zero duration → value == to).
    pub fn immediate(to: f32) -> Self {
        Self {
            from: to,
            to,
            duration: 1e-4,
            elapsed: 1e-4,
            easing: Easing::Linear,
        }
    }

    /// Advances the tween by `dt` seconds.
    ///
    /// Returns `true` if the tween reached (or exceeded) its duration
    /// **during this tick**, i.e. it just finished. Returns `false`
    /// if it was already finished before.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.is_done() {
            return false;
        }
        self.elapsed += dt;
        self.is_done()
    }

    /// True if the duration has elapsed.
    pub fn is_done(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Raw progress in `[0, 1]` (without easing applied).
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Current value with easing applied.
    pub fn value(&self) -> f32 {
        let t = self.easing.apply(self.progress());
        self.from + (self.to - self.from) * t
    }

    /// Restarts the tween from zero with a new target.
    pub fn retarget(&mut self, to: f32, duration: f32) {
        self.from = self.value();
        self.to = to;
        self.duration = duration.max(1e-4);
        self.elapsed = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Easing::apply — general properties ----------

    #[test]
    fn all_easings_apply_zero_is_zero() {
        for e in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::Bounce,
            Easing::Elastic,
        ] {
            let v = e.apply(0.0);
            assert!(v.abs() < 1e-3, "{e:?}.apply(0) = {v}");
        }
    }

    #[test]
    fn all_easings_apply_one_is_one() {
        for e in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::Bounce,
            Easing::Elastic,
        ] {
            let v = e.apply(1.0);
            assert!((v - 1.0).abs() < 1e-3, "{e:?}.apply(1) = {v}");
        }
    }

    #[test]
    fn apply_clamps_input() {
        // Input outside [0, 1] must be clamped.
        assert_eq!(Easing::Linear.apply(-0.5), 0.0);
        assert_eq!(Easing::Linear.apply(1.5), 1.0);
        assert_eq!(Easing::EaseInOut.apply(2.0), 1.0);
        assert_eq!(Easing::EaseInOut.apply(-1.0), 0.0);
    }

    // ---------- Easing::Linear ----------

    #[test]
    fn linear_is_identity() {
        assert!((Easing::Linear.apply(0.0) - 0.0).abs() < 1e-6);
        assert!((Easing::Linear.apply(0.25) - 0.25).abs() < 1e-6);
        assert!((Easing::Linear.apply(0.5) - 0.5).abs() < 1e-6);
        assert!((Easing::Linear.apply(0.75) - 0.75).abs() < 1e-6);
        assert!((Easing::Linear.apply(1.0) - 1.0).abs() < 1e-6);
    }

    // ---------- Easing::EaseIn ----------

    #[test]
    fn ease_in_below_linear_at_midpoint() {
        // EaseIn = t² → at t=0.5, value 0.25 < 0.5.
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
    }

    #[test]
    fn ease_in_is_monotonic() {
        let mut prev = 0.0;
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let v = Easing::EaseIn.apply(t);
            assert!(v >= prev - 1e-6);
            prev = v;
        }
    }

    // ---------- Easing::EaseOut ----------

    #[test]
    fn ease_out_above_linear_at_midpoint() {
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
    }

    #[test]
    fn ease_out_is_monotonic() {
        let mut prev = 0.0;
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let v = Easing::EaseOut.apply(t);
            assert!(v >= prev - 1e-6);
            prev = v;
        }
    }

    // ---------- Easing::EaseInOut ----------

    #[test]
    fn ease_in_out_is_symmetric() {
        // Smoothstep: apply(0.5) = 0.5.
        assert!((Easing::EaseInOut.apply(0.5) - 0.5).abs() < 1e-6);
        // Symmetry: f(t) + f(1-t) = 1.
        for i in 1..10 {
            let t = i as f32 / 10.0;
            let a = Easing::EaseInOut.apply(t);
            let b = Easing::EaseInOut.apply(1.0 - t);
            assert!((a + b - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn ease_in_out_is_monotonic() {
        let mut prev = 0.0;
        for i in 0..=50 {
            let t = i as f32 / 50.0;
            let v = Easing::EaseInOut.apply(t);
            assert!(v >= prev - 1e-6);
            prev = v;
        }
    }

    // ---------- Easing::Bounce ----------

    #[test]
    fn bounce_endpoints_are_clean() {
        assert!(Easing::Bounce.apply(0.0).abs() < 1e-3);
        assert!((Easing::Bounce.apply(1.0) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn bounce_stays_in_range() {
        for i in 0..=50 {
            let t = i as f32 / 50.0;
            let v = Easing::Bounce.apply(t);
            assert!((0.0..=1.0).contains(&v), "bounce({t}) = {v} out of [0,1]");
        }
    }

    // ---------- Easing::Elastic ----------

    #[test]
    fn elastic_endpoints_are_clean() {
        assert_eq!(Easing::Elastic.apply(0.0), 0.0);
        assert_eq!(Easing::Elastic.apply(1.0), 1.0);
    }

    #[test]
    fn elastic_overshoots_above_one_somewhere() {
        // Elastic must exceed 1.0 before settling.
        let mut overshot = false;
        for i in 0..100 {
            let t = i as f32 / 100.0;
            if Easing::Elastic.apply(t) > 1.01 {
                overshot = true;
                break;
            }
        }
        assert!(overshot, "Elastic should overshoot");
    }

    // ---------- Tween ----------

    #[test]
    fn tween_new_is_at_start() {
        let t = Tween::new(0.0, 10.0, 1.0, Easing::Linear);
        assert!(!t.is_done());
        assert_eq!(t.elapsed, 0.0);
        assert!((t.value() - 0.0).abs() < 1e-6);
        assert!((t.progress() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn tween_reaches_target_at_end() {
        let mut t = Tween::new(0.0, 10.0, 1.0, Easing::Linear);
        t.tick(1.0);
        assert!(t.is_done());
        assert!((t.value() - 10.0).abs() < 1e-4);
        assert!((t.progress() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn tween_interpolates_linearly() {
        let mut t = Tween::new(0.0, 10.0, 1.0, Easing::Linear);
        t.tick(0.5);
        assert!((t.value() - 5.0).abs() < 1e-4);
        assert!((t.progress() - 0.5).abs() < 1e-4);
    }

    #[test]
    fn tween_interpolates_with_easing() {
        let mut t = Tween::new(0.0, 10.0, 1.0, Easing::EaseIn);
        t.tick(0.5);
        // EaseIn(0.5) = 0.25 → value = 2.5.
        assert!((t.value() - 2.5).abs() < 1e-4);
    }

    #[test]
    fn tween_tick_returns_true_on_completion() {
        let mut t = Tween::new(0.0, 10.0, 1.0, Easing::Linear);
        assert!(!t.tick(0.5));
        assert!(t.tick(0.5));
        // Once finished, tick returns false.
        assert!(!t.tick(0.5));
    }

    #[test]
    fn tween_clamps_elapsed_to_duration() {
        let mut t = Tween::new(0.0, 10.0, 1.0, Easing::Linear);
        t.tick(5.0);
        assert!(t.is_done());
        assert!((t.value() - 10.0).abs() < 1e-4);
    }

    #[test]
    fn tween_negative_from() {
        let mut t = Tween::new(-5.0, 5.0, 1.0, Easing::Linear);
        t.tick(0.5);
        assert!((t.value() - 0.0).abs() < 1e-4);
    }

    #[test]
    fn tween_retarget_starts_from_current_value() {
        let mut t = Tween::new(0.0, 10.0, 1.0, Easing::Linear);
        t.tick(0.5); // value = 5
        t.retarget(20.0, 1.0);
        assert!((t.from - 5.0).abs() < 1e-4);
        assert_eq!(t.to, 20.0);
        assert_eq!(t.elapsed, 0.0);
        // After a 0.5 tick, we should be halfway between 5 and 20 = 12.5.
        t.tick(0.5);
        assert!((t.value() - 12.5).abs() < 1e-4);
    }

    #[test]
    fn tween_immediate_is_done() {
        let t = Tween::immediate(42.0);
        assert!(t.is_done());
        assert!((t.value() - 42.0).abs() < 1e-3);
    }

    #[test]
    fn tween_handles_zero_duration_safely() {
        let mut t = Tween::new(0.0, 10.0, 0.0, Easing::Linear);
        // Duration clamped to 1e-4.
        assert!(t.duration >= 1e-4);
        t.tick(1e-3);
        assert!(t.is_done());
    }

    #[test]
    fn tween_negative_dt_does_not_crash() {
        let mut t = Tween::new(0.0, 10.0, 1.0, Easing::Linear);
        let done_before = t.is_done();
        t.tick(-0.5);
        // Should not break anything.
        assert_eq!(t.is_done(), done_before || t.elapsed >= t.duration);
    }
}