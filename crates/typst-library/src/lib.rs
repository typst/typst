//! Typst's standard library.

#![allow(clippy::wildcard_in_or_patterns)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::comparison_chain)]

pub mod compute;
pub mod layout;
pub mod math;
pub mod meta;
pub mod prelude;
pub mod shared;
pub mod symbols;
pub mod text;
pub mod visualize;

use typst::eval::{Array, LangItems, Library, Module, Scope};
use typst::geom::{Align, Color, Dir, Smart};
use typst::model::{NativeElement, Styles};

use self::layout::LayoutRoot;

/// Construct the standard library.
pub fn build() -> Library {
    let math = math::module();
    let global = global(math.clone());
    Library { global, math, styles: styles(), items: items() }
}

/// Construct the module with global definitions.
#[tracing::instrument(skip_all)]
fn global(math: Module) -> Module {
    let mut global = Scope::deduplicating();
    text::define(&mut global);
    global.define_module(math);
    layout::define(&mut global);
    visualize::define(&mut global);
    meta::define(&mut global);
    symbols::define(&mut global);
    compute::define(&mut global);
    prelude(&mut global);
    Module::new("global", global)
}

/// Defines scoped values that are globally available, too.
fn prelude(global: &mut Scope) {
    global.reset_category();
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
    global.define("luma", Color::luma_data());
    global.define("oklab", Color::oklab_data());
    global.define("rgb", Color::rgb_data());
    global.define("cmyk", Color::cmyk_data());
    global.define("range", Array::range_data());
    global.define("ltr", Dir::LTR);
    global.define("rtl", Dir::RTL);
    global.define("ttb", Dir::TTB);
    global.define("btt", Dir::BTT);
    global.define("start", Align::START);
    global.define("left", Align::LEFT);
    global.define("center", Align::CENTER);
    global.define("right", Align::RIGHT);
    global.define("end", Align::END);
    global.define("top", Align::TOP);
    global.define("horizon", Align::HORIZON);
    global.define("bottom", Align::BOTTOM);
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
        text_elem: text::TextElem::elem(),
        text_str: |content| Some(content.to::<text::TextElem>()?.text()),
        smart_quote: |double| text::SmartquoteElem::new().with_double(double).pack(),
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
        heading_elem: meta::HeadingElem::elem(),
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
        math_attach: |base, t, b, tl, bl, tr, br| {
            let mut elem = math::AttachElem::new(base);
            if let Some(t) = t {
                elem.push_t(Some(t));
            }
            if let Some(b) = b {
                elem.push_b(Some(b));
            }
            if let Some(tl) = tl {
                elem.push_tl(Some(tl));
            }
            if let Some(bl) = bl {
                elem.push_bl(Some(bl));
            }
            if let Some(tr) = tr {
                elem.push_tr(Some(tr));
            }
            if let Some(br) = br {
                elem.push_br(Some(br));
            }
            elem.pack()
        },
        math_primes: |count| math::PrimesElem::new(count).pack(),
        math_accent: |base, accent| {
            math::AccentElem::new(base, math::Accent::new(accent)).pack()
        },
        math_frac: |num, denom| math::FracElem::new(num, denom).pack(),
        math_root: |index, radicand| {
            math::RootElem::new(radicand).with_index(index).pack()
        },
    }
}
