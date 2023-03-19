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

use typst::diag::At;
use typst::eval::{LangItems, Library, Module, Scope};
use typst::geom::{Align, Color, Dir, GenAlign, Smart};
use typst::model::{Element, Styles};

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
    global.define("text", text::TextElem::func());
    global.define("linebreak", text::LinebreakElem::func());
    global.define("smartquote", text::SmartQuoteElem::func());
    global.define("strong", text::StrongElem::func());
    global.define("emph", text::EmphElem::func());
    global.define("lower", text::lower);
    global.define("upper", text::upper);
    global.define("smallcaps", text::smallcaps);
    global.define("sub", text::SubElem::func());
    global.define("super", text::SuperElem::func());
    global.define("underline", text::UnderlineElem::func());
    global.define("strike", text::StrikeElem::func());
    global.define("overline", text::OverlineElem::func());
    global.define("raw", text::RawElem::func());
    global.define("lorem", text::lorem);

    // Math.
    global.define("math", math);

    // Layout.
    global.define("page", layout::PageElem::func());
    global.define("pagebreak", layout::PagebreakElem::func());
    global.define("v", layout::VElem::func());
    global.define("par", layout::ParElem::func());
    global.define("parbreak", layout::ParbreakElem::func());
    global.define("h", layout::HElem::func());
    global.define("box", layout::BoxElem::func());
    global.define("block", layout::BlockElem::func());
    global.define("list", layout::ListElem::func());
    global.define("enum", layout::EnumElem::func());
    global.define("terms", layout::TermsElem::func());
    global.define("table", layout::TableElem::func());
    global.define("stack", layout::StackElem::func());
    global.define("grid", layout::GridElem::func());
    global.define("columns", layout::ColumnsElem::func());
    global.define("colbreak", layout::ColbreakElem::func());
    global.define("place", layout::PlaceElem::func());
    global.define("align", layout::AlignElem::func());
    global.define("pad", layout::PadElem::func());
    global.define("repeat", layout::RepeatElem::func());
    global.define("move", layout::MoveElem::func());
    global.define("scale", layout::ScaleElem::func());
    global.define("rotate", layout::RotateElem::func());
    global.define("hide", layout::HideElem::func());
    global.define("measure", layout::measure);

    // Visualize.
    global.define("image", visualize::ImageElem::func());
    global.define("line", visualize::LineElem::func());
    global.define("rect", visualize::RectElem::func());
    global.define("square", visualize::SquareElem::func());
    global.define("ellipse", visualize::EllipseElem::func());
    global.define("circle", visualize::CircleElem::func());

    // Meta.
    global.define("document", meta::DocumentElem::func());
    global.define("ref", meta::RefElem::func());
    global.define("link", meta::LinkElem::func());
    global.define("outline", meta::OutlineElem::func());
    global.define("heading", meta::HeadingElem::func());
    global.define("figure", meta::FigureElem::func());
    global.define("cite", meta::CiteElem::func());
    global.define("bibliography", meta::BibliographyElem::func());
    global.define("locate", meta::locate);
    global.define("style", meta::style);
    global.define("counter", meta::counter);
    global.define("numbering", meta::numbering);
    global.define("state", meta::state);
    global.define("query", meta::query);

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
fn styles() -> Styles {
    Styles::new()
}

/// Construct the standard lang item mapping.
fn items() -> LangItems {
    LangItems {
        layout: |world, content, styles| content.layout_root(world, styles),
        em: text::TextElem::size_in,
        dir: text::TextElem::dir_in,
        space: || text::SpaceElem::new().pack(),
        linebreak: || text::LinebreakElem::new().pack(),
        text: |text| text::TextElem::new(text).pack(),
        text_func: text::TextElem::func(),
        text_str: |content| Some(content.to::<text::TextElem>()?.text()),
        smart_quote: |double| text::SmartQuoteElem::new().with_double(double).pack(),
        parbreak: || layout::ParbreakElem::new().pack(),
        strong: |body| text::StrongElem::new(body).pack(),
        emph: |body| text::EmphElem::new(body).pack(),
        raw: |text, lang, block| {
            let mut elem = text::RawElem::new(text).with_block(block);
            if let Some(lang) = lang {
                elem.push_lang(Some(lang));
            }
            elem.pack()
        },
        raw_languages: text::RawElem::languages,
        link: |url| meta::LinkElem::from_url(url).pack(),
        reference: |target, supplement| {
            let mut elem = meta::RefElem::new(target);
            if let Some(supplement) = supplement {
                elem.push_supplement(Smart::Custom(Some(meta::Supplement::Content(
                    supplement,
                ))));
            }
            elem.pack()
        },
        bibliography_keys: meta::BibliographyElem::keys,
        heading: |level, title| meta::HeadingElem::new(title).with_level(level).pack(),
        list_item: |body| layout::ListItem::new(body).pack(),
        enum_item: |number, body| {
            let mut elem = layout::EnumItem::new(body);
            if let Some(number) = number {
                elem.push_number(Some(number));
            }
            elem.pack()
        },
        term_item: |term, description| layout::TermItem::new(term, description).pack(),
        equation: |body, block| math::EquationElem::new(body).with_block(block).pack(),
        math_align_point: || math::AlignPointElem::new().pack(),
        math_delimited: |open, body, close| math::LrElem::new(open + body + close).pack(),
        math_attach: |base, bottom, top| {
            let mut elem = math::AttachElem::new(base);
            if let Some(bottom) = bottom {
                elem.push_bottom(Some(bottom));
            }
            if let Some(top) = top {
                elem.push_top(Some(top));
            }
            elem.pack()
        },
        math_accent: |base, accent| {
            math::AccentElem::new(base, math::Accent::new(accent)).pack()
        },
        math_frac: |num, denom| math::FracElem::new(num, denom).pack(),
        library_method: |vm, dynamic, method, args, span| {
            if let Some(counter) = dynamic.downcast::<meta::Counter>().cloned() {
                counter.call_method(vm, method, args, span)
            } else if let Some(state) = dynamic.downcast::<meta::State>().cloned() {
                state.call_method(vm, method, args, span)
            } else {
                Err(format!("type {} has no method `{method}`", dynamic.type_name()))
                    .at(span)
            }
        },
    }
}
