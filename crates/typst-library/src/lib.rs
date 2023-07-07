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

use typst::diag::At;
use typst::eval::{LangItems, Library, Module, Scope};
use typst::geom::Smart;
use typst::model::{Element, Styles};

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

    // Categories.
    text::define(&mut global);
    layout::define(&mut global);
    visualize::define(&mut global);
    meta::define(&mut global);
    compute::define(&mut global);
    symbols::define(&mut global);
    global.define("math", math);

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
        heading_func: meta::HeadingElem::func(),
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
        math_accent: |base, accent| {
            math::AccentElem::new(base, math::Accent::new(accent)).pack()
        },
        math_frac: |num, denom| math::FracElem::new(num, denom).pack(),
        math_root: |index, radicand| {
            math::RootElem::new(radicand).with_index(index).pack()
        },
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
