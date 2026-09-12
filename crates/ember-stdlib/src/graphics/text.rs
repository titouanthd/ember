// ember_stdlib/src/graphics/text.rs
//! Helpers pour le rendu de texte.

use macroquad::prelude::*;

/// Dessine un texte centré horizontalement autour de `center_x`.
///
/// Le paramètre `y` correspond à la **ligne de base** (comme `draw_text`).
///
/// # Exemple
/// ```ignore
/// # use ember_stdlib::graphics::text::draw_centered;
/// # use macroquad::prelude::*;
/// draw_centered("Score: 42", 400.0, 30.0, 28, WHITE);
/// ```
pub fn draw_centered(text: &str, center_x: f32, y: f32, font_size: u16, color: Color) {
    let size = measure_text(text, None, font_size, 1.0);
    draw_text(
        text,
        center_x - size.width / 2.0,
        y,
        font_size as f32,
        color,
    );
}

/// Dessine un texte centré avec une ombre portée décalée.
///
/// Utile pour les HUD sur fond clair, ou pour un style "rétro".
pub fn draw_centered_shadowed(
    text: &str,
    center_x: f32,
    y: f32,
    font_size: u16,
    color: Color,
    shadow_color: Color,
    shadow_offset: f32,
) {
    let size = measure_text(text, None, font_size, 1.0);
    let x = center_x - size.width / 2.0;
    draw_text(text, x + shadow_offset, y + shadow_offset, font_size as f32, shadow_color);
    draw_text(text, x, y, font_size as f32, color);
}

#[cfg(test)]
mod tests {
    // Ces fonctions dépendent de macroquad pour `measure_text`, qui
    // nécessite un contexte graphique. On ne peut donc pas les tester
    // unitairement sans un harnais macroquad complet.
    //
    // On se contente d'un test qui compile (vérifie que la signature
    // est correcte et que le module est bien exporté).
    #[test]
    fn test_module_compiles() {
        // Rien à tester, mais on garde un placeholder.
    }
}