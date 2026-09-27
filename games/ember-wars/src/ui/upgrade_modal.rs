//! Modal Upgrade Tree — grille de nœuds, achat, description, scroll.

use macroquad::prelude::*;

use ember_stdlib::ui::hit;

use crate::config::GameContext;
use crate::fonts;
use crate::menu::MenuState;
use crate::upgrades::{load_defs, UpgradeEffect, UpgradeNode, UpgradeTree};

// ---------- Layout constants ----------

const MODAL_MARGIN: f32 = 36.0;
const MODAL_PAD: f32 = 28.0;
const HEADER_H: f32 = 72.0;
const DESC_H: f32 = 130.0;

const NODE_W: f32 = 118.0;
const NODE_H: f32 = 50.0;
const NODE_GAP_X: f32 = 26.0;
const NODE_GAP_Y: f32 = 14.0;

const COLS: f32 = 9.0; // col -4..+4
const ROWS: f32 = 9.0; // row 0..8

const WHEEL_SCROLL: f32 = 60.0;
const KEY_SCROLL: f32 = 700.0;

// ---------- Layout ----------

fn modal_rect_for(vw: f32, vh: f32) -> Rect {
    Rect {
        x: MODAL_MARGIN,
        y: MODAL_MARGIN,
        w: vw - MODAL_MARGIN * 2.0,
        h: vh - MODAL_MARGIN * 2.0,
    }
}

fn modal_rect() -> Rect {
    modal_rect_for(screen_width(), screen_height())
}

fn tree_area(modal: Rect) -> Rect {
    Rect {
        x: modal.x + MODAL_PAD,
        y: modal.y + HEADER_H,
        w: modal.w - MODAL_PAD * 2.0,
        h: modal.h - HEADER_H - DESC_H - MODAL_PAD * 0.5,
    }
}

fn desc_area(modal: Rect) -> Rect {
    Rect {
        x: modal.x + MODAL_PAD,
        y: modal.y + modal.h - DESC_H + 4.0,
        w: modal.w - MODAL_PAD * 2.0,
        h: DESC_H - MODAL_PAD * 0.5,
    }
}

fn content_size() -> Vec2 {
    Vec2::new(
        COLS * NODE_W + (COLS - 1.0) * NODE_GAP_X,
        ROWS * NODE_H + (ROWS - 1.0) * NODE_GAP_Y,
    )
}

fn content_offset(tree: Rect) -> Vec2 {
    let cs = content_size();
    Vec2::new(
        ((tree.w - cs.x) * 0.5).max(0.0),
        ((tree.h - cs.y) * 0.5).max(0.0),
    )
}

fn max_scroll(tree: Rect) -> Vec2 {
    let cs = content_size();
    Vec2::new((cs.x - tree.w).max(0.0), (cs.y - tree.h).max(0.0))
}

fn clamp_scroll(scroll: Vec2, tree: Rect) -> Vec2 {
    let m = max_scroll(tree);
    Vec2::new(scroll.x.clamp(0.0, m.x), scroll.y.clamp(0.0, m.y))
}

fn node_center(node: &UpgradeNode, tree: Rect, scroll: Vec2) -> Vec2 {
    let off = content_offset(tree);
    let local_x = (node.col as f32 + 4.0) * (NODE_W + NODE_GAP_X) + NODE_W * 0.5;
    let local_y = node.row as f32 * (NODE_H + NODE_GAP_Y) + NODE_H * 0.5;
    Vec2::new(
        tree.x + off.x + local_x - scroll.x,
        tree.y + off.y + local_y - scroll.y,
    )
}

fn node_rect(node: &UpgradeNode, tree: Rect, scroll: Vec2) -> Rect {
    let c = node_center(node, tree, scroll);
    Rect {
        x: c.x - NODE_W * 0.5,
        y: c.y - NODE_H * 0.5,
        w: NODE_W,
        h: NODE_H,
    }
}

fn rect_intersects(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

fn scroll_to_show(node: &UpgradeNode, tree: Rect, scroll: Vec2) -> Vec2 {
    let local_x = (node.col as f32 + 4.0) * (NODE_W + NODE_GAP_X);
    let local_y = node.row as f32 * (NODE_H + NODE_GAP_Y);
    let pad = 16.0;
    let n_left = local_x - pad;
    let n_right = local_x + NODE_W + pad;
    let n_top = local_y - pad;
    let n_bottom = local_y + NODE_H + pad;

    let mut s = scroll;
    if n_left < s.x {
        s.x = n_left;
    }
    if n_right > s.x + tree.w {
        s.x = n_right - tree.w;
    }
    if n_top < s.y {
        s.y = n_top;
    }
    if n_bottom > s.y + tree.h {
        s.y = n_bottom - tree.h;
    }
    clamp_scroll(s, tree)
}

// ---------- Neighbor nav ----------

fn find_neighbor(from_idx: usize, dir: (i32, i32)) -> Option<usize> {
    let defs = load_defs();
    let from = defs.nodes.get(from_idx)?;
    let from_pos = Vec2::new(from.col as f32, from.row as f32);

    let dx = dir.0 as f32;
    let dy = dir.1 as f32;

    let mut best: Option<(f32, usize)> = None;
    for (i, n) in defs.nodes.iter().enumerate() {
        if i == from_idx {
            continue;
        }
        let to_pos = Vec2::new(n.col as f32, n.row as f32);
        let delta = to_pos - from_pos;
        let primary = delta.x * dx + delta.y * dy;
        if primary <= 0.0 {
            continue;
        }
        let secondary = (delta.x * dy - delta.y * dx).abs();
        let score = primary.abs() + secondary * 2.0;
        if best.is_none_or(|(s, _)| score < s) {
            best = Some((score, i));
        }
    }
    best.map(|(_, i)| i)
}

// ---------- Input ----------

pub fn handle_modal_input(tree: &mut UpgradeTree, state: &mut MenuState) -> bool {
    let modal = modal_rect();
    let tree_rect = tree_area(modal);

    if state.upgrade_focus.is_none() {
        state.upgrade_focus = Some(0);
    }

    let (_wx, wy) = mouse_wheel();
    if wy.abs() > 0.0 {
        state.upgrade_scroll.y -= wy * WHEEL_SCROLL;
    }

    let dt = get_frame_time();
    if is_key_down(KeyCode::PageDown) {
        state.upgrade_scroll.y += KEY_SCROLL * dt;
    }
    if is_key_down(KeyCode::PageUp) {
        state.upgrade_scroll.y -= KEY_SCROLL * dt;
    }
    if is_key_pressed(KeyCode::Home) {
        state.upgrade_scroll.y = 0.0;
    }
    if is_key_pressed(KeyCode::End) {
        state.upgrade_scroll.y = max_scroll(tree_rect).y;
    }

    state.upgrade_scroll = clamp_scroll(state.upgrade_scroll, tree_rect);

    let mut moved = false;
    if let Some(f) = state.upgrade_focus {
        if is_key_pressed(KeyCode::Up)
            && let Some(n) = find_neighbor(f, (0, -1))
        {
            state.upgrade_focus = Some(n);
            moved = true;
        }
        if is_key_pressed(KeyCode::Down)
            && let Some(n) = find_neighbor(f, (0, 1))
        {
            state.upgrade_focus = Some(n);
            moved = true;
        }
        if is_key_pressed(KeyCode::Left)
            && let Some(n) = find_neighbor(f, (-1, 0))
        {
            state.upgrade_focus = Some(n);
            moved = true;
        }
        if is_key_pressed(KeyCode::Right)
            && let Some(n) = find_neighbor(f, (1, 0))
        {
            state.upgrade_focus = Some(n);
            moved = true;
        }
    }

    if moved
        && let Some(f) = state.upgrade_focus
        && let Some(node) = load_defs().nodes.get(f)
    {
        state.upgrade_scroll = scroll_to_show(node, tree_rect, state.upgrade_scroll);
    }

    let mouse = Vec2::new(mouse_position().0, mouse_position().1);
    if is_mouse_button_pressed(MouseButton::Left) {
        let defs = load_defs();
        for (i, node) in defs.nodes.iter().enumerate() {
            let r = node_rect(node, tree_rect, state.upgrade_scroll);
            if hit::contains(r, mouse) {
                state.upgrade_focus = Some(i);
                return tree.buy(&node.id);
            }
        }
    }

    if (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space))
        && let Some(f) = state.upgrade_focus
        && let Some(node) = load_defs().nodes.get(f)
    {
        return tree.buy(&node.id);
    }

    false
}

// ---------- Draw ----------

pub fn draw_modal(ctx: &GameContext, tree: &UpgradeTree, state: &MenuState) {
    let modal = modal_rect();
    let tree_rect = tree_area(modal);
    let desc_rect = desc_area(modal);
    let scroll = state.upgrade_scroll;

    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.72),
    );

    draw_rectangle(
        modal.x,
        modal.y,
        modal.w,
        modal.h,
        Color::new(0.05, 0.06, 0.08, 1.0),
    );
    draw_rectangle_lines(modal.x, modal.y, modal.w, modal.h, 2.0, ctx.colors.accent);

    draw_rectangle(
        tree_rect.x,
        tree_rect.y,
        tree_rect.w,
        tree_rect.h,
        Color::new(0.04, 0.05, 0.07, 1.0),
    );
    draw_rectangle_lines(
        tree_rect.x,
        tree_rect.y,
        tree_rect.w,
        tree_rect.h,
        1.0,
        Color::new(0.16, 0.18, 0.22, 1.0),
    );

    let defs = load_defs();
    for node in &defs.nodes {
        for req in &node.requires {
            if let Some(parent) = defs.nodes.iter().find(|n| &n.id == req) {
                let a = node_center(parent, tree_rect, scroll);
                let b = node_center(node, tree_rect, scroll);
                let seg = Rect {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                    w: (b.x - a.x).abs() + 1.0,
                    h: (b.y - a.y).abs() + 1.0,
                };
                if !rect_intersects(seg, tree_rect) {
                    continue;
                }
                let parent_purchased = tree.is_purchased(&parent.id);
                let col = if parent_purchased {
                    Color::new(0.55, 0.75, 0.55, 0.9)
                } else {
                    Color::new(0.22, 0.24, 0.30, 1.0)
                };
                draw_line(a.x, a.y, b.x, b.y, 2.0, col);
            }
        }
    }

    for (i, node) in defs.nodes.iter().enumerate() {
        let r = node_rect(node, tree_rect, scroll);
        if !rect_intersects(r, tree_rect) {
            continue;
        }
        // Ne dessine que si le nœud est ENTIÈREMENT dans la vue — évite
        // un nœud à moitié coupé en haut/bas.
        if r.y < tree_rect.y || r.y + r.h > tree_rect.y + tree_rect.h {
            continue;
        }
        draw_node(ctx, tree, node, r, state.upgrade_focus == Some(i));
    }

    draw_tree_scrollbar(tree_rect, scroll);

    draw_header(ctx, tree, modal);
    draw_description(ctx, tree, state, desc_rect);
}

fn draw_node(
    ctx: &GameContext,
    tree: &UpgradeTree,
    node: &UpgradeNode,
    r: Rect,
    focused: bool,
) {
    let purchased = tree.is_purchased(&node.id);
    let accessible = tree.is_accessible(&node.id);
    let affordable = tree.gold >= node.cost;
    let can_buy = tree.can_buy(&node.id);

    let (bg, border) = if purchased {
        (
            Color::new(0.10, 0.18, 0.14, 1.0),
            Color::new(0.45, 1.0, 0.55, 1.0),
        )
    } else if !accessible {
        (
            Color::new(0.06, 0.06, 0.08, 1.0),
            Color::new(0.20, 0.20, 0.24, 1.0),
        )
    } else if can_buy {
        (Color::new(0.16, 0.14, 0.06, 1.0), ctx.colors.accent)
    } else if affordable {
        (
            Color::new(0.10, 0.10, 0.12, 1.0),
            Color::new(0.45, 0.45, 0.50, 1.0),
        )
    } else {
        (
            Color::new(0.08, 0.08, 0.10, 1.0),
            Color::new(0.35, 0.28, 0.28, 1.0),
        )
    };

    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, border);

    if focused {
        draw_rectangle_lines(
            r.x - 3.0,
            r.y - 3.0,
            r.w + 6.0,
            r.h + 6.0,
            2.0,
            Color::new(1.0, 1.0, 1.0, 0.9),
        );
    }

    let label = truncate(&node.name, 14);
    let dim = fonts::measure_bold(&label, 13.0);
    let text_color = if purchased {
        Color::new(0.65, 1.0, 0.75, 1.0)
    } else if !accessible {
        Color::new(0.42, 0.42, 0.46, 1.0)
    } else {
        ctx.colors.text
    };
    fonts::draw_text_bold(
        &label,
        r.x + (r.w - dim.width) * 0.5,
        r.y + 22.0,
        13.0,
        text_color,
    );

    if purchased {
        let check = "✓";
        let dim_c = fonts::measure_bold(check, 15.0);
        fonts::draw_text_bold(
            check,
            r.x + (r.w - dim_c.width) * 0.5,
            r.y + r.h - 6.0,
            15.0,
            Color::new(0.45, 1.0, 0.55, 1.0),
        );
    } else {
        let cost_txt = format!("{:.0}g", node.cost);
        let dim_c = fonts::measure_regular(&cost_txt, 11.0);
        let cost_color = if affordable && accessible {
            ctx.colors.gold
        } else {
            Color::new(0.58, 0.44, 0.44, 1.0)
        };
        fonts::draw_text_regular(
            &cost_txt,
            r.x + (r.w - dim_c.width) * 0.5,
            r.y + r.h - 6.0,
            11.0,
            cost_color,
        );
    }
}

fn draw_tree_scrollbar(tree_rect: Rect, scroll: Vec2) {
    let m = max_scroll(tree_rect);
    if m.y <= 0.0 {
        return;
    }
    let bar_x = tree_rect.x + tree_rect.w - 8.0;
    let track_h = tree_rect.h;
    let ratio = (tree_rect.h / (tree_rect.h + m.y)).clamp(0.0, 1.0);
    let thumb_h = (track_h * ratio).max(30.0);
    let frac = (scroll.y / m.y).clamp(0.0, 1.0);
    let thumb_y = tree_rect.y + (track_h - thumb_h) * frac;

    draw_rectangle(
        bar_x,
        tree_rect.y,
        4.0,
        track_h,
        Color::new(0.10, 0.11, 0.14, 1.0),
    );
    draw_rectangle(
        bar_x,
        thumb_y,
        4.0,
        thumb_h,
        Color::new(0.72, 0.74, 0.78, 0.85),
    );
}

fn draw_header(ctx: &GameContext, tree: &UpgradeTree, modal: Rect) {
    let h = Rect {
        x: modal.x,
        y: modal.y,
        w: modal.w,
        h: HEADER_H,
    };
    draw_rectangle(h.x, h.y, h.w, h.h, Color::new(0.06, 0.07, 0.10, 1.0));
    draw_line(
        h.x,
        h.y + h.h - 1.0,
        h.x + h.w,
        h.y + h.h - 1.0,
        1.0,
        Color::new(0.20, 0.22, 0.28, 1.0),
    );

    let x = modal.x + MODAL_PAD;
    let baseline = modal.y + 46.0;

    let title = "UPGRADES";
    let dim = fonts::measure_bold(title, 32.0);
    fonts::draw_text_bold(title, x, baseline, 32.0, ctx.colors.accent);

    fonts::draw_text_regular(
        "Forge your run",
        x + dim.width + 22.0,
        baseline,
        15.0,
        Color::new(0.55, 0.58, 0.62, 1.0),
    );

    let gold_txt = format!("GOLD  {:.0}", tree.gold);
    let dim_g = fonts::measure_bold(&gold_txt, 22.0);
    fonts::draw_text_bold(
        &gold_txt,
        modal.x + modal.w - MODAL_PAD - dim_g.width,
        baseline,
        22.0,
        ctx.colors.gold,
    );
}

fn draw_description(
    ctx: &GameContext,
    tree: &UpgradeTree,
    state: &MenuState,
    desc: Rect,
) {
    draw_rectangle(
        desc.x,
        desc.y,
        desc.w,
        desc.h,
        Color::new(0.06, 0.07, 0.10, 1.0),
    );
    draw_rectangle_lines(
        desc.x,
        desc.y,
        desc.w,
        desc.h,
        1.0,
        Color::new(0.20, 0.22, 0.28, 1.0),
    );

    let x = desc.x + MODAL_PAD * 0.6;

    let defs = load_defs();
    let node = state.upgrade_focus.and_then(|i| defs.nodes.get(i));

    let Some(node) = node else {
        fonts::draw_text_regular(
            "No node focused",
            x,
            desc.y + 32.0,
            15.0,
            Color::new(0.5, 0.5, 0.55, 1.0),
        );
        return;
    };

    let purchased = tree.is_purchased(&node.id);
    let accessible = tree.is_accessible(&node.id);
    let affordable = tree.gold >= node.cost;

    let mut y = desc.y + 28.0;
    fonts::draw_text_bold(&node.name, x, y, 20.0, ctx.colors.text);

    let (state_txt, state_col) = if purchased {
        ("PURCHASED".to_string(), Color::new(0.45, 1.0, 0.55, 1.0))
    } else if !accessible {
        ("LOCKED".to_string(), Color::new(0.55, 0.55, 0.60, 1.0))
    } else if affordable {
        ("AVAILABLE".to_string(), ctx.colors.gold)
    } else {
        ("TOO EXPENSIVE".to_string(), Color::new(0.85, 0.55, 0.50, 1.0))
    };
    let dim_s = fonts::measure_bold(&state_txt, 13.0);
    fonts::draw_text_bold(
        &state_txt,
        desc.x + desc.w - MODAL_PAD * 0.6 - dim_s.width,
        y,
        13.0,
        state_col,
    );

    y += 20.0;
    draw_line(
        x,
        y,
        desc.x + desc.w - MODAL_PAD * 0.6,
        y,
        1.0,
        Color::new(0.18, 0.20, 0.24, 1.0),
    );
    y += 20.0;

    let effect_txt = effect_label(&node.effect);
    fonts::draw_text_regular(
        &effect_txt,
        x,
        y,
        14.0,
        Color::new(0.75, 0.78, 0.82, 1.0),
    );
    y += 20.0;

    if !purchased {
        let cost_txt = format!("Cost: {:.0} gold", node.cost);
        let cost_col = if affordable && accessible {
            ctx.colors.gold
        } else {
            Color::new(0.60, 0.45, 0.45, 1.0)
        };
        fonts::draw_text_regular(&cost_txt, x, y, 12.0, cost_col);
        y += 16.0;
    }

    if !accessible && !node.requires.is_empty() {
        let reqs: Vec<&str> = node
            .requires
            .iter()
            .filter_map(|id| defs.node(id).map(|n| n.name.as_str()))
            .collect();
        if !reqs.is_empty() {
            let req_txt = format!("Requires: {}", reqs.join(", "));
            fonts::draw_text_regular(
                &truncate(&req_txt, 75),
                x,
                y,
                12.0,
                Color::new(0.65, 0.55, 0.55, 1.0),
            );
        }
    }

    let hint = "Tab Close  ·  Wheel Scroll  ·  ↑↓←→ Navigate  ·  Enter Buy";
    let dim_h = fonts::measure_regular(hint, 11.0);
    fonts::draw_text_regular(
        hint,
        desc.x + desc.w - MODAL_PAD * 0.6 - dim_h.width,
        desc.y + desc.h - 10.0,
        11.0,
        Color::new(0.42, 0.44, 0.50, 1.0),
    );
}

fn effect_label(effect: &UpgradeEffect) -> String {
    match effect {
        UpgradeEffect::None => "—".to_string(),
        UpgradeEffect::TowerHpMult(m) => format!("+{:.0}% tower HP", (m - 1.0) * 100.0),
        UpgradeEffect::TowerRegen(v) => format!("+{:.1} HP/s on your tower", v),
        UpgradeEffect::Fortress => "Tower gains a second HP bar (50%)".to_string(),
        UpgradeEffect::UnitHpMult(m) => format!("+{:.0}% unit HP", (m - 1.0) * 100.0),
        UpgradeEffect::UnitDamageMult(m) => {
            format!("+{:.0}% unit damage", (m - 1.0) * 100.0)
        }
        UpgradeEffect::UnitSpeedMult(m) => {
            format!("+{:.0}% unit movement speed", (m - 1.0) * 100.0)
        }
        UpgradeEffect::UnlockUnit(kind) => format!("Unlocks the {kind} unit"),
        UpgradeEffect::TurretDamageMult(m) => {
            format!("+{:.0}% catapult damage", (m - 1.0) * 100.0)
        }
        UpgradeEffect::TurretFireRateMult(m) => {
            format!("+{:.0}% catapult fire rate", (1.0 - m) * 100.0)
        }
        UpgradeEffect::MultiShot(n) => format!("Catapult fires {n} projectiles"),
        UpgradeEffect::TurretCritChance(c) => {
            format!("{:.0}% chance to deal double damage", c * 100.0)
        }
        UpgradeEffect::ManaRegenMult(m) => format!("+{:.0}% mana regen", (m - 1.0) * 100.0),
        UpgradeEffect::ManaCapMult(m) => format!("+{:.0}% mana cap", (m - 1.0) * 100.0),
        UpgradeEffect::ManaOnKillAdd(v) => format!("+{:.0} mana per enemy kill", v),
        UpgradeEffect::GoldPerKillAdd(v) => format!("+{:.0} gold per enemy kill", v),
        UpgradeEffect::CatapultRangeAdd(v) => format!("+{:.0}px catapult range", v),
        UpgradeEffect::CatapultHpMult(m) => {
            format!("+{:.0}% catapult HP", (m - 1.0) * 100.0)
        }
        UpgradeEffect::CatapultRebuildMult(m) => {
            format!("-{:.0}% catapult rebuild time", (1.0 - m) * 100.0)
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_tree_rect() -> Rect {
        Rect {
            x: 100.0,
            y: 100.0,
            w: 1400.0,
            h: 500.0,
        }
    }

    #[test]
    fn node_center_shifts_with_col() {
        let defs = load_defs();
        let n_left = defs.nodes.iter().find(|n| n.col == -4).unwrap();
        let n_right = defs.nodes.iter().find(|n| n.col == 4).unwrap();
        let tree = fake_tree_rect();
        let c_left = node_center(n_left, tree, Vec2::ZERO);
        let c_right = node_center(n_right, tree, Vec2::ZERO);
        assert!(c_right.x > c_left.x);
    }

    #[test]
    fn node_center_uses_layout_center() {
        let defs = load_defs();
        let root = defs.nodes.iter().find(|n| n.id == "root").unwrap();
        let tree = Rect {
            x: 0.0,
            y: 0.0,
            w: 2000.0,
            h: 1000.0,
        };
        let c = node_center(root, tree, Vec2::ZERO);
        assert!(
            (c.x - tree.w * 0.5).abs() < 2.0,
            "root x = {}, expected {}",
            c.x,
            tree.w * 0.5
        );
    }

    #[test]
    fn node_rect_wraps_center() {
        let defs = load_defs();
        let root = defs.nodes.iter().find(|n| n.id == "root").unwrap();
        let tree = fake_tree_rect();
        let c = node_center(root, tree, Vec2::ZERO);
        let r = node_rect(root, tree, Vec2::ZERO);
        assert!((r.x + r.w * 0.5 - c.x).abs() < 1e-6);
        assert!((r.y + r.h * 0.5 - c.y).abs() < 1e-6);
        assert!((r.w - NODE_W).abs() < 1e-6);
        assert!((r.h - NODE_H).abs() < 1e-6);
    }

    #[test]
    fn description_does_not_overlap_tree_area() {
        let m = modal_rect_for(1600.0, 900.0);
        let t = tree_area(m);
        let d = desc_area(m);
        assert!(d.y >= t.y + t.h - 1.0, "desc y={} tree bottom={}", d.y, t.y + t.h);
    }

    #[test]
    fn content_fits_in_tree_area_at_900_height() {
        // À la taille par défaut, tous les nœuds doivent être visibles
        // sans scroll (content ≤ tree).
        let m = modal_rect_for(1600.0, 900.0);
        let t = tree_area(m);
        let cs = content_size();
        assert!(
            cs.y <= t.h + 1.0,
            "content height {} > tree height {}",
            cs.y,
            t.h
        );
    }

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        let s = truncate("hello world", 8);
        assert!(s.chars().count() <= 8);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn max_scroll_is_zero_for_small_content() {
        let tree = Rect {
            x: 0.0,
            y: 0.0,
            w: 5000.0,
            h: 5000.0,
        };
        let m = max_scroll(tree);
        assert_eq!(m, Vec2::ZERO);
    }

    #[test]
    fn max_scroll_positive_for_small_view() {
        let tree = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let m = max_scroll(tree);
        assert!(m.x > 0.0);
        assert!(m.y > 0.0);
    }

    #[test]
    fn clamp_scroll_bounds() {
        let tree = Rect {
            x: 0.0,
            y: 0.0,
            w: 500.0,
            h: 500.0,
        };
        let clamped = clamp_scroll(Vec2::new(-100.0, 99_999.0), tree);
        assert!(clamped.x >= 0.0);
        assert!(clamped.y <= max_scroll(tree).y + 1e-3);
    }

    #[test]
    fn scroll_to_show_brings_node_into_view() {
        let defs = load_defs();
        let node = defs.nodes.iter().find(|n| n.row >= 6).unwrap();
        let tree = Rect {
            x: 0.0,
            y: 0.0,
            w: 1400.0,
            h: 400.0,
        };
        let scrolled = scroll_to_show(node, tree, Vec2::ZERO);
        let r = node_rect(node, tree, scrolled);
        assert!(r.y >= tree.y);
        assert!(r.y + r.h <= tree.y + tree.h + 1.0);
    }

    #[test]
    fn rect_intersects_detects_overlap() {
        let a = Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 };
        let b = Rect { x: 5.0, y: 5.0, w: 10.0, h: 10.0 };
        let c = Rect { x: 100.0, y: 100.0, w: 5.0, h: 5.0 };
        assert!(rect_intersects(a, b));
        assert!(!rect_intersects(a, c));
    }

    #[test]
    fn find_neighbor_down_moves_to_next_row() {
        let defs = load_defs();
        let root_idx = defs.nodes.iter().position(|n| n.id == "root").unwrap();
        let n = find_neighbor(root_idx, (0, 1));
        assert!(n.is_some());
        let target = &defs.nodes[n.unwrap()];
        assert!(target.row > 0);
    }
}