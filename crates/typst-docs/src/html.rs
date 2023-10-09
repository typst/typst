use std::fmt::{self, Debug, Formatter};
use std::ops::Range;

use comemo::Prehashed;
use ecow::EcoString;
use heck::{ToKebabCase, ToTitleCase};
use pulldown_cmark as md;
use serde::{Deserialize, Serialize};
use typed_arena::Arena;
use typst::diag::{FileResult, StrResult};
use typst::eval::{Bytes, Datetime, Library, Tracer};
use typst::font::{Font, FontBook};
use typst::geom::{Abs, Point, Size};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::World;
use unscanny::Scanner;
use yaml_front_matter::YamlFrontMatter;

use super::{contributors, OutlineItem, Resolver, FILE_DIR, FONTS, LIBRARY};

/// HTML documentation.
#[derive(Serialize)]
#[serde(transparent)]
pub struct Html {
    raw: String,
    #[serde(skip)]
    md: String,
    #[serde(skip)]
    description: Option<EcoString>,
    #[serde(skip)]
    outline: Vec<OutlineItem>,
}

impl Html {
    /// Create HTML from a raw string.
    pub fn new(raw: String) -> Self {
        Self {
            md: String::new(),
            raw,
            description: None,
            outline: vec![],
        }
    }

    /// Convert markdown to HTML.
    #[track_caller]
    pub fn markdown(resolver: &dyn Resolver, md: &str, nesting: Option<usize>) -> Self {
        let mut text = md;
        let mut description = None;
        let document = YamlFrontMatter::parse::<Metadata>(md);
        if let Ok(document) = &document {
            text = &document.content;
            description = Some(document.metadata.description.clone())
        }

        let options = md::Options::ENABLE_TABLES | md::Options::ENABLE_HEADING_ATTRIBUTES;

        let ids = Arena::new();
        let mut handler = Handler::new(text, resolver, nesting, &ids);
        let mut events = md::Parser::new_ext(text, options).peekable();
        let iter = std::iter::from_fn(|| loop {
            let mut event = events.next()?;
            handler.peeked = events.peek().and_then(|event| match event {
                md::Event::Text(text) => Some(text.clone()),
                _ => None,
            });
            if handler.handle(&mut event) {
                return Some(event);
            }
        });

        let mut raw = String::new();
        md::html::push_html(&mut raw, iter);
        raw.truncate(raw.trim_end().len());

        Html {
            md: text.into(),
            raw,
            description,
            outline: handler.outline,
        }
    }

    /// The raw HTML.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The original Markdown, if any.
    pub fn md(&self) -> &str {
        &self.md
    }

    /// The title of the HTML.
    ///
    /// Returns `None` if the HTML doesn't start with an `h1` tag.
    pub fn title(&self) -> Option<&str> {
        let mut s = Scanner::new(&self.raw);
        s.eat_if("<h1").then(|| {
            s.eat_until('>');
            s.eat_if('>');
            s.eat_until("</h1>")
        })
    }

    /// The outline of the HTML.
    pub fn outline(&self) -> Vec<OutlineItem> {
        self.outline.clone()
    }

    /// The description from the front matter.
    pub fn description(&self) -> Option<EcoString> {
        self.description.clone()
    }
}

impl Debug for Html {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Html({:?})", self.title().unwrap_or(".."))
    }
}

/// Front matter metadata.
#[derive(Deserialize)]
struct Metadata {
    description: EcoString,
}

struct Handler<'a> {
    text: &'a str,
    resolver: &'a dyn Resolver,
    peeked: Option<md::CowStr<'a>>,
    lang: Option<EcoString>,
    code: EcoString,
    outline: Vec<OutlineItem>,
    nesting: Option<usize>,
    ids: &'a Arena<String>,
}

impl<'a> Handler<'a> {
    fn new(
        text: &'a str,
        resolver: &'a dyn Resolver,
        nesting: Option<usize>,
        ids: &'a Arena<String>,
    ) -> Self {
        Self {
            text,
            resolver,
            peeked: None,
            lang: None,
            code: EcoString::new(),
            outline: vec![],
            nesting,
            ids,
        }
    }

    fn handle(&mut self, event: &mut md::Event<'a>) -> bool {
        match event {
            // Rewrite Markdown images.
            md::Event::Start(md::Tag::Image(_, path, _)) => {
                *path = self.handle_image(path).into();
            }

            // Rewrite HTML images.
            md::Event::Html(html) if html.starts_with("<img") => {
                let range = html_attr_range(html, "src").unwrap();
                let path = &html[range.clone()];
                let mut buf = html.to_string();
                buf.replace_range(range, &self.handle_image(path));
                *html = buf.into();
            }

            // Register HTML headings for the outline.
            md::Event::Start(md::Tag::Heading(level, id, _)) => {
                self.handle_heading(id, level);
            }

            // Also handle heading closings.
            md::Event::End(md::Tag::Heading(level, _, _)) => {
                nest_heading(level, self.nesting());
            }

            // Rewrite contributor sections.
            md::Event::Html(html) if html.starts_with("<contributors") => {
                let from = html_attr(html, "from").unwrap();
                let to = html_attr(html, "to").unwrap();
                let Some(output) = contributors(self.resolver, from, to) else {
                    return false;
                };
                *html = output.raw.into();
            }

            // Rewrite links.
            md::Event::Start(md::Tag::Link(ty, dest, _)) => {
                assert!(
                    matches!(ty, md::LinkType::Inline | md::LinkType::Reference),
                    "unsupported link type: {ty:?}",
                );

                *dest = match self.handle_link(dest) {
                    Ok(link) => link.into(),
                    Err(err) => panic!("invalid link: {dest} ({err})"),
                };
            }

            // Inline raw.
            md::Event::Code(code) => {
                let mut chars = code.chars();
                let parser = match (chars.next(), chars.next_back()) {
                    (Some('['), Some(']')) => typst::syntax::parse,
                    (Some('{'), Some('}')) => typst::syntax::parse_code,
                    _ => return true,
                };

                let root = parser(&code[1..code.len() - 1]);
                let html = typst::syntax::highlight_html(&root);
                *event = md::Event::Html(html.into());
            }

            // Code blocks.
            md::Event::Start(md::Tag::CodeBlock(md::CodeBlockKind::Fenced(lang))) => {
                self.lang = Some(lang.as_ref().into());
                self.code = EcoString::new();
                return false;
            }
            md::Event::End(md::Tag::CodeBlock(md::CodeBlockKind::Fenced(_))) => {
                let Some(lang) = self.lang.take() else { return false };
                let html = code_block(self.resolver, &lang, &self.code);
                *event = md::Event::Html(html.raw.into());
            }

            // Example with preview.
            md::Event::Text(text) => {
                if self.lang.is_some() {
                    self.code.push_str(text);
                    return false;
                }
            }

            _ => {}
        }

        true
    }

    fn handle_image(&self, link: &str) -> String {
        if let Some(file) = FILE_DIR.get_file(link) {
            self.resolver.image(link, file.contents())
        } else if let Some(url) = self.resolver.link(link) {
            url
        } else {
            panic!("missing image: {link}")
        }
    }

    fn handle_heading(
        &mut self,
        id_slot: &mut Option<&'a str>,
        level: &mut md::HeadingLevel,
    ) {
        nest_heading(level, self.nesting());
        if *level == md::HeadingLevel::H1 {
            return;
        }

        let default = self.peeked.as_ref().map(|text| text.to_kebab_case());
        let id: &'a str = match (&id_slot, default) {
            (Some(id), default) => {
                if Some(*id) == default.as_deref() {
                    eprintln!("heading id #{id} was specified unnecessarily");
                }
                id
            }
            (None, Some(default)) => self.ids.alloc(default).as_str(),
            (None, None) => panic!("missing heading id {}", self.text),
        };

        *id_slot = (!id.is_empty()).then_some(id);

        // Special case for things like "v0.3.0".
        let name = if id.starts_with('v') && id.contains('.') {
            id.into()
        } else {
            id.to_title_case().into()
        };

        let mut children = &mut self.outline;
        let mut depth = *level as usize;
        while depth > 2 {
            if !children.is_empty() {
                children = &mut children.last_mut().unwrap().children;
            }
            depth -= 1;
        }

        children.push(OutlineItem { id: id.into(), name, children: vec![] });
    }

    fn handle_link(&self, link: &str) -> StrResult<String> {
        if let Some(link) = self.resolver.link(link) {
            return Ok(link);
        }

        crate::link::resolve(link)
    }

    fn nesting(&self) -> usize {
        match self.nesting {
            Some(nesting) => nesting,
            None => panic!("headings are not allowed here:\n{}", self.text),
        }
    }
}

/// Render a code block to HTML.
fn code_block(resolver: &dyn Resolver, lang: &str, text: &str) -> Html {
    let mut display = String::new();
    let mut compile = String::new();
    for line in text.lines() {
        if let Some(suffix) = line.strip_prefix(">>>") {
            compile.push_str(suffix);
            compile.push('\n');
        } else if let Some(suffix) = line.strip_prefix("<<< ") {
            display.push_str(suffix);
            display.push('\n');
        } else {
            display.push_str(line);
            display.push('\n');
            compile.push_str(line);
            compile.push('\n');
        }
    }

    let mut parts = lang.split(':');
    let lang = parts.next().unwrap_or(lang);

    let mut zoom: Option<[Abs; 4]> = None;
    let mut single = false;
    if let Some(args) = parts.next() {
        single = true;
        if !args.contains("single") {
            zoom = args
                .split(',')
                .take(4)
                .map(|s| Abs::pt(s.parse().unwrap()))
                .collect::<Vec<_>>()
                .try_into()
                .ok();
        }
    }

    if lang.is_empty() {
        let mut buf = String::from("<pre>");
        md::escape::escape_html(&mut buf, &display).unwrap();
        buf.push_str("</pre>");
        return Html::new(buf);
    } else if !matches!(lang, "example" | "typ") {
        let set = &*typst_library::text::SYNTAXES;
        let buf = syntect::html::highlighted_html_for_string(
            &display,
            set,
            set.find_syntax_by_token(lang)
                .unwrap_or_else(|| panic!("unsupported highlighting language: {lang}")),
            &typst_library::text::THEME,
        )
        .expect("failed to highlight code");
        return Html::new(buf);
    }

    let root = typst::syntax::parse(&display);
    let highlighted = Html::new(typst::syntax::highlight_html(&root));
    if lang == "typ" {
        return Html::new(format!("<pre>{}</pre>", highlighted.as_str()));
    }

    let id = FileId::new(None, VirtualPath::new("main.typ"));
    let source = Source::new(id, compile);
    let world = DocWorld(source);

    let mut tracer = Tracer::new();
    let mut frames = match typst::compile(&world, &mut tracer) {
        Ok(doc) => doc.pages,
        Err(err) => {
            let msg = &err[0].message;
            panic!("while trying to compile:\n{text}:\n\nerror: {msg}");
        }
    };

    if let Some([x, y, w, h]) = zoom {
        frames[0].translate(Point::new(-x, -y));
        *frames[0].size_mut() = Size::new(w, h);
    }

    if single {
        frames.truncate(1);
    }

    let hash = typst::util::hash128(text);
    resolver.example(hash, highlighted, &frames)
}

/// Extract an attribute value from an HTML element.
fn html_attr<'a>(html: &'a str, attr: &str) -> Option<&'a str> {
    html.get(html_attr_range(html, attr)?)
}

/// Extract the range of the attribute value of an HTML element.
fn html_attr_range(html: &str, attr: &str) -> Option<Range<usize>> {
    let needle = format!("{attr}=\"");
    let offset = html.find(&needle)? + needle.len();
    let len = html[offset..].find('"')?;
    Some(offset..offset + len)
}

/// Increase the nesting level of a Markdown heading.
fn nest_heading(level: &mut md::HeadingLevel, nesting: usize) {
    *level = ((*level as usize) + nesting)
        .try_into()
        .unwrap_or(md::HeadingLevel::H6);
}

/// A world for example compilations.
struct DocWorld(Source);

impl World for DocWorld {
    fn library(&self) -> &Prehashed<Library> {
        &LIBRARY
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &FONTS.0
    }

    fn main(&self) -> Source {
        self.0.clone()
    }

    fn source(&self, _: FileId) -> FileResult<Source> {
        Ok(self.0.clone())
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        assert!(id.package().is_none());
        Ok(FILE_DIR
            .get_file(id.vpath().as_rootless_path())
            .unwrap_or_else(|| panic!("failed to load {:?}", id.vpath()))
            .contents()
            .into())
    }

    fn font(&self, index: usize) -> Option<Font> {
        Some(FONTS.1[index].clone())
    }

    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        Some(Datetime::from_ymd(1970, 1, 1).unwrap())
    }
}
