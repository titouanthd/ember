//! Property values as text: what the inspector shows and parses back.
//!
//! Text is parsed into the kind of [`Variant`] the property already
//! holds, so the same field always keeps its type:
//!
//! | Kind | Text |
//! |---|---|
//! | `Bool` | `true` / `false` |
//! | `Int` | `42` |
//! | `Float` | `1.5` (an integer is fine) |
//! | `String` | anything |
//! | `Vec2` | `64, 64` |
//! | `Color` | `r, g, b, a` in 0–1 (`a` optional) |

use ember_core::{Color, Variant};
use glam::Vec2;

/// `v` as the inspector shows it.
pub fn format(v: &Variant) -> String {
    match v {
        Variant::Nil => String::new(),
        Variant::Bool(b) => b.to_string(),
        Variant::Int(i) => i.to_string(),
        Variant::Float(f) => num(f.to_string()),
        Variant::String(s) | Variant::ObjectRef(s) => s.clone(),
        Variant::Vec2(p) => format!("{}, {}", num(p.x.to_string()), num(p.y.to_string())),
        Variant::Color(c) => [c.r, c.g, c.b, c.a]
            .iter()
            .map(|x| num(x.to_string()))
            .collect::<Vec<_>>()
            .join(", "),
        other => format!("{other:?}"),
    }
}

/// A float's shortest round-trip text, with `.0` added to whole numbers
/// so it still reads as a float.
fn num(s: String) -> String {
    if s.contains(['.', 'e', 'i', 'N']) {
        s
    } else {
        format!("{s}.0")
    }
}

fn floats(text: &str) -> Option<Vec<f32>> {
    text.split(',')
        .map(|p| p.trim().parse::<f32>().ok().filter(|f| f.is_finite()))
        .collect()
}

/// Parse `text` as the same kind as `current`. `Err` names the expected
/// form, for the inspector to show.
pub fn parse_like(current: &Variant, text: &str) -> Result<Variant, &'static str> {
    let t = text.trim();
    match current {
        Variant::Bool(_) => match t {
            "true" => Ok(Variant::Bool(true)),
            "false" => Ok(Variant::Bool(false)),
            _ => Err("true or false"),
        },
        Variant::Int(_) => t.parse().map(Variant::Int).map_err(|_| "an integer"),
        Variant::Float(_) => t
            .parse::<f64>()
            .ok()
            .filter(|f| f.is_finite())
            .map(Variant::Float)
            .ok_or("a number"),
        Variant::String(_) => Ok(Variant::String(text.to_string())),
        Variant::Vec2(_) => match floats(t).as_deref() {
            Some([x, y]) => Ok(Variant::Vec2(Vec2::new(*x, *y))),
            _ => Err("x, y"),
        },
        Variant::Color(_) => match floats(t).as_deref() {
            Some([r, g, b]) => Ok(Variant::Color(Color::new(*r, *g, *b, 1.0))),
            Some([r, g, b, a]) => Ok(Variant::Color(Color::new(*r, *g, *b, *a))),
            _ => Err("r, g, b[, a]"),
        },
        _ => Err("a value this editor cannot edit yet"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_then_parse_gives_the_value_back() {
        for v in [
            Variant::Bool(true),
            Variant::Int(-3),
            Variant::Float(2.0),
            Variant::Float(0.25),
            Variant::String("player.png".into()),
            Variant::Vec2(Vec2::new(32.0, 48.5)),
            Variant::Color(Color::new(1.0, 0.5, 0.0, 0.75)),
        ] {
            assert_eq!(parse_like(&v, &format(&v)), Ok(v.clone()), "{v:?}");
        }
    }

    #[test]
    fn formats_read_naturally() {
        assert_eq!(format(&Variant::Vec2(Vec2::new(32.0, 32.0))), "32.0, 32.0");
        assert_eq!(format(&Variant::Float(5.0)), "5.0");
        assert_eq!(
            format(&Variant::Color(Color::new(1.0, 0.0, 0.0, 1.0))),
            "1.0, 0.0, 0.0, 1.0"
        );
    }

    #[test]
    fn parsing_keeps_the_kind_and_is_lenient_on_spaces() {
        let size = Variant::Vec2(Vec2::ZERO);
        assert_eq!(
            parse_like(&size, " 64,64 "),
            Ok(Variant::Vec2(Vec2::new(64.0, 64.0)))
        );
        assert_eq!(
            parse_like(&Variant::Float(0.0), "3"),
            Ok(Variant::Float(3.0))
        );
        assert_eq!(
            parse_like(&Variant::Color(Color::default()), "1, 0, 0"),
            Ok(Variant::Color(Color::new(1.0, 0.0, 0.0, 1.0)))
        );
    }

    #[test]
    fn bad_text_names_the_expected_form() {
        assert_eq!(parse_like(&Variant::Vec2(Vec2::ZERO), "64"), Err("x, y"));
        assert_eq!(parse_like(&Variant::Float(0.0), "fast"), Err("a number"));
        assert_eq!(parse_like(&Variant::Float(0.0), "inf"), Err("a number"));
        assert_eq!(parse_like(&Variant::Int(0), "1.5"), Err("an integer"));
        assert_eq!(
            parse_like(&Variant::Bool(false), "yes"),
            Err("true or false")
        );
        assert_eq!(
            parse_like(&Variant::Nil, "x"),
            Err("a value this editor cannot edit yet")
        );
    }
}
