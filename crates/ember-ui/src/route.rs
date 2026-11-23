//! Input routing for one `UiRoot` (spec §9, ADR-036).

use crate::{ControlRegistry, FocusChanged, FocusManager, control::PendingSignal};
use ember_core::math::Rect;
use ember_input::{GamepadButton, Input, Key, MouseButton};
use ember_scene::{NodeId, NodeTree};

/// Keys that move focus instead of reaching the focused control.
const NEXT_KEYS: [Key; 2] = [Key::Tab, Key::Down];
const PREV_KEYS: [Key; 1] = [Key::Up];
/// Keys that activate the focused control (also gamepad South).
const ACTIVATE_KEYS: [Key; 2] = [Key::Enter, Key::Space];

/// Visible controls under `root`, depth first in tree order, each with
/// the clip its ancestors' [`Control::clip_children`](crate::Control::clip_children)
/// put on it (`None` = unclipped). A hidden or unregistered node hides
/// its whole subtree.
pub fn collect_controls(
    tree: &NodeTree,
    registry: &ControlRegistry,
    root: NodeId,
) -> Vec<(NodeId, Option<Rect>)> {
    fn walk(
        tree: &NodeTree,
        registry: &ControlRegistry,
        id: NodeId,
        clip: Option<Rect>,
        out: &mut Vec<(NodeId, Option<Rect>)>,
    ) {
        for &kid in tree.children(id) {
            let Some(c) = tree.get(kid).and_then(|n| registry.cast(n)) else {
                continue;
            };
            if !c.base().visible {
                continue;
            }
            out.push((kid, clip));
            let inner = match (clip, c.clip_children()) {
                (Some(outer), Some(own)) => Some(
                    outer
                        .intersect(&own)
                        .unwrap_or(Rect::new(own.x, own.y, 0.0, 0.0)),
                ),
                (outer, own) => own.or(outer),
            };
            walk(tree, registry, kid, inner, out);
        }
    }
    let mut out = Vec::new();
    walk(tree, registry, root, None, &mut out);
    out
}

/// What a routing pass produced: signals to emit, in order.
pub struct Routed {
    pub signals: Vec<(NodeId, PendingSignal)>,
}

/// Route one frame of `input` to the controls under `root`:
///
/// 1. hover: `on_mouse_enter` / `on_mouse_leave` as the pointer moves;
/// 2. mouse buttons: `on_mouse_down` to every hovered control (a left
///    press also focuses the deepest hovered focusable control and marks
///    controls `pressed`), `on_mouse_up` to hovered or pressed ones;
/// 3. wheel: `on_mouse_wheel(y)` to hovered controls;
/// 4. focus: `request` first, then a left click, then Tab / Down /
///    D-pad down (next), Up / D-pad up (previous);
/// 5. Enter / Space / gamepad South: `on_activate` on the focused control
///    (not Space if it [captures text](crate::Control::captures_text));
///    typed characters: `on_text_input`; then every other pressed key:
///    `on_key_press`.
///
/// The pointer only reaches a control inside its ancestors'
/// `clip_children` rects (a `ScrollContainer` hides what is scrolled out).
///
/// v0.1 does not consume events: every control under the pointer gets
/// them (spec §9.2).
pub fn route(
    tree: &mut NodeTree,
    registry: &ControlRegistry,
    focus: &mut FocusManager,
    root: NodeId,
    input: &Input,
    request: Option<Option<NodeId>>,
) -> Routed {
    let visible = collect_controls(tree, registry, root);
    let order: Vec<NodeId> = visible.iter().map(|&(id, _)| id).collect();
    let before = focus.focused();
    let pos = input.mouse_pos;

    // 1. Hover
    for &(id, clip) in &visible {
        let Some(c) = tree.get_mut(id).and_then(|n| registry.cast_mut(n)) else {
            continue;
        };
        let inside =
            c.wants_mouse() && c.rect().contains(pos) && clip.is_none_or(|r| r.contains(pos));
        if inside != c.base().hovered {
            c.base_mut().hovered = inside;
            if inside {
                c.on_mouse_enter();
            } else {
                c.on_mouse_leave();
            }
        }
    }

    // 2. Buttons
    let buttons = [
        (
            MouseButton::Left,
            input.mouse_left_pressed,
            input.mouse_left_released,
        ),
        (
            MouseButton::Right,
            input.mouse_right_pressed,
            input.mouse_right_released,
        ),
        (
            MouseButton::Middle,
            input.mouse_middle_pressed,
            input.mouse_middle_released,
        ),
    ];
    let mut click_focus = None;
    for (button, pressed, released) in buttons {
        let left = button == MouseButton::Left;
        for &id in &order {
            let Some(c) = tree.get_mut(id).and_then(|n| registry.cast_mut(n)) else {
                continue;
            };
            let hovered = c.base().hovered;
            if pressed && hovered {
                if left {
                    c.base_mut().pressed = true;
                    if c.focusable() {
                        click_focus = Some(id); // deepest wins: later in DFS order
                    }
                }
                c.on_mouse_down(button, pos);
            }
            if released && (hovered || (left && c.base().pressed)) {
                c.on_mouse_up(button, pos);
                if left {
                    c.base_mut().pressed = false;
                }
            }
        }
    }

    // 3. Wheel
    if input.wheel_moved() {
        for &id in &order {
            if let Some(c) = tree.get_mut(id).and_then(|n| registry.cast_mut(n))
                && c.base().hovered
            {
                c.on_mouse_wheel(input.mouse_wheel.y);
            }
        }
    }

    // 4. Focus
    let focusables: Vec<NodeId> = order
        .iter()
        .copied()
        .filter(|&id| {
            tree.get(id)
                .and_then(|n| registry.cast(n))
                .is_some_and(|c| c.focusable())
        })
        .collect();
    focus.validate(&focusables);
    if let Some(target) = request {
        let target = target.filter(|t| focusables.contains(t));
        focus.set(tree, registry, target);
    }
    if click_focus.is_some() {
        focus.set(tree, registry, click_focus);
    }
    if !focusables.is_empty() {
        for (button, forward) in [
            (GamepadButton::DPadDown, true),
            (GamepadButton::DPadUp, false),
        ] {
            if input.is_gamepad_pressed(button) {
                let target = focus.neighbour(&focusables, forward);
                focus.set(tree, registry, target);
            }
        }
    }
    let mut keys_for_control = Vec::new();
    for &key in &input.keys_pressed {
        let step = if NEXT_KEYS.contains(&key) {
            Some(true)
        } else if PREV_KEYS.contains(&key) {
            Some(false)
        } else {
            None
        };
        match step {
            Some(forward) if !focusables.is_empty() => {
                let target = focus.neighbour(&focusables, forward);
                focus.set(tree, registry, target);
            }
            _ => keys_for_control.push(key),
        }
    }

    // 5. Activation and keys for the focused control
    if let Some(id) = focus.focused()
        && let Some(c) = tree.get_mut(id).and_then(|n| registry.cast_mut(n))
    {
        // A text field types Space instead of activating on it.
        let captures = c.captures_text();
        let activates = |k: &Key| ACTIVATE_KEYS.contains(k) && !(captures && *k == Key::Space);
        let gamepad = input.is_gamepad_pressed(GamepadButton::South);
        if gamepad || keys_for_control.iter().any(activates) {
            c.on_activate();
        }
        // Text first: "type, then Backspace" in one frame erases the
        // typed character, not the one before it.
        for &ch in input.chars_typed.iter().filter(|ch| !ch.is_control()) {
            c.on_text_input(ch);
        }
        for key in keys_for_control.iter().filter(|k| !activates(k)) {
            c.on_key_press(*key);
        }
    }

    // Collect queued signals, in tree order, after the focus change.
    let mut signals: Vec<(NodeId, PendingSignal)> = Vec::new();
    let after = focus.focused();
    if after != before {
        signals.push((
            root,
            Box::new(move |ctx, _| {
                ctx.emit(FocusChanged {
                    from: before,
                    to: after,
                })
            }),
        ));
    }
    for &id in &order {
        if let Some(c) = tree.get_mut(id).and_then(|n| registry.cast_mut(n)) {
            for s in c.base_mut().take_signals() {
                signals.push((id, s));
            }
        }
    }
    Routed { signals }
}
