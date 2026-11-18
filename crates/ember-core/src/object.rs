//! The `Object` trait — dynamic typing for the scene layer.
//!
//! `Object` is the smallest common interface every type that can live
//! in a scene supports: a class name for serialization, and `Any`
//! downcasting for engine internals that need to reach concrete types.
//!
//! See `10-nodes-and-scenes.md §2.7` and ADR-019 for the derive's role.

use std::any::Any;

pub trait Object: Send + Sync + 'static {
    /// The type name as it appears in `.escene` files and `ClassDB`.
    ///
    /// `Self: Sized` because it is a static method — you call it as
    /// `T::class_name()`, not on a `&dyn Object`. The derive generates
    /// it from `#[class_name = "..."]`.
    fn class_name() -> &'static str
    where
        Self: Sized;

    /// Upcast to `&dyn Any` for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Upcast to `&mut dyn Any` for mutable downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy {
        name: String,
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

    #[test]
    fn class_name_is_the_registered_name() {
        assert_eq!(Dummy::class_name(), "Dummy");
    }

    #[test]
    fn as_any_downcasts() {
        let d = Dummy { name: "x".into() };
        assert_eq!(d.as_any().downcast_ref::<Dummy>().unwrap().name, "x");
    }

    #[test]
    fn as_any_mut_downcasts_and_mutates() {
        let mut d = Dummy { name: "x".into() };
        d.as_any_mut().downcast_mut::<Dummy>().unwrap().name = "y".into();
        assert_eq!(d.name, "y");
    }

    #[test]
    fn object_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<Dummy>();
    }
}
