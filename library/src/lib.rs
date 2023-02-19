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

use typst::geom::{Align, Color, Dir, GenAlign};
use typst::model::{LangItems, Library, Module, Node, NodeId, Scope, StyleMap};

use self::layout::LayoutRoot;

/// Construct the standard library.
pub fn build() -> Library {
    let math = math::module();
    let calc = compute::calc();
    let global = global(math.clone(), calc);
    Library { global, math, styles: styles(), items: items() }
}

/// Construct the module with global definitions.
fn global(math: Module, calc: Module) -> Module {
    let mut global = Scope::deduplicating();

    // Text.
    global.def_func::<text::TextNode>("text");
    global.def_func::<text::LinebreakNode>("linebreak");
    global.def_func::<text::SmartQuoteNode>("smartquote");
    global.def_func::<text::StrongNode>("strong");
    global.def_func::<text::EmphNode>("emph");
    global.def_func::<text::LowerFunc>("lower");
    global.def_func::<text::UpperFunc>("upper");
    global.def_func::<text::SmallcapsFunc>("smallcaps");
    global.def_func::<text::SubNode>("sub");
    global.def_func::<text::SuperNode>("super");
    global.def_func::<text::UnderlineNode>("underline");
    global.def_func::<text::StrikeNode>("strike");
    global.def_func::<text::OverlineNode>("overline");
    global.def_func::<text::RawNode>("raw");
    global.def_func::<text::LoremFunc>("lorem");

    // Math.
    global.define("math", math);

    // Layout.
    global.def_func::<layout::PageNode>("page");
    global.def_func::<layout::PagebreakNode>("pagebreak");
    global.def_func::<layout::VNode>("v");
    global.def_func::<layout::ParNode>("par");
    global.def_func::<layout::ParbreakNode>("parbreak");
    global.def_func::<layout::HNode>("h");
    global.def_func::<layout::BoxNode>("box");
    global.def_func::<layout::BlockNode>("block");
    global.def_func::<layout::ListNode>("list");
    global.def_func::<layout::EnumNode>("enum");
    global.def_func::<layout::TermsNode>("terms");
    global.def_func::<layout::TableNode>("table");
    global.def_func::<layout::StackNode>("stack");
    global.def_func::<layout::GridNode>("grid");
    global.def_func::<layout::ColumnsNode>("columns");
    global.def_func::<layout::ColbreakNode>("colbreak");
    global.def_func::<layout::PlaceNode>("place");
    global.def_func::<layout::AlignNode>("align");
    global.def_func::<layout::PadNode>("pad");
    global.def_func::<layout::RepeatNode>("repeat");
    global.def_func::<layout::MoveNode>("move");
    global.def_func::<layout::ScaleNode>("scale");
    global.def_func::<layout::RotateNode>("rotate");
    global.def_func::<layout::HideNode>("hide");

    // Visualize.
    global.def_func::<visualize::ImageNode>("image");
    global.def_func::<visualize::LineNode>("line");
    global.def_func::<visualize::RectNode>("rect");
    global.def_func::<visualize::SquareNode>("square");
    global.def_func::<visualize::EllipseNode>("ellipse");
    global.def_func::<visualize::CircleNode>("circle");

    // Meta.
    global.def_func::<meta::DocumentNode>("document");
    global.def_func::<meta::RefNode>("ref");
    global.def_func::<meta::LinkNode>("link");
    global.def_func::<meta::OutlineNode>("outline");
    global.def_func::<meta::HeadingNode>("heading");
    global.def_func::<meta::NumberingFunc>("numbering");

    // Symbols.
    global.define("sym", symbols::sym());
    global.define("emoji", symbols::emoji());

    // Compute.
    global.def_func::<compute::TypeFunc>("type");
    global.def_func::<compute::ReprFunc>("repr");
    global.def_func::<compute::PanicFunc>("panic");
    global.def_func::<compute::AssertFunc>("assert");
    global.def_func::<compute::EvalFunc>("eval");
    global.def_func::<compute::IntFunc>("int");
    global.def_func::<compute::FloatFunc>("float");
    global.def_func::<compute::LumaFunc>("luma");
    global.def_func::<compute::RgbFunc>("rgb");
    global.def_func::<compute::CmykFunc>("cmyk");
    global.def_func::<compute::SymbolFunc>("symbol");
    global.def_func::<compute::StrFunc>("str");
    global.def_func::<compute::LabelFunc>("label");
    global.def_func::<compute::RegexFunc>("regex");
    global.def_func::<compute::RangeFunc>("range");
    global.def_func::<compute::ReadFunc>("read");
    global.def_func::<compute::CsvFunc>("csv");
    global.def_func::<compute::JsonFunc>("json");
    global.def_func::<compute::XmlFunc>("xml");

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
        em: |styles| styles.get(text::TextNode::SIZE),
        dir: |styles| styles.get(text::TextNode::DIR),
        space: || text::SpaceNode.pack(),
        linebreak: || text::LinebreakNode { justify: false }.pack(),
        text: |text| text::TextNode(text).pack(),
        text_id: NodeId::of::<text::TextNode>(),
        text_str: |content| Some(&content.to::<text::TextNode>()?.0),
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
        heading: |level, body| meta::HeadingNode { level, title: body }.pack(),
        list_item: |body| layout::ListItem::List(body).pack(),
        enum_item: |number, body| layout::ListItem::Enum(number, body).pack(),
        term_item: |term, description| {
            layout::ListItem::Term(layout::TermItem { term, description }).pack()
        },
        formula: |body, block| math::FormulaNode { body, block }.pack(),
        math_align_point: || math::AlignPointNode.pack(),
        math_delimited: |open, body, close| {
            math::LrNode { body: open + body + close, size: None }.pack()
        },
        math_attach: |base, bottom, top| math::AttachNode { base, bottom, top }.pack(),
        math_accent: |base, accent| math::AccentNode { base, accent }.pack(),
        math_frac: |num, denom| math::FracNode { num, denom }.pack(),
    }
}
