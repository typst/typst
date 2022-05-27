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
    std.def_fn("lower", text::lower);
    std.def_fn("upper", text::upper);
    std.def_fn("smallcaps", text::smallcaps);

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
    std.def_fn("locate", layout::locate);
    std.def_fn("group", layout::group);

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

    // Utility.
    std.def_fn("type", utility::type_);
    std.def_fn("assert", utility::assert);
    std.def_fn("eval", utility::eval);
    std.def_fn("int", utility::int);
    std.def_fn("float", utility::float);
    std.def_fn("abs", utility::abs);
    std.def_fn("min", utility::min);
    std.def_fn("max", utility::max);
    std.def_fn("even", utility::even);
    std.def_fn("odd", utility::odd);
    std.def_fn("mod", utility::mod_);
    std.def_fn("range", utility::range);
    std.def_fn("luma", utility::luma);
    std.def_fn("rgb", utility::rgb);
    std.def_fn("cmyk", utility::cmyk);
    std.def_fn("repr", utility::repr);
    std.def_fn("str", utility::str);
    std.def_fn("regex", utility::regex);
    std.def_fn("letter", utility::letter);
    std.def_fn("roman", utility::roman);
    std.def_fn("symbol", utility::symbol);
    std.def_fn("lorem", utility::lorem);

    // Predefined colors.
    std.define("black", Color::BLACK);
    std.define("gray", Color::GRAY);
    std.define("silver", Color::SILVER);
    std.define("white", Color::WHITE);
    std.define("navy", Color::NAVY);
    std.define("blue", Color::BLUE);
    std.define("aqua", Color::AQUA);
    std.define("teal", Color::TEAL);
    std.define("eastern", Color::EASTERN);
    std.define("purple", Color::PURPLE);
    std.define("fuchsia", Color::FUCHSIA);
    std.define("maroon", Color::MAROON);
    std.define("red", Color::RED);
    std.define("orange", Color::ORANGE);
    std.define("yellow", Color::YELLOW);
    std.define("olive", Color::OLIVE);
    std.define("green", Color::GREEN);
    std.define("lime", Color::LIME);

    // Other constants.
    std.define("ltr", Dir::LTR);
    std.define("rtl", Dir::RTL);
    std.define("ttb", Dir::TTB);
    std.define("btt", Dir::BTT);
    std.define("start", RawAlign::Start);
    std.define("end", RawAlign::End);
    std.define("left", RawAlign::Specific(Align::Left));
    std.define("center", RawAlign::Specific(Align::Center));
    std.define("right", RawAlign::Specific(Align::Right));
    std.define("top", RawAlign::Specific(Align::Top));
    std.define("horizon", RawAlign::Specific(Align::Horizon));
    std.define("bottom", RawAlign::Specific(Align::Bottom));

    std
}
