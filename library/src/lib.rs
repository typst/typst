//! Typst's standard library.

pub mod basics;
pub mod compute;
pub mod layout;
pub mod math;
pub mod meta;
pub mod prelude;
pub mod shared;
pub mod text;
pub mod visualize;

use typst::geom::{Align, Color, Dir, GenAlign};
use typst::model::{LangItems, Library, Node, NodeId, Scope, StyleMap};

use self::layout::LayoutRoot;

/// Construct the standard library.
pub fn build() -> Library {
    Library { scope: scope(), styles: styles(), items: items() }
}

/// Construct the standard scope.
fn scope() -> Scope {
    let mut std = Scope::new();

    // Basics.
    std.def_node::<basics::HeadingNode>("heading");
    std.def_node::<basics::ListNode>("list");
    std.def_node::<basics::EnumNode>("enum");
    std.def_node::<basics::DescNode>("desc");
    std.def_node::<basics::TableNode>("table");

    // Text.
    std.def_node::<text::TextNode>("text");
    std.def_node::<text::LinebreakNode>("linebreak");
    std.def_node::<text::SymbolNode>("symbol");
    std.def_node::<text::SmartQuoteNode>("smartquote");
    std.def_node::<text::StrongNode>("strong");
    std.def_node::<text::EmphNode>("emph");
    std.def_fn("lower", text::lower);
    std.def_fn("upper", text::upper);
    std.def_fn("smallcaps", text::smallcaps);
    std.def_node::<text::SubNode>("sub");
    std.def_node::<text::SuperNode>("super");
    std.def_node::<text::UnderlineNode>("underline");
    std.def_node::<text::StrikeNode>("strike");
    std.def_node::<text::OverlineNode>("overline");
    std.def_node::<text::RawNode>("raw");

    // Math.
    std.def_node::<math::MathNode>("math");
    std.def_node::<math::AtomNode>("atom");
    std.def_node::<math::FracNode>("frac");
    std.define("sum", "∑");
    std.define("in", "∈");
    std.define("arrow", "→");
    std.define("NN", "ℕ");
    std.define("RR", "ℝ");

    // Layout.
    std.def_node::<layout::PageNode>("page");
    std.def_node::<layout::PagebreakNode>("pagebreak");
    std.def_node::<layout::FlowNode>("flow");
    std.def_node::<layout::VNode>("v");
    std.def_node::<layout::ParNode>("par");
    std.def_node::<layout::ParbreakNode>("parbreak");
    std.def_node::<layout::HNode>("h");
    std.def_node::<layout::BoxNode>("box");
    std.def_node::<layout::BlockNode>("block");
    std.def_node::<layout::StackNode>("stack");
    std.def_node::<layout::GridNode>("grid");
    std.def_node::<layout::ColumnsNode>("columns");
    std.def_node::<layout::ColbreakNode>("colbreak");
    std.def_node::<layout::PlaceNode>("place");
    std.def_node::<layout::AlignNode>("align");
    std.def_node::<layout::PadNode>("pad");
    std.def_node::<layout::RepeatNode>("repeat");
    std.def_node::<layout::MoveNode>("move");
    std.def_node::<layout::ScaleNode>("scale");
    std.def_node::<layout::RotateNode>("rotate");
    std.def_node::<layout::HideNode>("hide");

    // Visualize.
    std.def_node::<visualize::ImageNode>("image");
    std.def_node::<visualize::LineNode>("line");
    std.def_node::<visualize::RectNode>("rect");
    std.def_node::<visualize::SquareNode>("square");
    std.def_node::<visualize::EllipseNode>("ellipse");
    std.def_node::<visualize::CircleNode>("circle");

    // Meta.
    std.def_node::<meta::DocumentNode>("document");
    std.def_node::<meta::RefNode>("ref");
    std.def_node::<meta::LinkNode>("link");
    std.def_node::<meta::OutlineNode>("outline");

    // Compute.
    std.def_fn("type", compute::type_);
    std.def_fn("repr", compute::repr);
    std.def_fn("assert", compute::assert);
    std.def_fn("eval", compute::eval);
    std.def_fn("int", compute::int);
    std.def_fn("float", compute::float);
    std.def_fn("luma", compute::luma);
    std.def_fn("rgb", compute::rgb);
    std.def_fn("cmyk", compute::cmyk);
    std.def_fn("str", compute::str);
    std.def_fn("label", compute::label);
    std.def_fn("regex", compute::regex);
    std.def_fn("range", compute::range);
    std.def_fn("abs", compute::abs);
    std.def_fn("min", compute::min);
    std.def_fn("max", compute::max);
    std.def_fn("even", compute::even);
    std.def_fn("odd", compute::odd);
    std.def_fn("mod", compute::mod_);
    std.def_fn("csv", compute::csv);
    std.def_fn("json", compute::json);
    std.def_fn("xml", compute::xml);
    std.def_fn("lorem", compute::lorem);
    std.def_fn("numbering", compute::numbering);

    // Colors.
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
    std.define("start", GenAlign::Start);
    std.define("end", GenAlign::End);
    std.define("left", GenAlign::Specific(Align::Left));
    std.define("center", GenAlign::Specific(Align::Center));
    std.define("right", GenAlign::Specific(Align::Right));
    std.define("top", GenAlign::Specific(Align::Top));
    std.define("horizon", GenAlign::Specific(Align::Horizon));
    std.define("bottom", GenAlign::Specific(Align::Bottom));

    std
}

/// Construct the standard style map.
fn styles() -> StyleMap {
    StyleMap::new()
}

/// Construct the standard lang item mapping.
fn items() -> LangItems {
    LangItems {
        layout: |world, content, styles| content.layout_root(world, styles),
        em: |styles| styles.get(text::TextNode::SIZE),
        dir: |styles| styles.get(text::TextNode::DIR),
        space: || text::SpaceNode.pack(),
        linebreak: |justify| text::LinebreakNode { justify }.pack(),
        text: |text| text::TextNode(text).pack(),
        text_id: NodeId::of::<text::TextNode>(),
        text_str: |content| Some(&content.to::<text::TextNode>()?.0),
        symbol: |notation| text::SymbolNode(notation).pack(),
        smart_quote: |double| text::SmartQuoteNode { double }.pack(),
        parbreak: || layout::ParbreakNode.pack(),
        strong: |body| text::StrongNode(body).pack(),
        emph: |body| text::EmphNode(body).pack(),
        raw: |text, lang, block| {
            let content = text::RawNode { text, block }.pack();
            match lang {
                Some(_) => content.styled(text::RawNode::LANG, lang),
                None => content,
            }
        },
        link: |url| meta::LinkNode::from_url(url).pack(),
        ref_: |target| meta::RefNode(target).pack(),
        heading: |level, body| basics::HeadingNode { level, body }.pack(),
        list_item: |body| basics::ListItem::List(Box::new(body)).pack(),
        enum_item: |number, body| basics::ListItem::Enum(number, Box::new(body)).pack(),
        desc_item: |term, body| {
            basics::ListItem::Desc(Box::new(basics::DescItem { term, body })).pack()
        },
        math: |children, display| math::MathNode { children, display }.pack(),
        math_atom: |atom| math::AtomNode(atom).pack(),
        math_script: |base, sub, sup| math::ScriptNode { base, sub, sup }.pack(),
        math_frac: |num, denom| math::FracNode { num, denom }.pack(),
        math_align: |count| math::AlignNode(count).pack(),
    }
}
