//! The standard library.

mod insert;
mod layout;
mod style;

pub use insert::*;
pub use layout::*;
pub use style::*;

use fontdock::{FontStretch, FontStyle, FontWeight};

use crate::eval::Scope;
use crate::geom::Dir;

/// The scope containing the standard library.
pub fn _std() -> Scope {
    let mut std = Scope::new();

    // Functions.
    std.set("align", align);
    std.set("box", boxed);
    std.set("font", font);
    std.set("h", h);
    std.set("image", image);
    std.set("page", page);
    std.set("pagebreak", pagebreak);
    std.set("rgb", rgb);
    std.set("v", v);

    // Constants.
    std.set("left", Alignment::Left);
    std.set("center", Alignment::Center);
    std.set("right", Alignment::Right);
    std.set("top", Alignment::Top);
    std.set("bottom", Alignment::Bottom);
    std.set("ltr", Dir::LTR);
    std.set("rtl", Dir::RTL);
    std.set("ttb", Dir::TTB);
    std.set("btt", Dir::BTT);
    std.set("serif", FontFamily::Serif);
    std.set("sans-serif", FontFamily::SansSerif);
    std.set("monospace", FontFamily::Monospace);
    std.set("normal", FontStyle::Normal);
    std.set("italic", FontStyle::Italic);
    std.set("oblique", FontStyle::Oblique);
    std.set("thin", FontWeight::THIN);
    std.set("extralight", FontWeight::EXTRALIGHT);
    std.set("light", FontWeight::LIGHT);
    std.set("regular", FontWeight::REGULAR);
    std.set("medium", FontWeight::MEDIUM);
    std.set("semibold", FontWeight::SEMIBOLD);
    std.set("bold", FontWeight::BOLD);
    std.set("extrabold", FontWeight::EXTRABOLD);
    std.set("black", FontWeight::BLACK);
    std.set("ultra-condensed", FontStretch::UltraCondensed);
    std.set("extra-condensed", FontStretch::ExtraCondensed);
    std.set("condensed", FontStretch::Condensed);
    std.set("semi-condensed", FontStretch::SemiCondensed);
    std.set("normal", FontStretch::Normal);
    std.set("semi-expanded", FontStretch::SemiExpanded);
    std.set("expanded", FontStretch::Expanded);
    std.set("extra-expanded", FontStretch::ExtraExpanded);
    std.set("ultra-expanded", FontStretch::UltraExpanded);

    std
}
