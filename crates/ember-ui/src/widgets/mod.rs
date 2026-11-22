//! Built-in widgets (spec §5). `UiRoot::new` registers all of them.

mod image;
mod label;
mod panel;

pub use image::{Image, ImageMode};
pub use label::{Alignment, Label};
pub use panel::Panel;

/// `Object` + `NodeName` for a built-in control with a `name: String`
/// field. `ember-ui` cannot use `#[derive(Node)]` (`ember-macros` is not a
/// dependency, see `xtask check-layers`), so it writes the same impls.
macro_rules! ui_object {
    ($t:ty, $class:literal) => {
        impl ::ember_core::Object for $t {
            fn class_name() -> &'static str {
                $class
            }
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
                self
            }
        }

        impl ::ember_scene::NodeName for $t {
            fn name(&self) -> &str {
                &self.name
            }
            fn set_name(&mut self, name: ::std::string::String) {
                self.name = name;
            }
        }
    };
}

pub(crate) use ui_object;
