---
title: "24 — `ember_ui`"
aliases: ["24 — `ember_ui`"]
tags: ["ember/spec", "ember/wave-3"]
doc-status: "frozen"
former-path: ".worktrees/titouanthd-potential-carnival/docs/engine/v0.1/24-ember-ui.md"
imported: "2026-11-22"
wave: "3 — Crate Specifications"
---

# 24 — `ember_ui`

> **Status:** ✅ frozen 2026-11-22 (Phase 5 entry gate G7) — changes need an ADR. Not implemented yet (`crates/ember-ui` is a stub).
> **Audience:** game authors, implementers
> **Prerequisites:** [[21-ember-scene]], [[23-ember-render-macroquad]]

> **Not yet implemented.** `crates/ember-ui/src/lib.rs` contains only a doc
> comment; no module, type or trait in this document exists. It is still
> a dependency of the `ember` facade crate. `#[derive(Node)]` +
> `impl Node for T { … }` as used below works since ADR-030; the struct still
> needs `#[class_name]` and a user-declared `name: String`, and the derive
> generates no `new()` or `#[export]` handling (ADR-019, [[26-ember-macros]]). Only pong's `Scoreboard`
> (hand-drawn text via `DrawContext`) stands in for UI in games today.

## 0. Engine contract for v0.1 (frozen 2026-11-22)

What the implementation relies on, so the sections below can stay short:

| Need | Engine API | ADR |
|---|---|---|
| Widgets are nodes with behaviour | `#[derive(Node)]` gives `Object` + `NodeName`; each widget writes `impl Node for T { … }` and `impl Control for T { … }`. **Every derived struct declares `name: String`** — the snippets below omit it for brevity. | ADR-019, ADR-030 |
| `UiDrawContext::push_clip` / `pop_clip` | `RenderingServer::push_clip_rect` / `pop_clip_rect`, screen pixels, nested pushes intersect | ADR-031 |
| `TextInput` characters | `Input::chars_typed` (layout-aware, no control chars); editing keys via `keys_pressed`: Backspace, Delete, Left, Right, Home, End, Enter | ADR-032 |
| `ScrollContainer` wheel | `Input::mouse_wheel.y` (`> 0` = up → `scroll_y` decreases, as in §4.5) | ADR-032 |
| Focus / activation | `Input::primary_action()` activates the focused control; Tab / Shift+Tab move focus | ADR-010 |
| UI coordinates | screen space; `UiRoot` draws at `z = 100.0`, ignores the camera | §12 Q4 |

The open questions in §12 are **decided** as their provisional answers.

## 1. Purpose

Specify retained-mode UI: the `Control` trait, layout algorithms
(anchors, vbox/hbox, stretch), widget nodes (Button, Label, Panel,
Scroll, TextInput), the focus system, and the theming system.

`ember_ui` is optional. Games without UI (bullet-hell, asteroids) don't
depend on it. Games with UI get the same widgets, styled consistently.

## 2. Module layout

```
ember_ui/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── prelude.rs
│   ├── control.rs        Control trait
│   ├── layout/
│   │   ├── mod.rs
│   │   ├── anchors.rs    Anchors
│   │   ├── vbox.rs       VBox
│   │   ├── hbox.rs       HBox
│   │   ├── stretch.rs    Stretch
│   │   └── scroll.rs     ScrollContainer
│   ├── widgets/
│   │   ├── button.rs     Button
│   │   ├── label.rs      Label
│   │   ├── panel.rs      Panel
│   │   ├── text_input.rs TextInput
│   │   └── image.rs      Image
│   ├── focus.rs          FocusManager
│   ├── theme.rs          Theme
│   └── error.rs          UiError
```

## 3. `Control` trait

### 3.1 Trait

```rust
pub trait Control: Node {
    /// Layout request. Called by parent controls during their own layout.
    fn measure(&self, available: Vec2) -> Vec2;

    /// Final rect. Set by parent during layout.
    fn rect(&self) -> Rect;

    /// Resize to a new rect.
    fn set_rect(&mut self, rect: Rect);

    /// Anchor-driven layout input.
    fn anchors(&self) -> Anchors { Anchors::TOP_LEFT }

    /// Minimum size hint (padding, border).
    fn min_size(&self) -> Vec2 { Vec2::ZERO }

    /// True if this control can receive focus.
    fn focusable(&self) -> bool { false }

    /// True if this control wants mouse events.
    fn wants_mouse(&self) -> bool { true }

    // --- Event handlers (all defaulted) ---
    fn on_mouse_enter(&mut self) {}
    fn on_mouse_leave(&mut self) {}
    fn on_mouse_down(&mut self, _button: MouseButton, _pos: Vec2) {}
    fn on_mouse_up(&mut self, _button: MouseButton, _pos: Vec2) {}
    fn on_mouse_wheel(&mut self, _delta: f32) {}
    fn on_key_press(&mut self, _key: Key) {}
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}

    // --- Drawing ---
    fn draw_control(&self, ctx: &mut UiDrawContext);
}
```

### 3.2 `UiDrawContext`

Extends `DrawContext` with UI-specific helpers:

```rust
pub struct UiDrawContext<'a> {
    inner: &'a mut DrawContext<'a>,
    theme: &'a Theme,
    focus: Option<NodeId>,
    clip_stack: Vec<Rect>,
}

impl<'a> UiDrawContext<'a> {
    pub fn render(&mut self) -> &mut dyn RenderingServer;
    pub fn theme(&self) -> &Theme;
    pub fn has_focus(&self, id: NodeId) -> bool;
    pub fn push_clip(&mut self, rect: Rect);
    pub fn pop_clip(&mut self);
    pub fn draw_background(&mut self, rect: Rect, color: Color);
    pub fn draw_border(&mut self, rect: Rect, thickness: f32, color: Color);
    pub fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color);
}
```

### 3.3 Control as Node

`Control: Node` means every control has `ready`, `process`,
`physics_process`, `draw`, `finalize` from the `Node` trait. The
default implementations in `Control`-derived types:

- `ready`: measure children, request layout.
- `process`: forward input to `on_*` handlers.
- `draw`: call `draw_control`.

Games do **not** implement `Node` on controls directly — they extend the
`Control` trait. A macro `#[derive(Control)]` (part of `ember_macros`,
v0.2) will generate the boilerplate.

## 4. Layout

### 4.1 The layout model

Every control participates in its parent's layout. The parent:

1. Calls `children[i].measure(available)` to get each child's desired size.
2. Decides each child's final rect (via anchors, vbox, hbox, stretch, etc.).
3. Calls `children[i].set_rect(rect)`.

Layout is **one pass** (no multi-pass constraint solving). This is the
same approach Godot's `Container` uses and is fast enough for real games.

### 4.2 `Anchors`

```rust
#[derive(Copy, Clone, Debug)]
pub struct Anchors {
    pub left: f32,     // 0.0 = left edge of parent, 1.0 = right edge
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Anchors {
    pub const FULL_RECT: Self = Self { left: 0.0, top: 0.0, right: 1.0, bottom: 1.0 };
    pub const TOP_LEFT: Self = Self { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 };
    pub const CENTER: Self = Self { left: 0.5, top: 0.5, right: 0.5, bottom: 0.5 };

    pub fn compute_rect(self, parent: Rect) -> Rect;
}
```

`compute_rect` maps the 0..1 anchors into `parent`'s coordinates:

```
left_x   = parent.x + self.left   * parent.w
right_x  = parent.x + self.right  * parent.w
top_y    = parent.y + self.top    * parent.h
bottom_y = parent.y + self.bottom * parent.h
rect = Rect { x: left_x, y: top_y, w: right_x - left_x, h: bottom_y - top_y }
```

For `FULL_RECT`, the child fills the parent. For `TOP_LEFT`, the child
has zero size (parent then uses `min_size()`).

### 4.3 `VBox` / `HBox`

```rust
#[derive(Node)]
#[class_name = "VBox"]
pub struct VBox {
    children_controls: Vec<NodeId>,
    spacing: f32,
    padding: Rect,
}
```

VBox stacks its `Control` children top-to-bottom:

1. `measure`: `sum(children.min_size().y) + spacing * (n - 1) + padding.vertical()`
2. `set_rect`: each child gets `(parent.x + padding.x, y, parent.w - padding.horizontal(), child_min_size.y)`.
3. `y` advances by `child_height + spacing`.

HBox mirrors on the x axis.

### 4.4 `Stretch`

Wraps a single child. Two modes:

- `StretchMode::Fill`: child gets the parent's rect (minus padding).
- `StretchMode::Fit`: child keeps its measured size, centered in parent.

### 4.5 `ScrollContainer`

```rust
#[derive(Node)]
#[class_name = "ScrollContainer"]
pub struct ScrollContainer {
    content: NodeId,
    scroll_y: f32,
    max_scroll: f32,
}
```

Wraps a `VBox` (or any control) taller than the visible area. Provides
scrolling via mouse wheel and keyboard (arrow keys when focused).

`draw_control`:
1. Push a clip rect (the container's rect).
2. Draw content offset by `scroll_y`.
3. Pop clip.

`on_mouse_wheel(delta)`: `scroll_y = (scroll_y - delta * SCROLL_SPEED).clamp(0, max_scroll)`.

## 5. Widgets

### 5.1 `Label`

```rust
#[derive(Node)]
#[class_name = "Label"]
pub struct Label {
    text: String,
    size: f32,
    color: Color,
    alignment: Alignment,
}

impl Control for Label {
    fn measure(&self, available: Vec2) -> Vec2 {
        self.size_vec(available)  // cached measure_text result
    }
    fn draw_control(&self, ctx: &mut UiDrawContext) {
        let rect = self.rect();
        let pos = self.alignment.place(rect, self.measure(rect.size()));
        ctx.draw_text(&self.text, pos, self.size, self.color);
    }
}
```

### 5.2 `Button`

```rust
#[derive(Node)]
#[class_name = "Button"]
pub struct Button {
    label: String,
    rect: Rect,
    state: ButtonState,   // Normal, Hover, Pressed, Disabled
    disabled: bool,
}

impl Control for Button {
    fn measure(&self, _available: Vec2) -> Vec2 {
        let text = self.text_size();
        Vec2::new(text.x + PADDING_H * 2.0, text.y + PADDING_V * 2.0)
    }

    fn on_mouse_enter(&mut self) { self.state = ButtonState::Hover; }
    fn on_mouse_leave(&mut self) { self.state = ButtonState::Normal; }
    fn on_mouse_down(&mut self, button, _pos) {
        if !self.disabled && button == MouseButton::Left {
            self.state = ButtonState::Pressed;
        }
    }
    fn on_mouse_up(&mut self, button, pos) {
        if !self.disabled && button == MouseButton::Left && self.rect.contains(pos) {
            self.emit_clicked();
        }
        self.state = if self.rect.contains(pos) { ButtonState::Hover } else { ButtonState::Normal };
    }

    fn draw_control(&self, ctx: &mut UiDrawContext) {
        ctx.draw_background(self.rect, ctx.theme().button_bg(self.state));
        ctx.draw_border(self.rect, 1.0, ctx.theme().button_border(self.state));
        let text_size = ctx.render().measure_text(&self.label, 16.0);
        let pos = self.rect.center() - text_size * 0.5;
        ctx.draw_text(&self.label, pos, 16.0, ctx.theme().button_text(self.state));
    }
}
```

`ButtonClicked` is a signal emitted by every button (`#[signal]` in
`ember_ui`). Games connect to it to react.

### 5.3 `Panel`

```rust
#[derive(Node)]
#[class_name = "Panel"]
pub struct Panel {
    rect: Rect,
    background: Color,
    border: Option<(f32, Color)>,
    padding: Rect,
}
```

Used as a layout container. Has no logic; just draws a colored rect with
optional border.

### 5.4 `TextInput`

```rust
#[derive(Node)]
#[class_name = "TextInput"]
pub struct TextInput {
    text: String,
    placeholder: String,
    cursor: usize,
    focused: bool,
}
```

Single-line text input. Handles `on_key_press` (characters, backspace,
delete, arrows), `on_focus`/`on_blur`. Emits `TextChanged` and
`TextSubmitted` signals.

**v0.1 scope:** single-line, no selection, no copy/paste. Full text
editing is a v0.2 feature.

### 5.5 `Image`

```rust
#[derive(Node)]
#[class_name = "Image"]
pub struct Image {
    handle: Handle<Texture>,
    rect: Rect,
    stretch_mode: StretchMode,
}
```

Draws a texture in a rect. Stretch modes: Fill (aspect-breaking),
AspectFit (letterbox), AspectFill (crop).

## 6. Focus system

### 6.1 `FocusManager`

```rust
pub struct FocusManager {
    focused: Option<NodeId>,
    focusables: Vec<NodeId>,   // in tree order
}

impl FocusManager {
    pub fn tick(&mut self, tree: &NodeTree, input: &Input);
    pub fn focused(&self) -> Option<NodeId>;
    pub fn set_focus(&mut self, id: Option<NodeId>);
    pub fn focus_next(&mut self);
    pub fn focus_prev(&mut self);
}
```

### 6.2 Keyboard navigation

- `Tab` / arrow keys move focus forward/backward.
- `Enter` / `Space` activates the focused control (calls `on_mouse_up`
  equivalent).

Focus state is drawn by the widget (a border or highlight color from
`Theme`).

### 6.3 Focus semantics

- Only one control has focus at a time.
- Focus follows tree order (depth-first).
- Blurring a control that doesn't want to lose focus (a `TextInput`
  with pending input) can reject the change — see `on_blur() -> bool`
  (v0.2).

## 7. Theming

### 7.1 `Theme`

```rust
pub struct Theme {
    pub font_size: f32,
    pub font_size_large: f32,
    pub text: Color,
    pub text_dim: Color,
    pub panel_bg: Color,
    pub panel_border: Color,
    pub button_bg_normal: Color,
    pub button_bg_hover: Color,
    pub button_bg_pressed: Color,
    pub button_bg_disabled: Color,
    pub button_text_normal: Color,
    pub button_text_disabled: Color,
    pub focus_border: Color,
    pub scrollbar: Color,
}

impl Theme {
    pub fn dark() -> Self;
    pub fn light() -> Self;
    pub fn from_ron(path: &Path) -> Result<Self, UiError>;
}
```

### 7.2 Using a theme

Themes are installed on the `NodeTree`'s UI root:

```rust
#[derive(Node)]
#[class_name = "UiRoot"]
pub struct UiRoot {
    theme: Theme,
}

impl Node for UiRoot {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.tree().set_ui_theme(self.theme.clone());
    }
}
```

Child controls read `ctx.theme()` in `draw_control`.

### 7.3 Per-widget overrides

A widget that needs a non-theme color overrides in its own fields:

```rust
struct CustomButton {
    bg: Option<Color>,   // None = use theme
    // ...
}
```

## 8. UI tree pattern

Every game with UI has the same root structure:

```
Root
├── World        (game world, rendered first)
└── UiRoot       (ember_ui, rendered on top)
    └── VBox
        ├── Label
        ├── Button
        └── ...
```

The `UiRoot` sets `z = 100.0` so it always draws above the world. Its
`theme` is set once and inherited by all descendants.

## 9. Input routing

### 9.1 Order

When a frame's input arrives:

1. `FocusManager::tick` processes `Tab` / arrows, updates focus.
2. If a control has focus, it gets keyboard events first (`on_key_press`).
3. Mouse events go top-down through the tree — the deepest control
   whose rect contains the mouse gets them (`on_mouse_enter` etc.).
4. Controls can consume events (`on_mouse_down` returning `true`
   prevents propagation to parents) — v0.2, v0.1 propagates everywhere.

### 9.2 Mouse consumption

For v0.1, mouse events propagate to all controls under the pointer. A
game that needs "click-through" vs "consume" manages it manually via
`Control::wants_mouse` (false = skip me).

## 10. Example: a menu

```rust
#[derive(Node)]
#[class_name = "MainMenu"]
pub struct MainMenu { root: NodeId }

impl Node for MainMenu {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let vbox = ctx.spawn(VBox::new(vec![
            // buttons created dynamically below
        ]));
        // ...

        let play = ctx.spawn(Button::new("Play"));
        ctx.tree().attach(play, vbox);

        ctx.connect::<ButtonClicked>(play, |ctx, _| {
            ctx.emit::<StartGame>(StartGame);
        });

        self.root = vbox;
    }
}
```

## 11. Design notes

### 11.1 Why one-pass layout, not constraint solving

Constraint solvers (like Flutter's) are powerful but complex and slow.
Godot's `Container` uses single-pass layout and covers 95% of UI
patterns. Ember follows Godot — simple is right.

### 11.2 Why anchors and not percentages

Anchors express "left edge at 20% of parent" (`left: 0.2`). Percentages
express "left edge at 20px + 20% of parent width". Anchors are more
composable and are what every engine uses since CSS 1.

### 11.3 Why a `Control` trait and not bare nodes

Because layout, focus, and input routing are cross-cutting. A button
isn't just a rect — it's a rect that measures text, changes state on
hover, emits on click. `Control` bundles the cross-cutting methods so
each widget only implements the specifics.

### 11.4 Why v0.1 has no drag-and-drop

Because drag-and-drop is 3× the complexity of click handling (drag
threshold, drag ghost, drop targets, `CanDrop` negotiation, cancellation).
The 17 games don't need it; shipping it half-done would be worse than
not shipping it.

## 12. Open questions (decided 2026-11-22: provisional answers adopted)

- **Q1:** Should `Control::on_mouse_up` return `bool` (consumed)?
  *Provisional:* v0.2. v0.1 propagates everywhere, and 17/17 games work.
- **Q2:** Should `TextInput` support copy/paste? *Provisional:* v0.2.
  Needs clipboard access (platform-specific).
- **Q3:** Should `Theme` be a `Resource` (asset-loaded) or a struct?
  *Provisional:* struct in v0.1, `Resource` in v0.2 when the resource
  system lands.
- **Q4:** Should the UI use a separate `RenderingServer` pass? *Provisional:*
  no. Same server, `z = 100.0` on `UiRoot`. Keeps the renderer simple.

## 13. Cross-references

- `Node` and `Control: Node` → [[21-ember-scene#4. `Node` trait|21-ember-scene.md §4]]
- Rendering under the hood → [[23-ember-render-macroquad]]
- Theme asset loading → [[25-ember-assets]]
- Signals used by widgets → [[11-signals]]
