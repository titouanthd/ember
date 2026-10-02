//! Objective feasibility report.
//!
//! For each level, runs a greedy AI that plays optimally for the
//! level's specific objective (clearing the target jade, chaining the
//! required number of cascades, or scoring) and reports whether the
//! objective is reachable within the move budget.
//!
//! Run with:
//!
//!     cargo test --release -p jade-garden --test objectives -- --nocapture
//!
//! Release is strongly recommended — debug is 10-30× slower.

use ember_core::rng::Rng;

use jade_garden::components::{GRID_H, GRID_W};
use jade_garden::grid::Grid;
use jade_garden::level::{self, LevelConfig, Objective};
use jade_garden::scoring::score_for_match;

fn levels() -> Vec<LevelConfig> {
    pollster::block_on(level::load_levels()).expect("levels.ron should parse")
}

/// (cell A, cell B, objective gain) for a candidate swap.
type SwapCandidate = ((usize, usize), (usize, usize), u32);

/// Resolves all cascades from a swapped grid, returning how much
/// progress toward `objective` was earned. Matches the logic in
/// `systems.rs::begin_resolution` exactly.
fn resolve_cascades_objective(
    grid: &mut Grid,
    objective: Objective,
    rng: &mut Rng,
) -> u32 {
    let mut progress = 0u32;
    let mut cascade_level = 0u32;
    loop {
        let matches = grid.find_matches();
        if matches.is_empty() {
            break;
        }

        match objective {
            Objective::Score(_) => {
                progress += score_for_match(matches.len(), cascade_level);
            }
            Objective::ClearJade(jade, _) => {
                let n = matches
                    .iter()
                    .filter(|&&(r, c)| {
                        grid.get(r, c).map(|t| t.jade == jade).unwrap_or(false)
                    })
                    .count() as u32;
                progress += n;
            }
            Objective::FillPoem(_) => {
                // Only cascades count — the first match after a swap is
                // free, matching the new game logic in `systems.rs`.
                if cascade_level > 0 {
                    progress += 1;
                }
            }
        }

        grid.remove_and_collapse(&matches, rng);
        cascade_level += 1;
        if cascade_level > 15 {
            break;
        }
    }
    progress
}

/// Evaluates a single swap for the objective, without mutating `grid`.
fn eval_swap_objective(
    grid: &Grid,
    a: (usize, usize),
    b: (usize, usize),
    objective: Objective,
    rng_seed: u32,
) -> Option<u32> {
    let mut test = *grid;
    test.swap(a, b);
    if !test.has_match_at(a.0, a.1) && !test.has_match_at(b.0, b.1) {
        return None;
    }
    let mut rng = Rng::new(rng_seed);
    // Allow zero-gain swaps: the greedy needs to be able to make
    // setup moves (bring target jades together for future turns).
    Some(resolve_cascades_objective(&mut test, objective, &mut rng))
}

/// Plays a level with a greedy AI targeting the level's objective.
/// Returns the final objective progress.
fn play_greedy_for_objective(seed: u32, moves: u32, objective: Objective) -> u32 {
    let mut grid = Grid::new(seed);
    let mut refill_rng = Rng::new(seed ^ 0xC0FFEE);

    let mut progress = 0u32;

    for move_counter in 0..moves {
        if !grid.has_any_valid_move() {
            grid.reshuffle(&mut refill_rng);
        }

        let eval_seed = 0xBEEF_0000u32.wrapping_add(move_counter);

        let mut best: Option<SwapCandidate> = None;

        // Horizontal pairs.
        for row in 0..GRID_H {
            for col in 0..(GRID_W - 1) {
                let a = (row, col);
                let b = (row, col + 1);
                if let Some(g) = eval_swap_objective(&grid, a, b, objective, eval_seed)
                    && best.is_none_or(|(_, _, s)| g > s)
                {
                    best = Some((a, b, g));
                }
            }
        }
        // Vertical pairs.
        for row in 0..(GRID_H - 1) {
            for col in 0..GRID_W {
                let a = (row, col);
                let b = (row + 1, col);
                if let Some(g) = eval_swap_objective(&grid, a, b, objective, eval_seed)
                    && best.is_none_or(|(_, _, s)| g > s)
                {
                    best = Some((a, b, g));
                }
            }
        }

        let Some((a, b, _)) = best else {
            break;
        };

        grid.swap(a, b);
        let mut play_rng = Rng::new(eval_seed);
        progress += resolve_cascades_objective(&mut grid, objective, &mut play_rng);
    }

    progress
}

#[test]
fn objective_feasibility_report() {
    let levels = levels();

    println!();
    println!("=== jade-garden — objective feasibility (greedy AI, target-aware) ===");
    println!();
    println!(
        "{:<5} {:<28} {:<22} {:>6} {:>8} {:>10} {:>7}  verdict",
        "id", "name", "objective", "moves", "target", "AI prog", "ratio"
    );
    println!("{}", "-".repeat(120));

    for lvl in &levels {
        let ai = play_greedy_for_objective(lvl.seed, lvl.moves, lvl.objective);

        let target = match lvl.objective {
            Objective::Score(n) => n,
            Objective::ClearJade(_, n) => n,
            Objective::FillPoem(n) => n as u32,
        };
        let ratio = ai as f32 / target as f32;

        let verdict = if ai < target {
            "UNREACHABLE"
        } else if ratio < 1.15 {
            "TIGHT"
        } else if ratio < 2.0 {
            "OK"
        } else {
            "EASY"
        };

        println!(
            "{:<5} {:<28} {:<22} {:>6} {:>8} {:>10} {:>7.2}  {}",
            lvl.id,
            lvl.name,
            level::objective_label(lvl.objective),
            lvl.moves,
            target,
            ai,
            ratio,
            verdict,
        );
    }

    println!();
    println!("Legend:");
    println!("  AI prog = objective progress reached by a greedy AI playing");
    println!("            specifically for THIS level's objective.");
    println!("  ratio   = AI prog / target");
    println!("    < 1.00  UNREACHABLE   objective can't be met even by perfect play");
    println!("    1.0-1.15 TIGHT        achievable but with no margin");
    println!("    1.15-2.0 OK           comfortable");
    println!("    > 2.00  EASY          trivial");
    println!();
}

/// Scans seeds for each level and prints the best seed found.
///
/// Run with:
///
///     cargo test --release -p jade-garden --test objectives -- --ignored --nocapture
#[test]
#[ignore]
fn find_best_seeds() {
    const MIN_RATIO: f32 = 1.30;
    const MAX_RATIO: f32 = 2.00;
    const SEED_MAX: u32 = 300;

    let levels = levels();

    println!();
    println!("=== Best seeds per level (greedy AI, target-aware) ===");
    println!();
    println!(
        "{:<5} {:<28} {:>6} {:>6} {:>8}  prog/target",
        "id", "name", "old", "new", "ratio"
    );
    println!("{}", "-".repeat(80));

    for lvl in &levels {
        // Uncomment this line to scan only one level at a time:
        // if lvl.id != "l11" { continue; }

        let target = match lvl.objective {
            Objective::Score(n) => n,
            Objective::ClearJade(_, n) => n,
            Objective::FillPoem(n) => n as u32,
        };

        let mut best_seed = lvl.seed;
        let mut best_ratio = 0.0f32;
        let mut best_prog = 0u32;
        let mut max_prog = 0u32;
        let mut max_seed = 0u32;

        for seed in 1..=SEED_MAX {
            let prog =
                play_greedy_for_objective(seed, lvl.moves, lvl.objective);
            if prog > max_prog {
                max_prog = prog;
                max_seed = seed;
            }
            let ratio = prog as f32 / target as f32;
            if (MIN_RATIO..=MAX_RATIO).contains(&ratio) {
                let mid = (MIN_RATIO + MAX_RATIO) * 0.5;
                let dist = (ratio - mid).abs();
                let current_dist =
                    if best_ratio == 0.0 { f32::MAX } else { (best_ratio - mid).abs() };
                if dist < current_dist {
                    best_seed = seed;
                    best_ratio = ratio;
                    best_prog = prog;
                }
            }
        }

        let max_ratio = max_prog as f32 / target as f32;

        if best_ratio == 0.0 {
            println!(
                "{:<5} {:<28} {:>6} {:>6} {:>8.2}  NO SEED IN BAND, max = {} (seed {})",
                lvl.id, lvl.name, lvl.seed, max_seed, max_ratio, max_prog, max_seed
            );
        } else {
            let marker = if best_seed != lvl.seed { "←" } else { " " };
            println!(
                "{:<5} {:<28} {:>6} {:>6} {:>8.2}  {}/{} {}",
                lvl.id, lvl.name, lvl.seed, best_seed, best_ratio,
                best_prog, target, marker,
            );
        }
    }
    println!();
    println!("If `new` differs from `old`, copy it into assets/levels.ron");
    println!("(the `seed:` field on each level).");
    println!();
}