//! Layout: anchors now, containers (`VBox`, `HBox`, `Stretch`,
//! `ScrollContainer`) in Phase 5.2.

mod anchors;

pub use anchors::Anchors;

use crate::{ChildLayout, ControlRegistry, Theme};
use ember_core::math::Rect;
use ember_scene::{NodeId, NodeTree};
use std::sync::Arc;

/// Lay out every control under `parent` inside `content`, depth first.
///
/// For each child that is a registered control: measure it, place it
/// (the parent control's [`crate::Control::arrange_children`] if
/// `parent` is itself a control and returns rects, otherwise the child's
/// anchors), hand it `theme`, then recurse into its
/// [`crate::Control::content_rect`]. Children that are not registered
/// controls are skipped together with their subtree.
pub fn layout_children(
    tree: &mut NodeTree,
    registry: &ControlRegistry,
    parent: NodeId,
    content: Rect,
    theme: &Arc<Theme>,
) {
    let kids: Vec<NodeId> = tree.children(parent).to_vec();
    let infos: Vec<ChildLayout> = kids
        .iter()
        .filter_map(|&id| {
            let control = registry.cast(tree.get(id)?)?;
            Some(ChildLayout {
                id,
                anchors: control.anchors(),
                offset: control.base().offset,
                min_size: control.min_size(),
                desired: control.measure(content.size()),
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
        layout_children(tree, registry, info.id, inner, theme);
    }
}
