//! Typst's standard library.
#![deny(
    absolute_paths_not_starting_with_crate,
    future_incompatible,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    missing_debug_implementations,
    missing_docs,
    non_ascii_idents,
    nonstandard_style,
    noop_method_call,
    pointer_structural_match,
    private_in_public,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(clippy::pedantic, clippy::dbg_macro, clippy::print_stderr, clippy::print_stdout)]
#![allow(clippy::module_name_repetitions)]
#![deny(unsafe_code)]

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
#[must_use]
pub fn build() -> Library {
    let math = math::module();
    let calc = compute::calc::module();
    let global = global(math.clone(), calc);
    Library { global, math, styles: styles(), items: items() }
}

/// Construct the module with global definitions.
fn global(math: Module, calc: Module) -> Module {
    let mut global = Scope::deduplicating();

    text::define(&mut global);
    layout::define(&mut global);
    visualize::define(&mut global);
    meta::define(&mut global);
    symbols::define(&mut global);
    compute::define(&mut global);
    define_colors(&mut global);

    global.define("calc", calc);
    global.define("math", math);

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

fn define_colors(scope: &mut Scope) {
    scope.define("black", Color::BLACK);
    scope.define("gray", Color::GRAY);
    scope.define("silver", Color::SILVER);
    scope.define("white", Color::WHITE);
    scope.define("navy", Color::NAVY);
    scope.define("blue", Color::BLUE);
    scope.define("aqua", Color::AQUA);
    scope.define("teal", Color::TEAL);
    scope.define("eastern", Color::EASTERN);
    scope.define("purple", Color::PURPLE);
    scope.define("fuchsia", Color::FUCHSIA);
    scope.define("maroon", Color::MAROON);
    scope.define("red", Color::RED);
    scope.define("orange", Color::ORANGE);
    scope.define("yellow", Color::YELLOW);
    scope.define("olive", Color::OLIVE);
    scope.define("green", Color::GREEN);
    scope.define("lime", Color::LIME);
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
