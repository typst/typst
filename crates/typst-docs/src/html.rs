use std::ops::Range;

use comemo::Prehashed;
use pulldown_cmark as md;
use typed_arena::Arena;
use typst::diag::FileResult;
use typst::eval::{Bytes, Datetime, Tracer};
use typst::font::{Font, FontBook};
use typst::geom::{Point, Size};
use typst::syntax::{FileId, Source};
use typst::World;
use yaml_front_matter::YamlFrontMatter;

use super::*;

/// HTML documentation.
#[derive(Serialize)]
#[serde(transparent)]
pub struct Html {
    raw: String,
    #[serde(skip)]
    md: String,
    #[serde(skip)]
    description: Option<String>,
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
    pub fn markdown(resolver: &dyn Resolver, md: &str) -> Self {
        Self::markdown_with_id_base(resolver, md, "")
    }

    /// Convert markdown to HTML, preceding all fragment identifiers with the
    /// `id_base`.
    #[track_caller]
    pub fn markdown_with_id_base(
        resolver: &dyn Resolver,
        md: &str,
        id_base: &str,
    ) -> Self {
        let mut text = md;
        let mut description = None;
        let document = YamlFrontMatter::parse::<Metadata>(md);
        if let Ok(document) = &document {
            text = &document.content;
            description = Some(document.metadata.description.clone())
        }

        let options = md::Options::ENABLE_TABLES | md::Options::ENABLE_HEADING_ATTRIBUTES;

        let ids = Arena::new();
        let mut handler = Handler::new(resolver, id_base.into(), &ids);
        let iter = md::Parser::new_ext(text, options)
            .filter_map(|mut event| handler.handle(&mut event).then_some(event));

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
        s.eat_if("<h1>").then(|| s.eat_until("</h1>"))
    }

    /// The outline of the HTML.
    pub fn outline(&self) -> Vec<OutlineItem> {
        self.outline.clone()
    }

    /// The description from the front matter.
    pub fn description(&self) -> Option<String> {
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
    description: String,
}

struct Handler<'a> {
    resolver: &'a dyn Resolver,
    lang: Option<String>,
    code: String,
    outline: Vec<OutlineItem>,
    id_base: String,
    ids: &'a Arena<String>,
}

impl<'a> Handler<'a> {
    fn new(resolver: &'a dyn Resolver, id_base: String, ids: &'a Arena<String>) -> Self {
        Self {
            resolver,
            lang: None,
            code: String::new(),
            outline: vec![],
            id_base,
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
            md::Event::Start(md::Tag::Heading(level, Some(id), _)) => {
                self.handle_heading(id, level);
            }

            // Also handle heading closings.
            md::Event::End(md::Tag::Heading(level, Some(_), _)) => {
                if *level > md::HeadingLevel::H1 && !self.id_base.is_empty() {
                    nest_heading(level);
                }
            }

            // Rewrite contributor sections.
            md::Event::Html(html) if html.starts_with("<contributors") => {
                let from = html_attr(html, "from").unwrap();
                let to = html_attr(html, "to").unwrap();
                let Some(output) = contributors(self.resolver, from, to) else { return false };
                *html = output.raw.into();
            }

            // Rewrite links.
            md::Event::Start(md::Tag::Link(ty, dest, _)) => {
                assert!(
                    matches!(ty, md::LinkType::Inline | md::LinkType::Reference),
                    "unsupported link type: {ty:?}",
                );

                *dest = self
                    .handle_link(dest)
                    .unwrap_or_else(|| panic!("invalid link: {dest}"))
                    .into();
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
                let html = typst::ide::highlight_html(&root);
                *event = md::Event::Html(html.into());
            }

            // Code blocks.
            md::Event::Start(md::Tag::CodeBlock(md::CodeBlockKind::Fenced(lang))) => {
                self.lang = Some(lang.as_ref().into());
                self.code = String::new();
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
        if let Some(file) = FILES.get_file(link) {
            self.resolver.image(link, file.contents())
        } else if let Some(url) = self.resolver.link(link) {
            url
        } else {
            panic!("missing image: {link}")
        }
    }

    fn handle_heading(&mut self, id: &mut &'a str, level: &mut md::HeadingLevel) {
        if *level == md::HeadingLevel::H1 {
            return;
        }

        // Special case for things like "v0.3.0".
        let name = if id.starts_with('v') && id.contains('.') {
            id.to_string()
        } else {
            id.to_title_case()
        };

        let mut children = &mut self.outline;
        let mut depth = *level as usize;
        while depth > 2 {
            if !children.is_empty() {
                children = &mut children.last_mut().unwrap().children;
            }
            depth -= 1;
        }

        // Put base before id.
        if !self.id_base.is_empty() {
            nest_heading(level);
            *id = self.ids.alloc(format!("{}-{id}", self.id_base)).as_str();
        }

        children.push(OutlineItem { id: id.to_string(), name, children: vec![] });
    }

    fn handle_link(&self, link: &str) -> Option<String> {
        if link.starts_with('#') || link.starts_with("http") {
            return Some(link.into());
        }

        if !link.starts_with('$') {
            return self.resolver.link(link);
        }

        let root = link.split('/').next()?;
        let rest = &link[root.len()..].trim_matches('/');
        let base = match root {
            "$tutorial" => "/docs/tutorial/",
            "$reference" => "/docs/reference/",
            "$category" => "/docs/reference/",
            "$syntax" => "/docs/reference/syntax/",
            "$styling" => "/docs/reference/styling/",
            "$scripting" => "/docs/reference/scripting/",
            "$types" => "/docs/reference/types/",
            "$type" => "/docs/reference/types/",
            "$func" => "/docs/reference/",
            "$guides" => "/docs/guides/",
            "$packages" => "/docs/packages/",
            "$changelog" => "/docs/changelog/",
            "$community" => "/docs/community/",
            _ => panic!("unknown link root: {root}"),
        };

        let mut route = base.to_string();
        if root == "$type" && rest.contains('.') {
            let mut parts = rest.split('.');
            let ty = parts.next()?;
            let method = parts.next()?;
            route.push_str(ty);
            route.push_str("/#methods-");
            route.push_str(method);
        } else if root == "$func" {
            let mut parts = rest.split('.').peekable();
            let first = parts.peek().copied();
            let mut focus = &LIBRARY.global;
            while let Some(m) = first.and_then(|name| module(focus, name).ok()) {
                focus = m;
                parts.next();
            }

            let name = parts.next()?;

            let value = focus.get(name).ok()?;
            let Value::Func(func) = value else { return None };
            let info = func.info()?;
            route.push_str(info.category);
            route.push('/');

            if let Some(group) = GROUPS
                .iter()
                .filter(|_| first == Some("math"))
                .find(|group| group.functions.iter().any(|func| func == info.name))
            {
                route.push_str(&group.name);
                route.push_str("/#");
                route.push_str(info.name);
                if let Some(param) = parts.next() {
                    route.push_str("-parameters-");
                    route.push_str(param);
                }
            } else {
                route.push_str(name);
                route.push('/');
                if let Some(next) = parts.next() {
                    if info.params.iter().any(|param| param.name == next) {
                        route.push_str("#parameters-");
                        route.push_str(next);
                    } else if info.scope.iter().any(|(name, _)| name == next) {
                        route.push('#');
                        route.push_str(info.name);
                        route.push('-');
                        route.push_str(next);
                    } else {
                        return None;
                    }
                }
            }
        } else {
            route.push_str(rest);
        }

        if !route.contains('#') && !route.ends_with('/') {
            route.push('/');
        }

        Some(route)
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
    let highlighted = Html::new(typst::ide::highlight_html(&root));
    if lang == "typ" {
        return Html::new(format!("<pre>{}</pre>", highlighted.as_str()));
    }

    let id = FileId::new(None, Path::new("/main.typ"));
    let source = Source::new(id, compile);
    let world = DocWorld(source);
    let mut tracer = Tracer::default();

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
fn nest_heading(level: &mut md::HeadingLevel) {
    *level = match &level {
        md::HeadingLevel::H1 => md::HeadingLevel::H2,
        md::HeadingLevel::H2 => md::HeadingLevel::H3,
        md::HeadingLevel::H3 => md::HeadingLevel::H4,
        md::HeadingLevel::H4 => md::HeadingLevel::H5,
        md::HeadingLevel::H5 => md::HeadingLevel::H6,
        v => **v,
    };
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
        Ok(FILES
            .get_file(id.path().strip_prefix("/").unwrap())
            .unwrap_or_else(|| panic!("failed to load {:?}", id.path().display()))
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
