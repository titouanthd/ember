//! Balance report: for each level, simulates a **greedy AI** that plays
//! every move optimally (full cascade resolution for each candidate
//! swap, keep the best one) and reports the maximum score reachable in
//! the level's move budget.
//!
//! Run with:
//!
//!     cargo test -p jade-garden --test balance -- --nocapture
//!
//! For speed (recommended — debug is 10-30x slower):
//!
//!     cargo test --release -p jade-garden --test balance -- --nocapture
//!
//! The `ratio` column is `AI score / star_target`. It tells you how
//! hard it is to earn a single star by pure score:
//!
//! - ratio < 0.50   → TOO EASY
//! - 0.50 – 0.65    → easy
//! - 0.65 – 0.90    → OK
//! - 0.90 – 1.00    → HARD (near-perfect play needed)
//! - ratio >= 1.00  → IMPOSSIBLE (even a perfect AI can't reach it)
//!
//! Note: for `ClearJade` and `FillPoem` levels, the AI maximises pure
//! score, not the actual objective. It's a useful upper bound but the
//! real requirement might be lower (a human playing for the objective
//! might score less). For `Score` levels, the AI score is exact.

use ember_core::rng::Rng;

use jade_garden::components::{GRID_H, GRID_W};
use jade_garden::grid::Grid;
use jade_garden::level::{self, Objective};
use jade_garden::scoring::score_for_match;

/// (cell A, cell B, total cascade score) for a candidate swap.
type SwapCandidate = ((usize, usize), (usize, usize), u32);

/// Resolves every cascade from the current state of the grid, returning
/// the total score earned. Leaves the grid in a stable (no-match) state.
fn resolve_cascades(grid: &mut Grid, rng: &mut Rng) -> u32 {
    let mut total = 0u32;
    let mut cascade_level = 0u32;
    loop {
        let matches = grid.find_matches();
        if matches.is_empty() {
            break;
        }
        total += score_for_match(matches.len(), cascade_level);
        grid.remove_and_collapse(&matches, rng);
        cascade_level += 1;
        if cascade_level > 15 {
            break;
        }
    }
    total
}

/// Simulates swapping `a` and `b` on a copy of `grid` and returns the
/// total cascade score, or `None` if the swap produces no match.
fn eval_swap(
    grid: &Grid,
    a: (usize, usize),
    b: (usize, usize),
    rng_seed: u32,
) -> Option<u32> {
    let mut test = *grid;
    test.swap(a, b);
    if !test.has_match_at(a.0, a.1) && !test.has_match_at(b.0, b.1) {
        return None;
    }
    let mut rng = Rng::new(rng_seed);
    let score = resolve_cascades(&mut test, &mut rng);
    if score == 0 { None } else { Some(score) }
}

/// Plays a full level with a greedy AI. Returns the total score.
fn play_greedy(seed: u32, moves: u32) -> u32 {
    let mut grid = Grid::new(seed);
    // Separate RNG for refills — deterministic so the report is stable.
    let mut refill_rng = Rng::new(seed ^ 0xC0FFEE);

    let mut total_score = 0u32;

    for move_counter in 0..moves {
        // Detect dead-lock (rare at level start, more likely after
        // cascades).
        if !grid.has_any_valid_move() {
            grid.reshuffle(&mut refill_rng);
        }

        // Deterministic per-move seed. Used both for evaluation and for
        // the real application, so the AI's prediction matches reality.
        let eval_seed = 0xBEEF_0000u32.wrapping_add(move_counter);

        let mut best: Option<SwapCandidate> = None;

        // Horizontal pairs.
        for row in 0..GRID_H {
            for col in 0..(GRID_W - 1) {
                let a = (row, col);
                let b = (row, col + 1);
                if let Some(sc) = eval_swap(&grid, a, b, eval_seed)
                    && best.is_none_or(|(_, _, s)| sc > s)
                {
                    best = Some((a, b, sc));
                }
            }
        }
        // Vertical pairs.
        for row in 0..(GRID_H - 1) {
            for col in 0..GRID_W {
                let a = (row, col);
                let b = (row + 1, col);
                if let Some(sc) = eval_swap(&grid, a, b, eval_seed)
                    && best.is_none_or(|(_, _, s)| sc > s)
                {
                    best = Some((a, b, sc));
                }
            }
        }

        let Some((a, b, _)) = best else {
            // No playable swap at all — bail out.
            break;
        };

        // Apply the winning swap and resolve it for real.
        grid.swap(a, b);
        let mut play_rng = Rng::new(eval_seed);
        total_score += resolve_cascades(&mut grid, &mut play_rng);
    }

    total_score
}

#[test]
fn ai_balance_report() {
    let levels = level::load_levels().expect("levels.ron should parse");

    println!();
    println!("=== jade-garden — AI balance report (greedy, full cascade) ===");
    println!();
    println!(
        "{:<5} {:<28} {:<22} {:>6} {:>7} {:>9} {:>7}  verdict",
        "id", "name", "objective", "moves", "target", "AI score", "ratio"
    );
    println!("{}", "-".repeat(122));

    for lvl in &levels {
        let ai = play_greedy(lvl.seed, lvl.moves);
        let ratio = ai as f32 / lvl.star_target as f32;

        let verdict = if ratio < 1.00 {
            "IMPOSSIBLE"   // target above AI ceiling
        } else if ratio < 1.50 {
            "HARD"         // needs >67% of perfect play
        } else if ratio < 2.50 {
            "OK"           // 40-67% of perfect play
        } else if ratio < 4.00 {
            "EASY"         // 25-40% of perfect play
        } else {
            "TRIVIAL"      // under 25% of perfect play
        };

        // Non-Score objectives get a "~" because the AI maximises pure
        // score, not the objective itself.
        let id_cell = match lvl.objective {
            Objective::Score(_) => lvl.id.clone(),
            _ => format!("~{}", lvl.id),
        };

        println!(
            "{:<5} {:<28} {:<22} {:>6} {:>7} {:>9} {:>7.2}  {}",
            id_cell,
            lvl.name,
            level::objective_label(lvl.objective),
            lvl.moves,
            lvl.star_target,
            ai,
            ratio,
            verdict,
        );
    }

    // ─── Invariant: no level may be IMPOSSIBLE ───
    // If this fails, either a star_target is too high, or a seed
    // changed and the move budget no longer suffices. Fix the level
    // (rebalance or reseed), not the test.
    let mut impossible: Vec<(String, u32, u32)> = Vec::new();
    for lvl in &levels {
        let ai = play_greedy(lvl.seed, lvl.moves);
        if ai < lvl.star_target {
            impossible.push((lvl.id.clone(), lvl.star_target, ai));
        }
    }
    assert!(
        impossible.is_empty(),
        "IMPOSSIBLE levels detected (id, target, ai): {impossible:?}"
    );

    println!("Legend:");
    println!("  ratio = AI score / star_target   (how much headroom you have)");
    println!("    < 1.00  IMPOSSIBLE    target above AI ceiling");
    println!("    1.0-1.5 HARD          needs >67% of perfect play");
    println!("    1.5-2.5 OK            40-67% of perfect play");
    println!("    2.5-4.0 EASY          25-40% of perfect play");
    println!("    > 4.00  TRIVIAL       under 25% of perfect play");
    println!("  ~     = non-Score objective: AI maximises pure score only,");
    println!("          the real objective may require suboptimal swaps.");
    println!();
}