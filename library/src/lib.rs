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
    std.def_func::<basics::HeadingNode>("heading");
    std.def_func::<basics::ListNode>("list");
    std.def_func::<basics::EnumNode>("enum");
    std.def_func::<basics::TermsNode>("terms");
    std.def_func::<basics::TableNode>("table");

    // Text.
    std.def_func::<text::TextNode>("text");
    std.def_func::<text::LinebreakNode>("linebreak");
    std.def_func::<text::SymbolNode>("symbol");
    std.def_func::<text::SmartQuoteNode>("smartquote");
    std.def_func::<text::StrongNode>("strong");
    std.def_func::<text::EmphNode>("emph");
    std.def_func::<text::LowerFunc>("lower");
    std.def_func::<text::UpperFunc>("upper");
    std.def_func::<text::SmallcapsFunc>("smallcaps");
    std.def_func::<text::SubNode>("sub");
    std.def_func::<text::SuperNode>("super");
    std.def_func::<text::UnderlineNode>("underline");
    std.def_func::<text::StrikeNode>("strike");
    std.def_func::<text::OverlineNode>("overline");
    std.def_func::<text::RawNode>("raw");

    // Math.
    std.def_func::<math::MathNode>("math");
    std.def_func::<math::AccNode>("acc");
    std.def_func::<math::FracNode>("frac");
    std.def_func::<math::BinomNode>("binom");
    std.def_func::<math::ScriptNode>("script");
    std.def_func::<math::SqrtNode>("sqrt");
    std.def_func::<math::FloorNode>("floor");
    std.def_func::<math::CeilNode>("ceil");
    std.def_func::<math::VecNode>("vec");
    std.def_func::<math::CasesNode>("cases");
    std.def_func::<math::SerifNode>("serif");
    std.def_func::<math::SansNode>("sans");
    std.def_func::<math::BoldNode>("bold");
    std.def_func::<math::ItalNode>("ital");
    std.def_func::<math::CalNode>("cal");
    std.def_func::<math::FrakNode>("frak");
    std.def_func::<math::MonoNode>("mono");
    std.def_func::<math::BbNode>("bb");

    // Layout.
    std.def_func::<layout::PageNode>("page");
    std.def_func::<layout::PagebreakNode>("pagebreak");
    std.def_func::<layout::VNode>("v");
    std.def_func::<layout::ParNode>("par");
    std.def_func::<layout::ParbreakNode>("parbreak");
    std.def_func::<layout::HNode>("h");
    std.def_func::<layout::BoxNode>("box");
    std.def_func::<layout::BlockNode>("block");
    std.def_func::<layout::StackNode>("stack");
    std.def_func::<layout::GridNode>("grid");
    std.def_func::<layout::ColumnsNode>("columns");
    std.def_func::<layout::ColbreakNode>("colbreak");
    std.def_func::<layout::PlaceNode>("place");
    std.def_func::<layout::AlignNode>("align");
    std.def_func::<layout::PadNode>("pad");
    std.def_func::<layout::RepeatNode>("repeat");
    std.def_func::<layout::MoveNode>("move");
    std.def_func::<layout::ScaleNode>("scale");
    std.def_func::<layout::RotateNode>("rotate");
    std.def_func::<layout::HideNode>("hide");

    // Visualize.
    std.def_func::<visualize::ImageNode>("image");
    std.def_func::<visualize::LineNode>("line");
    std.def_func::<visualize::RectNode>("rect");
    std.def_func::<visualize::SquareNode>("square");
    std.def_func::<visualize::EllipseNode>("ellipse");
    std.def_func::<visualize::CircleNode>("circle");

    // Meta.
    std.def_func::<meta::DocumentNode>("document");
    std.def_func::<meta::RefNode>("ref");
    std.def_func::<meta::LinkNode>("link");
    std.def_func::<meta::OutlineNode>("outline");

    // Compute.
    std.def_func::<compute::TypeFunc>("type");
    std.def_func::<compute::ReprFunc>("repr");
    std.def_func::<compute::AssertFunc>("assert");
    std.def_func::<compute::EvalFunc>("eval");
    std.def_func::<compute::IntFunc>("int");
    std.def_func::<compute::FloatFunc>("float");
    std.def_func::<compute::LumaFunc>("luma");
    std.def_func::<compute::RgbFunc>("rgb");
    std.def_func::<compute::CmykFunc>("cmyk");
    std.def_func::<compute::StrFunc>("str");
    std.def_func::<compute::LabelFunc>("label");
    std.def_func::<compute::RegexFunc>("regex");
    std.def_func::<compute::RangeFunc>("range");
    std.def_func::<compute::AbsFunc>("abs");
    std.def_func::<compute::MinFunc>("min");
    std.def_func::<compute::MaxFunc>("max");
    std.def_func::<compute::EvenFunc>("even");
    std.def_func::<compute::OddFunc>("odd");
    std.def_func::<compute::ModFunc>("mod");
    std.def_func::<compute::ReadFunc>("read");
    std.def_func::<compute::CsvFunc>("csv");
    std.def_func::<compute::JsonFunc>("json");
    std.def_func::<compute::XmlFunc>("xml");
    std.def_func::<compute::LoremFunc>("lorem");
    std.def_func::<compute::NumberingFunc>("numbering");

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
        linebreak: || text::LinebreakNode { justify: false }.pack(),
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
        heading: |level, body| basics::HeadingNode { level, title: body }.pack(),
        list_item: |body| layout::ListItem::List(body).pack(),
        enum_item: |number, body| layout::ListItem::Enum(number, body).pack(),
        term_item: |term, description| {
            layout::ListItem::Term(basics::TermItem { term, description }).pack()
        },
        math: |children, block| math::MathNode { children, block }.pack(),
        math_atom: |atom| math::AtomNode(atom).pack(),
        math_script: |base, sub, sup| math::ScriptNode { base, sub, sup }.pack(),
        math_frac: |num, denom| math::FracNode { num, denom }.pack(),
        math_align_point: |count| math::AlignPointNode(count).pack(),
    }
}
