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
    global.define("text", text::TextNode::id());
    global.define("linebreak", text::LinebreakNode::id());
    global.define("smartquote", text::SmartQuoteNode::id());
    global.define("strong", text::StrongNode::id());
    global.define("emph", text::EmphNode::id());
    global.define("lower", text::lower);
    global.define("upper", text::upper);
    global.define("smallcaps", text::smallcaps);
    global.define("sub", text::SubNode::id());
    global.define("super", text::SuperNode::id());
    global.define("underline", text::UnderlineNode::id());
    global.define("strike", text::StrikeNode::id());
    global.define("overline", text::OverlineNode::id());
    global.define("raw", text::RawNode::id());
    global.define("lorem", text::lorem);

    // Math.
    global.define("math", math);

    // Layout.
    global.define("page", layout::PageNode::id());
    global.define("pagebreak", layout::PagebreakNode::id());
    global.define("v", layout::VNode::id());
    global.define("par", layout::ParNode::id());
    global.define("parbreak", layout::ParbreakNode::id());
    global.define("h", layout::HNode::id());
    global.define("box", layout::BoxNode::id());
    global.define("block", layout::BlockNode::id());
    global.define("list", layout::ListNode::id());
    global.define("enum", layout::EnumNode::id());
    global.define("terms", layout::TermsNode::id());
    global.define("table", layout::TableNode::id());
    global.define("stack", layout::StackNode::id());
    global.define("grid", layout::GridNode::id());
    global.define("columns", layout::ColumnsNode::id());
    global.define("colbreak", layout::ColbreakNode::id());
    global.define("place", layout::PlaceNode::id());
    global.define("align", layout::AlignNode::id());
    global.define("pad", layout::PadNode::id());
    global.define("repeat", layout::RepeatNode::id());
    global.define("move", layout::MoveNode::id());
    global.define("scale", layout::ScaleNode::id());
    global.define("rotate", layout::RotateNode::id());
    global.define("hide", layout::HideNode::id());

    // Visualize.
    global.define("image", visualize::ImageNode::id());
    global.define("line", visualize::LineNode::id());
    global.define("rect", visualize::RectNode::id());
    global.define("square", visualize::SquareNode::id());
    global.define("ellipse", visualize::EllipseNode::id());
    global.define("circle", visualize::CircleNode::id());

    // Meta.
    global.define("document", meta::DocumentNode::id());
    global.define("ref", meta::RefNode::id());
    global.define("link", meta::LinkNode::id());
    global.define("outline", meta::OutlineNode::id());
    global.define("heading", meta::HeadingNode::id());
    global.define("figure", meta::FigureNode::id());
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
        raw_languages: text::RawNode::languages,
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
