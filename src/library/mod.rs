//! The standard library.
//!
//! Call [`new`] to obtain a [`Scope`] containing all standard library
//! definitions.

pub mod align;
pub mod columns;
pub mod container;
pub mod deco;
pub mod flow;
pub mod grid;
pub mod heading;
pub mod hide;
pub mod image;
pub mod link;
pub mod list;
pub mod math;
pub mod numbering;
pub mod pad;
pub mod page;
pub mod par;
pub mod place;
pub mod raw;
pub mod shape;
pub mod spacing;
pub mod stack;
pub mod table;
pub mod text;
pub mod transform;

pub mod utility;
pub use self::image::*;
pub use align::*;
pub use columns::*;
pub use container::*;
pub use deco::*;
pub use flow::*;
pub use grid::*;
pub use heading::*;
pub use hide::*;
pub use link::*;
pub use list::*;
pub use math::*;
pub use numbering::*;
pub use pad::*;
pub use page::*;
pub use par::*;
pub use place::*;
pub use raw::*;
pub use shape::*;
pub use spacing::*;
pub use stack::*;
pub use table::*;
pub use text::*;
pub use transform::*;
pub use utility::*;

/// Helpful imports for creating library functionality.
pub mod prelude {
    pub use std::fmt::{self, Debug, Formatter};
    pub use std::hash::Hash;
    pub use std::num::NonZeroUsize;
    pub use std::sync::Arc;

    pub use typst_macros::class;

    pub use crate::diag::{with_alternative, At, StrResult, TypResult};
    pub use crate::eval::{
        Arg, Args, Cast, Construct, Func, Merge, Property, Scope, Set, Show, ShowNode,
        Smart, StyleChain, StyleMap, StyleVec, Template, Value,
    };
    pub use crate::frame::*;
    pub use crate::geom::*;
    pub use crate::layout::{
        Constrain, Constrained, Constraints, Layout, LayoutNode, Regions,
    };
    pub use crate::syntax::{Span, Spanned};
    pub use crate::util::{EcoString, OptionExt};
    pub use crate::Vm;
}

use prelude::*;

/// Construct a scope containing all standard library definitions.
pub fn new() -> Scope {
    let mut std = Scope::new();

    // Structure and semantics.
    std.def_class::<PageNode>("page");
    std.def_class::<PagebreakNode>("pagebreak");
    std.def_class::<ParNode>("par");
    std.def_class::<ParbreakNode>("parbreak");
    std.def_class::<LinebreakNode>("linebreak");
    std.def_class::<TextNode>("text");
    std.def_class::<StrongNode>("strong");
    std.def_class::<EmphNode>("emph");
    std.def_class::<RawNode>("raw");
    std.def_class::<MathNode>("math");
    std.def_class::<DecoNode<UNDERLINE>>("underline");
    std.def_class::<DecoNode<STRIKETHROUGH>>("strike");
    std.def_class::<DecoNode<OVERLINE>>("overline");
    std.def_class::<LinkNode>("link");
    std.def_class::<HeadingNode>("heading");
    std.def_class::<ListNode<UNORDERED>>("list");
    std.def_class::<ListNode<ORDERED>>("enum");
    std.def_class::<TableNode>("table");
    std.def_class::<ImageNode>("image");
    std.def_class::<ShapeNode<RECT>>("rect");
    std.def_class::<ShapeNode<SQUARE>>("square");
    std.def_class::<ShapeNode<ELLIPSE>>("ellipse");
    std.def_class::<ShapeNode<CIRCLE>>("circle");

    // Layout.
    std.def_class::<HNode>("h");
    std.def_class::<VNode>("v");
    std.def_class::<BoxNode>("box");
    std.def_class::<BlockNode>("block");
    std.def_class::<AlignNode>("align");
    std.def_class::<PadNode>("pad");
    std.def_class::<PlaceNode>("place");
    std.def_class::<TransformNode<MOVE>>("move");
    std.def_class::<TransformNode<SCALE>>("scale");
    std.def_class::<TransformNode<ROTATE>>("rotate");
    std.def_class::<HideNode>("hide");
    std.def_class::<StackNode>("stack");
    std.def_class::<GridNode>("grid");
    std.def_class::<ColumnsNode>("columns");
    std.def_class::<ColbreakNode>("colbreak");

    // Utility functions.
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
    std.def_func("even", even);
    std.def_func("odd", odd);
    std.def_func("mod", modulo);
    std.def_func("range", range);
    std.def_func("rgb", rgb);
    std.def_func("cmyk", cmyk);
    std.def_func("lower", lower);
    std.def_func("upper", upper);
    std.def_func("letter", letter);
    std.def_func("roman", roman);
    std.def_func("symbol", symbol);
    std.def_func("len", len);
    std.def_func("sorted", sorted);

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
    std.def_const("serif", FontFamily::Serif);
    std.def_const("sans-serif", FontFamily::SansSerif);
    std.def_const("monospace", FontFamily::Monospace);

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
