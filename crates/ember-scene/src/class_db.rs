//! Node construction through `ClassDB`. `ember-core` cannot name `Node`,
//! so the node constructor rides along as a `ClassDB` extension (ADR-024).

use crate::signal::Signal;
use crate::{Node, NodeId, NodeTree};
use ember_core::ClassDB;

type NodeCtor = Box<dyn Fn() -> Box<dyn Node> + Send + Sync>;

struct NodeFactory(NodeCtor);

type Wire = Box<dyn Fn(&mut NodeTree, NodeId) + Send + Sync>;

struct Receiver {
    signal: String,
    method: String,
    wire: Wire,
}

/// The methods of one class that scene connections can call.
#[derive(Default)]
struct Receivers(Vec<Receiver>);

pub trait ClassDbNodes {
    /// Register a node class built with `T::default()`.
    fn register_node<T: Node + Default>(&mut self);
    /// Register a node class with a custom constructor.
    fn register_node_with<T: Node>(&mut self, ctor: fn() -> T);
    /// A fresh node of `class_name`; `None` if unregistered or if the
    /// class was registered as a plain `Object`.
    fn instantiate_node(&self, class_name: &str) -> Option<Box<dyn Node>>;
    /// Let a scene connection `(signal: <signal>, to: <an N>, method:
    /// <method>)` call `f` on the target with each `T` emitted (spec 30
    /// §8). `N` must be registered first.
    fn register_receiver<N: Node, T: Signal>(
        &mut self,
        signal: &str,
        method: &str,
        f: fn(&mut N, &T),
    );
    /// Connect `target` (of class `class_name`) to `signal`, calling
    /// `method`. `false` if the class has no such receiver.
    fn wire_connection(
        &self,
        class_name: &str,
        signal: &str,
        method: &str,
        tree: &mut NodeTree,
        target: NodeId,
    ) -> bool;
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

    fn register_receiver<N: Node, T: Signal>(
        &mut self,
        signal: &str,
        method: &str,
        f: fn(&mut N, &T),
    ) {
        let class = N::class_name();
        assert!(
            self.is_registered(class),
            "register_receiver: register the node class `{class}` first"
        );
        self.set_extension(class, Receivers::default());
        let wire: Wire = Box::new(move |tree: &mut NodeTree, target: NodeId| {
            tree.connect::<T>(target, move |ctx, payload| {
                let id = ctx.current_id();
                let node = ctx.tree().get_mut(id);
                if let Some(n) = node.and_then(|n| n.as_any_mut().downcast_mut::<N>()) {
                    f(n, payload);
                }
            });
        });
        let receivers = self
            .extension_mut::<Receivers>(class)
            .expect("just inserted");
        receivers.0.push(Receiver {
            signal: signal.to_string(),
            method: method.to_string(),
            wire,
        });
    }

    fn wire_connection(
        &self,
        class_name: &str,
        signal: &str,
        method: &str,
        tree: &mut NodeTree,
        target: NodeId,
    ) -> bool {
        let Some(receivers) = self.extension::<Receivers>(class_name) else {
            return false;
        };
        let Some(r) = receivers
            .0
            .iter()
            .find(|r| r.signal == signal && r.method == method)
        else {
            return false;
        };
        (r.wire)(tree, target);
        true
    }
}
