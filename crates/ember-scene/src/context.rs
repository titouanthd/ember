//! `NodeContext` and `DrawContext`.
//!
//! **Placeholders.** The real contexts (tree, servers, assets, input)
//! land in Phase 6. For now they are empty structs so that `Node`'s
//! lifecycle methods can have real signatures and `#[derive(Node)]`
//! tests can call `ready` / `draw` with a valid argument.

use std::marker::PhantomData;

pub struct NodeContext<'a> {
    _phantom: PhantomData<&'a mut ()>,
}

pub struct DrawContext<'a> {
    _phantom: PhantomData<&'a mut ()>,
}

impl Default for NodeContext<'_> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl Default for DrawContext<'_> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contexts_are_constructible() {
        let _: NodeContext = NodeContext::default();
        let _: DrawContext = DrawContext::default();
    }
}
