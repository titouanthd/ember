# 15 — Input

> **Status:** ⬜ draft
> **Audience:** game authors, implementers
> **Prerequisites:** [14-main-loop.md](14-main-loop.md)

## 1. Purpose

Explain how Ember reads input: raw key/mouse state, the
`primary_action` convention, action maps, dead zones for analog input,
and the gamepad and touch stubs.

The key takeaway: **most games should use `primary_action` for start /
confirm, and action maps for gameplay**. Raw key checks are for the rare
case where you need exact key state.

## 2. Raw vs mapped input

Ember exposes two layers of input:

### 2.1 Raw input

Direct key/mouse state, polled every frame:

```rust
ctx.input().key(Key::Space)          // is Space held down?
ctx.input().pressed(Key::Enter)      // was Enter pressed this frame?
ctx.input().mouse_position()         // current mouse position
ctx.input().mouse_pressed(MouseButton::Left)
```

This is what you use when you need exact hardware-level state.

### 2.2 Mapped input (action maps)

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

### 3.2 Why it exists

Every game currently implements the same pattern:

```rust
if input.pressed(Key::Space) || input.pressed(Key::Enter) || input.mouse_pressed(MouseButton::Left) {
    self.start();
}
```

That's three checks per game, times 18 games, times every menu
transition. `primary_action` collapses it to one call:

```rust
if ctx.input().primary_action() {
    self.start();
}
```

And it works consistently across platforms — a touch on mobile maps to
`primary_action`, a gamepad button on desktop maps to `primary_action`.

### 3.3 What it does not include

- Escape (that's `secondary_action` or a raw check)
- Backspace (same)
- Any modifier (Shift, Ctrl, Alt)

The convention is **"press to advance"** — the same action as a
touchscreen tap.

## 4. The `Input` struct

```rust
pub struct Input {
    // Keyboard
    keys_down: HashSet<Key>,
    keys_pressed: HashSet<Key>,
    keys_released: HashSet<Key>,

    // Mouse
    mouse_pos: Vec2,
    mouse_buttons_down: HashSet<MouseButton>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_wheel: f32,

    // Gamepad (stub in v0.1)
    gamepads: Vec<GamepadState>,

    // Touch (stub in v0.1)
    touches: Vec<TouchState>,

    // Actions
    actions: ActionMap,
    prev_actions: HashSet<String>,
}
```

### 4.1 Query methods

```rust
impl Input {
    // Keyboard
    pub fn key(&self, key: Key) -> bool;
    pub fn pressed(&self, key: Key) -> bool;
    pub fn released(&self, key: Key) -> bool;

    // Mouse
    pub fn mouse_position(&self) -> Vec2;
    pub fn mouse_button(&self, button: MouseButton) -> bool;
    pub fn mouse_pressed(&self, button: MouseButton) -> bool;
    pub fn mouse_wheel(&self) -> f32;

    // Actions
    pub fn action(&self, name: &str) -> bool;
    pub fn action_pressed(&self, name: &str) -> bool;
    pub fn action_released(&self, name: &str) -> bool;
    pub fn action_axis(&self, name: &str) -> f32;   // -1.0 to 1.0

    // Convenience
    pub fn primary_action(&self) -> bool;
    pub fn secondary_action(&self) -> bool;   // Escape / Backspace
}
```

### 4.2 Frame semantics

`pressed` and `released` refer to **this frame only**. A key that was
pressed three frames ago returns `false` for `pressed` on frame 4.
`key` (or `action`) returns `true` for as long as the key is held.

This is the standard input model from every game engine since the 90s.
Alternatives (event queues, callbacks) exist but are worse for games
because they decouple input from the frame where the game logic runs.

## 5. Action map format

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
ctx.assets().reload_action_map("input.ron");
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

**Stub.** The `GamepadState` struct exists but is not populated — macroquad
has minimal gamepad support and we defer full support to v0.2.

```rust
pub struct GamepadState {
    pub buttons: HashSet<GamepadButton>,
    pub axes: HashMap<GamepadAxis, f32>,
    pub connected: bool,
}
```

Games that need gamepad support today should query `input.gamepads()`
and handle the empty case.

### 7.2 v0.2 plan

- Full macroquad gamepad integration.
- Per-gamepad action maps (player 1 uses gamepad 0, player 2 uses gamepad 1).
- Analog trigger support.

## 8. Touch

### 8.1 v0.1 status

**Stub.** `TouchState` exists but only `primary_action` is wired up (a
tap maps to `primary_action`). This is enough for menu-driven games on
mobile; full multi-touch is v0.2.

```rust
pub struct TouchState {
    pub id: u64,
    pub position: Vec2,
    pub phase: TouchPhase,   // Started, Moved, Ended, Cancelled
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
        if ctx.input().pressed(Key::Escape) {
            ctx.emit::<QuitGame>(QuitGame);
        }
    }
}
```

### 9.2 Player movement with action map

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
    if ctx.input().pressed(Key::F1) {
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

Because none of the 18 existing games use them. Adding full support is
a real feature (SDL-style device enumeration, per-device state, device
hotplug), not a weekend task. The stubs exist so the API can be designed
now and implemented in v0.2 without breaking changes.

## 11. Open questions

- **Q1:** Should `primary_action` include gamepad "Start" or just "A"?
  *Provisional:* just "South" (A/Cross). Start is separate
  (`action("pause")` by default).
- **Q2:** Should actions support chords (Ctrl+S)?
  *Provisional:* not in v0.1. Single key per binding.
- **Q3:** Should action maps be hot-swappable at runtime?
  *Provisional:* yes — reloading the RON file updates the map. Useful
  for testing bindings without a restart.
- **Q4:** Should we expose raw keyboard scan codes instead of logical
  keys? *Provisional:* no. Logical keys (Key::A) map to physical scan
  codes via the OS. This is what every game engine does.
- **Q5:** Should touch be mapped to actions, or stay raw?
  *Provisional:* touch maps to `primary_action` (tap) and
  `secondary_action` (two-finger tap) for menus. Full multi-touch is
  raw.

## 12. Cross-references

- `Input` in the loop → [14-main-loop.md §3](14-main-loop.md#3-the-loop)
- Action map loading → [13-assets.md §7](13-assets.md#7-format-detection-and-loaders)
- Full `Input` API → [20-ember-core.md](20-ember-core.md)
- `primary_action` ADR → [50-design-decisions.md](50-design-decisions.md)