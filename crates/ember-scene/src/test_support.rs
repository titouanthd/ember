//! Minimal hand-written node for unit tests.

use crate::Node;
use ember_core::Object;
use std::any::Any;

pub struct Dummy {
    name: String,
}

impl Dummy {
    pub fn named(name: &str) -> Self {
        Self { name: name.into() }
    }
}

impl Object for Dummy {
    fn class_name() -> &'static str {
        "Dummy"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Node for Dummy {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}
