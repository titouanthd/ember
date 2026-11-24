//! Procedural macros for the Ember engine.
//!
//! Three macros:
//!
//! - `#[derive(Node)]` — generates `impl Object` and `impl NodeName` for a
//!   struct. Requires `#[class_name = "..."]` and a `name: String`
//!   field. The user writes `impl Node for T { … }` with only the
//!   lifecycle the node needs (`impl Node for T {}` for none). See
//!   ADR-019, ADR-030 and ADR-037.
//! - `#[signal]` — no-op marker. Any `Send + Sync + 'static` type is
//!   already a `Signal` via a blanket impl; the attribute exists to
//!   make signal declarations greppable.
//! - `#[export]` — a field attribute read by `#[derive(Node)]`: the field
//!   becomes a property (`property_list`, `get_property`,
//!   `set_property` on `NodeName`) that `.escene` files load and the
//!   scene writer saves. Its type must implement
//!   `ember_core::VariantValue`. `#[export(skip)]` is the same as no
//!   attribute (ADR-037).

use proc_macro::TokenStream;

mod node;

/// Generate `Object` and `NodeName` implementations, including property
/// access for `#[export]` fields. Pair it with an `impl Node for T { … }`
/// block (ADR-030, ADR-037).
///
/// Requires:
/// - `#[class_name = "..."]` on the struct
/// - A field `name: String` in the struct
///
/// See `10-nodes-and-scenes.md §4` and ADR-019.
#[proc_macro_derive(Node, attributes(class_name, export))]
pub fn derive_node(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match node::derive(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// No-op attribute. Marks a type as a signal for readability and future
/// tooling. Does not modify the type.
#[proc_macro_attribute]
pub fn signal(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
