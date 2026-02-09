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

// HTML spec § 13.1.2 Elements

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

// HTML spec § 3.2.5.2 Kinds of content

/// Whether an element is considered metadata content.
pub fn is_metadata_content(tag: HtmlTag) -> bool {
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

/// Wether an element is considered flow content.
pub fn is_flow_content(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::a
            | self::abbr
            | self::address
            | self::area
            | self::article
            | self::aside
            | self::audio
            | self::b
            | self::bdi
            | self::bdo
            | self::blockquote
            | self::br
            | self::button
            | self::canvas
            | self::cite
            | self::code
            | self::data
            | self::datalist
            | self::del
            | self::details
            | self::dfn
            | self::dialog
            | self::div
            | self::dl
            | self::em
            | self::embed
            | self::fieldset
            | self::figure
            | self::footer
            | self::form
            | self::h1
            | self::h2
            | self::h3
            | self::h4
            | self::h5
            | self::h6
            | self::header
            | self::hgroup
            | self::hr
            | self::i
            | self::iframe
            | self::img
            | self::input
            | self::ins
            | self::kbd
            | self::label
            | self::link
            | self::main
            | self::map
            | self::mark
            | self::menu
            | self::meta
            | self::meter
            | self::nav
            | self::noscript
            | self::object
            | self::ol
            | self::output
            | self::p
            | self::picture
            | self::pre
            | self::progress
            | self::q
            | self::ruby
            | self::s
            | self::samp
            | self::script
            | self::search
            | self::section
            | self::select
            | self::slot
            | self::small
            | self::span
            | self::strong
            | self::sub
            | self::sup
            | self::table
            | self::template
            | self::textarea
            | self::time
            | self::u
            | self::ul
            | self::var
            | self::video
            | self::wbr
    )
}

/// Whether an element is considered sectioning content.
pub fn is_sectioning_content(tag: HtmlTag) -> bool {
    matches!(tag, self::article | self::aside | self::nav | self::section)
}

/// Whether an element is considered heading content.
pub fn is_heading_content(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::h1 | self::h2 | self::h3 | self::h4 | self::h5 | self::h6 | self::hgroup
    )
}

/// Whether an element is considered phrasing content.
pub fn is_phrasing_content(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::a
            | self::abbr
            | self::area
            | self::audio
            | self::b
            | self::bdi
            | self::bdo
            | self::br
            | self::button
            | self::canvas
            | self::cite
            | self::code
            | self::data
            | self::datalist
            | self::del
            | self::dfn
            | self::em
            | self::embed
            | self::i
            | self::iframe
            | self::img
            | self::input
            | self::ins
            | self::kbd
            | self::label
            | self::link
            | self::map
            | self::mark
            | self::meta
            | self::meter
            | self::noscript
            | self::object
            | self::output
            | self::picture
            | self::progress
            | self::q
            | self::ruby
            | self::s
            | self::samp
            | self::script
            | self::select
            | self::slot
            | self::small
            | self::span
            | self::strong
            | self::sub
            | self::sup
            | self::template
            | self::textarea
            | self::time
            | self::u
            | self::var
            | self::video
            | self::wbr
    )
}

/// Whether an element is considered embedded content.
pub fn is_embedded_content(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::audio
            | self::canvas
            | self::embed
            | self::iframe
            | self::img
            | self::object
            | self::picture
            | self::video
    )
}

/// Whether an element is considered interactive content.
pub fn is_interactive_content(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::a
            | self::audio
            | self::button
            | self::details
            | self::embed
            | self::iframe
            | self::img
            | self::input
            | self::label
            | self::select
            | self::textarea
            | self::video
    )
}

/// Whether an element is considered palpable content.
pub fn is_palpable_content(tag: HtmlTag) -> bool {
    matches!(
        tag,
        self::a
            | self::abbr
            | self::address
            | self::article
            | self::aside
            | self::audio
            | self::b
            | self::bdi
            | self::bdo
            | self::blockquote
            | self::button
            | self::canvas
            | self::cite
            | self::code
            | self::data
            | self::del
            | self::details
            | self::dfn
            | self::div
            | self::dl
            | self::em
            | self::embed
            | self::fieldset
            | self::figure
            | self::footer
            | self::form
            | self::h1
            | self::h2
            | self::h3
            | self::h4
            | self::h5
            | self::h6
            | self::header
            | self::hgroup
            | self::i
            | self::iframe
            | self::img
            | self::input
            | self::ins
            | self::kbd
            | self::label
            | self::main
            | self::map
            | self::mark
            | self::menu
            | self::meter
            | self::nav
            | self::object
            | self::ol
            | self::output
            | self::p
            | self::picture
            | self::pre
            | self::progress
            | self::q
            | self::ruby
            | self::s
            | self::samp
            | self::search
            | self::section
            | self::select
            | self::small
            | self::span
            | self::strong
            | self::sub
            | self::sup
            | self::table
            | self::textarea
            | self::time
            | self::u
            | self::ul
            | self::var
            | self::video
    )
}

/// Whether an element is considered a script-supporting element.
pub fn is_script_supporting_element(tag: HtmlTag) -> bool {
    matches!(tag, self::script | self::template)
}

// Defaults of the CSS `display` property.

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
    )
}
