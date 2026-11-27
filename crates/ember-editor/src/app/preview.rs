//! The preview: the scene's own nodes drawn in a clipped area through
//! [`preview_camera`] (ADR-040), the selection outlined, and click to
//! select ([`pick`]: screen → scene through the same camera).

use super::{INSPECTOR_W, OUTLINE_W, TOOLBAR_H, WINDOW_H, WINDOW_W};
use crate::Document;
use ember_core::Color;
use ember_core::math::Rect;
use ember_scene::{DrawContext, NodeId};
use ember_servers::Camera2d;
use glam::Vec2;

/// Scene origin inside the preview.
const PREVIEW_MARGIN: f32 = 20.0;
/// The square a node with a `position` but no `size` covers.
const MARKER: f32 = 12.0;

/// The camera that fits a scene into the preview (ADR-040). With the
/// game's `viewport`, the whole resolution is scaled down to fit (never
/// up) and centred; without, the scene's origin sits near the top-left
/// at zoom 1.
pub fn preview_camera(viewport: Option<Vec2>) -> Camera2d {
    let area = preview_rect();
    let corner = Vec2::new(area.x, area.y);
    let Some(size) = viewport.filter(|s| s.x > 0.0 && s.y > 0.0) else {
        return Camera2d::placing_origin_at(corner + Vec2::splat(PREVIEW_MARGIN), 1.0);
    };
    let room = Vec2::new(area.w, area.h) - Vec2::splat(2.0 * PREVIEW_MARGIN);
    let zoom = (room.x / size.x).min(room.y / size.y).min(1.0);
    let origin = corner + (Vec2::new(area.w, area.h) - size * zoom) / 2.0;
    Camera2d::placing_origin_at(origin, zoom)
}

/// The preview area: between the outline and the inspector.
pub fn preview_rect() -> Rect {
    Rect::new(
        OUTLINE_W,
        TOOLBAR_H,
        WINDOW_W - OUTLINE_W - INSPECTOR_W,
        WINDOW_H - TOOLBAR_H,
    )
}

/// The scene-space rect a node covers, from its exported `position`
/// and `size` (top-left + size; a node without `size` covers a small
/// square centred on `position`). `None` without a `position`.
pub fn node_rect(doc: &Document, id: NodeId) -> Option<Rect> {
    let props = doc.properties(id).ok()?;
    let get = |k: &str| {
        props
            .iter()
            .find(|(key, _)| *key == k)
            .and_then(|(_, v)| v.as_vec2())
    };
    let pos = get("position")?;
    Some(match get("size") {
        Some(size) => Rect::new(pos.x, pos.y, size.x, size.y),
        None => Rect::new(pos.x - MARKER / 2.0, pos.y - MARKER / 2.0, MARKER, MARKER),
    })
}

/// The topmost visible node whose rect holds `screen` (a window point):
/// highest `z`, then latest in draw (outline) order.
pub fn pick(doc: &Document, viewport: Option<Vec2>, screen: Vec2) -> Option<NodeId> {
    let p = preview_camera(viewport).screen_to_world(screen);
    let tree = doc.tree();
    let mut best: Option<(f32, NodeId)> = None;
    for row in doc.outline() {
        let Some(node) = tree.get(row.id) else {
            continue;
        };
        let hidden = std::iter::successors(Some(row.id), |&id| tree.parent(id))
            .any(|id| tree.get(id).is_some_and(|n| !n.visible()));
        if hidden || !node_rect(doc, row.id).is_some_and(|r| r.contains(p)) {
            continue;
        }
        if best.is_none_or(|(z, _)| node.z() >= z) {
            best = Some((node.z(), row.id));
        }
    }
    best.map(|(_, id)| id)
}

/// Draw the scene, the viewport frame and the selection outline.
pub fn draw(
    ctx: &mut DrawContext,
    doc: &Document,
    selected: Option<NodeId>,
    viewport: Option<Vec2>,
) {
    let area = preview_rect();
    let r = ctx.render();
    r.draw_rect(area, Color::new(0.06, 0.06, 0.08, 1.0));
    r.push_clip_rect(area);
    let camera = r.camera();
    r.set_camera(preview_camera(viewport));
    if let Some(size) = viewport {
        r.draw_rect(Rect::new(0.0, 0.0, size.x, size.y), Color::BLACK);
    }
    doc.tree().draw(ctx);
    if let Some(size) = viewport {
        ctx.render().draw_rect_outline(
            Rect::new(0.0, 0.0, size.x, size.y),
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.25),
        );
    }
    if let Some(rect) = selected.and_then(|id| node_rect(doc, id)) {
        let rect = Rect::new(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0);
        ctx.render()
            .draw_rect_outline(rect, 2.0, Color::new(1.0, 0.8, 0.2, 1.0));
    }
    let r = ctx.render();
    r.set_camera(camera);
    r.pop_clip_rect();
}
