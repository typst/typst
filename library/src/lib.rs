//! Typst's standard library.

pub mod base;
pub mod core;
pub mod graphics;
pub mod layout;
pub mod math;
pub mod prelude;
pub mod structure;
pub mod text;

use typst::geom::{Align, Color, Dir, GenAlign};
use typst::model::{LangItems, Library, Node, NodeId, Scope, StyleMap};

use self::layout::LayoutRoot;

/// Construct the standard library.
pub fn new() -> Library {
    Library { scope: scope(), styles: styles(), items: items() }
}

/// Construct the standard scope.
fn scope() -> Scope {
    let mut std = Scope::new();

    // Text.
    std.def_node::<text::SpaceNode>("space");
    std.def_node::<text::LinebreakNode>("linebreak");
    std.def_node::<text::SmartQuoteNode>("smartquote");
    std.def_node::<text::TextNode>("text");
    std.def_node::<text::ParNode>("par");
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

    // Base.
    std.def_fn("type", base::type_);
    std.def_fn("assert", base::assert);
    std.def_fn("eval", base::eval);
    std.def_fn("int", base::int);
    std.def_fn("float", base::float);
    std.def_fn("abs", base::abs);
    std.def_fn("min", base::min);
    std.def_fn("max", base::max);
    std.def_fn("even", base::even);
    std.def_fn("odd", base::odd);
    std.def_fn("mod", base::mod_);
    std.def_fn("range", base::range);
    std.def_fn("luma", base::luma);
    std.def_fn("rgb", base::rgb);
    std.def_fn("cmyk", base::cmyk);
    std.def_fn("repr", base::repr);
    std.def_fn("str", base::str);
    std.def_fn("regex", base::regex);
    std.def_fn("letter", base::letter);
    std.def_fn("roman", base::roman);
    std.def_fn("symbol", base::symbol);
    std.def_fn("lorem", base::lorem);
    std.def_fn("csv", base::csv);
    std.def_fn("json", base::json);
    std.def_fn("xml", base::xml);

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
        smart_quote: |double| text::SmartQuoteNode { double }.pack(),
        parbreak: || text::ParbreakNode.pack(),
        strong: |body| text::StrongNode(body).pack(),
        emph: |body| text::EmphNode(body).pack(),
        raw: |text, lang, block| {
            let content = text::RawNode { text, block }.pack();
            match lang {
                Some(_) => content.styled(text::RawNode::LANG, lang),
                None => content,
            }
        },
        link: |url| text::LinkNode::from_url(url).pack(),
        ref_: |target| structure::RefNode(target).pack(),
        heading: |level, body| structure::HeadingNode { level, body }.pack(),
        list_item: |body| structure::ListItem::List(Box::new(body)).pack(),
        enum_item: |number, body| {
            structure::ListItem::Enum(number, Box::new(body)).pack()
        },
        desc_item: |term, body| {
            structure::ListItem::Desc(Box::new(structure::DescItem { term, body })).pack()
        },
        math: |children, display| math::MathNode { children, display }.pack(),
        math_atom: |atom| math::AtomNode(atom).pack(),
        math_script: |base, sub, sup| math::ScriptNode { base, sub, sup }.pack(),
        math_frac: |num, denom| math::FracNode { num, denom }.pack(),
        math_align: |count| math::AlignNode(count).pack(),
    }
}
