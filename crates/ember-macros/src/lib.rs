//! Procedural macros for the Ember engine.
//!
//! Three macros:
//!
//! - `#[derive(Node)]` — generates `impl Object` and `impl NodeName` for a
//!   struct. Requires `#[class_name = "..."]` and a `name: String`
//!   field. The user writes `impl Node for T { … }` with only the
//!   lifecycle the node needs (`impl Node for T {}` for none). See
//!   ADR-019 and ADR-030.
//! - `#[signal]` — no-op marker. Any `Send + Sync + 'static` type is
//!   already a `Signal` via a blanket impl; the attribute exists to
//!   make signal declarations greppable.
//! - `#[export]` — not a standalone macro; recognized as a field
//!   attribute by `#[derive(Node)]`. No effect in v0.1.

use proc_macro::TokenStream;

mod node;

/// Generate `Object` and `NodeName` implementations. Pair it with an
/// `impl Node for T { … }` block (ADR-030).
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
