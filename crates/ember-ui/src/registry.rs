//! `ControlRegistry`: turns `dyn Node` back into `dyn Control`.
//!
//! The tree stores `Box<dyn Node>`, and Rust cannot cast a `dyn Node` to
//! a `dyn Control`. Each control type registers a pair of cast functions
//! keyed by its `TypeId` (ADR-034); layout and input routing look nodes up
//! here.

use crate::Control;
use ember_scene::Node;
use std::any::{Any, TypeId};
use std::collections::HashMap;

type CastRef = for<'a> fn(&'a dyn Node) -> Option<&'a dyn Control>;
type CastMut = for<'a> fn(&'a mut dyn Node) -> Option<&'a mut dyn Control>;

#[derive(Clone, Copy)]
struct Casts {
    as_ref: CastRef,
    as_mut: CastMut,
}

fn cast_ref<T: Control>(node: &dyn Node) -> Option<&dyn Control> {
    node.as_any().downcast_ref::<T>().map(|c| c as &dyn Control)
}

fn cast_mut<T: Control>(node: &mut dyn Node) -> Option<&mut dyn Control> {
    node.as_any_mut()
        .downcast_mut::<T>()
        .map(|c| c as &mut dyn Control)
}

#[derive(Clone, Default)]
pub struct ControlRegistry {
    casts: HashMap<TypeId, Casts>,
}

impl ControlRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry with every built-in control: `Label`, `Button`, `Panel`,
    /// `Image`, `TextInput`, `ScrollContainer`, `VBox`, `HBox`, `Stretch`.
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        r.register::<crate::Label>()
            .register::<crate::Button>()
            .register::<crate::Panel>()
            .register::<crate::Image>()
            .register::<crate::TextInput>()
            .register::<crate::ScrollContainer>()
            .register::<crate::VBox>()
            .register::<crate::HBox>()
            .register::<crate::Stretch>();
        r
    }

    /// Make nodes of type `T` visible to layout and input as controls.
    /// Registering twice is harmless.
    pub fn register<T: Control>(&mut self) -> &mut Self {
        self.casts.insert(
            TypeId::of::<T>(),
            Casts {
                as_ref: cast_ref::<T>,
                as_mut: cast_mut::<T>,
            },
        );
        self
    }

    pub fn is_registered<T: Control>(&self) -> bool {
        self.casts.contains_key(&TypeId::of::<T>())
    }

    pub fn len(&self) -> usize {
        self.casts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.casts.is_empty()
    }

    /// `node` as a control, if its concrete type is registered.
    pub fn cast<'a>(&self, node: &'a dyn Node) -> Option<&'a dyn Control> {
        let id = Any::type_id(node.as_any());
        (self.casts.get(&id)?.as_ref)(node)
    }

    pub fn cast_mut<'a>(&self, node: &'a mut dyn Node) -> Option<&'a mut dyn Control> {
        let id = Any::type_id(node.as_any());
        (self.casts.get(&id)?.as_mut)(node)
    }
}

impl std::fmt::Debug for ControlRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlRegistry")
            .field("types", &self.casts.len())
            .finish()
    }
}
