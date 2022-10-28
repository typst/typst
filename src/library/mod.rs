//! The standard library.
//!
//! Call [`scope`] to obtain a [`Scope`] containing all standard library
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
pub fn scope() -> Scope {
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
    std.def_node::<text::SuperNode>("super");
    std.def_node::<text::SubNode>("sub");
    std.def_node::<text::LinkNode>("link");
    std.def_node::<text::RepeatNode>("repeat");
    std.def_fn("lower", text::lower);
    std.def_fn("upper", text::upper);
    std.def_fn("smallcaps", text::smallcaps);

    // Structure.
    std.def_node::<structure::RefNode>("ref");
    std.def_node::<structure::HeadingNode>("heading");
    std.def_node::<structure::ListNode>("list");
    std.def_node::<structure::EnumNode>("enum");
    std.def_node::<structure::DescNode>("desc");
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
    std.def_node::<layout::MoveNode>("move");
    std.def_node::<layout::ScaleNode>("scale");
    std.def_node::<layout::RotateNode>("rotate");

    // Graphics.
    std.def_node::<graphics::ImageNode>("image");
    std.def_node::<graphics::LineNode>("line");
    std.def_node::<graphics::RectNode>("rect");
    std.def_node::<graphics::SquareNode>("square");
    std.def_node::<graphics::EllipseNode>("ellipse");
    std.def_node::<graphics::CircleNode>("circle");
    std.def_node::<graphics::HideNode>("hide");

    // Math.
    std.def_node::<math::MathNode>("math");
    std.define("sum", "∑");
    std.define("in", "∈");
    std.define("arrow", "→");
    std.define("NN", "ℕ");
    std.define("RR", "ℝ");

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
    std.def_fn("csv", utility::csv);
    std.def_fn("json", utility::json);
    std.def_fn("xml", utility::xml);

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

/// Construct the standard role map.
pub fn roles() -> RoleMap {
    RoleMap {
        strong: |body| Content::show(text::StrongNode(body)),
        emph: |body| Content::show(text::EmphNode(body)),
        raw: |text, lang, block| {
            let node = Content::show(text::RawNode { text, block });
            match lang {
                Some(_) => node.styled(text::RawNode::LANG, lang),
                None => node,
            }
        },
        link: |url| Content::show(text::LinkNode::from_url(url)),
        ref_: |target| Content::show(structure::RefNode(target)),
        heading: |level, body| Content::show(structure::HeadingNode { level, body }),
        list_item: |body| Content::Item(structure::ListItem::List(Box::new(body))),
        enum_item: |number, body| {
            Content::Item(structure::ListItem::Enum(number, Box::new(body)))
        },
        desc_item: |term, body| {
            Content::Item(structure::ListItem::Desc(Box::new(structure::DescItem {
                term,
                body,
            })))
        },
    }
}

/// Additional methods on content.
pub trait ContentExt {
    /// Make this content strong.
    fn strong(self) -> Self;

    /// Make this content emphasized.
    fn emph(self) -> Self;

    /// Underline this content.
    fn underlined(self) -> Self;
}

impl ContentExt for Content {
    fn strong(self) -> Self {
        Self::show(text::StrongNode(self))
    }

    fn emph(self) -> Self {
        Self::show(text::EmphNode(self))
    }

    fn underlined(self) -> Self {
        Self::show(text::DecoNode::<{ text::UNDERLINE }>(self))
    }
}

/// Additional methods for the style chain.
pub trait StyleMapExt {
    /// Set a font family composed of a preferred family and existing families
    /// from a style chain.
    fn set_family(&mut self, preferred: text::FontFamily, existing: StyleChain);
}

impl StyleMapExt for StyleMap {
    fn set_family(&mut self, preferred: text::FontFamily, existing: StyleChain) {
        self.set(
            text::TextNode::FAMILY,
            std::iter::once(preferred)
                .chain(existing.get(text::TextNode::FAMILY).iter().cloned())
                .collect(),
        );
    }
}

/// Additional methods for layout nodes.
pub trait LayoutNodeExt {
    /// Set alignments for this node.
    fn aligned(self, aligns: Axes<Option<RawAlign>>) -> Self;

    /// Pad this node at the sides.
    fn padded(self, padding: Sides<Rel<Length>>) -> Self;

    /// Transform this node's contents without affecting layout.
    fn moved(self, delta: Axes<Rel<Length>>) -> Self;
}

impl LayoutNodeExt for LayoutNode {
    fn aligned(self, aligns: Axes<Option<RawAlign>>) -> Self {
        if aligns.any(Option::is_some) {
            layout::AlignNode { aligns, child: self }.pack()
        } else {
            self
        }
    }

    fn padded(self, padding: Sides<Rel<Length>>) -> Self {
        if !padding.left.is_zero()
            || !padding.top.is_zero()
            || !padding.right.is_zero()
            || !padding.bottom.is_zero()
        {
            layout::PadNode { padding, child: self }.pack()
        } else {
            self
        }
    }

    fn moved(self, delta: Axes<Rel<Length>>) -> Self {
        if delta.any(|r| !r.is_zero()) {
            layout::MoveNode { delta, child: self }.pack()
        } else {
            self
        }
    }
}
