//! Breadth-first search for the shortest solution of a level.
//!
//! Every move costs one, so BFS finds a solution with the fewest moves:
//! that number is the level's par. The tests run it over every level, so a
//! level that cannot be solved in par fails the build.

use crate::level::Level;
use crate::rules::{Dir, State, apply};
use std::collections::{HashMap, VecDeque};

/// States explored before [`solve`] gives up. Hand-made levels stay far
/// below it; it only guards against a runaway search.
pub const DEFAULT_STATE_LIMIT: usize = 1_000_000;

/// The shortest move list that wins `level`, or `None` if there is none
/// within [`DEFAULT_STATE_LIMIT`] states.
pub fn solve(level: &Level) -> Option<Vec<Dir>> {
    solve_within(level, DEFAULT_STATE_LIMIT)
}

/// [`solve`] with an explicit state budget.
pub fn solve_within(level: &Level, limit: usize) -> Option<Vec<Dir>> {
    let start = State::initial(level);
    if start.won {
        return Some(Vec::new());
    }
    // state -> (parent, move that reached it); the start has no parent.
    let mut parents: HashMap<State, Option<(State, Dir)>> = HashMap::new();
    parents.insert(start.clone(), None);
    let mut queue = VecDeque::from([start]);

    while let Some(state) = queue.pop_front() {
        for dir in Dir::ALL {
            let Ok((next, _)) = apply(level, &state, dir) else {
                continue;
            };
            if parents.contains_key(&next) {
                continue;
            }
            let won = next.won;
            parents.insert(next.clone(), Some((state.clone(), dir)));
            if won {
                return Some(path_to(&parents, next));
            }
            if parents.len() >= limit {
                return None;
            }
            queue.push_back(next);
        }
    }
    None
}

fn path_to(parents: &HashMap<State, Option<(State, Dir)>>, mut state: State) -> Vec<Dir> {
    let mut moves = Vec::new();
    while let Some(Some((parent, dir))) = parents.get(&state) {
        moves.push(*dir);
        state = parent.clone();
    }
    moves.reverse();
    moves
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(text: &str) -> Level {
        Level::parse(text).unwrap()
    }

    #[test]
    fn finds_the_shortest_solution() {
        // Down lights the lantern on the way, right enters the gate.
        let moves = solve(&level("@..\n.L.\n..G")).unwrap();
        assert_eq!(Dir::format_moves(&moves), "DR");
    }

    #[test]
    fn a_walled_off_lantern_has_no_solution() {
        assert_eq!(solve(&level("@.#L#\n..###\n...G.")), None);
    }

    #[test]
    fn gives_up_past_the_state_limit() {
        let open = level("@.....\n......\n......\n.....L\n.....G");
        assert!(solve(&open).is_some());
        assert_eq!(solve_within(&open, 2), None);
    }
}
