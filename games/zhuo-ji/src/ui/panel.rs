//! Panneau "premium" : fond vert jade, double bordure or, diamants aux
//! 4 coins. Style partagé par le menu titre, l'écran d'aide et les
//! modals de fin de main / fin de match.

use ember::prelude::*;

use crate::config::Colors;

/// Dessine un panneau décoratif avec un fond jade, une double bordure
/// dorée et quatre petits diamants aux coins.
///
/// `alpha` s'applique à tout (utile pour les fade-in des modals).
/// `diamond_size` contrôle la taille des losanges d'angle.
pub fn draw_premium_panel(
    gfx: &mut dyn RenderingServer,
    rect: Rect,
    alpha: f32,
    diamond_size: f32,
    colors: &Colors,
) {
    let (px, py, pw, ph) = (rect.x, rect.y, rect.w, rect.h);

    // Fond jade.
    gfx.draw_rect(
        Rect::new(px, py, pw, ph),
        Color::new(0.10, 0.18, 0.13, alpha),
    );

    // Double bordure dorée.
    gfx.draw_rect_outline(
        Rect::new(px, py, pw, ph),
        3.0,
        with_alpha(colors.gold_dark, alpha),
    );
    gfx.draw_rect_outline(
        Rect::new(px + 4.0, py + 4.0, pw - 8.0, ph - 8.0),
        1.5,
        with_alpha(colors.gold_light, alpha),
    );

    // Diamants aux 4 coins.
    let gold_light = with_alpha(colors.gold_light, alpha);
    let gold_dark = with_alpha(colors.gold_dark, alpha);
    for (dx, dy) in [
        (px + 12.0, py + 12.0),
        (px + pw - 12.0, py + 12.0),
        (px + 12.0, py + ph - 12.0),
        (px + pw - 12.0, py + ph - 12.0),
    ] {
        gfx.draw_regular_polygon(
            Vec2::new(dx, dy),
            4,
            diamond_size,
            45.0_f32.to_radians(),
            gold_light,
        );
        gfx.draw_regular_polygon_outline(
            Vec2::new(dx, dy),
            4,
            diamond_size,
            45.0_f32.to_radians(),
            1.0,
            gold_dark,
        );
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}
