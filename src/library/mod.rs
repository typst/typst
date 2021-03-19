//! The standard library.
//!
//! Call [`new`] to obtain a [`Scope`] containing all standard library
//! definitions.

mod align;
mod base;
mod font;
mod image;
mod pad;
mod page;
mod shapes;
mod spacing;

pub use self::image::*;
pub use align::*;
pub use base::*;
pub use font::*;
pub use pad::*;
pub use page::*;
pub use shapes::*;
pub use spacing::*;

use std::fmt::{self, Display, Formatter};

use fontdock::{FontStyle, FontWeight};

use crate::eval::{Scope, ValueAny, ValueFunc};
use crate::layout::*;
use crate::prelude::*;
use crate::shaping::VerticalFontMetric;

/// Construct a scope containing all standard library definitions.
pub fn new() -> Scope {
    let mut std = Scope::new();
    macro_rules! set {
        (func: $name:expr, $func:expr) => {
            std.def_const($name, ValueFunc::new(Some($name.into()), $func))
        };
        (any: $var:expr, $any:expr) => {
            std.def_const($var, ValueAny::new($any))
        };
    }

    // Functions.
    set!(func: "align", align);
    set!(func: "font", font);
    set!(func: "h", h);
    set!(func: "image", image);
    set!(func: "pad", pad);
    set!(func: "page", page);
    set!(func: "pagebreak", pagebreak);
    set!(func: "rect", rect);
    set!(func: "repr", repr);
    set!(func: "rgb", rgb);
    set!(func: "type", type_);
    set!(func: "v", v);

    // Constants.
    set!(any: "left", AlignValue::Left);
    set!(any: "center", AlignValue::Center);
    set!(any: "right", AlignValue::Right);
    set!(any: "top", AlignValue::Top);
    set!(any: "bottom", AlignValue::Bottom);
    set!(any: "ltr", Dir::LTR);
    set!(any: "rtl", Dir::RTL);
    set!(any: "ttb", Dir::TTB);
    set!(any: "btt", Dir::BTT);
    set!(any: "serif", FontFamily::Serif);
    set!(any: "sans-serif", FontFamily::SansSerif);
    set!(any: "monospace", FontFamily::Monospace);
    set!(any: "normal", FontStyle::Normal);
    set!(any: "italic", FontStyle::Italic);
    set!(any: "oblique", FontStyle::Oblique);
    set!(any: "thin", FontWeight::THIN);
    set!(any: "extralight", FontWeight::EXTRALIGHT);
    set!(any: "light", FontWeight::LIGHT);
    set!(any: "regular", FontWeight::REGULAR);
    set!(any: "medium", FontWeight::MEDIUM);
    set!(any: "semibold", FontWeight::SEMIBOLD);
    set!(any: "bold", FontWeight::BOLD);
    set!(any: "extrabold", FontWeight::EXTRABOLD);
    set!(any: "black", FontWeight::BLACK);
    set!(any: "ascender", VerticalFontMetric::Ascender);
    set!(any: "cap-height", VerticalFontMetric::CapHeight);
    set!(any: "x-height", VerticalFontMetric::XHeight);
    set!(any: "baseline", VerticalFontMetric::Baseline);
    set!(any: "descender", VerticalFontMetric::Descender);

    std
}

typify! {
    Dir: "direction"
}
