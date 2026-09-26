//! ember-wars — boucle principale.

use std::path::Path;

use ember_wars::config::GameContext;
use ember_wars::menu::{self, AppScreen, MenuState};
use ember_wars::progress::{self, stars_for_victory};
use ember_wars::render;
use ember_wars::systems::{Game, Phase};
use ember_wars::tower::ShotKind;
use ember_wars::ui::spawn_bar::SpawnBar;
use ember_wars::ui::upgrade_modal;
use macroquad::prelude::*;

#[macroquad::main("ember-wars")]
async fn main() {
    // Charge DejaVu Sans Mono comme police par défaut (Unicode complet :
    // ★☆, accents, symboles étendus). Fallback silencieux vers la police
    // par défaut de macroquad si le fichier est absent.
    ember_wars::fonts::load_default_font().await;

    let mut ctx = GameContext::from_env();
    ctx.viewport_w = screen_width();
    ctx.viewport_h = screen_height();

    let (mut progress_data, progress_store) =
        progress::load(Path::new(env!("CARGO_MANIFEST_DIR")));

    let mut screen = AppScreen::Menu;
    let mut menu_state = MenuState::new();
    let mut game: Option<Game> = None;
    let spawn_bar = SpawnBar::new();
    let mut last_level_id: Option<String> = None;
    let mut level_resolved = false;

    loop {
        let dt = get_frame_time().min(0.05);
        clear_background(ctx.colors.bg);

        match screen {
            AppScreen::Menu => {
                // Tab : toggle la modal d'upgrades dans le menu.
                if is_key_pressed(KeyCode::Tab) {
                    menu_state.upgrade_modal_open = !menu_state.upgrade_modal_open;
                    if menu_state.upgrade_modal_open && menu_state.upgrade_focus.is_none() {
                        menu_state.upgrade_focus = Some(0);
                    }
                }
                if menu_state.upgrade_modal_open && is_key_pressed(KeyCode::Escape) {
                    menu_state.upgrade_modal_open = false;
                }

                if menu_state.upgrade_modal_open {
                    let bought = upgrade_modal::handle_modal_input(
                        &mut progress_data.tree,
                        &mut menu_state.upgrade_focus,
                    );
                    if bought {
                        progress_store.save(&progress_data);
                    }
                    menu::draw_menu(&ctx, &menu_state, &progress_data);
                    upgrade_modal::draw_modal(
                        &ctx,
                        &progress_data.tree,
                        menu_state.upgrade_focus,
                    );
                } else {
                    menu::draw_menu(&ctx, &menu_state, &progress_data);
                    if let Some(level_id) =
                        menu::handle_menu_input(&mut menu_state, &progress_data)
                    {
                        match Game::new(&ctx, &level_id, seed_from_time(), &progress_data) {
                            Ok(g) => {
                                last_level_id = Some(level_id);
                                level_resolved = false;
                                game = Some(g);
                                screen = AppScreen::Game;
                            }
                            Err(e) => {
                                eprintln!("Game::new failed: {e}");
                            }
                        }
                    }
                }
            }
            AppScreen::Game => {
                if let Some(g) = game.as_mut() {
                    g.camera.viewport_w = screen_width();

                    if g.phase == Phase::Playing {
                        let mut cam_dir = 0.0;
                        if is_key_down(KeyCode::Left) {
                            cam_dir -= 1.0;
                        }
                        if is_key_down(KeyCode::Right) {
                            cam_dir += 1.0;
                        }

                        let mouse_screen = Vec2::new(mouse_position().0, mouse_position().1);
                        let mouse_world = Vec2::new(
                            g.camera.screen_to_world(mouse_screen.x),
                            mouse_screen.y,
                        );

                        let mut consumed = false;
                        if is_mouse_button_pressed(MouseButton::Left)
                            && let Some(kind) = spawn_bar.handle_click(g)
                        {
                            g.try_player_spawn(&kind);
                            consumed = true;
                        }

                        let fire_click =
                            is_mouse_button_pressed(MouseButton::Left) && !consumed;
                        g.tick(dt, mouse_world, fire_click, cam_dir);
                    }

                    let shake = g.juice.screen_shake.offset();

                    render::draw_game(&ctx, g, shake);
                    spawn_bar.draw(&ctx, g);
                    render::draw_end_overlay(&ctx, g);

                    // Résolution de fin de niveau (une seule fois).
                    if g.phase != Phase::Playing && !level_resolved {
                        level_resolved = true;
                        let level_id = g.level.id.clone();
                        let gold_earned = g.stats.gold_earned;
                        let reward = if g.phase == Phase::Won {
                            g.level.reward_gold
                        } else {
                            0.0
                        };
                        // Sync bank (gold total) + reward + stars.
                        progress_data.tree.gold = g.player_upgrades.gold + reward;
                        if g.phase == Phase::Won {
                            let stars = stars_for_victory(g.tower_hp_fraction());
                            // NB : record_win crédite aussi `gold_earned`,
                            // mais ici on a déjà mis la bank à jour. On
                            // utilise une version qui n'ajoute pas d'or.
                            let prev = progress_data.stars_for(&level_id);
                            if stars > prev {
                                progress_data.stars.insert(level_id, stars);
                            }
                        }
                        let _ = gold_earned;
                        progress_store.save(&progress_data);
                    }
                }
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            match screen {
                AppScreen::Menu => {
                    if menu_state.upgrade_modal_open {
                        menu_state.upgrade_modal_open = false;
                    } else {
                        break;
                    }
                }
                AppScreen::Game => {
                    screen = AppScreen::Menu;
                    game = None;
                }
            }
        }

        if is_key_pressed(KeyCode::R)
            && screen == AppScreen::Game
            && let Some(level_id) = last_level_id.as_ref()
            && let Ok(new_game) =
                Game::new(&ctx, level_id, seed_from_time(), &progress_data)
        {
            game = Some(new_game);
            level_resolved = false;
        }

        if is_key_pressed(KeyCode::Enter)
            && screen == AppScreen::Game
            && let Some(g) = game.as_ref()
            && g.phase != Phase::Playing
        {
            screen = AppScreen::Menu;
            game = None;
        }

        if screen == AppScreen::Game
            && let Some(g) = game.as_mut()
        {
            if is_key_pressed(KeyCode::Key1) {
                g.turret.shot_kind = ShotKind::Basic;
            }
            if is_key_pressed(KeyCode::Key2) {
                g.turret.shot_kind = ShotKind::Piercing;
            }
            if is_key_pressed(KeyCode::Key3) {
                g.turret.shot_kind = ShotKind::Explosive;
            }
        }

        next_frame().await;
    }
}

fn seed_from_time() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32 ^ d.subsec_nanos())
        .unwrap_or(0xC0FFEE)
}