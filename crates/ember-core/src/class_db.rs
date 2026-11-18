//! `ClassDB`: class name -> constructor. Created by the game and passed
//! by reference; there is no global registry (principle I3, ADR-024).

use crate::Object;
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};

struct Entry {
    construct: Box<dyn Fn() -> Box<dyn Object> + Send + Sync>,
    // Layer-specific constructors (`ember-scene` stores a node
    // constructor here), keyed by the extension's type.
    extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

#[derive(Default)]
pub struct ClassDB {
    classes: BTreeMap<&'static str, Entry>,
}

impl ClassDB {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `T` under `T::class_name()`. Idempotent: the first
    /// registration of a name wins.
    pub fn register<T: Object + Default>(&mut self) {
        self.register_with::<T>(T::default);
    }

    /// Register `T` with a custom constructor.
    pub fn register_with<T: Object>(&mut self, ctor: fn() -> T) {
        self.classes
            .entry(T::class_name())
            .or_insert_with(|| Entry {
                construct: Box::new(move || Box::new(ctor())),
                extensions: HashMap::new(),
            });
    }

    pub fn is_registered(&self, class_name: &str) -> bool {
        self.classes.contains_key(class_name)
    }

    /// A fresh instance of `class_name`, or `None` if unregistered.
    pub fn instantiate(&self, class_name: &str) -> Option<Box<dyn Object>> {
        self.classes.get(class_name).map(|e| (e.construct)())
    }

    /// Registered class names, sorted.
    pub fn names(&self) -> Vec<&'static str> {
        self.classes.keys().copied().collect()
    }

    /// Attach a layer-specific value `ext` to an already registered
    /// class. A second value of the same type is ignored. Returns false
    /// if the class is unregistered.
    pub fn set_extension<E: Any + Send + Sync>(&mut self, class_name: &str, ext: E) -> bool {
        let Some(entry) = self.classes.get_mut(class_name) else {
            return false;
        };
        entry
            .extensions
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(ext));
        true
    }

    pub fn extension<E: Any + Send + Sync>(&self, class_name: &str) -> Option<&E> {
        self.classes
            .get(class_name)?
            .extensions
            .get(&TypeId::of::<E>())?
            .downcast_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Orc {
        hp: u32,
    }
    #[derive(Default)]
    struct Elf;

    macro_rules! object {
        ($t:ty, $n:literal) => {
            impl Object for $t {
                fn class_name() -> &'static str {
                    $n
                }
                fn as_any(&self) -> &dyn Any {
                    self
                }
                fn as_any_mut(&mut self) -> &mut dyn Any {
                    self
                }
            }
        };
    }
    object!(Orc, "Orc");
    object!(Elf, "Elf");

    #[test]
    fn instantiate_builds_registered_classes_only() {
        let mut db = ClassDB::new();
        db.register::<Orc>();
        let obj = db.instantiate("Orc").expect("registered");
        assert!(obj.as_any().is::<Orc>());
        assert!(db.instantiate("Elf").is_none());
    }

    #[test]
    fn custom_constructor_is_used() {
        let mut db = ClassDB::new();
        db.register_with::<Orc>(|| Orc { hp: 9 });
        let obj = db.instantiate("Orc").unwrap();
        assert_eq!(obj.as_any().downcast_ref::<Orc>().unwrap().hp, 9);
    }

    #[test]
    fn registering_twice_keeps_the_first() {
        let mut db = ClassDB::new();
        db.register_with::<Orc>(|| Orc { hp: 1 });
        db.register_with::<Orc>(|| Orc { hp: 2 });
        let obj = db.instantiate("Orc").unwrap();
        assert_eq!(obj.as_any().downcast_ref::<Orc>().unwrap().hp, 1);
        assert_eq!(db.names(), ["Orc"]);
    }

    #[test]
    fn names_are_sorted_and_extensions_are_per_class() {
        let mut db = ClassDB::new();
        db.register::<Orc>();
        db.register::<Elf>();
        assert_eq!(db.names(), ["Elf", "Orc"]);
        assert!(db.set_extension("Orc", 5u32));
        assert!(!db.set_extension("Nope", 5u32));
        assert_eq!(db.extension::<u32>("Orc"), Some(&5));
        assert_eq!(db.extension::<u32>("Elf"), None);
    }
}
