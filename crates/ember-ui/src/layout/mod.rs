//! Layout: anchors, margins and the containers (`VBox`, `HBox`,
//! `Stretch`). One pass per frame, run by `UiRoot` (spec §4.1).

mod anchors;
mod boxes;
mod margins;
mod stretch;

pub use anchors::Anchors;
pub use boxes::{HBox, VBox};
pub use margins::Margins;
pub use stretch::{Stretch, StretchMode};

use crate::{ChildLayout, ControlRegistry, Measure, Theme};
use ember_core::math::Rect;
use ember_scene::{NodeId, NodeTree};
use ember_servers::RenderingServer;
use glam::Vec2;
use std::sync::Arc;

/// Desired size of the control `id`, children first (bottom-up):
/// `measure_children(max(measure, min_size), child sizes)`, at least
/// `min_size`. `None` if `id` is not a registered control.
pub fn measure_node(
    tree: &NodeTree,
    registry: &ControlRegistry,
    m: &Measure,
    id: NodeId,
    available: Vec2,
) -> Option<Vec2> {
    let control = registry.cast(tree.get(id)?)?;
    let own = control.measure(available, m).max(control.min_size());
    let children: Vec<Vec2> = tree
        .children(id)
        .iter()
        .filter_map(|&kid| measure_node(tree, registry, m, kid, available))
        .collect();
    Some(
        control
            .measure_children(own, &children)
            .max(control.min_size()),
    )
}

/// Lay out every control under `parent` inside `content`, depth first.
///
/// For each child that is a registered control: measure it (with its
/// subtree), place it (the parent control's
/// [`crate::Control::arrange_children`] if `parent` is a control and
/// returns rects, otherwise the child's anchors), hand it `theme`, then
/// recurse into its [`crate::Control::content_rect`]. Children that are
/// not registered controls are skipped together with their subtree.
pub fn layout_children(
    tree: &mut NodeTree,
    registry: &ControlRegistry,
    render: &dyn RenderingServer,
    parent: NodeId,
    content: Rect,
    theme: &Arc<Theme>,
) {
    let m = Measure::new(render, theme);
    let kids: Vec<NodeId> = tree.children(parent).to_vec();
    let infos: Vec<ChildLayout> = kids
        .iter()
        .filter_map(|&id| {
            let desired = measure_node(tree, registry, &m, id, content.size())?;
            let control = registry.cast(tree.get(id)?)?;
            Some(ChildLayout {
                id,
                anchors: control.anchors(),
                offset: control.base().offset,
                min_size: control.min_size(),
                desired,
            })
        })
        .collect();

    let arranged = tree
        .get(parent)
        .and_then(|n| registry.cast(n))
        .and_then(|p| p.arrange_children(content, &infos));

    for (i, info) in infos.iter().enumerate() {
        let rect = arranged
            .as_ref()
            .and_then(|rects| rects.get(i).copied())
            .unwrap_or_else(|| info.anchored_rect(content));
        let Some(control) = tree.get_mut(info.id).and_then(|n| registry.cast_mut(n)) else {
            continue;
        };
        control.base_mut().theme = Arc::clone(theme);
        control.set_rect(rect);
        let inner = control.content_rect();
        layout_children(tree, registry, render, info.id, inner, theme);
    }
}
