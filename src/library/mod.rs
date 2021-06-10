//! The standard library.
//!
//! Call [`new`] to obtain a [`Scope`] containing all standard library
//! definitions.

mod align;
mod basic;
mod decorations;
mod font;
mod grid;
mod image;
mod lang;
mod math;
mod pad;
mod page;
mod par;
mod shapes;
mod spacing;
mod stack;

pub use self::image::*;
pub use align::*;
pub use basic::*;
pub use decorations::*;
pub use font::*;
pub use grid::*;
pub use lang::*;
pub use math::*;
pub use pad::*;
pub use page::*;
pub use par::*;
pub use shapes::*;
pub use spacing::*;
pub use stack::*;

use std::fmt::{self, Display, Formatter};

use crate::color::RgbaColor;
use crate::eval::{EvalContext, FuncArgs, Scope, TemplateValue, Value};
use crate::exec::{Exec, FontFamily};
use crate::font::{FontStyle, FontWeight, VerticalFontMetric};
use crate::geom::*;
use crate::syntax::Spanned;

/// Construct a scope containing all standard library definitions.
pub fn new() -> Scope {
    let mut std = Scope::new();

    // Library functions.
    std.def_func("align", align);
    std.def_func("circle", circle);
    std.def_func("ellipse", ellipse);
    std.def_func("font", font);
    std.def_func("grid", grid);
    std.def_func("h", h);
    std.def_func("image", image);
    std.def_func("lang", lang);
    std.def_func("max", max);
    std.def_func("min", min);
    std.def_func("overline", overline);
    std.def_func("pad", pad);
    std.def_func("page", page);
    std.def_func("pagebreak", pagebreak);
    std.def_func("par", par);
    std.def_func("rect", rect);
    std.def_func("repr", repr);
    std.def_func("rgb", rgb);
    std.def_func("square", square);
    std.def_func("stack", stack);
    std.def_func("strike", strike);
    std.def_func("type", type_);
    std.def_func("underline", underline);
    std.def_func("v", v);

    // Colors.
    std.def_const("white", RgbaColor::WHITE);
    std.def_const("black", RgbaColor::BLACK);
    std.def_const("eastern", RgbaColor::new(0x23, 0x9D, 0xAD, 0xFF));
    std.def_const("conifer", RgbaColor::new(0x9f, 0xEB, 0x52, 0xFF));
    std.def_const("forest", RgbaColor::new(0x43, 0xA1, 0x27, 0xFF));

    // Arbitrary constants.
    std.def_any("start", AlignValue::Start);
    std.def_any("center", AlignValue::Center);
    std.def_any("end", AlignValue::End);
    std.def_any("left", AlignValue::Left);
    std.def_any("right", AlignValue::Right);
    std.def_any("top", AlignValue::Top);
    std.def_any("bottom", AlignValue::Bottom);
    std.def_any("ltr", Dir::LTR);
    std.def_any("rtl", Dir::RTL);
    std.def_any("ttb", Dir::TTB);
    std.def_any("btt", Dir::BTT);
    std.def_any("serif", FontFamily::Serif);
    std.def_any("sans-serif", FontFamily::SansSerif);
    std.def_any("monospace", FontFamily::Monospace);
    std.def_any("normal", FontStyle::Normal);
    std.def_any("italic", FontStyle::Italic);
    std.def_any("oblique", FontStyle::Oblique);
    std.def_any("regular", FontWeight::REGULAR);
    std.def_any("bold", FontWeight::BOLD);
    std.def_any("ascender", VerticalFontMetric::Ascender);
    std.def_any("cap-height", VerticalFontMetric::CapHeight);
    std.def_any("x-height", VerticalFontMetric::XHeight);
    std.def_any("baseline", VerticalFontMetric::Baseline);
    std.def_any("descender", VerticalFontMetric::Descender);

    std
}

value! {
    Dir: "direction"
}
