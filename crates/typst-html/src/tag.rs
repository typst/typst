//! Predefined constants for HTML tags.

#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::HtmlTag;

pub const a: HtmlTag = HtmlTag::constant("a");
pub const abbr: HtmlTag = HtmlTag::constant("abbr");
pub const address: HtmlTag = HtmlTag::constant("address");
pub const area: HtmlTag = HtmlTag::constant("area");
pub const article: HtmlTag = HtmlTag::constant("article");
pub const aside: HtmlTag = HtmlTag::constant("aside");
pub const audio: HtmlTag = HtmlTag::constant("audio");
pub const b: HtmlTag = HtmlTag::constant("b");
pub const base: HtmlTag = HtmlTag::constant("base");
pub const bdi: HtmlTag = HtmlTag::constant("bdi");
pub const bdo: HtmlTag = HtmlTag::constant("bdo");
pub const blockquote: HtmlTag = HtmlTag::constant("blockquote");
pub const body: HtmlTag = HtmlTag::constant("body");
pub const br: HtmlTag = HtmlTag::constant("br");
pub const button: HtmlTag = HtmlTag::constant("button");
pub const canvas: HtmlTag = HtmlTag::constant("canvas");
pub const caption: HtmlTag = HtmlTag::constant("caption");
pub const cite: HtmlTag = HtmlTag::constant("cite");
pub const code: HtmlTag = HtmlTag::constant("code");
pub const col: HtmlTag = HtmlTag::constant("col");
pub const colgroup: HtmlTag = HtmlTag::constant("colgroup");
pub const data: HtmlTag = HtmlTag::constant("data");
pub const datalist: HtmlTag = HtmlTag::constant("datalist");
pub const dd: HtmlTag = HtmlTag::constant("dd");
pub const del: HtmlTag = HtmlTag::constant("del");
pub const details: HtmlTag = HtmlTag::constant("details");
pub const dfn: HtmlTag = HtmlTag::constant("dfn");
pub const dialog: HtmlTag = HtmlTag::constant("dialog");
pub const div: HtmlTag = HtmlTag::constant("div");
pub const dl: HtmlTag = HtmlTag::constant("dl");
pub const dt: HtmlTag = HtmlTag::constant("dt");
pub const em: HtmlTag = HtmlTag::constant("em");
pub const embed: HtmlTag = HtmlTag::constant("embed");
pub const fieldset: HtmlTag = HtmlTag::constant("fieldset");
pub const figcaption: HtmlTag = HtmlTag::constant("figcaption");
pub const figure: HtmlTag = HtmlTag::constant("figure");
pub const footer: HtmlTag = HtmlTag::constant("footer");
pub const form: HtmlTag = HtmlTag::constant("form");
pub const h1: HtmlTag = HtmlTag::constant("h1");
pub const h2: HtmlTag = HtmlTag::constant("h2");
pub const h3: HtmlTag = HtmlTag::constant("h3");
pub const h4: HtmlTag = HtmlTag::constant("h4");
pub const h5: HtmlTag = HtmlTag::constant("h5");
pub const h6: HtmlTag = HtmlTag::constant("h6");
pub const head: HtmlTag = HtmlTag::constant("head");
pub const header: HtmlTag = HtmlTag::constant("header");
pub const hgroup: HtmlTag = HtmlTag::constant("hgroup");
pub const hr: HtmlTag = HtmlTag::constant("hr");
pub const html: HtmlTag = HtmlTag::constant("html");
pub const i: HtmlTag = HtmlTag::constant("i");
pub const iframe: HtmlTag = HtmlTag::constant("iframe");
pub const img: HtmlTag = HtmlTag::constant("img");
pub const input: HtmlTag = HtmlTag::constant("input");
pub const ins: HtmlTag = HtmlTag::constant("ins");
pub const kbd: HtmlTag = HtmlTag::constant("kbd");
pub const label: HtmlTag = HtmlTag::constant("label");
pub const legend: HtmlTag = HtmlTag::constant("legend");
pub const li: HtmlTag = HtmlTag::constant("li");
pub const link: HtmlTag = HtmlTag::constant("link");
pub const main: HtmlTag = HtmlTag::constant("main");
pub const map: HtmlTag = HtmlTag::constant("map");
pub const mark: HtmlTag = HtmlTag::constant("mark");
pub const menu: HtmlTag = HtmlTag::constant("menu");
pub const meta: HtmlTag = HtmlTag::constant("meta");
pub const meter: HtmlTag = HtmlTag::constant("meter");
pub const nav: HtmlTag = HtmlTag::constant("nav");
pub const noscript: HtmlTag = HtmlTag::constant("noscript");
pub const object: HtmlTag = HtmlTag::constant("object");
pub const ol: HtmlTag = HtmlTag::constant("ol");
pub const optgroup: HtmlTag = HtmlTag::constant("optgroup");
pub const option: HtmlTag = HtmlTag::constant("option");
pub const output: HtmlTag = HtmlTag::constant("output");
pub const p: HtmlTag = HtmlTag::constant("p");
pub const picture: HtmlTag = HtmlTag::constant("picture");
pub const pre: HtmlTag = HtmlTag::constant("pre");
pub const progress: HtmlTag = HtmlTag::constant("progress");
pub const q: HtmlTag = HtmlTag::constant("q");
pub const rp: HtmlTag = HtmlTag::constant("rp");
pub const rt: HtmlTag = HtmlTag::constant("rt");
pub const ruby: HtmlTag = HtmlTag::constant("ruby");
pub const s: HtmlTag = HtmlTag::constant("s");
pub const samp: HtmlTag = HtmlTag::constant("samp");
pub const script: HtmlTag = HtmlTag::constant("script");
pub const search: HtmlTag = HtmlTag::constant("search");
pub const section: HtmlTag = HtmlTag::constant("section");
pub const select: HtmlTag = HtmlTag::constant("select");
pub const slot: HtmlTag = HtmlTag::constant("slot");
pub const small: HtmlTag = HtmlTag::constant("small");
pub const source: HtmlTag = HtmlTag::constant("source");
pub const span: HtmlTag = HtmlTag::constant("span");
pub const strong: HtmlTag = HtmlTag::constant("strong");
pub const style: HtmlTag = HtmlTag::constant("style");
pub const sub: HtmlTag = HtmlTag::constant("sub");
pub const summary: HtmlTag = HtmlTag::constant("summary");
pub const sup: HtmlTag = HtmlTag::constant("sup");
pub const table: HtmlTag = HtmlTag::constant("table");
pub const tbody: HtmlTag = HtmlTag::constant("tbody");
pub const td: HtmlTag = HtmlTag::constant("td");
pub const template: HtmlTag = HtmlTag::constant("template");
pub const textarea: HtmlTag = HtmlTag::constant("textarea");
pub const tfoot: HtmlTag = HtmlTag::constant("tfoot");
pub const th: HtmlTag = HtmlTag::constant("th");
pub const thead: HtmlTag = HtmlTag::constant("thead");
pub const time: HtmlTag = HtmlTag::constant("time");
pub const title: HtmlTag = HtmlTag::constant("title");
pub const tr: HtmlTag = HtmlTag::constant("tr");
pub const track: HtmlTag = HtmlTag::constant("track");
pub const u: HtmlTag = HtmlTag::constant("u");
pub const ul: HtmlTag = HtmlTag::constant("ul");
pub const var: HtmlTag = HtmlTag::constant("var");
pub const video: HtmlTag = HtmlTag::constant("video");
pub const wbr: HtmlTag = HtmlTag::constant("wbr");

/// Whether this is a void tag whose associated element may not have
/// children.
pub fn is_void(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::area
            | self::base
            | self::br
            | self::col
            | self::embed
            | self::hr
            | self::img
            | self::input
            | self::link
            | self::meta
            | self::source
            | self::track
            | self::wbr
    )
}

/// Whether this is a tag containing raw text.
pub fn is_raw(tag: HtmlTag) -> bool {
    matches!(tag, self::script | self::style)
}

/// Whether this is a tag containing escapable raw text.
pub fn is_escapable_raw(tag: HtmlTag) -> bool {
    matches!(tag, self::textarea | self::title)
}

/// Whether an element is considered metadata.
pub fn is_metadata(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::base
            | self::link
            | self::meta
            | self::noscript
            | self::script
            | self::style
            | self::template
            | self::title
    )
}

/// Whether nodes with the tag have the CSS property `display: block` by
/// default.
pub fn is_block_by_default(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::html
            | self::head
            | self::body
            | self::article
            | self::aside
            | self::h1
            | self::h2
            | self::h3
            | self::h4
            | self::h5
            | self::h6
            | self::hgroup
            | self::nav
            | self::section
            | self::dd
            | self::dl
            | self::dt
            | self::menu
            | self::ol
            | self::ul
            | self::address
            | self::blockquote
            | self::dialog
            | self::div
            | self::fieldset
            | self::figure
            | self::figcaption
            | self::footer
            | self::form
            | self::header
            | self::hr
            | self::legend
            | self::main
            | self::p
            | self::pre
            | self::search
    )
}

/// Whether the element is inline-level as opposed to being block-level.
///
/// Not sure whether this distinction really makes sense. But we somehow
/// need to decide what to put into automatic paragraphs. A `<strong>`
/// should merged into a paragraph created by realization, but a `<div>`
/// shouldn't.
///
/// <https://www.w3.org/TR/html401/struct/global.html#block-inline>
/// <https://developer.mozilla.org/en-US/docs/Glossary/Inline-level_content>
/// <https://github.com/orgs/mdn/discussions/353>
pub fn is_inline_by_default(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::abbr
            | self::a
            | self::bdi
            | self::b
            | self::br
            | self::bdo
            | self::code
            | self::cite
            | self::dfn
            | self::data
            | self::i
            | self::em
            | self::mark
            | self::kbd
            | self::rp
            | self::q
            | self::ruby
            | self::rt
            | self::samp
            | self::s
            | self::span
            | self::small
            | self::sub
            | self::strong
            | self::time
            | self::sup
            | self::var
            | self::u
            | self::mathml::math
    )
}

/// Whether nodes with the tag have the CSS property `display: table(-.*)?`
/// by default.
pub fn is_tabular_by_default(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::table
            | self::thead
            | self::tbody
            | self::tfoot
            | self::tr
            | self::th
            | self::td
            | self::caption
            | self::col
            | self::colgroup
            | self::mathml::mtable
            | self::mathml::mtr
            | self::mathml::mtd
    )
}

/// Whether this is a foreign element which has a self-closing tag.
pub fn is_foreign_self_closing(tag: HtmlTag) -> bool {
    self::mathml::is_self_closing(tag)
}

/// Elements in the MathML namespace.
/// (Only the ones defined in MathML Core at the moment.)
pub mod mathml {
    use super::HtmlTag;

    pub const annotation: HtmlTag = HtmlTag::constant("annotation");
    pub const annotation_xml: HtmlTag = HtmlTag::constant("annotation-xml");
    pub const maction: HtmlTag = HtmlTag::constant("maction");
    pub const math: HtmlTag = HtmlTag::constant("math");
    pub const merror: HtmlTag = HtmlTag::constant("merror");
    pub const mfrac: HtmlTag = HtmlTag::constant("mfrac");
    pub const mi: HtmlTag = HtmlTag::constant("mi");
    pub const mmultiscripts: HtmlTag = HtmlTag::constant("mmultiscripts");
    pub const mn: HtmlTag = HtmlTag::constant("mn");
    pub const mo: HtmlTag = HtmlTag::constant("mo");
    pub const mover: HtmlTag = HtmlTag::constant("mover");
    pub const mpadded: HtmlTag = HtmlTag::constant("mpadded");
    pub const mphantom: HtmlTag = HtmlTag::constant("mphantom");
    pub const mprescripts: HtmlTag = HtmlTag::constant("mprescripts");
    pub const mroot: HtmlTag = HtmlTag::constant("mroot");
    pub const mrow: HtmlTag = HtmlTag::constant("mrow");
    pub const ms: HtmlTag = HtmlTag::constant("ms");
    pub const mspace: HtmlTag = HtmlTag::constant("mspace");
    pub const msqrt: HtmlTag = HtmlTag::constant("msqrt");
    pub const mstyle: HtmlTag = HtmlTag::constant("mstyle");
    pub const msub: HtmlTag = HtmlTag::constant("msub");
    pub const msubsup: HtmlTag = HtmlTag::constant("msubsup");
    pub const msup: HtmlTag = HtmlTag::constant("msup");
    pub const mtable: HtmlTag = HtmlTag::constant("mtable");
    pub const mtd: HtmlTag = HtmlTag::constant("mtd");
    pub const mtext: HtmlTag = HtmlTag::constant("mtext");
    pub const mtr: HtmlTag = HtmlTag::constant("mtr");
    pub const munder: HtmlTag = HtmlTag::constant("munder");
    pub const munderover: HtmlTag = HtmlTag::constant("munderover");
    pub const semantics: HtmlTag = HtmlTag::constant("semantics");

    pub fn is_self_closing(tag: HtmlTag) -> bool {
        matches!(tag, self::mspace | self::mprescripts)
    }
}
