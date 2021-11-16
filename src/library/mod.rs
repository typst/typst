//! The standard library.
//!
//! Call [`new`] to obtain a [`Scope`] containing all standard library
//! definitions.

mod align;
mod container;
mod deco;
mod document;
mod flow;
mod grid;
mod image;
mod pad;
mod page;
mod par;
mod shape;
mod spacing;
mod stack;
mod text;
mod transform;
mod utility;

/// Helpful imports for creating library functionality.
mod prelude {
    pub use std::rc::Rc;

    pub use crate::diag::{At, TypResult};
    pub use crate::eval::{Args, EvalContext, Template, Value};
    pub use crate::frame::*;
    pub use crate::geom::*;
    pub use crate::layout::*;
    pub use crate::syntax::{Span, Spanned};
    pub use crate::util::{EcoString, OptionExt};
}

pub use self::image::*;
pub use align::*;
pub use container::*;
pub use deco::*;
pub use document::*;
pub use flow::*;
pub use grid::*;
pub use pad::*;
pub use page::*;
pub use par::*;
pub use shape::*;
pub use spacing::*;
pub use stack::*;
pub use text::*;
pub use transform::*;
pub use utility::*;

use crate::eval::{Scope, Value};
use crate::geom::*;
use crate::style::FontFamily;

/// Construct a scope containing all standard library definitions.
pub fn new() -> Scope {
    let mut std = Scope::new();

    // Text.
    std.def_func("font", font);
    std.def_func("par", par);
    std.def_func("strike", strike);
    std.def_func("underline", underline);
    std.def_func("overline", overline);
    std.def_func("link", link);

    // Layout.
    std.def_func("page", page);
    std.def_func("pagebreak", pagebreak);
    std.def_func("h", h);
    std.def_func("v", v);
    std.def_func("align", align);
    std.def_func("box", box_);
    std.def_func("block", block);
    std.def_func("flow", flow);
    std.def_func("pad", pad);
    std.def_func("move", move_);
    std.def_func("stack", stack);
    std.def_func("grid", grid);

    // Elements.
    std.def_func("image", image);
    std.def_func("rect", rect);
    std.def_func("square", square);
    std.def_func("ellipse", ellipse);
    std.def_func("circle", circle);

    // Utility.
    std.def_func("assert", assert);
    std.def_func("type", type_);
    std.def_func("repr", repr);
    std.def_func("join", join);
    std.def_func("int", int);
    std.def_func("float", float);
    std.def_func("str", str);
    std.def_func("abs", abs);
    std.def_func("min", min);
    std.def_func("max", max);
    std.def_func("range", range);
    std.def_func("rgb", rgb);
    std.def_func("lower", lower);
    std.def_func("upper", upper);
    std.def_func("len", len);
    std.def_func("sorted", sorted);

    // Colors.
    std.def_const("white", RgbaColor::WHITE);
    std.def_const("black", RgbaColor::BLACK);
    std.def_const("eastern", RgbaColor::new(0x23, 0x9D, 0xAD, 0xFF));
    std.def_const("conifer", RgbaColor::new(0x9f, 0xEB, 0x52, 0xFF));
    std.def_const("forest", RgbaColor::new(0x43, 0xA1, 0x27, 0xFF));

    // Arbitrary constants.
    std.def_const("ltr", Dir::LTR);
    std.def_const("rtl", Dir::RTL);
    std.def_const("ttb", Dir::TTB);
    std.def_const("btt", Dir::BTT);
    std.def_const("start", Align::Start);
    std.def_const("center", Align::Center);
    std.def_const("end", Align::End);
    std.def_const("left", Align::Left);
    std.def_const("right", Align::Right);
    std.def_const("top", Align::Top);
    std.def_const("bottom", Align::Bottom);
    std.def_const("serif", FontFamily::Serif);
    std.def_const("sans-serif", FontFamily::SansSerif);
    std.def_const("monospace", FontFamily::Monospace);

    std
}

dynamic! {
    Dir: "direction",
}

dynamic! {
    Align: "alignment",
}

dynamic! {
    FontFamily: "font family",
    Value::Str(string) => Self::Named(string.to_lowercase()),
}
