//! Small shared utilities.
//!
//! `hash_u32` re-exported from `ember_core::hash` (extracted Session 10
//! after ember-wars and jade-garden had 3+ verbatim copies).
//!
//! `fmt_time` stays local — the M:SS format is specific to this game's
//! best-time display (`ember_stdlib::ui::TimerDisplay::format` uses
//! MM:SS for HUD timers, a different convention).

pub use ember_core::hash::hash_u32;

/// Formats a duration in seconds as `M:SS` (no leading zero on the
/// minutes). Rounds to the nearest second.
///
/// `fmt_time(0.0) == "0:00"`, `fmt_time(65.4) == "1:05"`.
pub fn fmt_time(secs: f32) -> String {
    let total = secs.max(0.0).round() as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_time_zero() {
        assert_eq!(fmt_time(0.0), "0:00");
    }

    #[test]
    fn fmt_time_below_a_minute() {
        assert_eq!(fmt_time(5.0), "0:05");
        assert_eq!(fmt_time(59.4), "0:59");
    }

    #[test]
    fn fmt_time_rounds_up_to_the_next_minute() {
        assert_eq!(fmt_time(59.6), "1:00");
    }

    #[test]
    fn fmt_time_minutes_and_hours() {
        assert_eq!(fmt_time(60.0), "1:00");
        assert_eq!(fmt_time(65.0), "1:05");
        assert_eq!(fmt_time(3599.0), "59:59");
        assert_eq!(fmt_time(3600.0), "60:00");
    }

    #[test]
    fn fmt_time_rounds_to_nearest() {
        assert_eq!(fmt_time(65.4), "1:05");
        assert_eq!(fmt_time(65.5), "1:06");
    }

    #[test]
    fn fmt_time_negative_clamps_to_zero() {
        assert_eq!(fmt_time(-5.0), "0:00");
    }
}