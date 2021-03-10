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

use fontdock::{FontStretch, FontStyle, FontWeight};

use crate::eval::{Scope, ValueAny, ValueFunc};
use crate::exec::Softness;
use crate::layout::*;
use crate::prelude::*;

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
    set!(func: "box", box_);
    set!(func: "font", font);
    set!(func: "h", h);
    set!(func: "image", image);
    set!(func: "pad", pad);
    set!(func: "page", page);
    set!(func: "pagebreak", pagebreak);
    set!(func: "repr", repr);
    set!(func: "rgb", rgb);
    set!(func: "type", type_);
    set!(func: "v", v);

    // Constants.
    set!(any: "left", Alignment::Left);
    set!(any: "center", Alignment::Center);
    set!(any: "right", Alignment::Right);
    set!(any: "top", Alignment::Top);
    set!(any: "bottom", Alignment::Bottom);
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
    set!(any: "ultra-condensed", FontStretch::UltraCondensed);
    set!(any: "extra-condensed", FontStretch::ExtraCondensed);
    set!(any: "condensed", FontStretch::Condensed);
    set!(any: "semi-condensed", FontStretch::SemiCondensed);
    set!(any: "normal", FontStretch::Normal);
    set!(any: "semi-expanded", FontStretch::SemiExpanded);
    set!(any: "expanded", FontStretch::Expanded);
    set!(any: "extra-expanded", FontStretch::ExtraExpanded);
    set!(any: "ultra-expanded", FontStretch::UltraExpanded);

    std
}

typify! {
    Dir: "direction"
}
