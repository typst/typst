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
    std.def_node::<text::TextNode>("text");
    std.def_node::<text::ParNode>("par");
    std.def_node::<text::LinebreakNode>("linebreak");
    std.def_node::<text::ParbreakNode>("parbreak");
    std.def_node::<text::StrongNode>("strong");
    std.def_node::<text::EmphNode>("emph");
    std.def_node::<text::RawNode>("raw");
    std.def_node::<text::UnderlineNode>("underline");
    std.def_node::<text::StrikethroughNode>("strike");
    std.def_node::<text::OverlineNode>("overline");
    std.def_node::<text::LinkNode>("link");
    std.def_node::<text::RepeatNode>("repeat");

    // Structure.
    std.def_node::<structure::HeadingNode>("heading");
    std.def_node::<structure::ListNode>("list");
    std.def_node::<structure::EnumNode>("enum");
    std.def_node::<structure::TableNode>("table");

    // Layout.
    std.def_node::<layout::PageNode>("page");
    std.def_node::<layout::PagebreakNode>("pagebreak");
    std.def_node::<layout::HNode>("h");
    std.def_node::<layout::VNode>("v");
    std.def_node::<layout::BoxNode>("box");
    std.def_node::<layout::BlockNode>("block");
    std.def_node::<layout::AlignNode>("align");
    std.def_node::<layout::PadNode>("pad");
    std.def_node::<layout::StackNode>("stack");
    std.def_node::<layout::GridNode>("grid");
    std.def_node::<layout::ColumnsNode>("columns");
    std.def_node::<layout::ColbreakNode>("colbreak");
    std.def_node::<layout::PlaceNode>("place");

    // Graphics.
    std.def_node::<graphics::ImageNode>("image");
    std.def_node::<graphics::LineNode>("line");
    std.def_node::<graphics::RectNode>("rect");
    std.def_node::<graphics::SquareNode>("square");
    std.def_node::<graphics::EllipseNode>("ellipse");
    std.def_node::<graphics::CircleNode>("circle");
    std.def_node::<graphics::MoveNode>("move");
    std.def_node::<graphics::ScaleNode>("scale");
    std.def_node::<graphics::RotateNode>("rotate");
    std.def_node::<graphics::HideNode>("hide");

    // Math.
    std.def_node::<math::MathNode>("math");

    // Utility functions.
    std.def_fn("type", utility::type_);
    std.def_fn("assert", utility::assert);
    std.def_fn("int", utility::int);
    std.def_fn("float", utility::float);
    std.def_fn("abs", utility::abs);
    std.def_fn("min", utility::min);
    std.def_fn("max", utility::max);
    std.def_fn("even", utility::even);
    std.def_fn("odd", utility::odd);
    std.def_fn("mod", utility::mod_);
    std.def_fn("range", utility::range);
    std.def_fn("rgb", utility::rgb);
    std.def_fn("cmyk", utility::cmyk);
    std.def_fn("repr", utility::repr);
    std.def_fn("str", utility::str);
    std.def_fn("lower", utility::lower);
    std.def_fn("upper", utility::upper);
    std.def_fn("letter", utility::letter);
    std.def_fn("roman", utility::roman);
    std.def_fn("symbol", utility::symbol);
    std.def_fn("lipsum", utility::lipsum);

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
    std.def_const("start", RawAlign::Start);
    std.def_const("end", RawAlign::End);
    std.def_const("left", RawAlign::Specific(Align::Left));
    std.def_const("center", RawAlign::Specific(Align::Center));
    std.def_const("right", RawAlign::Specific(Align::Right));
    std.def_const("top", RawAlign::Specific(Align::Top));
    std.def_const("horizon", RawAlign::Specific(Align::Horizon));
    std.def_const("bottom", RawAlign::Specific(Align::Bottom));

    std
}
