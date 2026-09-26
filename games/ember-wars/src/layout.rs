//! Layout runtime — positions calculées depuis `screen_width/height`.

use macroquad::prelude::{screen_height, screen_width};

pub fn scr_w() -> f32 {
    screen_width()
}

pub fn scr_h() -> f32 {
    screen_height()
}

pub fn cx() -> f32 {
    scr_w() * 0.5
}

pub fn cy() -> f32 {
    scr_h() * 0.5
}

/// Hauteur de la spawn bar en bas de l'écran.
pub const SPAWN_BAR_H: f32 = 90.0;

/// Largeur du panneau d'upgrades à droite.
pub const UPGRADE_PANEL_W: f32 = 220.0;

/// Position Y par défaut du sol (valeur de référence pour tests).
///
/// **À l'exécution**, `Game.ground_y` est calculé à partir de
/// `GameContext.viewport_h` (typiquement `screen_height() * 0.78`).
/// Cette constante n'est utilisée que par les tests headless.
pub const GROUND_Y: f32 = 480.0;