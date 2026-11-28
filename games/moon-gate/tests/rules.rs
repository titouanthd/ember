//! One scenario per rule of `moon_gate::rules`, on tiny boards.

use moon_gate::{Dir, Event, Game, Level, MoveError, State, apply};

fn level(text: &str) -> Level {
    Level::parse(text).expect("test level parses")
}

fn play(text: &str, moves: &str) -> (Level, State) {
    let level = level(text);
    let mut state = State::initial(&level);
    for dir in Dir::parse_moves(moves).unwrap() {
        state = apply(&level, &state, dir).expect("move is legal").0;
    }
    (level, state)
}

fn board(level: &Level, state: &State) -> String {
    state.render(level)
}

fn lines(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| format!("{l}\n"))
        .collect()
}

// —— Slide ———————————————————————————————————————————————————————————————

#[test]
fn slides_until_a_wall() {
    let (level, state) = play("#@...#G", "R");
    assert_eq!(state.spirit, (4, 0));
    assert_eq!(board(&level, &state), "#...@#O\n");
}

#[test]
fn the_board_edge_blocks() {
    let (_, state) = play("@...\n...G", "R");
    assert_eq!(state.spirit, (3, 0));
}

#[test]
fn every_move_reports_each_step() {
    let level = level("@..#G");
    let (_, events) = apply(&level, &State::initial(&level), Dir::Right).unwrap();
    assert_eq!(
        events,
        vec![
            Event::Step {
                from: (0, 0),
                to: (1, 0)
            },
            Event::Step {
                from: (1, 0),
                to: (2, 0)
            },
        ]
    );
}

#[test]
fn a_move_that_changes_nothing_is_refused() {
    let level = level("#@.G");
    let start = State::initial(&level);
    assert_eq!(apply(&level, &start, Dir::Left), Err(MoveError::Blocked));
    assert_eq!(apply(&level, &start, Dir::Up), Err(MoveError::Blocked));
}

#[test]
fn directions_round_trip_as_text() {
    let moves = Dir::parse_moves("udLR").unwrap();
    assert_eq!(moves, vec![Dir::Up, Dir::Down, Dir::Left, Dir::Right]);
    assert_eq!(Dir::format_moves(&moves), "UDLR");
    assert_eq!(Dir::parse_moves("RX"), None);
}

// —— Lanterns and gate ——————————————————————————————————————————————————

#[test]
fn lanterns_next_to_the_start_are_lit() {
    let level = level("L@.\n.L.\nG..");
    let state = State::initial(&level);
    assert!(state.is_lit(0) && state.is_lit(1));
    assert!(state.gate_open(&level));
}

#[test]
fn a_lantern_lights_when_the_spirit_passes_it() {
    let text = "
        ..L..
        @...#
        G....
    ";
    let level = level(text);
    let (state, events) = apply(&level, &State::initial(&level), Dir::Right).unwrap();
    assert_eq!(state.spirit, (3, 1));
    assert!(state.is_lit(0));
    assert!(events.contains(&Event::Lit { lantern: 0 }));
    assert!(events.contains(&Event::GateOpened));
}

#[test]
fn a_lantern_blocks_like_a_wall() {
    let (_, state) = play("@..L\nG...", "R");
    assert_eq!(state.spirit, (2, 0));
}

#[test]
fn a_closed_gate_blocks() {
    let (level, state) = play("@..G\n...L", "R");
    assert_eq!(state.spirit, (2, 0));
    assert!(!state.won);
    // (2, 0) is not next to the lantern at (3, 1): the gate stays shut.
    assert!(!state.gate_open(&level));
}

#[test]
fn sliding_into_the_open_gate_wins() {
    let level = level("@..\n.L.\n..G");
    let (state, _) = apply(&level, &State::initial(&level), Dir::Down).unwrap();
    assert_eq!(state.spirit, (0, 2));
    assert!(state.gate_open(&level) && !state.won);
    let (state, events) = apply(&level, &state, Dir::Right).unwrap();
    assert!(state.won);
    assert_eq!(state.spirit, (2, 2));
    assert_eq!(events.last(), Some(&Event::Won));
    assert_eq!(apply(&level, &state, Dir::Left), Err(MoveError::Won));
}

#[test]
fn no_lanterns_means_the_gate_starts_open() {
    let (_, state) = play("@..G", "R");
    assert!(state.won);
}

#[test]
fn lighting_the_last_lantern_opens_the_gate_mid_slide() {
    let text = "
        .L...
        @...G
    ";
    let level = level(text);
    let (state, events) = apply(&level, &State::initial(&level), Dir::Right).unwrap();
    assert!(state.won);
    let opened = events.iter().position(|e| *e == Event::GateOpened).unwrap();
    let won = events.iter().position(|e| *e == Event::Won).unwrap();
    assert!(opened < won);
}

// —— Stone ——————————————————————————————————————————————————————————————

#[test]
fn a_slide_into_a_stone_pushes_it_one_cell_and_stops() {
    let level = level("@..o..#G");
    let (state, events) = apply(&level, &State::initial(&level), Dir::Right).unwrap();
    assert_eq!(state.spirit, (2, 0));
    assert_eq!(state.stones, vec![(4, 0)]);
    assert_eq!(
        events.last(),
        Some(&Event::Push {
            from: (3, 0),
            to: (4, 0)
        })
    );
    // Next push: one more cell each time.
    let (state, events) = apply(&level, &state, Dir::Right).unwrap();
    assert_eq!(state.spirit, (3, 0));
    assert_eq!(state.stones, vec![(5, 0)]);
    assert_eq!(events.len(), 2);
}

#[test]
fn a_stone_that_cannot_move_just_stops_the_spirit() {
    let (_, state) = play("@..o#G", "R");
    assert_eq!(state.spirit, (2, 0));
    assert_eq!(state.stones, vec![(3, 0)]);
    let level = level("@..o#G");
    assert_eq!(apply(&level, &state, Dir::Right), Err(MoveError::Blocked));
}

#[test]
fn stones_only_move_onto_plain_floor() {
    for blocker in ['o', 'L', 'G', '1', '=', 'x', '#'] {
        let row = format!(".@o{blocker}");
        let text = match blocker {
            // Keep the level valid: a gate and paired pools.
            'G' => row,
            '1' => format!("{row}\n...1\n...G"),
            _ => format!("{row}\n...G"),
        };
        let level = level(&text);
        let start = State::initial(&level);
        assert_eq!(
            apply(&level, &start, Dir::Right),
            Err(MoveError::Blocked),
            "stone pushed onto {blocker:?}"
        );
    }
}

// —— Ink pool ———————————————————————————————————————————————————————————

#[test]
fn a_pool_sends_the_spirit_out_of_its_twin_in_the_same_direction() {
    let text = "
        @.1..
        .....
        .1..#
        ....G
    ";
    let level = level(text);
    let (state, events) = apply(&level, &State::initial(&level), Dir::Right).unwrap();
    assert!(events.contains(&Event::Teleport {
        from: (2, 0),
        to: (1, 2)
    }));
    assert_eq!(state.spirit, (3, 2));
}

#[test]
fn the_spirit_rests_on_the_twin_when_blocked_beyond_it() {
    let (_, state) = play("@1.1#G", "R");
    assert_eq!(state.spirit, (3, 0));
    // Leaving the twin does not teleport, but sliding back into the first
    // pool would send it out of the twin again, forever.
    let level = level("@1.1#G");
    assert_eq!(apply(&level, &state, Dir::Left), Err(MoveError::Endless));
}

#[test]
fn pools_light_lanterns_on_both_ends() {
    let text = "
        .L...
        @1.1#
        ...L.
        G....
    ";
    let (level, state) = play(text, "R");
    assert_eq!(state.spirit, (3, 1));
    assert_eq!(state.lit, level.all_lit());
}

#[test]
fn pools_in_a_cycle_refuse_the_move() {
    let text = "
        3@1G
        1.2.
        2.3.
    ";
    let level = level(text);
    assert_eq!(
        apply(&level, &State::initial(&level), Dir::Right),
        Err(MoveError::Endless)
    );
}

// —— Paper bridge ———————————————————————————————————————————————————————

#[test]
fn a_bridge_tears_after_the_spirit_crosses_it() {
    let text = "@=.#\n...G";
    let level = level(text);
    let (state, events) = apply(&level, &State::initial(&level), Dir::Right).unwrap();
    assert_eq!(state.spirit, (2, 0));
    assert!(state.is_torn(0));
    assert!(events.contains(&Event::Tore { bridge: 0 }));
    assert_eq!(board(&level, &state), lines("\n.x@#\n...O"));
    // Now a hole: the way back stops before it.
    assert_eq!(apply(&level, &state, Dir::Left), Err(MoveError::Blocked));
}

#[test]
fn a_bridge_tears_only_when_the_spirit_leaves_it() {
    let text = "@=#\n...\nG..";
    let (_, state) = play(text, "R");
    assert_eq!(state.spirit, (1, 0));
    assert!(!state.is_torn(0));
    let (_, state) = play(text, "RD");
    assert!(state.is_torn(0));
}

// —— Game: undo, restart, par ———————————————————————————————————————————

#[test]
fn undo_and_restart() {
    let mut game = Game::new(level("@..L\n....\n...G"));
    assert_eq!(game.moves(), 0);
    assert!(!game.undo());

    game.play(Dir::Right).unwrap();
    game.play(Dir::Down).unwrap();
    assert_eq!(game.moves(), 2);
    assert_eq!(game.state().spirit, (2, 2));

    assert!(game.undo());
    assert_eq!(game.moves(), 1);
    assert_eq!(game.state().spirit, (2, 0));

    // A refused move is not counted.
    assert_eq!(game.play(Dir::Right), Err(MoveError::Blocked));
    assert_eq!(game.moves(), 1);

    game.restart();
    assert_eq!(game.moves(), 0);
    assert_eq!(game.state(), &State::initial(game.level()));
}

#[test]
fn undo_restores_stones_lanterns_and_bridges() {
    let mut game = Game::new(level("@=.o.\n..L..\n....G"));
    let before = game.state().clone();
    game.play(Dir::Right).unwrap();
    assert_ne!(game.state(), &before);
    game.undo();
    assert_eq!(game.state(), &before);
}

#[test]
fn winning_within_par_earns_the_star() {
    let mut game = Game::new(level("@..\n.L.\n..G"));
    assert!(!game.within_par(2));
    game.play(Dir::Down).unwrap();
    game.play(Dir::Right).unwrap();
    assert!(game.is_won());
    assert!(game.within_par(2));
    assert!(!game.within_par(1));
}
