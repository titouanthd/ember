//! `Variant`: dynamic values at the `Object` / `.escene` boundary.
//! RON syntax: `Float(1.0)`, `Vec2(1.0, 2.0)`, `Color(r, g, b, a)`.

use crate::{Color, error::CoreError};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "Wire", into = "Wire")]
#[non_exhaustive]
pub enum Variant {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2(Vec2),
    Color(Color),
    Array(Vec<Variant>),
    Map(BTreeMap<String, Variant>),
    /// `uid://...`
    ObjectRef(String),
}

// The on-disk shape; keeps `Vec2` and `Color` as flat tuples.
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Wire {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2(f32, f32),
    Color(f32, f32, f32, f32),
    Array(Vec<Variant>),
    Map(BTreeMap<String, Variant>),
    ObjectRef(String),
}

impl From<Wire> for Variant {
    fn from(w: Wire) -> Self {
        match w {
            Wire::Nil => Self::Nil,
            Wire::Bool(v) => Self::Bool(v),
            Wire::Int(v) => Self::Int(v),
            Wire::Float(v) => Self::Float(v),
            Wire::String(v) => Self::String(v),
            Wire::Vec2(x, y) => Self::Vec2(Vec2::new(x, y)),
            Wire::Color(r, g, b, a) => Self::Color(Color::new(r, g, b, a)),
            Wire::Array(v) => Self::Array(v),
            Wire::Map(v) => Self::Map(v),
            Wire::ObjectRef(v) => Self::ObjectRef(v),
        }
    }
}

impl From<Variant> for Wire {
    fn from(v: Variant) -> Self {
        match v {
            Variant::Nil => Self::Nil,
            Variant::Bool(v) => Self::Bool(v),
            Variant::Int(v) => Self::Int(v),
            Variant::Float(v) => Self::Float(v),
            Variant::String(v) => Self::String(v),
            Variant::Vec2(v) => Self::Vec2(v.x, v.y),
            Variant::Color(c) => Self::Color(c.r, c.g, c.b, c.a),
            Variant::Array(v) => Self::Array(v),
            Variant::Map(v) => Self::Map(v),
            Variant::ObjectRef(v) => Self::ObjectRef(v),
        }
    }
}

impl Variant {
    pub fn nil() -> Self {
        Self::Nil
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// `Float` as is; `Int` widened, since `1` and `1.0` are both
    /// natural in a hand-written scene.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            Self::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) | Self::ObjectRef(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_vec2(&self) -> Option<Vec2> {
        match self {
            Self::Vec2(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_color(&self) -> Option<Color> {
        match self {
            Self::Color(v) => Some(*v),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Nil => "Nil",
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::String(_) => "String",
            Self::Vec2(_) => "Vec2",
            Self::Color(_) => "Color",
            Self::Array(_) => "Array",
            Self::Map(_) => "Map",
            Self::ObjectRef(_) => "ObjectRef",
        }
    }

    pub fn from_ron(s: &str) -> Result<Self, CoreError> {
        ron::from_str(s).map_err(|e| CoreError::Parse(e.to_string()))
    }

    pub fn to_ron(&self) -> String {
        ron::to_string(self).expect("a Variant always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_syntax_parses() {
        assert_eq!(Variant::from_ron("Float(3.5)"), Ok(Variant::Float(3.5)));
        assert_eq!(
            Variant::from_ron("Vec2(1.0, 2.0)"),
            Ok(Variant::Vec2(Vec2::new(1.0, 2.0)))
        );
        assert_eq!(
            Variant::from_ron("Color(1.0, 0.0, 0.0, 1.0)"),
            Ok(Variant::Color(Color::new(1.0, 0.0, 0.0, 1.0)))
        );
        assert_eq!(Variant::from_ron("Nil"), Ok(Variant::Nil));
    }

    #[test]
    fn every_kind_round_trips() {
        let all = Variant::Array(vec![
            Variant::Nil,
            Variant::Bool(true),
            Variant::Int(-4),
            Variant::Float(0.25),
            Variant::String("hi".into()),
            Variant::Vec2(Vec2::new(1.5, -2.0)),
            Variant::Color(Color::new(0.1, 0.2, 0.3, 1.0)),
            Variant::Map(BTreeMap::from([("k".to_string(), Variant::Int(1))])),
            Variant::ObjectRef("uid://a".into()),
        ]);
        assert_eq!(Variant::from_ron(&all.to_ron()), Ok(all));
    }

    #[test]
    fn accessors_and_errors() {
        assert_eq!(Variant::Int(2).as_f64(), Some(2.0));
        assert_eq!(Variant::Float(2.0).as_i64(), None);
        assert_eq!(Variant::Bool(true).type_name(), "Bool");
        assert!(Variant::from_ron("Bogus(1)").is_err());
    }
}
