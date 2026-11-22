---
title: "15 — Input"
aliases: ["15 — Input"]
tags: ["ember/spec", "ember/wave-2"]
doc-status: "partial"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/15-input.md"
imported: "2026-11-22"
wave: "2 — Concepts"
---

# 15 — Input

> **Status:** 🟡 partial — reviewed 2026-11-19; raw `Input` snapshot, `primary_action`/`secondary_action`, touch points and `InputSink` implemented; action maps, analog axes, dead zones, mouse wheel and real gamepad are not
> **Audience:** game authors, implementers
> **Prerequisites:** [[14-main-loop]]

## 1. Purpose

Explain how Ember reads input: raw key/mouse state, the
`primary_action` convention, action maps, dead zones for analog input,
and the gamepad and touch support. Raw input and `primary_action` /
`secondary_action` are implemented in `ember_input`; **action maps,
analog axes and dead zones (§2.2, §5, §6) are planned and *not yet
implemented*.**

The key takeaway: **most games should use `primary_action` for start /
confirm, and action maps for gameplay**. Raw key checks are for the rare
case where you need exact key state.

## 2. Raw vs mapped input

Ember exposes two layers of input:

### 2.1 Raw input

Direct key/mouse state, polled every frame:

```rust
ctx.input().is_key_down(Key::Space)       // is Space held down?
ctx.input().is_key_pressed(Key::Enter)    // was Enter pressed this frame?
ctx.input().is_key_released(Key::Enter)   // released this frame?
ctx.input().mouse_pos                     // current mouse position (Vec2 field)
ctx.input().is_mouse_pressed(MouseButton::Left)
```

This is what you use when you need exact hardware-level state.

### 2.2 Mapped input (action maps) — *not yet implemented*

`Input` has no `action*` methods and no `ActionMap` exists. The design:

Named actions, resolved from a configuration file:

```rust
ctx.input().action("jump")           // is "jump" held?
ctx.input().action_pressed("jump")   // was "jump" just pressed?
```

Actions map hardware keys to semantic names. `"jump"` might be Space on
keyboard and A on gamepad. The game doesn't care which.

### 2.3 Which to use

| Use case | Layer |
|---|---|
| Menu navigation (arrow keys, WASD) | mapped (`"menu_up"`, `"menu_down"`) |
| Start / confirm | `primary_action()` (see §3) |
| Gameplay movement | mapped (`"move_left"`, `"move_right"`) |
| Debug keys (F1, F2) | raw (`Key::F1`) |
| Cheat codes | raw |

## 3. `primary_action`

### 3.1 What it is

`primary_action()` is a **unified "start / confirm / advance" input**.
It returns `true` when any of the following is pressed this frame:

- Space
- Enter
- Left mouse click
- Gamepad South (A on Xbox, Cross on PlayStation)
- Any touch point that started this frame

### 3.2 Why it exists

Every game currently implements the same pattern:

```rust
if input.is_key_pressed(Key::Space) || input.is_key_pressed(Key::Enter) || input.is_mouse_pressed(MouseButton::Left) {
    self.start();
}
```

That's three checks per game, times 17 games, times every menu
transition. `primary_action` collapses it to one call:

```rust
if ctx.input().primary_action() {
    self.start();
}
```

And it works consistently across platforms — a touch on mobile maps to
`primary_action`, a gamepad button on desktop maps to `primary_action`
(ADR-010). `secondary_action()` is the cancel counterpart: Escape,
Backspace, right mouse click, Gamepad East.

### 3.3 What it does not include

- Escape / Backspace (those are `secondary_action`)
- Any modifier (Shift, Ctrl, Alt)

The convention is **"press to advance"** — the same action as a
touchscreen tap.

## 4. The `Input` struct

```rust
// ember_input::Input — a plain, `Default`, `Clone` snapshot built once per
// frame by the backend. Deliberately not #[non_exhaustive] (ADR-018).
pub struct Input {
    pub mouse_pos: Vec2,
    pub mouse_left_pressed: bool,   pub mouse_left_down: bool,   pub mouse_left_released: bool,
    pub mouse_right_pressed: bool,  pub mouse_right_down: bool,  pub mouse_right_released: bool,
    pub mouse_middle_pressed: bool, pub mouse_middle_down: bool, pub mouse_middle_released: bool,
    pub keys_pressed: Vec<Key>,
    pub keys_down: Vec<Key>,
    pub keys_released: Vec<Key>,
    pub gamepad_pressed: Vec<GamepadButton>,
    pub gamepad_down: Vec<GamepadButton>,
    pub gamepad_released: Vec<GamepadButton>,
    pub touch_points: Vec<TouchPoint>,
}
```

There is no mouse wheel, no `ActionMap` and no analog gamepad state yet.
`ember_stdlib::input::Input` is a separate, older macroquad-based helper.

### 4.1 Query methods

```rust
impl Input {
    // Keyboard
    pub fn is_key_down(&self, key: Key) -> bool;
    pub fn is_key_pressed(&self, key: Key) -> bool;
    pub fn is_key_released(&self, key: Key) -> bool;
    pub fn pressed_or_down(&self, key: Key) -> bool;

    // Mouse (position is the `mouse_pos` field)
    pub fn is_mouse_down(&self, button: MouseButton) -> bool;
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool;
    pub fn is_mouse_released(&self, button: MouseButton) -> bool;

    // Gamepad
    pub fn is_gamepad_down(&self, button: GamepadButton) -> bool;
    pub fn is_gamepad_pressed(&self, button: GamepadButton) -> bool;
    pub fn is_gamepad_released(&self, button: GamepadButton) -> bool;

    // Touch
    pub fn touches_started(&self) -> impl Iterator<Item = &TouchPoint>;
    pub fn touches_active(&self) -> impl Iterator<Item = &TouchPoint>;

    // Convenience
    pub fn primary_action(&self) -> bool;
    pub fn secondary_action(&self) -> bool;   // Escape / Backspace / right click / gamepad East
}

// Implemented by anything a runner feeds an Input to each frame (SceneLoop does).
pub trait InputSink { fn set_input(&mut self, input: Input); }
```

*Planned, not yet implemented:* `action`, `action_pressed`, `action_released`,
`action_axis`, `mouse_wheel`.

### 4.2 Frame semantics

`pressed` and `released` refer to **this frame only**. A key that was
pressed three frames ago returns `false` for `is_key_pressed` on frame 4.
`is_key_down` returns `true` for as long as the key is held.

This is the standard input model from every game engine since the 90s.
Alternatives (event queues, callbacks) exist but are worse for games
because they decouple input from the frame where the game logic runs.

## 5. Action map format

> *Not yet implemented (§5–§6): no `ActionMap`, no `input.ron` loader.*

### 5.1 RON file

Action maps are loaded from `assets/input.ron`:

```ron
(
    actions: [
        (name: "move_left",  keys: [A, Left],        gamepad: [LeftStickLeft]),
        (name: "move_right", keys: [D, Right],       gamepad: [LeftStickRight]),
        (name: "move_up",    keys: [W, Up],          gamepad: [LeftStickUp]),
        (name: "move_down",  keys: [S, Down],        gamepad: [LeftStickDown]),
        (name: "jump",       keys: [Space],          gamepad: [South]),
        (name: "attack",     keys: [J],              gamepad: [West]),
        (name: "pause",      keys: [Escape, P],      gamepad: [Start]),
    ],
)
```

### 5.2 Multiple keys per action

An action can bind to multiple keys. `action("move_left")` returns `true`
if **any** of them is held. This is what lets WASD and arrow keys work
simultaneously without extra code.

### 5.3 Multiple actions per key

A key can be bound to multiple actions. `Space` bound to both `"jump"`
and `"menu_confirm"` returns `true` for both when pressed. This is
uncommon but legal — the action map is not a partition.

### 5.4 Re-loading

```rust
ctx.assets().reload_action_map("input.ron");   // hypothetical; NodeContext has no assets()
```

Reloading is hot-reload-aware: editing `input.ron` updates the active
bindings on the next frame.

## 6. Analog input and dead zones

### 6.1 Analog actions

`action_axis("move_left")` returns a `f32` in `[-1.0, 1.0]`:

- `-1.0` — fully left (keyboard: `A` pressed; gamepad: left stick full left)
- `0.0` — neutral
- `1.0` — fully right

For digital-only actions (keys with no analog equivalent), `action_axis`
returns `-1.0`, `0.0`, or `1.0` — no in-between.

### 6.2 Dead zones

Gamepad sticks are noisy near zero. The action map allows per-action
dead zones:

```ron
(
    actions: [
        (name: "move_left", keys: [A], gamepad: [LeftStickLeft], dead_zone: 0.15),
    ],
)
```

Values below the dead zone are clamped to `0.0`. This is applied
automatically by `action_axis`.

### 6.3 Combining keyboard and gamepad

If both are active, the gamepad value wins (it has more precision).
This is a v0.1 convention; a v0.2 enhancement lets games override.

## 7. Gamepad

### 7.1 v0.1 status

**Partial.** `Input` carries digital button lists
(`gamepad_pressed/down/released: Vec<GamepadButton>`, with `GamepadButton`
= `South`, `East`, `North`, `West`, `Start`, `Select`, `LeftBumper`,
`RightBumper`, `DPadUp/Down/Left/Right`) and `primary_action` /
`secondary_action` honour them. The macroquad backend never fills these
lists (macroquad has no gamepad support), so they are always empty at
runtime. There is no `GamepadState`, no axes and no `input.gamepads()`;
those are *not yet implemented*.

### 7.2 v0.2 plan

- Full macroquad gamepad integration.
- Per-gamepad action maps (player 1 uses gamepad 0, player 2 uses gamepad 1).
- Analog trigger support.

## 8. Touch

### 8.1 v0.1 status

**Partial.** The macroquad backend fills `Input::touch_points` with
`TouchPoint`s; `touches_started()` / `touches_active()` iterate them and a
started touch triggers `primary_action` (a tap). Gesture recognition and
a two-finger `secondary_action` are not implemented.

```rust
pub struct TouchPoint {
    pub id: u64,
    pub position: Vec2,
    pub phase: TouchPhase,   // Started, Moved, Stationary, Ended, Cancelled (#[non_exhaustive])
}
```

### 8.2 v0.2 plan

- Full multi-touch events.
- Gesture recognition (pinch, swipe) via a `gesture` module.
- Web and native mobile treated identically.

## 9. Examples

### 9.1 Menu confirm

```rust
impl Node for MainMenu {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        if ctx.input().primary_action() {
            ctx.emit::<StartGame>(StartGame);
        }
        if ctx.input().secondary_action() {
            ctx.emit::<QuitGame>(QuitGame);
        }
    }
}
```

### 9.2 Player movement with action map

*(Illustrative — uses the not-yet-implemented action API; today use
`is_key_down(Key::A)` etc.)*

```rust
impl Node for Player {
    fn physics_process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input();
        let mut mv = Vec2::ZERO;
        if input.action("move_left")  { mv.x -= 1.0; }
        if input.action("move_right") { mv.x += 1.0; }
        if input.action("move_up")    { mv.y -= 1.0; }
        if input.action("move_down")  { mv.y += 1.0; }
        let speed = if input.action("sprint") { self.sprint_speed } else { self.walk_speed };
        self.pos += mv.normalize_or_zero() * speed * dt;
    }
}
```

### 9.3 Analog stick movement

*(Illustrative — `action_axis` is not yet implemented.)*

```rust
fn physics_process(&mut self, dt: f32, ctx: &mut NodeContext) {
    let input = ctx.input();
    let dx = input.action_axis("move_horizontal");
    let dy = input.action_axis("move_vertical");
    let mv = Vec2::new(dx, dy);
    self.pos += mv * self.speed * dt;
}
```

### 9.4 Debug key

```rust
fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
    if ctx.input().is_key_pressed(Key::F1) {
        self.debug_overlay = !self.debug_overlay;
    }
}
```

## 10. Design notes

### 10.1 Why `primary_action` and not a full action map for everything

Because menu navigation shouldn't require a config file. Every game has
a "press to start" — the `primary_action` convention gives that for
free. A game that wants a custom menu key uses an action map.

### 10.2 Why raw keys are still exposed

Because some keys don't belong in actions. Debug overlays (F1), cheat
codes, screenshot keys — these are not game input, and forcing them into
an action map is bureaucratic.

The rule of thumb: **if the player rebinds it, use an action; if the
developer presses it, use raw.**

### 10.3 Why dead zones are per-action, not global

Because different analog inputs have different noise. A movement stick
needs `0.15`; a menu cursor needs `0.3`; a precise aim needs `0.05`.
Global dead zones are a 2008-era design that everyone has since
abandoned.

### 10.4 Why gamepad/touch are stubs in v0.1

Because none of the 17 existing games use them. (Touch points and digital
gamepad buttons are now in `Input`; axes and device management are not.) Adding full support is
a real feature (SDL-style device enumeration, per-device state, device
hotplug), not a weekend task. The stubs exist so the API can be designed
now and implemented in v0.2 without breaking changes.

## 11. Open questions

- **Q1:** Should `primary_action` include gamepad "Start" or just "A"?
  *Resolved:* just "South" (A/Cross), plus touch start. Start is separate.
- **Q2:** Should actions support chords (Ctrl+S)?
  *Provisional:* not in v0.1. Single key per binding.
- **Q3:** Should action maps be hot-swappable at runtime?
  *Provisional:* yes — reloading the RON file updates the map. Useful
  for testing bindings without a restart.
- **Q4:** Should we expose raw keyboard scan codes instead of logical
  keys? *Provisional:* no. Logical keys (Key::A) map to physical scan
  codes via the OS. This is what every game engine does.
- **Q5:** Should touch be mapped to actions, or stay raw?
  *Provisional:* touch maps to `primary_action` (tap, implemented) and
  `secondary_action` (two-finger tap, not yet implemented) for menus. Full multi-touch is
  raw.

## 12. Cross-references

- `Input` in the loop → [[14-main-loop#3. The loop|14-main-loop.md §3]]
- Action map loading (planned) → [[13-assets#7. Format detection and loaders|13-assets.md §7]]
- Full `Input` API → `crates/ember-input/src/input.rs`
- `primary_action` ADR → [[50-design-decisions]]
