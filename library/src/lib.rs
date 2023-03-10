//! Typst's standard library.

pub mod compute;
pub mod layout;
pub mod math;
pub mod meta;
pub mod prelude;
pub mod shared;
pub mod symbols;
pub mod text;
pub mod visualize;

use typst::eval::{LangItems, Library, Module, Scope};
use typst::geom::{Align, Color, Dir, GenAlign};
use typst::model::{Node, NodeId, StyleMap};

use self::layout::LayoutRoot;

/// Construct the standard library.
pub fn build() -> Library {
    let math = math::module();
    let calc = compute::calc::module();
    let global = global(math.clone(), calc);
    Library { global, math, styles: styles(), items: items() }
}

/// Construct the module with global definitions.
fn global(math: Module, calc: Module) -> Module {
    let mut global = Scope::deduplicating();

    // Text.
    global.define("text", text::TextNode::func());
    global.define("linebreak", text::LinebreakNode::func());
    global.define("smartquote", text::SmartQuoteNode::func());
    global.define("strong", text::StrongNode::func());
    global.define("emph", text::EmphNode::func());
    global.define("lower", text::lower);
    global.define("upper", text::upper);
    global.define("smallcaps", text::smallcaps);
    global.define("sub", text::SubNode::func());
    global.define("super", text::SuperNode::func());
    global.define("underline", text::UnderlineNode::func());
    global.define("strike", text::StrikeNode::func());
    global.define("overline", text::OverlineNode::func());
    global.define("raw", text::RawNode::func());
    global.define("lorem", text::lorem);

    // Math.
    global.define("math", math);

    // Layout.
    global.define("page", layout::PageNode::func());
    global.define("pagebreak", layout::PagebreakNode::func());
    global.define("v", layout::VNode::func());
    global.define("par", layout::ParNode::func());
    global.define("parbreak", layout::ParbreakNode::func());
    global.define("h", layout::HNode::func());
    global.define("box", layout::BoxNode::func());
    global.define("block", layout::BlockNode::func());
    global.define("list", layout::ListNode::func());
    global.define("enum", layout::EnumNode::func());
    global.define("terms", layout::TermsNode::func());
    global.define("table", layout::TableNode::func());
    global.define("stack", layout::StackNode::func());
    global.define("grid", layout::GridNode::func());
    global.define("columns", layout::ColumnsNode::func());
    global.define("colbreak", layout::ColbreakNode::func());
    global.define("place", layout::PlaceNode::func());
    global.define("align", layout::AlignNode::func());
    global.define("pad", layout::PadNode::func());
    global.define("repeat", layout::RepeatNode::func());
    global.define("move", layout::MoveNode::func());
    global.define("scale", layout::ScaleNode::func());
    global.define("rotate", layout::RotateNode::func());
    global.define("hide", layout::HideNode::func());

    // Visualize.
    global.define("image", visualize::ImageNode::func());
    global.define("line", visualize::LineNode::func());
    global.define("rect", visualize::RectNode::func());
    global.define("square", visualize::SquareNode::func());
    global.define("ellipse", visualize::EllipseNode::func());
    global.define("circle", visualize::CircleNode::func());

    // Meta.
    global.define("document", meta::DocumentNode::func());
    global.define("ref", meta::RefNode::func());
    global.define("link", meta::LinkNode::func());
    global.define("outline", meta::OutlineNode::func());
    global.define("heading", meta::HeadingNode::func());
    global.define("numbering", meta::numbering);

    // Symbols.
    global.define("sym", symbols::sym());
    global.define("emoji", symbols::emoji());

    // Compute.
    global.define("type", compute::type_);
    global.define("repr", compute::repr);
    global.define("panic", compute::panic);
    global.define("assert", compute::assert);
    global.define("eval", compute::eval);
    global.define("int", compute::int);
    global.define("float", compute::float);
    global.define("luma", compute::luma);
    global.define("rgb", compute::rgb);
    global.define("cmyk", compute::cmyk);
    global.define("symbol", compute::symbol);
    global.define("str", compute::str);
    global.define("label", compute::label);
    global.define("regex", compute::regex);
    global.define("range", compute::range);
    global.define("read", compute::read);
    global.define("csv", compute::csv);
    global.define("json", compute::json);
    global.define("xml", compute::xml);

    // Calc.
    global.define("calc", calc);

    // Colors.
    global.define("black", Color::BLACK);
    global.define("gray", Color::GRAY);
    global.define("silver", Color::SILVER);
    global.define("white", Color::WHITE);
    global.define("navy", Color::NAVY);
    global.define("blue", Color::BLUE);
    global.define("aqua", Color::AQUA);
    global.define("teal", Color::TEAL);
    global.define("eastern", Color::EASTERN);
    global.define("purple", Color::PURPLE);
    global.define("fuchsia", Color::FUCHSIA);
    global.define("maroon", Color::MAROON);
    global.define("red", Color::RED);
    global.define("orange", Color::ORANGE);
    global.define("yellow", Color::YELLOW);
    global.define("olive", Color::OLIVE);
    global.define("green", Color::GREEN);
    global.define("lime", Color::LIME);

    // Other constants.
    global.define("ltr", Dir::LTR);
    global.define("rtl", Dir::RTL);
    global.define("ttb", Dir::TTB);
    global.define("btt", Dir::BTT);
    global.define("start", GenAlign::Start);
    global.define("end", GenAlign::End);
    global.define("left", GenAlign::Specific(Align::Left));
    global.define("center", GenAlign::Specific(Align::Center));
    global.define("right", GenAlign::Specific(Align::Right));
    global.define("top", GenAlign::Specific(Align::Top));
    global.define("horizon", GenAlign::Specific(Align::Horizon));
    global.define("bottom", GenAlign::Specific(Align::Bottom));

    Module::new("global").with_scope(global)
}

/// Construct the standard style map.
fn styles() -> StyleMap {
    StyleMap::new()
}

/// Construct the standard lang item mapping.
fn items() -> LangItems {
    LangItems {
        layout: |world, content, styles| content.layout_root(world, styles),
        em: text::TextNode::size_in,
        dir: text::TextNode::dir_in,
        space: || text::SpaceNode::new().pack(),
        linebreak: || text::LinebreakNode::new().pack(),
        text: |text| text::TextNode::new(text).pack(),
        text_id: NodeId::of::<text::TextNode>(),
        text_str: |content| Some(content.to::<text::TextNode>()?.text()),
        smart_quote: |double| text::SmartQuoteNode::new().with_double(double).pack(),
        parbreak: || layout::ParbreakNode::new().pack(),
        strong: |body| text::StrongNode::new(body).pack(),
        emph: |body| text::EmphNode::new(body).pack(),
        raw: |text, lang, block| {
            let mut node = text::RawNode::new(text).with_block(block);
            if let Some(lang) = lang {
                node = node.with_lang(Some(lang));
            }
            node.pack()
        },
        link: |url| meta::LinkNode::from_url(url).pack(),
        ref_: |target| meta::RefNode::new(target).pack(),
        heading: |level, title| meta::HeadingNode::new(title).with_level(level).pack(),
        list_item: |body| layout::ListItem::new(body).pack(),
        enum_item: |number, body| layout::EnumItem::new(body).with_number(number).pack(),
        term_item: |term, description| layout::TermItem::new(term, description).pack(),
        formula: |body, block| math::FormulaNode::new(body).with_block(block).pack(),
        math_align_point: || math::AlignPointNode::new().pack(),
        math_delimited: |open, body, close| math::LrNode::new(open + body + close).pack(),
        math_attach: |base, bottom, top| {
            math::AttachNode::new(base).with_bottom(bottom).with_top(top).pack()
        },
        math_accent: |base, accent| {
            math::AccentNode::new(base, math::Accent::new(accent)).pack()
        },
        math_frac: |num, denom| math::FracNode::new(num, denom).pack(),
    }
}
