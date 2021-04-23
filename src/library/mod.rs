//! The standard library.
//!
//! Call [`_new`] to obtain a [`Scope`] containing all standard library
//! definitions.

mod align;
mod basic;
mod font;
mod image;
mod lang;
mod markup;
mod pad;
mod page;
mod par;
mod shapes;
mod spacing;

pub use self::image::*;
pub use align::*;
pub use basic::*;
pub use font::*;
pub use lang::*;
pub use markup::*;
pub use pad::*;
pub use page::*;
pub use par::*;
pub use shapes::*;
pub use spacing::*;

use std::fmt::{self, Display, Formatter};

use crate::eval::{
    AnyValue, EvalContext, FuncArgs, FuncValue, Scope, TemplateValue, Value,
};
use crate::exec::{Exec, FontFamily};
use crate::font::{FontStyle, FontWeight, VerticalFontMetric};
use crate::geom::*;
use crate::syntax::{Node, Spanned};

/// Construct a scope containing all standard library definitions.
pub fn _new() -> Scope {
    let mut std = Scope::new();

    macro_rules! func {
        ($name:expr, $func:expr) => {
            std.def_const($name, FuncValue::new(Some($name.into()), $func))
        };
    }

    macro_rules! constant {
        ($var:expr, $any:expr) => {
            std.def_const($var, AnyValue::new($any))
        };
    }

    // Syntax functions.
    func!(Node::LINEBREAK, linebreak);
    func!(Node::PARBREAK, parbreak);
    func!(Node::STRONG, strong);
    func!(Node::EMPH, emph);
    func!(Node::HEADING, heading);
    func!(Node::RAW, raw);

    // Library functions.
    func!("align", align);
    func!("circle", circle);
    func!("ellipse", ellipse);
    func!("font", font);
    func!("h", h);
    func!("image", image);
    func!("lang", lang);
    func!("pad", pad);
    func!("page", page);
    func!("pagebreak", pagebreak);
    func!("par", par);
    func!("rect", rect);
    func!("repr", repr);
    func!("rgb", rgb);
    func!("square", square);
    func!("type", type_);
    func!("v", v);

    // Constants.
    constant!("start", AlignValue::Start);
    constant!("center", AlignValue::Center);
    constant!("end", AlignValue::End);
    constant!("left", AlignValue::Left);
    constant!("right", AlignValue::Right);
    constant!("top", AlignValue::Top);
    constant!("bottom", AlignValue::Bottom);
    constant!("ltr", Dir::LTR);
    constant!("rtl", Dir::RTL);
    constant!("ttb", Dir::TTB);
    constant!("btt", Dir::BTT);
    constant!("serif", FontFamily::Serif);
    constant!("sans-serif", FontFamily::SansSerif);
    constant!("monospace", FontFamily::Monospace);
    constant!("normal", FontStyle::Normal);
    constant!("italic", FontStyle::Italic);
    constant!("oblique", FontStyle::Oblique);
    constant!("thin", FontWeight::THIN);
    constant!("extralight", FontWeight::EXTRALIGHT);
    constant!("light", FontWeight::LIGHT);
    constant!("regular", FontWeight::REGULAR);
    constant!("medium", FontWeight::MEDIUM);
    constant!("semibold", FontWeight::SEMIBOLD);
    constant!("bold", FontWeight::BOLD);
    constant!("extrabold", FontWeight::EXTRABOLD);
    constant!("black", FontWeight::BLACK);
    constant!("ascender", VerticalFontMetric::Ascender);
    constant!("cap-height", VerticalFontMetric::CapHeight);
    constant!("x-height", VerticalFontMetric::XHeight);
    constant!("baseline", VerticalFontMetric::Baseline);
    constant!("descender", VerticalFontMetric::Descender);

    std
}

typify! {
    Dir: "direction"
}
