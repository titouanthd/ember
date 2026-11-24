//! Headless run of the node-based Simon on mock backends.

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedAudioCall};
use simon::GameState;
use simon::components::SimonColor;
use simon::config::load_config;
use simon::scene::{ColorPad, Simon, build_tree};
use simon::systems::PlayPhase;

type Game = SceneLoop<MockRenderer, MockAudio>;

const DT: f32 = 1.0 / 60.0;

fn game() -> Game {
    let cfg = load_config();
    let (w, h) = (cfg.window_w, cfg.window_h);
    let mut g = SceneLoop::new(
        build_tree(cfg, None),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    assert!(g.init());
    g.draw(); // the renderer knows the viewport from the first frame on
    step(&mut g, Input::default());
    g
}

/// One frame, as the window runner does it: input, process, draw.
fn step(g: &mut Game, input: Input) {
    g.set_input(input);
    assert!(g.process(DT));
    g.draw();
}

fn press(g: &mut Game, key: Key) {
    step(
        g,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    );
}

fn simon(g: &mut Game) -> &mut Simon {
    let root = g.tree.root().unwrap();
    g.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Simon>()
        .unwrap()
}

fn pads(g: &Game) -> Vec<NodeId> {
    let ui = simon_ui(g);
    ui.pads.to_vec()
}

fn simon_ui(g: &Game) -> simon::scene::Ui {
    let root = g.tree.root().unwrap();
    g.tree
        .get(root)
        .unwrap()
        .as_any()
        .downcast_ref::<Simon>()
        .unwrap()
        .ui()
        .unwrap()
}

fn pad(g: &Game, color: SimonColor) -> &ColorPad {
    let id = pads(g)[color.index()];
    g.tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<ColorPad>()
        .unwrap()
}

fn visible(g: &Game, id: NodeId) -> bool {
    g.tree.get(id).unwrap().visible()
}

fn label(g: &Game, id: NodeId) -> String {
    g.tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<Label>()
        .unwrap()
        .text
        .clone()
}

fn plays(g: &Game) -> usize {
    g.audio()
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedAudioCall::Play { .. }))
        .count()
}

/// Left click (press frame, release frame) at `pos`.
fn click_at(g: &mut Game, pos: Vec2) {
    step(
        g,
        Input {
            mouse_pos: pos,
            mouse_left_pressed: true,
            mouse_left_down: true,
            ..Input::default()
        },
    );
    step(
        g,
        Input {
            mouse_pos: pos,
            mouse_left_released: true,
            ..Input::default()
        },
    );
}

fn click(g: &mut Game, color: SimonColor) {
    let r = pad(g, color).rect();
    click_at(g, Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5));
}

/// Run frames until the player may click.
fn until_waiting(g: &mut Game) {
    for _ in 0..10_000 {
        if simon(g).world.phase.is_accepting_input() {
            return;
        }
        step(g, Input::default());
    }
    panic!("never reached Waiting");
}

fn start(g: &mut Game) {
    press(g, Key::Space);
    assert_eq!(simon(g).state, GameState::Playing);
}

#[test]
fn opens_on_the_start_screen_with_sounds_loaded() {
    let g = game();
    let ui = simon_ui(&g);
    assert!(visible(&g, ui.start));
    assert!(!visible(&g, ui.play));
    assert!(!visible(&g, ui.game_over));
    let loaded = g
        .audio()
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedAudioCall::LoadClip(_)))
        .count();
    assert_eq!(loaded, 4);
}

#[test]
fn space_starts_and_shows_the_playfield() {
    let mut g = game();
    start(&mut g);
    let ui = simon_ui(&g);
    assert!(!visible(&g, ui.start));
    assert!(visible(&g, ui.play));
    assert_eq!(label(&g, ui.status), "Get ready...");
    assert_eq!(label(&g, ui.score), "Score: 0");
}

#[test]
fn pads_sit_on_the_layout_circles() {
    let g = game();
    let cfg = load_config();
    let centers = simon::components::button_centers(&cfg);
    for color in SimonColor::ALL {
        let r = pad(&g, color).rect();
        let c = centers[color.index()];
        assert_eq!(
            r,
            Rect::new(
                c.x - cfg.button_radius,
                c.y - cfg.button_radius,
                cfg.button_radius * 2.0,
                cfg.button_radius * 2.0
            )
        );
    }
}

#[test]
fn playback_lights_the_pad_and_plays_its_sound() {
    let mut g = game();
    start(&mut g);
    let first = simon(&mut g).world.sequence.get(0).unwrap();
    for _ in 0..1000 {
        if matches!(simon(&mut g).world.phase, PlayPhase::Showing { .. })
            && simon(&mut g).world.lit_color().is_some()
        {
            break;
        }
        step(&mut g, Input::default());
    }
    step(&mut g, Input::default()); // UI catches up on the next process
    assert!(pad(&g, first).lit);
    assert!(!pad(&g, first).enabled, "no clicks while showing");
    assert_eq!(plays(&g), 1);
}

#[test]
fn clicking_the_sequence_completes_the_round() {
    let mut g = game();
    start(&mut g);
    until_waiting(&mut g);
    step(&mut g, Input::default());
    let color = simon(&mut g).world.sequence.get(0).unwrap();
    assert!(pad(&g, color).enabled);
    let before = plays(&g);
    click(&mut g, color);
    assert_eq!(simon(&mut g).world.score, 1);
    assert_eq!(simon(&mut g).world.sequence.len(), 2);
    assert!(plays(&g) > before, "click sound");
    step(&mut g, Input::default());
    assert_eq!(label(&g, simon_ui(&g).score), "Score: 1");
}

#[test]
fn a_wrong_pad_ends_the_run_and_r_returns_to_start() {
    let mut g = game();
    start(&mut g);
    until_waiting(&mut g);
    step(&mut g, Input::default());
    let right = simon(&mut g).world.sequence.get(0).unwrap();
    let wrong = SimonColor::from_index((right.index() + 1) % 4).unwrap();
    click(&mut g, wrong);
    assert_eq!(simon(&mut g).state, GameState::GameOver);
    step(&mut g, Input::default());
    let ui = simon_ui(&g);
    assert!(visible(&g, ui.game_over));
    assert!(visible(&g, ui.play), "playfield stays under the box");
    assert_eq!(label(&g, ui.game_over_line), "Score: 0  ·  Best: 0");

    press(&mut g, Key::R);
    assert_eq!(simon(&mut g).state, GameState::Start);
    step(&mut g, Input::default());
    assert!(visible(&g, ui.start));
    assert!(!visible(&g, ui.game_over));
}

#[test]
fn clicks_outside_the_circle_or_too_early_are_ignored() {
    let mut g = game();
    start(&mut g);
    let color = simon(&mut g).world.sequence.get(0).unwrap();
    click(&mut g, color); // still in PreRound
    assert_eq!(simon(&mut g).world.score, 0);
    assert_eq!(simon(&mut g).state, GameState::Playing);

    until_waiting(&mut g);
    step(&mut g, Input::default());
    let r = pad(&g, color).rect();
    click_at(&mut g, Vec2::new(r.x + 2.0, r.y + 2.0)); // square corner
    assert_eq!(simon(&mut g).world.score, 0);
    assert_eq!(simon(&mut g).state, GameState::Playing);
}

#[test]
fn hovering_a_pad_brightens_it_only_when_clickable() {
    let mut g = game();
    start(&mut g);
    let r = pad(&g, SimonColor::Blue).rect();
    let centre = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
    let hover = Input {
        mouse_pos: centre,
        ..Input::default()
    };
    step(&mut g, hover.clone());
    let p = pad(&g, SimonColor::Blue);
    assert_eq!(
        p.is_bright(),
        p.lit,
        "hover alone does nothing before Waiting"
    );
    until_waiting(&mut g);
    step(&mut g, hover.clone());
    step(&mut g, hover);
    assert!(pad(&g, SimonColor::Blue).is_bright());
}
