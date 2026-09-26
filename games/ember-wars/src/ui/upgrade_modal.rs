//! Modal plein écran de l'arbre de progression (utilisable depuis le menu).

use glam::Vec2;
use macroquad::prelude::*;

use ember_stdlib::ui::hit;

use crate::config::GameContext;
use crate::upgrades::{load_defs, UpgradeEffect, UpgradeNode, UpgradeTree, UpgradeTreeDefs};

pub const NODE_W: f32 = 130.0;
pub const NODE_H: f32 = 46.0;
pub const COL_SPACING: f32 = 170.0;
pub const PANEL_MARGIN: f32 = 40.0;
pub const HEADER_H: f32 = 60.0;
pub const DESCRIPTION_H: f32 = 90.0;
pub const FOOTER_H: f32 = 34.0;
pub const ROW_SPACING_MIN: f32 = 32.0;
pub const ROW_SPACING_MAX: f32 = 80.0;

/// Layout calculé une fois par frame, à partir des defs et du panel.
#[derive(Debug, Clone, Copy)]
pub struct ModalLayout {
    pub panel: Rect,
    pub center_x: f32,
    pub top_y: f32,
    pub col_spacing: f32,
    pub row_spacing: f32,
    pub description_y: f32,
}

impl ModalLayout {
    pub fn compute(defs: &UpgradeTreeDefs) -> Self {
        let panel = Rect {
            x: PANEL_MARGIN,
            y: PANEL_MARGIN,
            w: screen_width() - PANEL_MARGIN * 2.0,
            h: screen_height() - PANEL_MARGIN * 2.0,
        };

        let max_row = defs.nodes.iter().map(|n| n.row).max().unwrap_or(0);
        let num_rows = max_row.max(0) as f32;

        let tree_top = panel.y + HEADER_H;
        let tree_bottom = panel.y + panel.h - DESCRIPTION_H - FOOTER_H;
        let tree_h = (tree_bottom - tree_top).max(80.0);

        let row_spacing = if num_rows > 0.0 {
            ((tree_h - NODE_H) / num_rows).clamp(ROW_SPACING_MIN, ROW_SPACING_MAX)
        } else {
            0.0
        };

        let description_y = panel.y + panel.h - DESCRIPTION_H - FOOTER_H;

        Self {
            panel,
            center_x: panel.x + panel.w * 0.5,
            top_y: tree_top + NODE_H * 0.5,
            col_spacing: COL_SPACING,
            row_spacing,
            description_y,
        }
    }

    pub fn node_center(&self, node: &UpgradeNode) -> Vec2 {
        Vec2::new(
            self.center_x + node.col as f32 * self.col_spacing,
            self.top_y + node.row as f32 * self.row_spacing,
        )
    }

    pub fn node_rect(&self, node: &UpgradeNode) -> Rect {
        let c = self.node_center(node);
        Rect {
            x: c.x - NODE_W * 0.5,
            y: c.y - NODE_H * 0.5,
            w: NODE_W,
            h: NODE_H,
        }
    }
}

pub fn draw_modal(ctx: &GameContext, tree: &UpgradeTree, focus: Option<usize>) {
    let vw = screen_width();
    let vh = screen_height();

    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.0, 0.0, 0.0, 0.82));

    let defs = load_defs();
    let layout = ModalLayout::compute(defs);
    let panel = layout.panel;

    draw_rectangle(panel.x, panel.y, panel.w, panel.h, Color::new(0.06, 0.08, 0.10, 1.0));
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2.0, ctx.colors.accent);

    let header_base = panel.y + 40.0;
    draw_text("UPGRADE TREE", panel.x + 24.0, header_base, 28.0, ctx.colors.accent);
    let gold_txt = format!("GOLD {:.0}", tree.gold);
    let dim = measure_text(&gold_txt, None, 24, 1.0);
    draw_text(
        &gold_txt,
        panel.x + panel.w - dim.width - 24.0,
        header_base,
        24.0,
        ctx.colors.gold,
    );
    draw_line(
        panel.x + 16.0,
        panel.y + HEADER_H - 4.0,
        panel.x + panel.w - 16.0,
        panel.y + HEADER_H - 4.0,
        1.0,
        Color::new(0.25, 0.28, 0.34, 1.0),
    );

    // Connexions.
    for node in &defs.nodes {
        let to = layout.node_center(node);
        for req in &node.requires {
            if let Some(req_node) = defs.node(req) {
                let from = layout.node_center(req_node);
                let purchased = tree.is_purchased(&node.id) && tree.is_purchased(req);
                let color = if purchased {
                    Color::new(0.4, 0.85, 0.4, 0.85)
                } else {
                    Color::new(0.30, 0.32, 0.38, 0.7)
                };
                draw_line(from.x, from.y, to.x, to.y, 2.0, color);
            }
        }
    }

    // Nœuds.
    let mouse = Vec2::new(mouse_position().0, mouse_position().1);
    for (i, node) in defs.nodes.iter().enumerate() {
        let r = layout.node_rect(node);
        let purchased = tree.is_purchased(&node.id);
        let accessible = tree.is_accessible(&node.id);
        let can_buy = tree.can_buy(&node.id);
        let hovered = hit::contains(r, mouse);
        let focused = focus == Some(i);

        let (bg, border) = if purchased {
            (
                Color::new(0.10, 0.22, 0.12, 1.0),
                Color::new(0.40, 0.85, 0.40, 1.0),
            )
        } else if can_buy {
            (Color::new(0.14, 0.18, 0.26, 1.0), ctx.colors.accent)
        } else if accessible {
            (
                Color::new(0.14, 0.14, 0.16, 1.0),
                Color::new(0.55, 0.55, 0.60, 1.0),
            )
        } else {
            (
                Color::new(0.09, 0.09, 0.11, 1.0),
                Color::new(0.25, 0.25, 0.30, 1.0),
            )
        };

        let border_color = if hovered || focused { WHITE } else { border };
        let border_width = if hovered || focused { 2.5 } else { 1.5 };

        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, border_width, border_color);

        let name = truncate(&node.name, 15);
        let dim = measure_text(&name, None, 13, 1.0);
        let text_color = if purchased {
            Color::new(0.7, 1.0, 0.7, 1.0)
        } else if accessible {
            ctx.colors.text
        } else {
            Color::new(0.45, 0.45, 0.50, 1.0)
        };
        draw_text(
            &name,
            r.x + r.w * 0.5 - dim.width * 0.5,
            r.y + 18.0,
            13.0,
            text_color,
        );

        let sub = if purchased {
            "OWNED".to_string()
        } else {
            format!("{:.0}g", node.cost)
        };
        let sub_color = if purchased {
            Color::new(0.5, 0.9, 0.5, 1.0)
        } else if can_buy {
            ctx.colors.gold
        } else {
            Color::new(0.45, 0.45, 0.50, 1.0)
        };
        let dsub = measure_text(&sub, None, 11, 1.0);
        draw_text(
            &sub,
            r.x + r.w * 0.5 - dsub.width * 0.5,
            r.y + 34.0,
            11.0,
            sub_color,
        );
    }

    draw_description_panel(ctx, tree, &layout, defs, focus);
}

fn draw_description_panel(
    ctx: &GameContext,
    tree: &UpgradeTree,
    layout: &ModalLayout,
    defs: &UpgradeTreeDefs,
    focus: Option<usize>,
) {
    let panel = layout.panel;
    let desc_y = layout.description_y;
    let desc_h = DESCRIPTION_H;
    let footer_h = FOOTER_H;

    draw_line(
        panel.x + 16.0,
        desc_y - 8.0,
        panel.x + panel.w - 16.0,
        desc_y - 8.0,
        1.0,
        Color::new(0.25, 0.28, 0.34, 1.0),
    );

    let desc_rect_x = panel.x + 16.0;
    let desc_rect_w = panel.w - 32.0;
    draw_rectangle(
        desc_rect_x,
        desc_y,
        desc_rect_w,
        desc_h - 6.0,
        Color::new(0.04, 0.05, 0.07, 1.0),
    );
    draw_rectangle_lines(
        desc_rect_x,
        desc_y,
        desc_rect_w,
        desc_h - 6.0,
        1.0,
        Color::new(0.22, 0.25, 0.30, 1.0),
    );

    if let Some(idx) = focus
        && let Some(node) = defs.nodes.get(idx)
    {
        let title = format!("{}  —  {}", node.name, describe_effect(&node.effect));
        draw_text(&title, desc_rect_x + 12.0, desc_y + 26.0, 20.0, ctx.colors.text);

        let sub = describe_state(node, tree);
        let sub_color = if tree.is_purchased(&node.id) {
            Color::new(0.5, 0.9, 0.5, 1.0)
        } else if !tree.is_accessible(&node.id) {
            Color::new(0.65, 0.65, 0.70, 1.0)
        } else if tree.can_buy(&node.id) {
            ctx.colors.gold
        } else {
            ctx.colors.accent
        };
        draw_text(&sub, desc_rect_x + 12.0, desc_y + 54.0, 16.0, sub_color);
    } else {
        draw_text(
            "Hover or navigate to a node to see details.",
            desc_rect_x + 12.0,
            desc_y + 34.0,
            16.0,
            Color::new(0.55, 0.55, 0.60, 1.0),
        );
    }

    let footer_y = panel.y + panel.h - footer_h;
    draw_rectangle(
        panel.x + 1.0,
        footer_y,
        panel.w - 2.0,
        footer_h - 1.0,
        Color::new(0.03, 0.04, 0.05, 1.0),
    );
    let hint = "[Arrows] navigate   [Enter] buy   [Tab/Esc] close";
    let dim = measure_text(hint, None, 14, 1.0);
    draw_text(
        hint,
        panel.x + panel.w * 0.5 - dim.width * 0.5,
        footer_y + 22.0,
        14.0,
        ctx.colors.text,
    );
}

/// Gère l'input modal. Retourne `true` si un achat a eu lieu
/// (à persister). Le focus est passé par référence pour être conservé
/// entre les frames côté appelant.
pub fn handle_modal_input(tree: &mut UpgradeTree, focus: &mut Option<usize>) -> bool {
    let defs = load_defs();
    let layout = ModalLayout::compute(defs);
    let mouse = Vec2::new(mouse_position().0, mouse_position().1);
    let total = defs.nodes.len();
    let mut purchased = false;

    for (i, node) in defs.nodes.iter().enumerate() {
        if hit::contains(layout.node_rect(node), mouse) {
            *focus = Some(i);
        }
    }

    if is_mouse_button_pressed(MouseButton::Left) {
        for node in &defs.nodes {
            if hit::contains(layout.node_rect(node), mouse) {
                if tree.can_buy(&node.id) {
                    tree.buy(&node.id);
                    purchased = true;
                }
                return purchased;
            }
        }
    }

    let pressed_left = is_key_pressed(KeyCode::Left);
    let pressed_right = is_key_pressed(KeyCode::Right);
    let pressed_up = is_key_pressed(KeyCode::Up);
    let pressed_down = is_key_pressed(KeyCode::Down);
    if pressed_left || pressed_right || pressed_up || pressed_down {
        let current = focus.unwrap_or(0);
        let (dx, dy) = if pressed_left {
            (-1, 0)
        } else if pressed_right {
            (1, 0)
        } else if pressed_up {
            (0, -1)
        } else {
            (0, 1)
        };
        if let Some(next) = find_neighbor(defs, current, dx, dy) {
            *focus = Some(next);
        } else {
            *focus = Some(current.min(total.saturating_sub(1)));
        }
    }

    if is_key_pressed(KeyCode::Enter)
        && let Some(idx) = *focus
        && let Some(node) = defs.nodes.get(idx)
        && tree.can_buy(&node.id)
    {
        tree.buy(&node.id);
        purchased = true;
    }

    purchased
}

fn find_neighbor(
    defs: &UpgradeTreeDefs,
    from_idx: usize,
    dx: i32,
    dy: i32,
) -> Option<usize> {
    let from = defs.nodes.get(from_idx)?;
    let mut best: Option<(f32, usize)> = None;

    for (i, node) in defs.nodes.iter().enumerate() {
        if i == from_idx {
            continue;
        }
        let ddx = node.col - from.col;
        let ddy = node.row - from.row;

        let in_dir = (dx > 0 && ddx > 0)
            || (dx < 0 && ddx < 0)
            || (dy > 0 && ddy > 0)
            || (dy < 0 && ddy < 0);
        if !in_dir {
            continue;
        }

        let primary = (ddx.abs() + ddy.abs()) as f32;
        let secondary = if dx != 0 { ddy.abs() as f32 } else { ddx.abs() as f32 };
        let score = primary + secondary * 2.0;

        if best.is_none_or(|(s, _)| score < s) {
            best = Some((score, i));
        }
    }

    best.map(|(_, i)| i)
}

fn describe_effect(effect: &UpgradeEffect) -> String {
    match effect {
        UpgradeEffect::None => "Root node".to_string(),
        UpgradeEffect::TowerHpMult(m) => format!("Tower HP ×{:.2}", m),
        UpgradeEffect::TowerRegen(v) => format!("Tower +{:.1} HP/s", v),
        UpgradeEffect::Fortress => "Second HP bar".to_string(),
        UpgradeEffect::UnitHpMult(m) => format!("Unit HP ×{:.2}", m),
        UpgradeEffect::UnitDamageMult(m) => format!("Unit damage ×{:.2}", m),
        UpgradeEffect::UnlockUnit(k) => format!("Unlocks {}", k),
        UpgradeEffect::TurretDamageMult(m) => format!("Turret damage ×{:.2}", m),
        UpgradeEffect::TurretFireRateMult(m) => format!("Fire rate ×{:.2}", m),
        UpgradeEffect::MultiShot(n) => format!("{} projectiles/shot", n),
        UpgradeEffect::ManaRegenMult(m) => format!("Mana regen ×{:.2}", m),
        UpgradeEffect::ManaCapMult(m) => format!("Mana cap ×{:.2}", m),
        UpgradeEffect::GoldPerKillAdd(v) => format!("+{:.0} gold/kill", v),
    }
}

fn describe_state(node: &UpgradeNode, tree: &UpgradeTree) -> String {
    if tree.is_purchased(&node.id) {
        return "Purchased".to_string();
    }
    if !tree.is_accessible(&node.id) {
        let missing: Vec<&str> = node
            .requires
            .iter()
            .filter(|r| !tree.is_purchased(r))
            .map(|s| s.as_str())
            .collect();
        return format!("Locked — requires: {}", missing.join(", "));
    }
    if tree.can_buy(&node.id) {
        return format!("Buy for {:.0} gold", node.cost);
    }
    format!("Need {:.0} more gold", node.cost - tree.gold)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_panel() -> Rect {
        Rect {
            x: 40.0,
            y: 40.0,
            w: 1200.0,
            h: 640.0,
        }
    }

    fn make_layout(panel: Rect) -> ModalLayout {
        ModalLayout {
            panel,
            center_x: panel.x + panel.w * 0.5,
            top_y: panel.y + HEADER_H + NODE_H * 0.5,
            col_spacing: COL_SPACING,
            row_spacing: 60.0,
            description_y: panel.y + panel.h - DESCRIPTION_H - FOOTER_H,
        }
    }

    #[test]
    fn node_center_uses_layout_center() {
        let defs = load_defs();
        let layout = make_layout(fake_panel());
        let root = defs.node("root").unwrap();
        let c = layout.node_center(root);
        assert!((c.x - (40.0 + 600.0)).abs() < 1e-3);
    }

    #[test]
    fn node_center_shifts_with_col() {
        let defs = load_defs();
        let layout = make_layout(fake_panel());
        let node = defs.node("turret_dmg_1").unwrap();
        let c = layout.node_center(node);
        assert!((c.x - (40.0 + 600.0 + COL_SPACING)).abs() < 1e-3);
    }

    #[test]
    fn node_rect_wraps_center() {
        let defs = load_defs();
        let layout = make_layout(fake_panel());
        let node = defs.node("root").unwrap();
        let r = layout.node_rect(node);
        assert!((r.w - NODE_W).abs() < 1e-6);
        assert!((r.h - NODE_H).abs() < 1e-6);
    }

    #[test]
    fn description_does_not_overlap_tree_area() {
        let layout = make_layout(fake_panel());
        let tree_bottom = layout.top_y + 6.0 * layout.row_spacing + NODE_H * 0.5;
        assert!(layout.description_y >= tree_bottom);
    }

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        assert_eq!(truncate("hello world", 6), "hello…");
    }
}