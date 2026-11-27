//! Clock-style duration text for HUD timers.
//!
//! The pure half of the old macroquad `ui::TimerDisplay` (Phase 8): games
//! draw the string themselves through `RenderingServer`.

/// Formats `seconds` as `MM:SS`, or `HH:MM:SS` from one hour on.
/// Truncates to the whole second; negative values clamp to zero.
pub fn format_time(seconds: f32) -> String {
    let total = seconds.max(0.0) as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_a_minute() {
        assert_eq!(format_time(5.0), "00:05");
        assert_eq!(format_time(59.9), "00:59");
    }

    #[test]
    fn minutes() {
        assert_eq!(format_time(65.0), "01:05");
        assert_eq!(format_time(3599.0), "59:59");
    }

    #[test]
    fn hours() {
        assert_eq!(format_time(3600.0), "01:00:00");
        assert_eq!(format_time(3661.0), "01:01:01");
    }

    #[test]
    fn negative_clamps_to_zero() {
        assert_eq!(format_time(-10.0), "00:00");
    }
}
