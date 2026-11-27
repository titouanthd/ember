//! `#[export(enum = "…")]` (ADR-049): the field is exported, its names
//! come back as `PropertyHint::Enum`, the value travels as a string.

use ember_core::{Variant, VariantValue};
use ember_macros::Node;
use ember_scene::{NodeName, PropertyHint};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Level {
    #[default]
    Easy,
    Hard,
}

impl VariantValue for Level {
    fn to_variant(&self) -> Variant {
        Variant::String(format!("{self:?}"))
    }
    fn from_variant(v: &Variant) -> Option<Self> {
        match v {
            Variant::String(s) if s == "Easy" => Some(Level::Easy),
            Variant::String(s) if s == "Hard" => Some(Level::Hard),
            _ => None,
        }
    }
}

#[derive(Node, Default)]
#[class_name = "Settings"]
struct Settings {
    name: String,
    #[export(enum = "Easy, Hard")]
    level: Level,
    #[export(enum = "Left,Right")]
    side: String,
    #[export]
    speed: f32,
}

impl ember_scene::Node for Settings {}

#[test]
fn enum_fields_are_exported_with_their_names() {
    let s = Settings::default();
    assert_eq!(s.property_list(), &["level", "side", "speed"]);
    assert_eq!(
        s.property_hint("level"),
        PropertyHint::Enum(&["Easy", "Hard"])
    );
    assert_eq!(
        s.property_hint("side"),
        PropertyHint::Enum(&["Left", "Right"])
    );
    assert_eq!(s.property_hint("speed"), PropertyHint::None);
    assert_eq!(s.property_hint("nope"), PropertyHint::None);
}

#[test]
fn enum_values_travel_as_strings() {
    let mut s = Settings::default();
    assert_eq!(
        s.get_property("level"),
        Some(Variant::String("Easy".into()))
    );
    assert!(s.set_property("level", &Variant::String("Hard".into())));
    assert_eq!(s.level, Level::Hard);
    assert!(!s.set_property("level", &Variant::String("Medium".into())));
    assert!(s.set_property("side", &Variant::String("Right".into())));
    assert_eq!(s.side, "Right");
}
