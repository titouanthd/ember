//! Node construction through `ClassDB`. `ember-core` cannot name `Node`,
//! so the node constructor rides along as a `ClassDB` extension (ADR-024).

use crate::Node;
use ember_core::ClassDB;

type NodeCtor = Box<dyn Fn() -> Box<dyn Node> + Send + Sync>;

struct NodeFactory(NodeCtor);

pub trait ClassDbNodes {
    /// Register a node class built with `T::default()`.
    fn register_node<T: Node + Default>(&mut self);
    /// Register a node class with a custom constructor.
    fn register_node_with<T: Node>(&mut self, ctor: fn() -> T);
    /// A fresh node of `class_name`; `None` if unregistered or if the
    /// class was registered as a plain `Object`.
    fn instantiate_node(&self, class_name: &str) -> Option<Box<dyn Node>>;
}

impl ClassDbNodes for ClassDB {
    fn register_node<T: Node + Default>(&mut self) {
        self.register_node_with::<T>(T::default);
    }

    fn register_node_with<T: Node>(&mut self, ctor: fn() -> T) {
        self.register_with::<T>(ctor);
        self.set_extension(
            T::class_name(),
            NodeFactory(Box::new(move || Box::new(ctor()))),
        );
    }

    fn instantiate_node(&self, class_name: &str) -> Option<Box<dyn Node>> {
        Some((self.extension::<NodeFactory>(class_name)?.0)())
    }
}
