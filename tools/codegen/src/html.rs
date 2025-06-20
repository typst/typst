//! Usage: `cargo run -p typst-codegen -- html path/to/spec/dir`
//!
//! The spec dir will automatically be populated with the necessary
//! specifications if one is missing.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::{Display, Write};
use std::path::PathBuf;

use regex::Regex;
use scraper::{ElementRef, Html, Selector};

pub fn main() {
    // Directory where specs will be read from / downloaded into.
    let dir = std::env::args_os().nth(2).map(PathBuf::from);
    let mut ctx = Context {
        html: load_spec(&dir, "html", "https://html.spec.whatwg.org/"),
        fetch: load_spec(&dir, "fetch", "https://fetch.spec.whatwg.org/"),
        referrer: load_spec(
            &dir,
            "referrer",
            "https://w3c.github.io/webappsec-referrer-policy/",
        ),
        aria: load_spec(&dir, "aria", "https://www.w3.org/TR/wai-aria-1.1/"),
        strings: vec![],
    };

    let attr_infos = collect_attributes(&mut ctx);
    let element_infos = collect_elements(&ctx, &attr_infos);
    let output = Output {
        tags: element_infos.iter().map(|info| info.name.clone()).collect(),
        attrs: attr_infos.iter().map(|info| info.name.clone()).collect(),
        attr_global_count: attr_infos
            .iter()
            .filter(|info| matches!(info.applies_to, Applicable::Globally))
            .count(),
        element_infos,
        attr_infos,
        attr_strings: ctx.strings,
    };

    let path = "crates/typst-library/src/html/generated.rs";
    let code = codegen(&output);
    std::fs::write(path, code).unwrap();

    println!("Success!");
}

/// Reads a spec from the directory or, if it does not exist, fetches and stores it.
fn load_spec(spec_dir: &Option<PathBuf>, name: &str, url: &str) -> ElementRef<'static> {
    let text = if let Some(dir) = spec_dir {
        let path = dir.join(name).with_extension("html");
        if path.exists() {
            println!("Reading from {}", path.display());
            std::fs::read_to_string(&path).unwrap()
        } else {
            let text = crate::fetch(url);
            println!("Writing to {}", path.display());
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(&path, &text).unwrap();
            text
        }
    } else {
        crate::fetch(url)
    };
    Box::leak(Box::new(Html::parse_document(&text))).root_element()
}

struct Context<'a> {
    html: ElementRef<'a>,
    fetch: ElementRef<'a>,
    referrer: ElementRef<'a>,
    aria: ElementRef<'a>,
    strings: Vec<(String, String)>,
}

struct Output {
    tags: BTreeSet<String>,
    attrs: BTreeSet<String>,
    attr_global_count: usize,
    element_infos: Vec<ElementInfo>,
    attr_infos: Vec<AttrInfo>,
    attr_strings: Vec<(String, String)>,
}

struct ElementInfo {
    name: String,
    docs: String,
    attributes: Vec<usize>,
}

struct AttrInfo {
    name: String,
    docs: String,
    ty: String,
    applies_to: Applicable,
}

enum Applicable {
    Globally,
    Elements(Vec<String>),
}

impl Applicable {
    fn applies_specifically_to(&self, tag: &str) -> bool {
        match self {
            Self::Globally => false,
            Self::Elements(elements) => elements.iter().any(|s| s == tag),
        }
    }
}

/// Creates a lazily initialized static value.
macro_rules! lazy {
    ($ty:ty = $init:expr) => {{
        static VAL: ::std::sync::LazyLock<$ty> = ::std::sync::LazyLock::new(|| $init);
        &*VAL
    }};
}

/// Creates a static CSS selector.
macro_rules! s {
    ($s:literal) => {
        lazy!(Selector = Selector::parse($s).unwrap())
    };
}

/// Creates a lazily initialized regular expression.
macro_rules! re {
    ($s:expr) => {
        lazy!(Regex = Regex::new($s).unwrap())
    };
}

/// Like `match`, but with regular expressions!
macro_rules! regex_match {
    ($text:expr, {
        $($re:literal $(if $guard:expr)? => $out:expr,)*
        _ => $final:expr $(,)?
    }) => {{
        let __text = $text;
        match () {
            $(_ if re!(&concat!("(?i)^", $re, "$")).is_match(__text)
                $(&& $guard)? => $out,)*
            _ => $final
        }
    }};
}

/// Collects all attributes with documentation and descriptions.
fn collect_attributes(ctx: &mut Context) -> Vec<AttrInfo> {
    let mut infos = vec![];
    collect_html_attributes(ctx, &mut infos);
    collect_aria_attributes(ctx, &mut infos);
    infos.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
    infos
}

/// Global attributes should come first and attributes be binary-searchable.
fn sort_key(attr: &AttrInfo) -> impl Ord + '_ {
    (matches!(attr.applies_to, Applicable::Elements(_)), &attr.name)
}

/// Collects attributes from the HTML spec.
fn collect_html_attributes(ctx: &mut Context, infos: &mut Vec<AttrInfo>) {
    for tr in ctx.html.select_first(s!("#attributes-1")).select(s!("tbody > tr")) {
        let name = tr.select_text(s!("th code"));
        let elements = tr.select_first(s!("td:nth-of-type(1)"));
        let mut docs = docs(&tr.select_text(s!("td:nth-of-type(2)")));

        let applies_to = if elements.inner_text().trim() == "HTML elements" {
            Applicable::Globally
        } else {
            Applicable::Elements(
                elements.select(s!("code")).map(|elem| elem.inner_text()).collect(),
            )
        };

        let ty_cell = tr.select_first(s!("td:nth-of-type(3)"));
        let ty = determine_type(ctx, &name, ty_cell, &mut docs);
        infos.push(AttrInfo { name, docs, ty, applies_to });
    }

    // HTML spec is missing this.
    infos.push(AttrInfo {
        name: "rel".into(),
        docs: "Relationship between the document containing \
               the form and its action destination"
            .into(),
        ty: rel_type(ctx),
        applies_to: Applicable::Elements(vec!["form".into()]),
    });
}

/// Collects attributes from the ARIA spec.
fn collect_aria_attributes(ctx: &mut Context, infos: &mut Vec<AttrInfo>) {
    // Collect ARIA roles.
    let role_dl = ctx.aria.select_first(s!("#index_role"));
    infos.push(AttrInfo {
        name: "role".into(),
        docs: "An ARIA role.".into(),
        ty: create_str_enum(
            ctx,
            role_dl
                .select(s!("dt code"))
                .zip(role_dl.select(s!("dd")))
                .map(|(code, dd)| (code.inner_text(), dd.inner_text())),
        ),
        applies_to: Applicable::Globally,
    });

    // Collect ARIA property and state attributes.
    let attrs_dl = ctx.aria.select_first(s!("#index_state_prop"));
    for (dt, dd) in attrs_dl.select(s!("dt")).zip(attrs_dl.select(s!("dd"))) {
        let docs = docs(&dd.inner_text());
        if docs.contains("Deprecated") {
            continue;
        }

        let name = dt.inner_text();
        let ty = determine_aria_type(ctx, &name);
        infos.push(AttrInfo { name, docs, ty, applies_to: Applicable::Globally });
    }
}

/// Collects all HTML elements.
fn collect_elements(ctx: &Context, attrs: &[AttrInfo]) -> Vec<ElementInfo> {
    let mut infos = vec![];
    for tr in ctx
        .html
        .select_first(s!("#elements-3 ~ table"))
        .select(s!("tbody > tr"))
    {
        for code in tr.select(s!("th code")) {
            let name = code.inner_text();

            // These are special and not normal HTML elements.
            if matches!(name.as_str(), "svg" | "math") {
                continue;
            }

            let docs = docs(&tr.select_text(s!("td:first-of-type")));
            let attributes = collect_attr_indices(tr, &name, attrs);

            infos.push(ElementInfo { name, docs, attributes });
        }
    }
    infos
}

/// Collects the indices of the attribute infos for an element's attributes.
fn collect_attr_indices(tr: ElementRef, tag: &str, attrs: &[AttrInfo]) -> Vec<usize> {
    let mut indices = vec![];
    for elem in tr.select(s!("td:nth-of-type(5) code")) {
        let name = elem.inner_text();

        // Ignore the event handle attributes on the body element that are
        // for some reason documented (unlike other event handle attributes).
        if tag == "body" && name.starts_with("on") {
            continue;
        }

        let index = attrs
            .iter()
            .position(|attr| {
                attr.name == name && attr.applies_to.applies_specifically_to(tag)
            })
            .unwrap_or_else(|| panic!("failed to find attribute {name} for {tag}"));

        indices.push(index)
    }
    indices.sort();
    assert!(indices.is_sorted_by_key(|&i| &attrs[i].name));
    indices
}

/// Determines the Rust type for an HTML attribute.
fn determine_type(
    ctx: &mut Context,
    attr: &str,
    cell: ElementRef,
    docs: &mut String,
) -> String {
    let textual_ty = cell.inner_text().trim().trim_end_matches("*").replace("\n", " ");
    if let Some(ty) = try_parse_alternation(ctx, &textual_ty) {
        return ty;
    }

    regex_match!(textual_ty.as_str(), {
        "autofill field name.*" => "Str".into(),
        "css declarations" => "Str".into(),
        "id" => "Str".into(),
        "regular expression.*" => "Str".into(),
        "serialized permissions policy" => "Str".into(),
        "text" => "Str".into(),
        "the source of an iframe srcdoc document" => "Str".into(),
        "valid (non-empty )?url.*" => "Str".into(),
        "valid bcp 47 language tag" => "Str".into(),
        "valid custom element name.*" => "Str".into(),
        "valid hash-name reference" => "Str".into(),
        "valid mime type string" => "Str".into(),
        "valid integer" => "i64".into(),
        "valid non-negative integer" => "u64".into(),
        "valid non-negative integer greater than zero" => "NonZeroU64".into(),
        "valid floating-point number" => "f64".into(),
        "valid float.* greater than zero, or \"any\"" => {
            format!("Or<PositiveF64, {}>", create_str_literal(ctx, "any"))
        },
        "css <color>" => "Color".into(),
        "valid date string with optional time" => "Datetime".into(),
        "valid month string.*valid duration string" => "Or<Datetime, Duration>".into(),
        "boolean attribute" => "NamedBool".into(),
        "valid bcp 47 language tag or the empty string" => "StrOptionEmpty<Str>".into(),
        ".*until-found.*" if attr == "hidden" => {
            format!("Or<NamedBool, {}>", create_str_literal(ctx, "until-found"))
        },
        ".*true.*empty string.*" if matches!(attr, "spellcheck" | "writingsuggestions") => "NamedBool".into(),
        "valid list of floating-point numbers" => {
            write!(docs, " Expects an array of floating point numbers.").unwrap();
            "TokenList<f64, ',', false>".into()
        },
        ".*space-separated tokens.*" if attr == "rel" => rel_type(ctx),
        ".*space-separated tokens.*" if attr == "sandbox" => {
            let variants = cell
                .select(s!("code"))
                .map(|elem| elem.inner_text());
            let ty = create_str_enum(ctx, variants);
            format!("TokenList<{ty}, ' '>")
        },
        ".*space-separated tokens.*consisting of one code point.*" => {
            write!(docs, " Expects a single-codepoint string or an array thereof.").unwrap();
            "TokenList<char, ','>".into()
        },
        ".*space-separated tokens.*consisting of sizes" => {
            write!(
                docs,
                " Expects an array of sizes. Each size is specified as an \
                  array of two integers (width and height).",
            ).unwrap();
            "TokenList<IconSize, ' ', false>".into()
        },
        ".*space-separated tokens.*" => "TokenList<Str, ' '>".into(),
        ".*comma-separated tokens.*" => "TokenList<Str, ','>".into(),
        "valid media query list" => "Str".into(),
        "ascii case-insensitive match for \"utf-8\"" => create_str_literal(ctx, "utf-8"),
        "varies" if matches!(attr, "min" | "max") => "InputBound".into(),
        "varies" if attr == "value" => "InputValue".into(),
        "comma-separated list of image candidate strings" => {
            write!(
                docs,
                " Expects an array of dictionaries with the keys \
                  `src` (string) and `width` (integer) or `density` (float).",
            ).unwrap();
            "TokenList<ImageCandidate, ',', false>".into()
        },
        "valid source size list" => {
            write!(
                docs,
                " Expects an array of dictionaries with the keys \
                  `condition` (string) and `size` (length).",
            ).unwrap();
            "TokenList<SourceSize, ',', false>".into()
        },
        "input type keyword" => {
            let variants = ctx
                .html
                .select(s!(
                    "table#attr-input-type-keywords > tbody > tr > td:first-child code"
                ))
                .map(|elem| elem.inner_text());
            create_str_enum(ctx, variants)
        },
        "referrer policy" => {
            let variants = ctx
                .referrer
                .select_first(s!("h2#referrer-policies ~ p"))
                .select(s!("code"))
                .map(|elem| elem.inner_text());
            let ty = create_str_enum(ctx, variants);
            format!("StrOptionEmpty<{ty}>")
        },
        "potential destination.*" => {
            let variants = ctx
                .fetch
                .select_first(s!("p:has(#destination-type)"))
                .select(s!("code"))
                .map(|elem| elem.inner_text());
            create_str_enum(ctx, variants)
        },
        "valid navigable target name or keyword" => {
            let variants = ctx
                .html
                .select(s!("#valid-browsing-context-name-or-keyword code"))
                .map(|elem| elem.inner_text());
            format!("Or<{}, Str>", create_str_enum(ctx, variants))
        },
        _ => panic!("not handled: {textual_ty} for {attr}"),
    })
}

fn rel_type(ctx: &mut Context) -> String {
    let variants = ctx
        .html
        .select_first(s!("#table-link-relations"))
        .select(s!("tbody tr td:first-of-type code"))
        .map(|elem| elem.inner_text());
    let ty = create_str_enum(ctx, variants);
    format!("TokenList<{ty}, ' '>")
}

/// Tries to parse an attribute's textual as a semicolon-separate list of
/// strings.
fn try_parse_alternation(ctx: &mut Context, textual_ty: &str) -> Option<String> {
    let mut fallback = false;

    let mut variants = vec![];
    for piece in textual_ty.split(";") {
        let piece = piece.trim();
        if piece.starts_with('"') && piece.ends_with('"') {
            variants.push(piece[1..piece.len() - 1].to_owned());
            continue;
        }
        match piece {
            "a custom command keyword" => fallback = true,
            s if s.starts_with("a valid MIME type string") => fallback = true,
            _ if !piece.is_empty() => return None,
            _ => {}
        }
    }

    let mut ty = create_str_enum(ctx, variants);
    if fallback {
        ty = format!("Or<{ty}, Str>");
    }

    Some(ty)
}

/// Determines the Rust type for an ARIA attribute.
fn determine_aria_type(ctx: &mut Context, attr: &str) -> String {
    let table_sel = format!("h4[id^=\"{attr}\"] ~ table[class$=\"-features\"]");
    let ty_cell = ctx
        .aria
        .select_first(&Selector::parse(&table_sel).unwrap())
        .select_first(s!("td[class$=\"-value\"]"));

    match ty_cell.inner_text().as_str() {
        "ID reference" => "Str".into(),
        "ID reference list" => "TokenList<Str, ' '>".into(),
        "integer" => "i64".into(),
        "number" => "f64".into(),
        "string" => "Str".into(),
        "token" => determine_aria_values(ctx, attr),
        "token list" => {
            let ty = determine_aria_values(ctx, attr);
            format!("TokenList<{ty}, ' '>")
        }
        "tristate" => {
            format!(
                "Or<StrBool, {}>",
                create_str_literal(
                    ctx,
                    (
                        "mixed".into(),
                        "An intermediate value between true and false.".into()
                    )
                )
            )
        }
        "true/false" => "StrBool".into(),
        "true/false/undefined" => "StrOptionUndefined<StrBool>".into(),
        text => panic!("aria not handled: {text} for {attr}"),
    }
}

/// Determines the Rust type for an ARIA string enumeration.
fn determine_aria_values(ctx: &mut Context, attr: &str) -> String {
    let table_sel = format!("h4[id^=\"{attr}\"] ~ table.value-descriptions");
    let variants = ctx
        .aria
        .select_first(&Selector::parse(&table_sel).unwrap())
        .select(s!("tbody tr"))
        .map(|tr| {
            (
                tr.select_text(s!(".value-name"))
                    .trim_end_matches(" (default)")
                    .to_owned(),
                tr.select_text(s!(".value-description")),
            )
        });
    create_str_enum(ctx, variants)
}

/// Allocate a string literal type in the output.
fn create_str_literal(ctx: &mut Context, variant: impl EnumVariant) -> String {
    create_str_enum(ctx, vec![variant])
}

/// Allocates a string enum type in the output.
fn create_str_enum<V: EnumVariant>(
    ctx: &mut Context,
    variants: impl IntoIterator<Item = V>,
) -> String {
    let mut variants: Vec<_> = variants.into_iter().map(V::with_docs).collect();
    let mut extract = |list: &[&str]| {
        let has = list.iter().all(|item| variants.iter().any(|(v, _)| v == item));
        if has {
            variants.retain(|(v, _)| !list.contains(&v.as_str()));
        }
        has
    };

    let mut ty = "()".to_owned();

    if extract(&["true", "false"]) {
        ty = format!("Or<StrBool, {ty}>");
    } else if extract(&["yes", "no"]) {
        ty = format!("Or<YesNoBool, {ty}>");
    } else if extract(&["on", "off"]) {
        ty = format!("Or<OnOffBool, {ty}>");
    }

    if extract(&["ltr", "rtl"]) {
        ty = format!("Or<HorizontalDir, {ty}>");
    }

    if extract(&["none"]) {
        ty = format!("StrOptionNone<{ty}>");
    }

    if extract(&["auto"]) {
        ty = format!("Smart<{ty}>");
    }

    if variants.is_empty() {
        return re!("Or<(\\w+), \\(\\)>")
            .replace(&ty, |m: &regex::Captures| m[1].to_owned())
            .into_owned();
    }

    let len = variants.len();
    let start = (0..ctx.strings.len())
        .find(|&i| ctx.strings.get(i..i + len) == Some(variants.as_slice()))
        .unwrap_or_else(|| {
            let i = ctx.strings.len();
            ctx.strings.extend(variants);
            i
        });

    let end = start + len;
    ty.replace("()", &format!("StrEnum<{start}, {end}>"))
}

/// A variant in a stringy enum.
trait EnumVariant {
    fn with_docs(self) -> (String, String);
}

impl EnumVariant for &str {
    fn with_docs(self) -> (String, String) {
        (self.into(), String::new())
    }
}

impl EnumVariant for String {
    fn with_docs(self) -> (String, String) {
        (self, String::new())
    }
}

impl EnumVariant for (String, String) {
    fn with_docs(self) -> (String, String) {
        self
    }
}

/// Generates the output file.
fn codegen(output: &Output) -> String {
    let tags = output.tags.iter().map(|tag| {
        format!("    pub const {}: HtmlTag = HtmlTag::constant({tag:?});", ident(tag))
    });

    let attrs = output.attrs.iter().map(|attr| {
        format!("    pub const {}: HtmlAttr = HtmlAttr::constant({attr:?});", ident(attr))
    });

    let element_infos = output.element_infos.iter().map(|info| {
        let ElementInfo { name, docs, attributes } = info;
        format!(
            "    ElementInfo::new({name:?}, {docs:?}, &[{}]),",
            attributes
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    });

    let attr_infos = output.attr_infos.iter().map(|info| {
        let AttrInfo { name, docs, ty, .. } = info;
        format!("    AttrInfo::new::<{ty}>({name:?}, {docs:?}),")
    });

    let attr_strings = output.attr_strings.iter().map(|pair| format!("    {pair:?},"));

    let mut out = String::new();
    macro_rules! w {
        ($($tts:tt)*) => {
            writeln!(out,  $($tts)*).unwrap();
        }
    }

    w!("// This file is generated by `{}`.", file!());
    w!("// Do not edit by hand.");
    w!();
    w!("#![cfg_attr(rustfmt, rustfmt_skip)]");
    w!();
    w!("use std::num::NonZeroU64;");
    w!();
    w!("use crate::foundations::{{Datetime, Duration, PositiveF64, Smart, Str}};");
    w!("use crate::html::typed::*;");
    w!("use crate::visualize::Color;");
    w!();
    w!("#[allow(non_upper_case_globals)]");
    w!("pub mod tag {{");
    w!("    use crate::html::HtmlTag;");
    w!("{}", tags.join("\n"));
    w!("}}");
    w!();
    w!("#[allow(non_upper_case_globals)]");
    w!("pub mod attr {{");
    w!("    use crate::html::HtmlAttr;");
    w!("{}", attrs.join("\n"));
    w!("}}");
    w!();
    w!("pub const ELEMENTS: &[ElementInfo] = &[");
    w!("{}", element_infos.join("\n"));
    w!("];");
    w!();
    w!("pub const ATTRS: &[AttrInfo] = &[");
    w!("{}", attr_infos.join("\n"));
    w!("];");
    w!();
    w!("pub const ATTRS_GLOBAL: usize = {};", output.attr_global_count);
    w!();
    w!("pub const ATTR_STRINGS: &[(&str, &str)] = &[");
    w!("{}", attr_strings.join("\n"));
    w!("];");

    out
}

/// Postprocesses documentation.
fn docs(text: &str) -> String {
    text.replace("\n", " ")
        .replace_regex(re!("\\[[A-Z]+\\]"), "")
        .replace_regex(re!("\\s+"), " ")
        .trim()
        .trim_end_matches('.')
        .to_owned()
        + "."
}

/// Turns a tag or attribute name into a valid Rust identifier.
fn ident(name: &str) -> String {
    let string = name.replace("-", "_");
    if matches!(string.as_str(), "as" | "async" | "for" | "loop" | "type") {
        format!("r#{string}")
    } else {
        string
    }
}

/// Helpers methods on [`ElementRef`].
trait ElementRefExt<'a> {
    fn inner_text(&self) -> String;
    fn select_text(&self, selector: &Selector) -> String;
    fn select_first(&self, selector: &Selector) -> ElementRef<'a>;
}

impl<'a> ElementRefExt<'a> for ElementRef<'a> {
    fn inner_text(&self) -> String {
        self.text().collect()
    }

    fn select_text(&self, selector: &Selector) -> String {
        self.select(selector).flat_map(|elem| elem.text()).collect()
    }

    #[track_caller]
    fn select_first(&self, selector: &Selector) -> ElementRef<'a> {
        self.select(selector).next().expect("found no matching element")
    }
}

trait Join {
    fn join(self, separator: &str) -> String;
}

impl<I, T> Join for I
where
    I: Iterator<Item = T>,
    T: Display,
{
    fn join(self, separator: &str) -> String {
        self.map(|v| v.to_string()).collect::<Vec<_>>().join(separator)
    }
}

trait StrExt {
    fn replace_regex(&self, re: &Regex, replacement: &str) -> Cow<str>;
}

impl StrExt for str {
    fn replace_regex(&self, re: &Regex, replacement: &str) -> Cow<str> {
        re.replace_all(self, replacement)
    }
}
