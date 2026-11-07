//! Balance report: simulates a **greedy AI** that plays optimally
//! (buys the highest-ROI building including passives, advances paradigms
//! as soon as possible) and reports the time spent in each paradigm +
//! total time to reach Star.
//!
//! Run with:
//!
//!     cargo test --release -p accretion --test balance -- --nocapture
//!
//! For the click-rate sensitivity scan (slow, useful when tuning):
//!
//!     cargo test --release -p accretion --test balance -- --ignored --nocapture
//!
//! The `ratio` column compares the AI's time to the DESIGN's target
//! playtime per paradigm. The AI plays *optimally*, so a ratio of ~0.25
//! (AI takes 35 min, human takes 150 min) is expected. An optimal AI is
//! always 3-5× faster than a casual human in an incremental game.

use accretion::components::{BuildingId, Paradigm, ResourceId};
use accretion::economy::{
    self, BUILDINGS, PassiveEffect, compute_multipliers, id_to_def,
};
use accretion::paradigm;
use accretion::systems::Game;

// ─── Resource weight table ─────────────────────────────────────
//
// Approximate "value" of one unit of a resource in Dust-equivalents.
// Monotonic along the production chain, roughly ×10 per tier, with
// Metals/Water as a side branch at tier 2. Used to compare costs of
// buildings paid in different resources.

fn resource_weight(r: ResourceId) -> f64 {
    use ResourceId::*;
    match r {
        Dust => 1.0,
        Pebbles => 10.0,
        Planetesimals => 100.0,
        Metals => 200.0,
        Water => 200.0,
        Gas => 2_000.0,
        Light => 20_000.0,
        DarkMatter => 1_000_000.0,
    }
}

// ─── Production helpers ────────────────────────────────────────

/// Current production of `r` per second, given the game's buildings
/// and multipliers (prestige G included).
fn current_production(game: &Game, r: ResourceId) -> f64 {
    let mults = compute_multipliers(&game.buildings, game.g_multiplier);
    let mult = mults.get(&r).copied().unwrap_or(1.0);
    let mut total = 0.0;
    for inst in &game.buildings {
        let def = id_to_def(inst.id);
        if let Some((prod_r, amount)) = def.production
            && prod_r == r
            && def.cooldown > 0.0
            && inst.count > 0
        {
            total += amount * inst.count as f64 * mult / def.cooldown as f64;
        }
    }
    total
}

/// Marginal value (in Dust-equivalent per second) from buying one more
/// `id`. Handles both active producers and passive multipliers.
fn marginal_value(game: &Game, id: BuildingId) -> f64 {
    let def = id_to_def(id);
    if def.paradigm > game.paradigm {
        return 0.0;
    }

    if let Some((r, amount)) = def.production {
        let mults = compute_multipliers(&game.buildings, game.g_multiplier);
        let mult = mults.get(&r).copied().unwrap_or(1.0);
        let rate = amount * mult / def.cooldown as f64;
        return rate * resource_weight(r);
    }

    if let Some(effect) = def.passive_effect {
        let (v, target) = match effect {
            PassiveEffect::MultiplyResource(r, v) => (v, Some(r)),
            PassiveEffect::MultiplyAll(v) => (v, None),
        };
        // current_production already includes the existing passives of
        // this same kind, so marginal = current × (v - 1).
        let base_value = match target {
            Some(r) => current_production(game, r) * resource_weight(r),
            None => ResourceId::ALL
                .iter()
                .map(|r| current_production(game, *r) * resource_weight(*r))
                .sum::<f64>(),
        };
        return base_value * (v - 1.0);
    }

    0.0
}

/// Value-per-resource-spent for buying one more `id`.
fn roi(game: &Game, id: BuildingId) -> f64 {
    let mv = marginal_value(game, id);
    if mv <= 0.0 {
        return 0.0;
    }
    let def = id_to_def(id);
    let count = game.building_count(id);
    let cost: f64 = economy::cost_of(def, count)
        .iter()
        .map(|(r, c)| c * resource_weight(*r))
        .sum();
    if cost <= 0.0 {
        return 0.0;
    }
    mv / cost
}

// ─── Decision ─────────────────────────────────────────────────

/// Priority-based purchase decision:
///   1. Required buildings for the next transition.
///   2. Producers of a missing required resource.
///   3. Highest ROI among all affordable buildings (passives included).
fn choose_next_building(game: &Game) -> Option<BuildingId> {
    let cond = paradigm::next_condition(game.paradigm);

    // Priority 1: required buildings.
    if let Some(c) = cond {
        for (id, needed) in c.required_buildings {
            if game.building_count(*id) < *needed && game.can_buy(*id) {
                return Some(*id);
            }
        }

        // Priority 2: producers of required resources we're short on.
        for (resource, needed) in c.required_resources {
            let have = game.resources.get(resource).copied().unwrap_or(0.0);
            if have >= *needed {
                continue;
            }
            let mut best: Option<(BuildingId, f64)> = None;
            for def in BUILDINGS {
                if def.paradigm > game.paradigm {
                    continue;
                }
                let Some((r, _)) = def.production else { continue };
                if r != *resource {
                    continue;
                }
                if !game.can_buy(def.id) {
                    continue;
                }
                let rr = roi(game, def.id);
                if best.is_none_or(|(_, br)| rr > br) {
                    best = Some((def.id, rr));
                }
            }
            if let Some((id, _)) = best {
                return Some(id);
            }
        }
    }

    // Priority 3: highest ROI across everything affordable.
    let mut best: Option<(BuildingId, f64)> = None;
    for def in BUILDINGS {
        if def.paradigm > game.paradigm {
            continue;
        }
        if !game.can_buy(def.id) {
            continue;
        }
        let rr = roi(game, def.id);
        if rr > 0.0 && best.is_none_or(|(_, br)| rr > br) {
            best = Some((def.id, rr));
        }
    }
    best.map(|(id, _)| id)
}

// ─── Simulation ───────────────────────────────────────────────

/// One greedy-AI run's data.
struct RunReport {
    paradigm_times: Vec<(Paradigm, f32)>,
    total_time: f32,
    final_paradigm: Paradigm,
    final_resources: Vec<(ResourceId, f64)>,
    final_buildings: Vec<(BuildingId, u32)>,
}

impl RunReport {
    fn time_in(&self, i: usize) -> Option<f32> {
        self.paradigm_times
            .get(i + 1)
            .map(|(_, t)| t - self.paradigm_times[i].1)
    }

    /// Sum of held resources + sum(count × base_cost). Same formula as
    /// DESIGN §10 "total_mass_run" (option A, no per-run accumulator).
    fn total_mass_run(&self) -> f64 {
        let resources: f64 = self.final_resources.iter().map(|(_, v)| *v).sum();
        let buildings: f64 = self
            .final_buildings
            .iter()
            .map(|(b, n)| {
                *n as f64 * id_to_def(*b).base_cost.iter().map(|(_, v)| *v).sum::<f64>()
            })
            .sum();
        resources + buildings
    }

    /// DESIGN §10 G formula. Inlined for now — will be replaced by
    /// `prestige::compute_g_gain` in V1.1.
    fn g_gain(&self) -> u64 {
        let mass = self.total_mass_run();
        let paradigms = self.paradigm_times.len() as f64;
        ((mass + 1.0).log10() * (1.0 + 0.1 * paradigms))
            .floor()
            .max(1.0) as u64
    }

    /// DESIGN.md §3 target: 2-3h to Star. Use 2.5h as midpoint.
    const TARGET_TOTAL_SECONDS: f32 = 150.0 * 60.0;
}

/// Simulate one run with the greedy AI.
///
/// - `click_rate`: clicks per second (5.0 = active human).
/// - `sim_step`: simulation timestep in seconds.
/// - `max_time`: safety bound (e.g. 5h = 18000s).
/// - `g_total`: prestige bonus at start (0 for the first run).
fn simulate_greedy(
    click_rate: f32,
    sim_step: f32,
    max_time: f32,
    g_total: u64,
) -> RunReport {
    let mut game = Game::new();
    game.g_total = g_total;
    game.g_multiplier = 1.0 + 0.05 * g_total as f64;

    let mut paradigm_times = vec![(Paradigm::Dust, 0.0)];
    let mut click_acc = 0.0f32;
    let mut decision_acc = 0.0f32;
    let click_period = 1.0 / click_rate;
    const DECISION_PERIOD: f32 = 0.5;

    let mut time = 0.0f32;
    while time < max_time && game.paradigm != Paradigm::Star {
        game.tick(sim_step);
        time += sim_step;

        click_acc += sim_step;
        while click_acc >= click_period {
            click_acc -= click_period;
            game.click();
        }

        decision_acc += sim_step;
        if decision_acc >= DECISION_PERIOD {
            decision_acc = 0.0;
            if game.can_advance().is_some() {
                if let Some(new_p) = game.try_advance() {
                    paradigm_times.push((new_p, time));
                }
                continue;
            }
            if let Some(id) = choose_next_building(&game) {
                game.buy_building(id);
            }
        }
    }

    let final_resources = ResourceId::ALL
        .iter()
        .map(|r| (*r, game.resources[r]))
        .collect();
    let final_buildings = BuildingId::ALL
        .iter()
        .map(|b| (*b, game.building_count(*b)))
        .collect();

    RunReport {
        paradigm_times,
        total_time: time,
        final_paradigm: game.paradigm,
        final_resources,
        final_buildings,
    }
}

// ─── Verdicts ─────────────────────────────────────────────────

/// Expected human time per paradigm (DESIGN.md §9). Used only for
/// ratio/verdict — not a hard target.
fn expected_seconds(p: Paradigm) -> f32 {
    match p {
        Paradigm::Dust => 30.0,
        Paradigm::Pebbles => 300.0,
        Paradigm::Planetesimals => 600.0,
        Paradigm::Planets => 1200.0,
        Paradigm::GasGiant => 1800.0,
        Paradigm::Star => 3600.0,
    }
}

fn verdict(actual: f32, expected: f32) -> &'static str {
    let ratio = actual / expected;
    if ratio > 3.0 {
        "TOO SLOW"
    } else if ratio > 1.5 {
        "SLOW"
    } else if ratio > 0.5 {
        "OK"
    } else {
        "FAST"
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[test]
fn ai_balance_report() {
    let report = simulate_greedy(5.0, 0.1, 5.0 * 3600.0, 0);

    println!();
    println!("=== accretion — balance report (optimal greedy AI, 5 clicks/sec) ===");
    println!();
    println!(
        "{:<3} {:<16} {:>14} {:>14} {:>14} {:>8}  verdict",
        "#", "paradigm", "time_in (min)", "cumul (min)", "expected (min)", "ratio"
    );
    println!("{}", "-".repeat(96));

    for (i, (p, cumul)) in report.paradigm_times.iter().enumerate() {
        let cumul_min = cumul / 60.0;
        let expected = expected_seconds(*p) / 60.0;
        match report.time_in(i) {
            Some(t) => {
                let time_in = t / 60.0;
                println!(
                    "{:<3} {:<16} {:>14.1} {:>14.1} {:>14.1} {:>8.2}  {}",
                    i + 1,
                    p.name(),
                    time_in,
                    cumul_min,
                    expected,
                    time_in / expected,
                    verdict(t, expected_seconds(*p)),
                );
            }
            None => {
                println!(
                    "{:<3} {:<16} {:>14} {:>14.1} {:>14.1} {:>8}  (terminal)",
                    i + 1,
                    p.name(),
                    "—",
                    cumul_min,
                    expected,
                    "—",
                );
            }
        }
    }

    println!();
    println!("Final state at {:?}:", report.final_paradigm);
    println!("  Resources (non-zero):");
    for (r, v) in &report.final_resources {
        if *v > 0.0 {
            println!("    {:<14} {:>12.0}", r.name(), v);
        }
    }
    println!("  Active buildings:");
    for (b, n) in &report.final_buildings {
        if *n > 0 && id_to_def(*b).passive_effect.is_none() {
            println!("    {:<20} ×{}", id_to_def(*b).name, n);
        }
    }
    println!("  Passive buildings:");
    for (b, n) in &report.final_buildings {
        if *n > 0 && id_to_def(*b).passive_effect.is_some() {
            println!("    {:<20} ×{}", id_to_def(*b).name, n);
        }
    }

    let total_min = report.total_time / 60.0;
    let target_min = RunReport::TARGET_TOTAL_SECONDS / 60.0;
    println!();
    println!(
        "Total time to Star: {:.1} min (target: {:.0} min) → ratio {:.2}",
        total_min,
        target_min,
        total_min / target_min,
    );
    println!(
        "Final mass: {:.0}  →  G_gain if prestige now: +{}",
        report.total_mass_run(),
        report.g_gain(),
    );

    // ─── Invariant 1: must reach Star ───
    assert_eq!(
        report.final_paradigm,
        Paradigm::Star,
        "AI did not reach Star within {:.0} min",
        total_min,
    );

    // ─── Invariant 2: at least one passive must be bought ───
    let passives_bought: u32 = report
        .final_buildings
        .iter()
        .filter(|(b, _)| id_to_def(*b).passive_effect.is_some())
        .map(|(_, n)| *n)
        .sum();
    assert!(
        passives_bought >= 1,
        "AI bought no passive building — ROI is broken",
    );

    // ─── Invariant 3: no paradigm may be TOO SLOW ───
    let mut too_slow = Vec::new();
    for (i, (p, _)) in report.paradigm_times.iter().enumerate() {
        if let Some(t) = report.time_in(i)
            && verdict(t, expected_seconds(*p)) == "TOO SLOW"
        {
            too_slow.push((*p, t / 60.0));
        }
    }
    assert!(
        too_slow.is_empty(),
        "TOO SLOW paradigms (name, minutes): {too_slow:?}",
    );

    println!();
    println!("Legend:");
    println!("  ratio = time_in / expected (DESIGN.md §9 human estimate)");
    println!("    > 3.0    TOO SLOW      fix: reduce cost/target or add a producer");
    println!("    1.5-3.0  SLOW          tuning recommended");
    println!("    0.5-1.5  OK            on target");
    println!("    < 0.5    FAST          may feel trivial for humans");
    println!();
    println!("  Note: an *optimal* AI is typically 3-5× faster than a casual");
    println!("  human. A ratio of ~0.25 across the board is ideal — it means");
    println!("  the DESIGN target is roughly right for a human player.");
    println!();
}

/// Click-rate sensitivity: how much does manual clicking matter?
#[test]
#[ignore]
fn click_rate_sensitivity() {
    println!();
    println!("=== accretion — click rate sensitivity (to Star) ===");
    println!();
    println!(
        "{:>10} {:>16} {:>16}  verdict",
        "click_rate", "total (min)", "Δ vs 5/s (min)"
    );
    println!("{}", "-".repeat(66));

    let baseline = simulate_greedy(5.0, 0.1, 5.0 * 3600.0, 0).total_time / 60.0;

    for rate in [0.5, 1.0, 2.0, 5.0, 10.0, 20.0] {
        let r = simulate_greedy(rate, 0.1, 5.0 * 3600.0, 0);
        let total = r.total_time / 60.0;
        let delta = total - baseline;
        let v = if r.final_paradigm == Paradigm::Star {
            "ok"
        } else {
            "STUCK"
        };
        println!("{:>10.1} {:>16.1} {:>+16.1}  {}", rate, total, delta, v);
    }
}

/// Prestige loop: verify run 2 is measurably faster than run 1.
#[test]
fn prestige_loop_is_satisfying() {
    let r1 = simulate_greedy(5.0, 0.1, 5.0 * 3600.0, 0);
    assert_eq!(r1.final_paradigm, Paradigm::Star);

    let g = r1.g_gain();
    assert!(g >= 1, "G_gain must be >= 1 (got {g})");

    let r2 = simulate_greedy(5.0, 0.1, 5.0 * 3600.0, g);
    assert_eq!(r2.final_paradigm, Paradigm::Star);

    let ratio = r2.total_time / r1.total_time;

    println!();
    println!("=== Prestige loop ===");
    println!("  Run 1 (G=0):      {:.1} min,  mass {:.0},  G_gain +{}",
             r1.total_time / 60.0, r1.total_mass_run(), g);
    println!("  Run 2 (G={}):      {:.1} min,  ratio {:.2}",
             g, r2.total_time / 60.0, ratio);
    println!("  Target range: 0.55 – 0.85");

    assert!(
        ratio < 0.85,
        "prestige gain too weak (ratio {ratio:.2}, G={g})",
    );
    assert!(
        ratio > 0.45,
        "prestige gain too strong (ratio {ratio:.2}, G={g})",
    );
}