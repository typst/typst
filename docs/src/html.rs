use comemo::Prehashed;
use md::escape::escape_html;
use pulldown_cmark as md;
use typst::diag::FileResult;
use typst::font::{Font, FontBook};
use typst::geom::{Point, Size};
use typst::syntax::{Source, SourceId};
use typst::util::Buffer;
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
}

impl Html {
    /// Create HTML from a raw string.
    pub fn new(raw: String) -> Self {
        Self { md: String::new(), raw, description: None }
    }

    /// Convert markdown to HTML.
    #[track_caller]
    pub fn markdown(resolver: &dyn Resolver, md: &str) -> Self {
        let mut text = md;
        let mut description = None;
        let document = YamlFrontMatter::parse::<Metadata>(&md);
        if let Ok(document) = &document {
            text = &document.content;
            description = Some(document.metadata.description.clone())
        }

        let options = md::Options::ENABLE_TABLES | md::Options::ENABLE_HEADING_ATTRIBUTES;

        let mut handler = Handler::new(resolver);
        let iter = md::Parser::new_ext(text, options)
            .filter_map(|mut event| handler.handle(&mut event).then(|| event));

        let mut raw = String::new();
        md::html::push_html(&mut raw, iter);
        raw.truncate(raw.trim_end().len());

        Html { md: text.into(), raw, description }
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
}

impl<'a> Handler<'a> {
    fn new(resolver: &'a dyn Resolver) -> Self {
        Self { resolver, lang: None }
    }

    fn handle(&mut self, event: &mut md::Event) -> bool {
        let lang = self.lang.take();
        match event {
            // Rewrite Markdown images.
            md::Event::Start(md::Tag::Image(_, path, _)) => {
                *path = self.handle_image(path).into();
            }

            // Rewrite HTML images.
            md::Event::Html(html) if html.starts_with("<img") => {
                let needle = "src=\"";
                let offset = html.find(needle).unwrap() + needle.len();
                let len = html[offset..].find('"').unwrap();
                let range = offset..offset + len;
                let path = &html[range.clone()];
                let mut buf = html.to_string();
                buf.replace_range(range, &self.handle_image(path));
                *html = buf.into();
            }

            // Rewrite links.
            md::Event::Start(md::Tag::Link(ty, dest, _)) => {
                assert!(
                    matches!(ty, md::LinkType::Inline | md::LinkType::Reference),
                    "unsupported link type: {ty:?}",
                );

                let mut link = self
                    .handle_link(dest)
                    .unwrap_or_else(|| panic!("invalid link: {dest}"));

                if !link.contains('#') && !link.ends_with('/') {
                    link.push('/');
                }

                *dest = link.into();
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
                return false;
            }
            md::Event::End(md::Tag::CodeBlock(md::CodeBlockKind::Fenced(_))) => {
                return false;
            }

            // Example with preview.
            md::Event::Text(text) => {
                let Some(lang) = lang.as_deref() else { return true };
                let html = code_block(self.resolver, lang, text);
                *event = md::Event::Html(html.raw.into());
            }

            _ => {}
        }

        true
    }

    fn handle_image(&self, link: &str) -> String {
        if let Some(file) = IMAGES.get_file(link) {
            self.resolver.image(&link, file.contents()).into()
        } else if let Some(url) = self.resolver.link(link) {
            url
        } else {
            panic!("missing image: {link}")
        }
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
            route.push_str("/#methods--");
            route.push_str(method);
        } else if root == "$func" {
            let mut parts = rest.split('.');
            let name = parts.next()?;
            let param = parts.next();
            let value =
                LIBRARY.global.get(name).or_else(|_| LIBRARY.math.get(name)).ok()?;
            let Value::Func(func) = value else { return None };
            let info = func.info()?;
            route.push_str(info.category);
            route.push('/');
            route.push_str(name);
            route.push('/');
            if let Some(param) = param {
                route.push_str("#parameters--");
                route.push_str(param);
            }
        } else {
            route.push_str(rest);
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
    if let Some(args) = parts.next() {
        zoom = args
            .split(',')
            .take(4)
            .map(|s| Abs::pt(s.parse().unwrap()))
            .collect::<Vec<_>>()
            .try_into()
            .ok();
    }

    if !matches!(lang, "example" | "typ") {
        let mut buf = String::from("<pre>");
        escape_html(&mut buf, &display).unwrap();
        buf.push_str("</pre>");
        return Html::new(buf);
    }

    let root = typst::syntax::parse(&display);
    let highlighted = Html::new(typst::ide::highlight_html(&root));
    if lang == "typ" {
        return Html::new(format!("<pre>{}</pre>", highlighted.as_str()));
    }

    let source = Source::new(SourceId::from_u16(0), Path::new("main.typ"), compile);
    let world = DocWorld(source);
    let mut frames = match typst::compile(&world, &world.0) {
        Ok(doc) => doc.pages,
        Err(err) => panic!("failed to compile {text}: {err:?}"),
    };

    if let Some([x, y, w, h]) = zoom {
        frames[0].translate(Point::new(-x, -y));
        *frames[0].size_mut() = Size::new(w, h);
    }

    resolver.example(highlighted, &frames)
}

/// World for example compilations.
struct DocWorld(Source);

impl World for DocWorld {
    fn library(&self) -> &Prehashed<Library> {
        &LIBRARY
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &FONTS.0
    }

    fn font(&self, id: usize) -> Option<Font> {
        Some(FONTS.1[id].clone())
    }

    fn file(&self, path: &Path) -> FileResult<Buffer> {
        Ok(FILES
            .get_file(path)
            .unwrap_or_else(|| panic!("failed to load {path:?}"))
            .contents()
            .into())
    }

    fn resolve(&self, _: &Path) -> FileResult<SourceId> {
        unimplemented!()
    }

    fn source(&self, id: SourceId) -> &Source {
        assert_eq!(id.into_u16(), 0, "invalid source id");
        &self.0
    }
}
