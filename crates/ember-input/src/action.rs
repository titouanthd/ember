//! Input action map (ADR-052): named actions bound to keys, gamepad
//! buttons, mouse buttons and swipes, read back through `Input`.
//!
//! An [`ActionMap`] is the bindings (data, from RON or built in code).
//! An [`ActionResolver`] owns a map plus what it remembers between
//! frames (what was down, where each finger started) and turns each raw
//! [`Input`] into one with its [`Input::actions`] filled in. `SceneLoop`
//! runs one when given a map; a test can run one by hand.
//!
//! ```
//! # use ember_input::{ActionMap, ActionResolver, Input, Key};
//! let map = ActionMap::from_ron(r#"(
//!     actions: [ (name: "move_left", keys: [A, Left], swipe: [Left]) ],
//! )"#).unwrap();
//! let mut resolver = ActionResolver::new(map);
//! let mut input = Input { keys_down: vec![Key::A], ..Default::default() };
//! resolver.resolve(&mut input);
//! assert!(input.action_pressed("move_left"));
//! ```

use crate::gamepad::GamepadButton;
use crate::input::Input;
use crate::key::Key;
use crate::mouse::MouseButton;
use crate::touch::TouchPhase;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A swipe direction, in screen space (`Up` is towards the top).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Swipe {
    Left,
    Right,
    Up,
    Down,
}

impl Swipe {
    /// The dominant direction of `delta`; a tie goes to the horizontal.
    pub fn of(delta: Vec2) -> Self {
        if delta.x.abs() >= delta.y.abs() {
            if delta.x < 0.0 {
                Self::Left
            } else {
                Self::Right
            }
        } else if delta.y < 0.0 {
            Self::Up
        } else {
            Self::Down
        }
    }
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// One named action and everything that triggers it. Any binding is
/// enough (spec 15 §5.2); a key may serve several actions (§5.3).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<Key>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gamepad: Vec<GamepadButton>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mouse: Vec<MouseButton>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub swipe: Vec<Swipe>,
}

fn default_swipe_distance() -> f32 {
    ActionMap::DEFAULT_SWIPE_DISTANCE
}

fn is_default_swipe_distance(v: &f32) -> bool {
    *v == ActionMap::DEFAULT_SWIPE_DISTANCE
}

/// Named actions, as loaded from an `input.ron` file (spec 15 §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionMap {
    /// How far a finger (or a mouse drag, with `mouse_swipe`) must move,
    /// in viewport units, before it counts as a swipe.
    #[serde(
        default = "default_swipe_distance",
        skip_serializing_if = "is_default_swipe_distance"
    )]
    pub swipe_distance: f32,
    /// Also read a left-button mouse drag as a swipe (desktop and web
    /// builds of a touch game). Off by default: it would turn every
    /// drag into a swipe.
    #[serde(default, skip_serializing_if = "is_default")]
    pub mouse_swipe: bool,
    pub actions: Vec<Action>,
}

impl Default for ActionMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Why an action map was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionMapError {
    /// The text is not a valid action map.
    Parse(String),
    /// An action has an empty name.
    EmptyName,
    /// Two actions share this name.
    DuplicateName(String),
}

impl fmt::Display for ActionMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "action map: {e}"),
            Self::EmptyName => write!(f, "action map: an action has an empty name"),
            Self::DuplicateName(n) => write!(f, "action map: action `{n}` is defined twice"),
        }
    }
}

impl std::error::Error for ActionMapError {}

impl ActionMap {
    pub const DEFAULT_SWIPE_DISTANCE: f32 = 32.0;

    pub fn new() -> Self {
        Self {
            swipe_distance: Self::DEFAULT_SWIPE_DISTANCE,
            mouse_swipe: false,
            actions: Vec::new(),
        }
    }

    /// Parse and check a map: names are non-empty and unique.
    pub fn from_ron(text: &str) -> Result<Self, ActionMapError> {
        let map: Self = ron::from_str(text).map_err(|e| ActionMapError::Parse(e.to_string()))?;
        map.validate()?;
        Ok(map)
    }

    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("an action map always serializes")
    }

    /// The same checks as [`from_ron`](Self::from_ron), for a map built
    /// in code.
    pub fn validate(&self) -> Result<(), ActionMapError> {
        for (i, a) in self.actions.iter().enumerate() {
            if a.name.is_empty() {
                return Err(ActionMapError::EmptyName);
            }
            if self.actions[..i].iter().any(|b| b.name == a.name) {
                return Err(ActionMapError::DuplicateName(a.name.clone()));
            }
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Action> {
        self.actions.iter().find(|a| a.name == name)
    }

    /// The action `name`, added (with no bindings) if missing.
    pub fn action_mut(&mut self, name: &str) -> &mut Action {
        let at = match self.actions.iter().position(|a| a.name == name) {
            Some(at) => at,
            None => {
                self.actions.push(Action {
                    name: name.to_string(),
                    ..Default::default()
                });
                self.actions.len() - 1
            }
        };
        &mut self.actions[at]
    }

    /// Builder: bind `key` to `name`.
    pub fn with_key(mut self, name: &str, key: Key) -> Self {
        self.action_mut(name).keys.push(key);
        self
    }

    pub fn with_gamepad(mut self, name: &str, button: GamepadButton) -> Self {
        self.action_mut(name).gamepad.push(button);
        self
    }

    pub fn with_mouse(mut self, name: &str, button: MouseButton) -> Self {
        self.action_mut(name).mouse.push(button);
        self
    }

    pub fn with_swipe(mut self, name: &str, swipe: Swipe) -> Self {
        self.action_mut(name).swipe.push(swipe);
        self
    }
}

/// This frame's actions, filled by an [`ActionResolver`]. Each list is
/// in the map's action order. Empty when no map is running.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActionState {
    pub pressed: Vec<String>,
    pub down: Vec<String>,
    pub released: Vec<String>,
    /// The swipe recognised this frame, if any (at most one).
    pub swipe: Option<Swipe>,
}

/// Where a swipe started and whether it already fired.
#[derive(Debug, Clone, Copy)]
struct Stroke {
    start: Vec2,
    fired: bool,
}

/// Runs an [`ActionMap`] frame after frame.
#[derive(Debug, Clone)]
pub struct ActionResolver {
    map: ActionMap,
    /// Per action, in map order: down last frame.
    was_down: Vec<bool>,
    touches: HashMap<u64, Stroke>,
    mouse: Option<Stroke>,
}

impl ActionResolver {
    pub fn new(map: ActionMap) -> Self {
        let n = map.actions.len();
        Self {
            map,
            was_down: vec![false; n],
            touches: HashMap::new(),
            mouse: None,
        }
    }

    pub fn map(&self) -> &ActionMap {
        &self.map
    }

    /// Fill `input.actions` from `input`'s raw state.
    ///
    /// - An action is **down** while any of its keys, gamepad buttons or
    ///   mouse buttons is down (or pressed this frame), and on the one
    ///   frame a bound swipe is recognised.
    /// - **pressed** / **released** are edges of *down* across frames,
    ///   so holding `A` and then pressing `Left` does not press
    ///   `move_left` twice.
    /// - A **swipe** is recognised once per stroke, on the first frame
    ///   the finger is `swipe_distance` from where it touched down, in
    ///   the dominant direction.
    pub fn resolve(&mut self, input: &mut Input) {
        let swipe = self.track_swipes(input);
        let mut state = ActionState {
            swipe,
            ..Default::default()
        };
        for (i, action) in self.map.actions.iter().enumerate() {
            let down = action
                .keys
                .iter()
                .any(|k| input.is_key_down(*k) || input.is_key_pressed(*k))
                || action
                    .gamepad
                    .iter()
                    .any(|b| input.is_gamepad_down(*b) || input.is_gamepad_pressed(*b))
                || action
                    .mouse
                    .iter()
                    .any(|b| input.is_mouse_down(*b) || input.is_mouse_pressed(*b))
                || swipe.is_some_and(|s| action.swipe.contains(&s));
            let was = self.was_down[i];
            if down {
                state.down.push(action.name.clone());
                if !was {
                    state.pressed.push(action.name.clone());
                }
            } else if was {
                state.released.push(action.name.clone());
            }
            self.was_down[i] = down;
        }
        input.actions = state;
    }

    fn track_swipes(&mut self, input: &Input) -> Option<Swipe> {
        let distance = self.map.swipe_distance.max(0.0);
        let mut found = None;
        let mut check = |stroke: &mut Stroke, at: Vec2| {
            let delta = at - stroke.start;
            if !stroke.fired && delta.length() >= distance && delta != Vec2::ZERO {
                stroke.fired = true;
                found.get_or_insert(Swipe::of(delta));
            }
        };
        for t in &input.touch_points {
            match t.phase {
                TouchPhase::Started => {
                    self.touches.insert(
                        t.id,
                        Stroke {
                            start: t.position,
                            fired: false,
                        },
                    );
                }
                TouchPhase::Cancelled => {
                    self.touches.remove(&t.id);
                }
                TouchPhase::Ended => {
                    if let Some(mut s) = self.touches.remove(&t.id) {
                        check(&mut s, t.position);
                    }
                }
                _ => {
                    if let Some(s) = self.touches.get_mut(&t.id) {
                        check(s, t.position);
                    }
                }
            }
        }
        // A finger the backend stopped reporting is gone.
        let live: Vec<u64> = input.touch_points.iter().map(|t| t.id).collect();
        self.touches.retain(|id, _| live.contains(id));

        if self.map.mouse_swipe {
            if input.mouse_left_pressed {
                self.mouse = Some(Stroke {
                    start: input.mouse_pos,
                    fired: false,
                });
            } else if let Some(s) = &mut self.mouse {
                check(s, input.mouse_pos);
            }
            if input.mouse_left_released || !(input.mouse_left_down || input.mouse_left_pressed) {
                self.mouse = None;
            }
        }
        found
    }
}
