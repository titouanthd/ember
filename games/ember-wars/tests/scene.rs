//! Headless run of the node-based Ember Wars on mock backends.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ember::prelude::*;
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use ember_wars::catapult::ShotKind;
use ember_wars::config::GameContext;
use ember_wars::menu::{self, AppScreen, CONTENT_TOP};
use ember_wars::progress::Progress;
use ember_wars::scene::{EmberWars, WINDOW_H, WINDOW_W, build_tree};
use ember_wars::world::{CatapultNode, ProjectileNode, UnitNode, World};
use ember_wars::{Phase, Team, frame};

type Loop = SceneLoop<MockRenderer, MockAudio>;

fn fixed_seed() -> u32 {
    0xC0FFEE
}

/// A fresh campaign whose saves are counted, never written.
fn game_counting(saves: Arc<AtomicUsize>) -> Loop {
    let save = Box::new(move |_: &Progress| {
        saves.fetch_add(1, Ordering::SeqCst);
    });
    let mut l = SceneLoop::new(
        build_tree(
            GameContext::from_env(),
            Progress::default(),
            save,
            fixed_seed,
        ),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    )
    .with_viewport_following_window();
    assert!(l.init());
    l.resize(WINDOW_W, WINDOW_H);
    l
}

fn game() -> Loop {
    game_counting(Arc::new(AtomicUsize::new(0)))
}

/// One frame: draw (so the viewport is the window), then process.
fn frame_with(l: &mut Loop, input: Input, dt: f32) -> bool {
    l.draw();
    l.set_input(input);
    let running = l.process(dt);
    l.set_input(Input::default());
    running
}

fn step(l: &mut Loop, input: Input) -> bool {
    frame_with(l, input, 1.0 / 60.0)
}

fn press(l: &mut Loop, key: Key) -> bool {
    step(
        l,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    )
}

fn click_at(l: &mut Loop, pos: Vec2) {
    assert!(step(
        l,
        Input {
            mouse_pos: pos,
            mouse_left_pressed: true,
            mouse_left_down: true,
            ..Input::default()
        }
    ));
}

fn wars(l: &mut Loop) -> &mut EmberWars {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<EmberWars>()
        .unwrap()
}

/// The level being played.
fn world(l: &mut Loop) -> &mut World {
    let id = wars(l).world.expect("a level is being played");
    l.tree
        .get_mut(id)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<World>()
        .unwrap()
}

/// Every node of type `T`.
fn nodes<T: Node>(l: &Loop) -> Vec<&T> {
    l.tree
        .iter()
        .filter_map(|(_, n)| n.as_any().downcast_ref::<T>())
        .collect()
}

fn player_catapult(l: &Loop) -> &CatapultNode {
    nodes::<CatapultNode>(l)
        .into_iter()
        .find(|c| c.team == Team::Player)
        .unwrap()
}

/// The first level's card, clicked from the menu.
fn start_first_level(l: &mut Loop) {
    step(l, Input::default());
    let card = menu::level_rect(
        0,
        frame::screen_width(),
        menu::chapter_top_y(0, CONTENT_TOP),
    );
    click_at(l, card.center());
    assert_eq!(wars(l).app.screen, AppScreen::Game);
    assert!(wars(l).world.is_some());
}

#[test]
fn ready_installs_the_default_font() {
    let l = game();
    assert!(
        l.rendering()
            .calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::SetDefaultFont(Some(_))))
    );
}

#[test]
fn the_viewport_follows_the_window() {
    let mut l = game();
    l.resize(1200.0, 700.0);
    step(&mut l, Input::default());
    let ctx = &wars(&mut l).ctx;
    assert_eq!((ctx.viewport_w, ctx.viewport_h), (1200.0, 700.0));
    assert_eq!(frame::screen_width(), 1200.0);
}

#[test]
fn clicking_the_first_level_starts_it_and_the_battle_draws() {
    let mut l = game();
    start_first_level(&mut l);
    for _ in 0..120 {
        assert!(step(&mut l, Input::default()));
    }
    l.draw();
    let calls = l.rendering().calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawRect { .. }))
    );
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawText { .. }))
    );
}

#[test]
fn number_keys_pick_the_shot() {
    let mut l = game();
    start_first_level(&mut l);
    press(&mut l, Key::Num2);
    let shot = |l: &Loop| player_catapult(l).catapult.as_ref().unwrap().shot_kind;
    assert_eq!(shot(&l), ShotKind::Piercing);
    press(&mut l, Key::Num3);
    assert_eq!(shot(&l), ShotKind::Explosive);
}

#[test]
fn tab_toggles_the_upgrade_modal() {
    let mut l = game();
    step(&mut l, Input::default());
    assert!(press(&mut l, Key::Tab));
    assert!(wars(&mut l).app.menu_state.upgrade_modal_open);
    assert!(press(&mut l, Key::Tab));
    assert!(!wars(&mut l).app.menu_state.upgrade_modal_open);
    // Escape on the menu quits.
    assert!(!press(&mut l, Key::Escape));
}

/// Kept from the macroquad loop: with the modal open, one Escape closes
/// it in the menu branch, and the loop's later Escape check then sees the
/// menu with no modal and quits, in the same frame.
#[test]
fn escape_with_the_modal_open_still_quits() {
    let mut l = game();
    step(&mut l, Input::default());
    assert!(press(&mut l, Key::Tab));
    assert!(!press(&mut l, Key::Escape));
}

#[test]
fn a_finished_level_is_saved_once_and_enter_returns_to_the_menu() {
    let saves = Arc::new(AtomicUsize::new(0));
    let mut l = game_counting(saves.clone());
    start_first_level(&mut l);
    // End the battle: the player's tower falls.
    world(&mut l).game.phase = Phase::Lost;
    assert!(step(&mut l, Input::default()));
    assert!(step(&mut l, Input::default()));
    assert_eq!(saves.load(Ordering::SeqCst), 1);
    assert!(press(&mut l, Key::Enter));
    assert_eq!(wars(&mut l).app.screen, AppScreen::Menu);
    assert!(wars(&mut l).world.is_none());
    assert!(nodes::<World>(&l).is_empty());
}

#[test]
fn a_level_is_a_world_with_its_catapult_as_a_node() {
    let mut l = game();
    start_first_level(&mut l);
    let world_id = wars(&mut l).world.unwrap();
    let root = l.tree.root().unwrap();
    assert_eq!(l.tree.parent(world_id), Some(root));
    for path in ["Catapults/PlayerCatapult", "Units", "Projectiles"] {
        assert!(l.tree.find(world_id, path).is_some(), "{path}");
    }
    // The first level has no enemy catapult.
    assert!(l.tree.find(world_id, "Catapults/EnemyCatapult").is_none());
    // Between frames the catapult is in its node, not in the battle.
    step(&mut l, Input::default());
    assert!(player_catapult(&l).catapult.is_some());
}

#[test]
fn spawned_units_and_shots_become_nodes_and_dead_ones_go() {
    let mut l = game();
    start_first_level(&mut l);
    // Spawn a grunt from the bar, then fire the catapult.
    let slot = ember_wars::ui::spawn_bar::SpawnBar::slot_rect(0, 6, WINDOW_W, WINDOW_H);
    click_at(&mut l, slot.center());
    click_at(&mut l, Vec2::new(900.0, WINDOW_H * 0.6));
    let units = nodes::<UnitNode>(&l);
    assert!(
        units
            .iter()
            .any(|u| u.unit.as_ref().is_some_and(|u| u.team == Team::Player))
    );
    assert!(!nodes::<ProjectileNode>(&l).is_empty());
    // Every node holds its data between frames, and each unit has one.
    assert!(units.iter().all(|u| u.unit.is_some()));
    let ids: std::collections::HashSet<_> = units.iter().map(|u| u.id).collect();
    assert_eq!(ids.len(), units.len());

    // A unit that dies loses its node at the end of the frame.
    let doomed = units[0].id;
    let world_id = wars(&mut l).world.unwrap();
    let world_units = l.tree.find(world_id, "Units").unwrap();
    for id in l.tree.children(world_units).to_vec() {
        if let Some(n) = l
            .tree
            .get_mut(id)
            .and_then(|n| n.as_any_mut().downcast_mut::<UnitNode>())
            && n.id == doomed
            && let Some(u) = n.unit.as_mut()
        {
            u.hp = 0.0;
        }
    }
    step(&mut l, Input::default());
    assert!(nodes::<UnitNode>(&l).iter().all(|u| u.id != doomed));
}

#[test]
fn r_swaps_in_a_fresh_world() {
    let mut l = game();
    start_first_level(&mut l);
    let slot = ember_wars::ui::spawn_bar::SpawnBar::slot_rect(0, 6, WINDOW_W, WINDOW_H);
    click_at(&mut l, slot.center());
    let before = wars(&mut l).world.unwrap();
    assert!(!nodes::<UnitNode>(&l).is_empty());
    assert!(press(&mut l, Key::R));
    let after = wars(&mut l).world.unwrap();
    assert_ne!(before, after);
    assert!(!l.tree.contains(before));
    assert_eq!(nodes::<World>(&l).len(), 1);
    assert!(nodes::<UnitNode>(&l).is_empty());
    assert_eq!(world(&mut l).game.stats.elapsed, 0.0);
}
