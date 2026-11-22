//! Input routing for one `UiRoot` (spec §9, ADR-036).

use crate::{ControlRegistry, FocusChanged, FocusManager, control::PendingSignal};
use ember_input::{GamepadButton, Input, Key, MouseButton};
use ember_scene::{NodeId, NodeTree};

/// Keys that move focus instead of reaching the focused control.
const NEXT_KEYS: [Key; 2] = [Key::Tab, Key::Down];
const PREV_KEYS: [Key; 1] = [Key::Up];
/// Keys that activate the focused control (also gamepad South).
const ACTIVATE_KEYS: [Key; 2] = [Key::Enter, Key::Space];

/// Visible controls under `root`, depth first in tree order. A hidden or
/// unregistered node hides its whole subtree.
pub fn collect_controls(tree: &NodeTree, registry: &ControlRegistry, root: NodeId) -> Vec<NodeId> {
    fn walk(tree: &NodeTree, registry: &ControlRegistry, id: NodeId, out: &mut Vec<NodeId>) {
        for &kid in tree.children(id) {
            let Some(c) = tree.get(kid).and_then(|n| registry.cast(n)) else {
                continue;
            };
            if !c.base().visible {
                continue;
            }
            out.push(kid);
            walk(tree, registry, kid, out);
        }
    }
    let mut out = Vec::new();
    walk(tree, registry, root, &mut out);
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
/// 5. Enter / Space / gamepad South: `on_activate` on the focused control;
///    every other pressed key: `on_key_press` on it.
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
    let order = collect_controls(tree, registry, root);
    let before = focus.focused();
    let pos = input.mouse_pos;

    // 1. Hover
    for &id in &order {
        let Some(c) = tree.get_mut(id).and_then(|n| registry.cast_mut(n)) else {
            continue;
        };
        let inside = c.wants_mouse() && c.rect().contains(pos);
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
        let gamepad = input.is_gamepad_pressed(GamepadButton::South);
        if gamepad || keys_for_control.iter().any(|k| ACTIVATE_KEYS.contains(k)) {
            c.on_activate();
        }
        for key in keys_for_control {
            if !ACTIVATE_KEYS.contains(&key) {
                c.on_key_press(key);
            }
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
