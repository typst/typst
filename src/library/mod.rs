//! The standard library.
//!
//! Call [`new`] to obtain a [`Scope`] containing all standard library
//! definitions.

mod elements;
mod layout;
mod text;
mod utility;

pub use elements::*;
pub use layout::*;
pub use text::*;
pub use utility::*;

use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::color::{Color, RgbaColor};
use crate::eco::EcoString;
use crate::eval::{EvalContext, FuncArgs, Scope, Template, Type, Value};
use crate::exec::{Exec, FontFamily};
use crate::font::{FontStyle, FontWeight, VerticalFontMetric};
use crate::geom::*;
use crate::syntax::Spanned;

/// Construct a scope containing all standard library definitions.
pub fn new() -> Scope {
    let mut std = Scope::new();

    // Text.
    std.def_func("font", font);
    std.def_func("par", par);
    std.def_func("lang", lang);
    std.def_func("strike", strike);
    std.def_func("underline", underline);
    std.def_func("overline", overline);

    // Layout.
    std.def_func("page", page);
    std.def_func("pagebreak", pagebreak);
    std.def_func("h", h);
    std.def_func("v", v);
    std.def_func("align", align);
    std.def_func("box", boxed);
    std.def_func("block", block);
    std.def_func("pad", pad);
    std.def_func("stack", stack);
    std.def_func("grid", grid);

    // Elements.
    std.def_func("image", image);
    std.def_func("rect", rect);
    std.def_func("square", square);
    std.def_func("ellipse", ellipse);
    std.def_func("circle", circle);

    // Utility.
    std.def_func("type", type_);
    std.def_func("repr", repr);
    std.def_func("len", len);
    std.def_func("rgb", rgb);
    std.def_func("min", min);
    std.def_func("max", max);

    // Colors.
    std.def_const("white", RgbaColor::WHITE);
    std.def_const("black", RgbaColor::BLACK);
    std.def_const("eastern", RgbaColor::new(0x23, 0x9D, 0xAD, 0xFF));
    std.def_const("conifer", RgbaColor::new(0x9f, 0xEB, 0x52, 0xFF));
    std.def_const("forest", RgbaColor::new(0x43, 0xA1, 0x27, 0xFF));

    // Arbitrary constants.
    std.def_const("start", AlignValue::Start);
    std.def_const("center", AlignValue::Center);
    std.def_const("end", AlignValue::End);
    std.def_const("left", AlignValue::Left);
    std.def_const("right", AlignValue::Right);
    std.def_const("top", AlignValue::Top);
    std.def_const("bottom", AlignValue::Bottom);
    std.def_const("ltr", Dir::LTR);
    std.def_const("rtl", Dir::RTL);
    std.def_const("ttb", Dir::TTB);
    std.def_const("btt", Dir::BTT);
    std.def_const("serif", FontFamily::Serif);
    std.def_const("sans-serif", FontFamily::SansSerif);
    std.def_const("monospace", FontFamily::Monospace);
    std.def_const("normal", FontStyle::Normal);
    std.def_const("italic", FontStyle::Italic);
    std.def_const("oblique", FontStyle::Oblique);
    std.def_const("regular", FontWeight::REGULAR);
    std.def_const("bold", FontWeight::BOLD);
    std.def_const("ascender", VerticalFontMetric::Ascender);
    std.def_const("cap-height", VerticalFontMetric::CapHeight);
    std.def_const("x-height", VerticalFontMetric::XHeight);
    std.def_const("baseline", VerticalFontMetric::Baseline);
    std.def_const("descender", VerticalFontMetric::Descender);

    std
}

dynamic! {
    Dir: "direction",
}
