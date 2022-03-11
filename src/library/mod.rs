//! The standard library.
//!
//! Call [`new`] to obtain a [`Scope`] containing all standard library
//! definitions.

pub mod graphics;
pub mod layout;
pub mod math;
pub mod prelude;
pub mod structure;
pub mod text;
pub mod utility;

use prelude::*;

/// Construct a scope containing all standard library definitions.
pub fn new() -> Scope {
    let mut std = Scope::new();

    // Text.
    std.def_class::<text::TextNode>("text");
    std.def_class::<text::ParNode>("par");
    std.def_class::<text::LinebreakNode>("linebreak");
    std.def_class::<text::ParbreakNode>("parbreak");
    std.def_class::<text::StrongNode>("strong");
    std.def_class::<text::EmphNode>("emph");
    std.def_class::<text::RawNode>("raw");
    std.def_class::<text::UnderlineNode>("underline");
    std.def_class::<text::StrikethroughNode>("strike");
    std.def_class::<text::OverlineNode>("overline");
    std.def_class::<text::LinkNode>("link");

    // Structure.
    std.def_class::<structure::HeadingNode>("heading");
    std.def_class::<structure::ListNode>("list");
    std.def_class::<structure::EnumNode>("enum");
    std.def_class::<structure::TableNode>("table");

    // Layout.
    std.def_class::<layout::PageNode>("page");
    std.def_class::<layout::PagebreakNode>("pagebreak");
    std.def_class::<layout::HNode>("h");
    std.def_class::<layout::VNode>("v");
    std.def_class::<layout::BoxNode>("box");
    std.def_class::<layout::BlockNode>("block");
    std.def_class::<layout::AlignNode>("align");
    std.def_class::<layout::PadNode>("pad");
    std.def_class::<layout::StackNode>("stack");
    std.def_class::<layout::GridNode>("grid");
    std.def_class::<layout::ColumnsNode>("columns");
    std.def_class::<layout::ColbreakNode>("colbreak");
    std.def_class::<layout::PlaceNode>("place");

    // Graphics.
    std.def_class::<graphics::ImageNode>("image");
    std.def_class::<graphics::RectNode>("rect");
    std.def_class::<graphics::SquareNode>("square");
    std.def_class::<graphics::EllipseNode>("ellipse");
    std.def_class::<graphics::CircleNode>("circle");
    std.def_class::<graphics::MoveNode>("move");
    std.def_class::<graphics::ScaleNode>("scale");
    std.def_class::<graphics::RotateNode>("rotate");
    std.def_class::<graphics::HideNode>("hide");

    // Math.
    std.def_class::<math::MathNode>("math");

    // Utility functions.
    std.def_func("assert", utility::assert);
    std.def_func("type", utility::type_);
    std.def_func("repr", utility::repr);
    std.def_func("join", utility::join);
    std.def_func("int", utility::int);
    std.def_func("float", utility::float);
    std.def_func("str", utility::str);
    std.def_func("abs", utility::abs);
    std.def_func("min", utility::min);
    std.def_func("max", utility::max);
    std.def_func("even", utility::even);
    std.def_func("odd", utility::odd);
    std.def_func("mod", utility::modulo);
    std.def_func("range", utility::range);
    std.def_func("rgb", utility::rgb);
    std.def_func("cmyk", utility::cmyk);
    std.def_func("lower", utility::lower);
    std.def_func("upper", utility::upper);
    std.def_func("letter", utility::letter);
    std.def_func("roman", utility::roman);
    std.def_func("symbol", utility::symbol);
    std.def_func("len", utility::len);
    std.def_func("sorted", utility::sorted);

    // Predefined colors.
    std.def_const("black", Color::BLACK);
    std.def_const("gray", Color::GRAY);
    std.def_const("silver", Color::SILVER);
    std.def_const("white", Color::WHITE);
    std.def_const("navy", Color::NAVY);
    std.def_const("blue", Color::BLUE);
    std.def_const("aqua", Color::AQUA);
    std.def_const("teal", Color::TEAL);
    std.def_const("eastern", Color::EASTERN);
    std.def_const("purple", Color::PURPLE);
    std.def_const("fuchsia", Color::FUCHSIA);
    std.def_const("maroon", Color::MAROON);
    std.def_const("red", Color::RED);
    std.def_const("orange", Color::ORANGE);
    std.def_const("yellow", Color::YELLOW);
    std.def_const("olive", Color::OLIVE);
    std.def_const("green", Color::GREEN);
    std.def_const("lime", Color::LIME);

    // Other constants.
    std.def_const("ltr", Dir::LTR);
    std.def_const("rtl", Dir::RTL);
    std.def_const("ttb", Dir::TTB);
    std.def_const("btt", Dir::BTT);
    std.def_const("left", Align::Left);
    std.def_const("center", Align::Center);
    std.def_const("right", Align::Right);
    std.def_const("top", Align::Top);
    std.def_const("horizon", Align::Horizon);
    std.def_const("bottom", Align::Bottom);
    std.def_const("serif", text::FontFamily::Serif);
    std.def_const("sans-serif", text::FontFamily::SansSerif);
    std.def_const("monospace", text::FontFamily::Monospace);

    std
}

dynamic! {
    Dir: "direction",
}

castable! {
    usize,
    Expected: "non-negative integer",
    Value::Int(int) => int.try_into().map_err(|_| {
        if int < 0 {
            "must be at least zero"
        } else {
            "number too large"
        }
    })?,
}

castable! {
    NonZeroUsize,
    Expected: "positive integer",
    Value::Int(int) => Value::Int(int)
        .cast::<usize>()?
        .try_into()
        .map_err(|_| "must be positive")?,
}

castable! {
    Paint,
    Expected: "color",
    Value::Color(color) => Paint::Solid(color),
}

castable! {
    String,
    Expected: "string",
    Value::Str(string) => string.into(),
}

castable! {
    LayoutNode,
    Expected: "template",
    Value::Template(template) => template.pack(),
}
