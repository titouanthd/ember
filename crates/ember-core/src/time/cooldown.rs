//! Countdown timer — an `f32` that decrements toward 0.
//!
//! Extracted from 4 games (Asteroids, Bullet Hell, Zhuo Ji, ember-wars).
//! Replaces the ubiquitous pattern:
//!
//! ```ignore
//! if cd > 0.0 {
//!     cd = (cd - dt).max(0.0);
//! }
//! if cd <= 0.0 { /* ready */ }
//! ```
//!
//! # Usage
//!
//! Two constructors, with a deliberate distinction:
//!
//! - [`Cooldown::new`] returns a **running** cooldown (starts at `duration`,
//!   `is_active() == true`). This is the common case: "cooldown after shot".
//! - [`Cooldown::with_duration`] returns a **ready** cooldown that remembers
//!   its duration for later [`reset`](Cooldown::reset).
//!
//! ```
//! use ember_core::time::Cooldown;
//!
//! // Ready cooldown, can be triggered/reset later.
//! let mut cd = Cooldown::default();
//! assert!(cd.is_ready());
//!
//! cd.trigger(0.5);
//! assert!(cd.is_active());
//!
//! cd.tick(0.3);
//! assert!(!cd.is_ready());
//!
//! cd.tick(0.3);
//! assert!(cd.is_ready());
//! ```
//!
//! For "just expired" transitions (auto-fire, rebuild completion),
//! use [`Cooldown::tick_returning`]:
//!
//! ```
//! use ember_core::time::Cooldown;
//!
//! let mut cd = Cooldown::new(0.2);  // already running
//! assert!(!cd.tick_returning(0.1)); // still running
//! assert!(cd.tick_returning(0.2));  // just finished
//! assert!(!cd.tick_returning(0.1)); // already ready
//! ```

use serde::{Deserialize, Serialize};

/// A countdown timer. `remaining == 0` means "ready".
///
/// The duration is stored alongside the remaining time so that
/// [`fraction`](Self::fraction) can report progress and
/// [`reset`](Self::reset) can restart without an explicit argument.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Cooldown {
    remaining: f32,
    duration: f32,
}

impl Cooldown {
    /// Create a **running** cooldown with `remaining == duration`.
    ///
    /// Common case: "start a 0.5s cooldown after a shot" → `Cooldown::new(0.5)`.
    ///
    /// For a **ready** cooldown that just remembers its duration, use
    /// [`with_duration`](Self::with_duration).
    pub fn new(duration: f32) -> Self {
        let d = duration.max(0.0);
        Self {
            remaining: d,
            duration: d,
        }
    }

    /// Create a **ready** cooldown with the given duration.
    /// `remaining` starts at 0.
    ///
    /// Use this when you want a cooldown that starts ready but knows its
    /// future duration, so [`reset`](Self::reset) works without an argument.
    pub fn with_duration(duration: f32) -> Self {
        Self {
            remaining: 0.0,
            duration: duration.max(0.0),
        }
    }

    /// Create a cooldown already running with `remaining` seconds left.
    /// `duration` is the reference value for `fraction()`.
    ///
    /// Invariant garanti : `remaining <= duration`. Si `remaining > duration`
    /// est passé, `duration` est **rallongée** pour l'absorber (cas typique :
    /// "on re-trigger avec une durée plus longue"). `remaining < 0` est
    /// clampé à 0.
    pub fn from_remaining(remaining: f32, duration: f32) -> Self {
        let remaining = remaining.max(0.0);
        let duration = duration.max(remaining);
        Self {
            remaining,
            duration,
        }
    }

    /// Create a ready cooldown with no duration.
    pub fn ready() -> Self {
        Self::default()
    }

    /// Advance by `dt` seconds. Never goes below 0.
    /// Negative `dt` is ignored (safety against FPS spikes with weird sign).
    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        if self.remaining > 0.0 {
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }

    /// Advance by `dt` and return `true` if the cooldown **finished during
    /// this tick** (transition from active → ready).
    ///
    /// Returns `false` if the cooldown was already ready, or if `dt <= 0.0`
    /// (safety against FPS spikes with weird sign — matches [`tick`](Self::tick)).
    ///
    /// This is the safe way to trigger one-shot side effects (auto-fire,
    /// rebuild completion, turn timeout).
    pub fn tick_returning(&mut self, dt: f32) -> bool {
        if self.remaining <= 0.0 || dt <= 0.0 {
            return false;
        }
        self.remaining -= dt;
        if self.remaining <= 0.0 {
            self.remaining = 0.0;
            true
        } else {
            false
        }
    }

    /// Start the cooldown with the given `duration`.
    /// Replaces any previous duration.
    pub fn trigger(&mut self, duration: f32) {
        self.duration = duration.max(0.0);
        self.remaining = self.duration;
    }

    /// Restart the cooldown with its **stored** duration.
    /// No-op if the stored duration is 0 (nothing to restart).
    pub fn reset(&mut self) {
        self.remaining = self.duration;
    }

    /// Force the cooldown to finish immediately.
    /// `duration` is preserved.
    pub fn clear(&mut self) {
        self.remaining = 0.0;
    }

    /// True if `remaining <= 0`. A `Default` cooldown is ready.
    pub fn is_ready(&self) -> bool {
        self.remaining <= 0.0
    }

    /// Opposite of [`is_ready`](Self::is_ready).
    pub fn is_active(&self) -> bool {
        self.remaining > 0.0
    }

    /// Time remaining in seconds (>= 0).
    pub fn remaining(&self) -> f32 {
        self.remaining.max(0.0)
    }

    /// Duration of the last [`trigger`](Self::trigger).
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Fraction **elapsed** since `trigger`, in `[0, 1]`.
    /// 0 = just triggered, 1 = ready.
    /// Handles `duration == 0` by returning 1 (already done).
    pub fn fraction(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (1.0 - self.remaining / self.duration).clamp(0.0, 1.0)
        }
    }

    /// Fraction **remaining**, in `[0, 1]`.
    /// 1 = just triggered, 0 = ready. Useful for cooldown overlays
    /// (draw a shrinking bar or a growing shadow).
    pub fn remaining_fraction(&self) -> f32 {
        if self.duration <= 0.0 {
            0.0
        } else {
            (self.remaining / self.duration).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ready() {
        let cd = Cooldown::default();
        assert!(cd.is_ready());
        assert!(!cd.is_active());
        assert_eq!(cd.remaining(), 0.0);
        assert_eq!(cd.duration(), 0.0);
    }

    #[test]
    fn new_starts_active() {
        let cd = Cooldown::new(0.5);
        assert!(cd.is_active());
        assert!(!cd.is_ready());
        assert_eq!(cd.remaining(), 0.5);
        assert_eq!(cd.duration(), 0.5);
    }

    #[test]
    fn new_with_zero_duration_is_ready() {
        let cd = Cooldown::new(0.0);
        assert!(cd.is_ready());
        assert_eq!(cd.duration(), 0.0);
        assert_eq!(cd.remaining(), 0.0);
    }

    #[test]
    fn new_with_negative_duration_is_ready() {
        let cd = Cooldown::new(-1.0);
        assert!(cd.is_ready());
        assert_eq!(cd.duration(), 0.0);
        assert_eq!(cd.remaining(), 0.0);
    }

    #[test]
    fn with_duration_is_ready() {
        let cd = Cooldown::with_duration(0.5);
        assert!(cd.is_ready());
        assert!(!cd.is_active());
        assert_eq!(cd.remaining(), 0.0);
        assert_eq!(cd.duration(), 0.5);
    }

    #[test]
    fn with_duration_clamps_negative_duration() {
        let cd = Cooldown::with_duration(-1.0);
        assert_eq!(cd.duration(), 0.0);
        assert!(cd.is_ready());
    }

    #[test]
    fn with_duration_then_reset_starts_cooldown() {
        let mut cd = Cooldown::with_duration(0.5);
        assert!(cd.is_ready());
        cd.reset();
        assert!(cd.is_active());
        assert_eq!(cd.remaining(), 0.5);
    }

    #[test]
    fn trigger_starts_cooldown() {
        let mut cd = Cooldown::default();
        cd.trigger(0.5);
        assert!(cd.is_active());
        assert_eq!(cd.remaining(), 0.5);
        assert_eq!(cd.duration(), 0.5);
    }

    #[test]
    fn tick_reduces_remaining() {
        let mut cd = Cooldown::new(0.5);
        cd.tick(0.2);
        assert!((cd.remaining() - 0.3).abs() < 1e-6);
    }

    #[test]
    fn tick_clamps_at_zero() {
        let mut cd = Cooldown::new(0.5);
        cd.tick(10.0);
        assert_eq!(cd.remaining(), 0.0);
        assert!(cd.is_ready());
    }

    #[test]
    fn tick_ignores_negative_dt() {
        let mut cd = Cooldown::new(0.5);
        cd.tick(-1.0);
        assert!((cd.remaining() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn tick_on_ready_is_noop() {
        let mut cd = Cooldown::default();
        cd.tick(1.0);
        assert!(cd.is_ready());
        assert_eq!(cd.remaining(), 0.0);
    }

    #[test]
    fn tick_returning_false_while_running() {
        let mut cd = Cooldown::new(0.5);
        assert!(!cd.tick_returning(0.2));
        assert!(!cd.tick_returning(0.2));
    }

    #[test]
    fn tick_returning_true_on_expiry() {
        let mut cd = Cooldown::new(0.5);
        assert!(!cd.tick_returning(0.3));
        assert!(cd.tick_returning(0.3));
        assert!(cd.is_ready());
    }

    #[test]
    fn tick_returning_false_when_already_ready() {
        let mut cd = Cooldown::default();
        assert!(!cd.tick_returning(1.0));
        assert!(!cd.tick_returning(1.0));
    }

    #[test]
    fn tick_returning_fires_exactly_once() {
        let mut cd = Cooldown::new(0.1);
        let mut count = 0;
        for _ in 0..10 {
            if cd.tick_returning(0.05) {
                count += 1;
            }
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn reset_restarts_with_stored_duration() {
        let mut cd = Cooldown::new(0.5);
        cd.tick(0.3);
        cd.reset();
        assert!((cd.remaining() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn reset_on_default_is_noop() {
        let mut cd = Cooldown::default();
        cd.reset();
        assert!(cd.is_ready());
    }

    #[test]
    fn clear_forces_ready() {
        let mut cd = Cooldown::new(10.0);
        cd.clear();
        assert!(cd.is_ready());
        assert_eq!(cd.duration(), 10.0); // duration preserved
    }

    #[test]
    fn fraction_zero_right_after_new() {
        let cd = Cooldown::new(1.0);
        assert!((cd.fraction() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn fraction_one_when_ready() {
        let mut cd = Cooldown::new(1.0);
        cd.tick(1.0);
        assert!((cd.fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fraction_half_at_midpoint() {
        let mut cd = Cooldown::new(1.0);
        cd.tick(0.5);
        assert!((cd.fraction() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn fraction_one_when_duration_zero() {
        let cd = Cooldown::default();
        assert!((cd.fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn remaining_fraction_one_right_after_new() {
        let cd = Cooldown::new(1.0);
        assert!((cd.remaining_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn remaining_fraction_zero_when_ready() {
        let cd = Cooldown::default();
        assert!((cd.remaining_fraction() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn remaining_fraction_half_at_midpoint() {
        let mut cd = Cooldown::new(1.0);
        cd.tick(0.5);
        assert!((cd.remaining_fraction() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn from_remaining_sets_both_fields() {
        let cd = Cooldown::from_remaining(0.3, 0.5);
        assert!((cd.remaining() - 0.3).abs() < 1e-6);
        assert!((cd.duration() - 0.5).abs() < 1e-6);
        assert!(cd.is_active());
    }

    #[test]
    fn from_remaining_clamps_negative() {
        let cd = Cooldown::from_remaining(-1.0, 0.5);
        assert_eq!(cd.remaining(), 0.0);
        assert!(cd.is_ready());
    }

    #[test]
    fn from_remaining_extends_duration_when_remaining_exceeds() {
        // remaining > duration : duration est rallongée pour garder
        // l'invariant remaining <= duration.
        let cd = Cooldown::from_remaining(1.0, 0.5);
        assert!((cd.remaining() - 1.0).abs() < 1e-6);
        assert!((cd.duration() - 1.0).abs() < 1e-6);
        // Conséquence : fraction() = 0 (juste trigger), remaining_fraction() = 1.
        assert!((cd.fraction() - 0.0).abs() < 1e-6);
        assert!((cd.remaining_fraction() - 1.0).abs() < 1e-6);
        // Cohérence : impossible d'avoir remaining > duration après.
        assert!(cd.remaining() <= cd.duration() + 1e-6);
    }

    #[test]
    fn from_remaining_zero_duration_extends_to_remaining() {
        // Cas "durée inconnue mais reste n secondes" : duration = remaining.
        let cd = Cooldown::from_remaining(0.5, 0.0);
        assert!((cd.remaining() - 0.5).abs() < 1e-6);
        assert!((cd.duration() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn trigger_replaces_duration() {
        let mut cd = Cooldown::default();
        cd.trigger(0.5);
        cd.trigger(1.0);
        assert_eq!(cd.duration(), 1.0);
        assert_eq!(cd.remaining(), 1.0);
    }

    #[test]
    fn trigger_with_zero_duration_is_immediately_ready() {
        let mut cd = Cooldown::default();
        cd.trigger(0.0);
        assert!(cd.is_ready());
    }

    #[test]
    fn serde_roundtrip() {
        let mut cd = Cooldown::new(0.7);
        cd.tick(0.2);
        let s = ron::to_string(&cd).unwrap();
        let back: Cooldown = ron::from_str(&s).unwrap();
        assert_eq!(cd, back);
    }

    #[test]
    fn tick_returning_ignores_negative_dt() {
        let mut cd = Cooldown::new(0.5);
        // Negative dt must not advance the cooldown (and must not increase it).
        assert!(!cd.tick_returning(-1.0));
        assert!((cd.remaining() - 0.5).abs() < 1e-6);
        // Still running normally after the bogus call.
        assert!(!cd.tick_returning(0.2));
        assert!(cd.tick_returning(0.4));
    }
}