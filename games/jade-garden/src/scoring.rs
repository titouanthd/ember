//! Score and star computation.
//!
//! A match's score depends on the number of unique destroyed tiles and
//! on the cascade level (increasing multiplier at each successive
//! resolution after a fall).

/// Base score for a match, before the cascade multiplier.
///
/// - 3 tiles: 60 points
/// - Each additional tile: +20 points
///
/// So 4 = 80, 5 = 100, 6 = 120, etc.
pub fn base_score_for_tile_count(tile_count: usize) -> u32 {
    match tile_count {
        0..=2 => 0,
        3 => 60,
        n => 60 + (n as u32 - 3) * 20,
    }
}

/// Cascade multiplier.
///
/// Cascade 0 = first match of the sequence (multiplies ×1.0).
/// Cascade 1 = match from the fall (multiplies ×1.5).
/// Cascade N = multiplies ×(1.0 + N × 0.5), capped at ×5.0.
pub fn cascade_multiplier(cascade_level: u32) -> f32 {
    (1.0 + cascade_level as f32 * 0.5).min(5.0)
}

/// Maps a raw cascade level to a discrete presentation tier (0–4).
///
/// Tier 0: no combo. Tier 1: first combo. Tiers 2-4 escalate the
/// juice (radial bursts, vignette, slow-mo, bigger shake/flash).
pub fn cascade_tier(cascade_level: u32) -> u8 {
    match cascade_level {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 4,
    }
}

/// Final score for a match: `base × cascade_multiplier`.
pub fn score_for_match(tile_count: usize, cascade_level: u32) -> u32 {
    let base = base_score_for_tile_count(tile_count);
    (base as f32 * cascade_multiplier(cascade_level)).round() as u32
}

/// Computes the star count (0-3) for a level.
///
/// - 0: objective not met.
/// - 1: objective met (score ≥ star_target).
/// - 2: score ≥ 1.5 × star_target.
/// - 3: score ≥ 2.0 × star_target.
pub fn compute_stars(score: u32, star_target: u32) -> u8 {
    if score >= star_target.saturating_mul(2) {
        3
    } else if (score as f32) >= (star_target as f32) * 1.5 {
        2
    } else if score >= star_target {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- base_score_for_tile_count ----------

    #[test]
    fn base_score_below_three_is_zero() {
        assert_eq!(base_score_for_tile_count(0), 0);
        assert_eq!(base_score_for_tile_count(1), 0);
        assert_eq!(base_score_for_tile_count(2), 0);
    }

    #[test]
    fn base_score_three_tiles() {
        assert_eq!(base_score_for_tile_count(3), 60);
    }

    #[test]
    fn base_score_four_tiles() {
        assert_eq!(base_score_for_tile_count(4), 80);
    }

    #[test]
    fn base_score_five_tiles() {
        assert_eq!(base_score_for_tile_count(5), 100);
    }

    #[test]
    fn base_score_six_tiles() {
        assert_eq!(base_score_for_tile_count(6), 120);
    }

    #[test]
    fn base_score_seven_tiles() {
        assert_eq!(base_score_for_tile_count(7), 140);
    }

    #[test]
    fn base_score_ten_tiles() {
        // 60 + 7 * 20 = 200
        assert_eq!(base_score_for_tile_count(10), 200);
    }

    #[test]
    fn base_score_increments_by_twenty_per_tile() {
        for n in 3..20 {
            assert_eq!(
                base_score_for_tile_count(n + 1) - base_score_for_tile_count(n),
                20,
                "increment at n={n} should be 20"
            );
        }
    }

    #[test]
    fn base_score_is_monotonic() {
        let mut prev = 0;
        for n in 3..20 {
            let s = base_score_for_tile_count(n);
            assert!(s > prev, "score at n={n} not greater than previous");
            prev = s;
        }
    }

    // ---------- cascade_multiplier ----------

    #[test]
    fn cascade_zero_is_one() {
        assert!((cascade_multiplier(0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cascade_one_is_one_point_five() {
        assert!((cascade_multiplier(1) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn cascade_two_is_two() {
        assert!((cascade_multiplier(2) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn cascade_four_is_three() {
        assert!((cascade_multiplier(4) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn cascade_eight_is_five() {
        assert!((cascade_multiplier(8) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn cascade_caps_at_five() {
        assert!((cascade_multiplier(100) - 5.0).abs() < 1e-6);
        assert!((cascade_multiplier(u32::MAX) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn cascade_multiplier_is_monotonic() {
        let mut prev = 0.0;
        for c in 0..15 {
            let m = cascade_multiplier(c);
            assert!(m >= prev, "multiplier not monotonic at c={c}");
            prev = m;
        }
    }

    // ---------- score_for_match ----------

    #[test]
    fn score_three_no_cascade() {
        assert_eq!(score_for_match(3, 0), 60);
    }

    #[test]
    fn score_three_cascade_one() {
        // 60 * 1.5 = 90
        assert_eq!(score_for_match(3, 1), 90);
    }

    #[test]
    fn score_four_cascade_one() {
        // 80 * 1.5 = 120
        assert_eq!(score_for_match(4, 1), 120);
    }

    #[test]
    fn score_five_cascade_two() {
        // 100 * 2.0 = 200
        assert_eq!(score_for_match(5, 2), 200);
    }

    #[test]
    fn score_six_cascade_eight() {
        // 120 * 5.0 = 600
        assert_eq!(score_for_match(6, 8), 600);
    }

    #[test]
    fn score_zero_tiles_is_zero() {
        assert_eq!(score_for_match(0, 5), 0);
        assert_eq!(score_for_match(2, 10), 0);
    }

    // ---------- compute_stars ----------

    #[test]
    fn stars_zero_when_below_target() {
        assert_eq!(compute_stars(999, 1000), 0);
        assert_eq!(compute_stars(0, 1000), 0);
    }

    #[test]
    fn stars_one_at_target() {
        assert_eq!(compute_stars(1000, 1000), 1);
        assert_eq!(compute_stars(1001, 1000), 1);
    }

    #[test]
    fn stars_two_at_one_point_five_target() {
        assert_eq!(compute_stars(1500, 1000), 2);
        assert_eq!(compute_stars(1800, 1000), 2);
        assert_eq!(compute_stars(1999, 1000), 2);
    }

    #[test]
    fn stars_three_at_two_times_target() {
        assert_eq!(compute_stars(2000, 1000), 3);
        assert_eq!(compute_stars(5000, 1000), 3);
    }

    #[test]
    fn stars_three_at_boundary() {
        assert_eq!(compute_stars(2000, 1000), 3);
    }

    #[test]
    fn stars_with_small_targets() {
        assert_eq!(compute_stars(35, 30), 1);
        assert_eq!(compute_stars(45, 30), 2);
        assert_eq!(compute_stars(60, 30), 3);
    }

    #[test]
    fn stars_never_exceed_three() {
        for score in [100, 500, 1000, 5000, u32::MAX / 2] {
            assert!(compute_stars(score, 100) <= 3);
        }
    }

    #[test]
    fn stars_never_below_zero() {
        for score in [0, 10, 100, 1000] {
            let s = compute_stars(score, 100);
            assert!(s <= 3);
        }
    }

    #[test]
    fn twenty_moves_of_pure_match_three_reach_1200() {
        // 20 moves × 60 pts = 1200 minimum with no cascade.
        // With only a few cascades, we pass 1500.
        let base = base_score_for_tile_count(3);
        let total = 20 * base;
        assert!(
            total >= 1200,
            "20 pure match-3 should reach 1200, got {total}"
        );
    }

    #[test]
    fn cascade_tier_zero_for_first_match() {
        assert_eq!(cascade_tier(0), 0);
    }

    #[test]
    fn cascade_tier_is_monotonic_and_capped() {
        let mut prev = 0u8;
        for c in 0..20 {
            let t = cascade_tier(c);
            assert!(t >= prev);
            assert!(t <= 4);
            prev = t;
        }
    }

    #[test]
    fn cascade_tier_reaches_max() {
        assert_eq!(cascade_tier(4), 4);
        assert_eq!(cascade_tier(100), 4);
    }
}