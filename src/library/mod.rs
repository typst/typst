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
use crate::eval::{EvalContext, FuncArgs, Scope, TemplateValue, Value};
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

castable! {
    Dir: "direction"
}
