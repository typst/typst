use std::hash::Hash;
use std::ops::Range;
use std::sync::Arc;

use ecow::{eco_format, EcoString, EcoVec};
use once_cell::sync::Lazy;
use once_cell::unsync::Lazy as UnsyncLazy;
use syntect::highlighting as synt;
use syntect::parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{At, FileError, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, scope, Args, Array, Bytes, Content, Fold, NativeElement, Packed,
    PlainText, Show, ShowSet, Smart, StyleChain, Styles, Synthesize, Value,
};
use crate::layout::{BlockElem, Em, HAlignment};
use crate::model::{Figurable, ParElem};
use crate::syntax::{split_newlines, LinkedNode, Span, Spanned};
use crate::text::{
    FontFamily, FontList, Hyphenate, Lang, LinebreakElem, LocalName, Region,
    SmartQuoteElem, TextElem, TextSize,
};
use crate::util::option_eq;
use crate::visualize::Color;
use crate::{syntax, World};

// Shorthand for highlighter closures.
type StyleFn<'a> =
    &'a mut dyn FnMut(usize, &LinkedNode, Range<usize>, synt::Style) -> Content;
type LineFn<'a> = &'a mut dyn FnMut(usize, Range<usize>, &mut Vec<Content>);

/// Raw text with optional syntax highlighting.
///
/// Displays the text verbatim and in a monospace font. This is typically used
/// to embed computer code into your document.
///
/// # Example
/// ````example
/// Adding `rbx` to `rcx` gives
/// the desired result.
///
/// What is ```rust fn main()``` in Rust
/// would be ```c int main()``` in C.
///
/// ```rust
/// fn main() {
///     println!("Hello World!");
/// }
/// ```
///
/// This has ``` `backticks` ``` in it
/// (but the spaces are trimmed). And
/// ``` here``` the leading space is
/// also trimmed.
/// ````
///
/// # Syntax
/// This function also has dedicated syntax. You can enclose text in 1 or 3+
/// backticks (`` ` ``) to make it raw. Two backticks produce empty raw text.
/// When you use three or more backticks, you can additionally specify a
/// language tag for syntax highlighting directly after the opening backticks.
/// Within raw blocks, everything (except for the language tag, if applicable)
/// is rendered as is, in particular, there are no escape sequences.
///
/// The language tag is an identifier that directly follows the opening
/// backticks only if there are three or more backticks. If your text starts
/// with something that looks like an identifier, but no syntax highlighting is
/// needed, start the text with a single space (which will be trimmed) or use
/// the single backtick syntax. If your text should start or end with a
/// backtick, put a space before or after it (it will be trimmed).
#[elem(
    scope,
    title = "Raw Text / Code",
    Synthesize,
    Show,
    ShowSet,
    LocalName,
    Figurable,
    PlainText
)]
pub struct RawElem {
    /// The raw text.
    ///
    /// You can also use raw blocks creatively to create custom syntaxes for
    /// your automations.
    ///
    /// ````example
    /// // Parse numbers in raw blocks with the
    /// // `mydsl` tag and sum them up.
    /// #show raw.where(lang: "mydsl"): it => {
    ///   let sum = 0
    ///   for part in it.text.split("+") {
    ///     sum += int(part.trim())
    ///   }
    ///   sum
    /// }
    ///
    /// ```mydsl
    /// 1 + 2 + 3 + 4 + 5
    /// ```
    /// ````
    #[required]
    pub text: RawContent,

    /// Whether the raw text is displayed as a separate block.
    ///
    /// In markup mode, using one-backtick notation makes this `{false}`.
    /// Using three-backtick notation makes it `{true}` if the enclosed content
    /// contains at least one line break.
    ///
    /// ````example
    /// // Display inline code in a small box
    /// // that retains the correct baseline.
    /// #show raw.where(block: false): box.with(
    ///   fill: luma(240),
    ///   inset: (x: 3pt, y: 0pt),
    ///   outset: (y: 3pt),
    ///   radius: 2pt,
    /// )
    ///
    /// // Display block code in a larger block
    /// // with more padding.
    /// #show raw.where(block: true): block.with(
    ///   fill: luma(240),
    ///   inset: 10pt,
    ///   radius: 4pt,
    /// )
    ///
    /// With `rg`, you can search through your files quickly.
    /// This example searches the current directory recursively
    /// for the text `Hello World`:
    ///
    /// ```bash
    /// rg "Hello World"
    /// ```
    /// ````
    #[default(false)]
    pub block: bool,

    /// The language to syntax-highlight in.
    ///
    /// Apart from typical language tags known from Markdown, this supports the
    /// `{"typ"}` and `{"typc"}` tags for
    /// [Typst markup]($reference/syntax/#markup) and
    /// [Typst code]($reference/syntax/#code), respectively.
    ///
    /// ````example
    /// ```typ
    /// This is *Typst!*
    /// ```
    ///
    /// This is ```typ also *Typst*```, but inline!
    /// ````
    #[borrowed]
    pub lang: Option<EcoString>,

    /// The horizontal alignment that each line in a raw block should have.
    /// This option is ignored if this is not a raw block (if specified
    /// `block: false` or single backticks were used in markup mode).
    ///
    /// By default, this is set to `{start}`, meaning that raw text is
    /// aligned towards the start of the text direction inside the block
    /// by default, regardless of the current context's alignment (allowing
    /// you to center the raw block itself without centering the text inside
    /// it, for example).
    ///
    /// ````example
    /// #set raw(align: center)
    ///
    /// ```typc
    /// let f(x) = x
    /// code = "centered"
    /// ```
    /// ````
    #[default(HAlignment::Start)]
    pub align: HAlignment,

    /// One or multiple additional syntax definitions to load. The syntax
    /// definitions should be in the
    /// [`sublime-syntax` file format](https://www.sublimetext.com/docs/syntax.html).
    ///
    /// ````example
    /// #set raw(syntaxes: "SExpressions.sublime-syntax")
    ///
    /// ```sexp
    /// (defun factorial (x)
    ///   (if (zerop x)
    ///     ; with a comment
    ///     1
    ///     (* x (factorial (- x 1)))))
    /// ```
    /// ````
    #[parse(
        let (syntaxes, syntaxes_data) = parse_syntaxes(engine, args)?;
        syntaxes
    )]
    #[fold]
    pub syntaxes: SyntaxPaths,

    /// The raw file buffers of syntax definition files.
    #[internal]
    #[parse(syntaxes_data)]
    #[fold]
    pub syntaxes_data: Vec<Bytes>,

    /// The theme to use for syntax highlighting. Theme files should be in the
    /// in the [`tmTheme` file format](https://www.sublimetext.com/docs/color_schemes_tmtheme.html).
    ///
    /// Applying a theme only affects the color of specifically highlighted
    /// text. It does not consider the theme's foreground and background
    /// properties, so that you retain control over the color of raw text. You
    /// can apply the foreground color yourself with the [`text`] function and
    /// the background with a [filled block]($block.fill). You could also use
    /// the [`xml`] function to extract these properties from the theme.
    ///
    /// ````example
    /// #set raw(theme: "halcyon.tmTheme")
    /// #show raw: it => block(
    ///   fill: rgb("#1d2433"),
    ///   inset: 8pt,
    ///   radius: 5pt,
    ///   text(fill: rgb("#a2aabc"), it)
    /// )
    ///
    /// ```typ
    /// = Chapter 1
    /// #let hi = "Hello World"
    /// ```
    /// ````
    #[parse(
        let (theme_path, theme_data) = parse_theme(engine, args)?;
        theme_path.map(Some)
    )]
    #[borrowed]
    pub theme: Option<EcoString>,

    /// The raw file buffer of syntax theme file.
    #[internal]
    #[parse(theme_data.map(Some))]
    #[borrowed]
    pub theme_data: Option<Bytes>,

    /// The size for a tab stop in spaces. A tab is replaced with enough spaces to
    /// align with the next multiple of the size.
    ///
    /// ````example
    /// #set raw(tab-size: 8)
    /// ```tsv
    /// Year	Month	Day
    /// 2000	2	3
    /// 2001	2	1
    /// 2002	3	10
    /// ```
    /// ````
    #[default(2)]
    pub tab_size: usize,

    /// The stylized lines of raw text.
    ///
    /// Made accessible for the [`raw.line` element]($raw.line).
    /// Allows more styling control in `show` rules.
    #[synthesized]
    pub lines: Vec<Packed<RawLine>>,
}

#[scope]
impl RawElem {
    #[elem]
    type RawLine;
}

impl RawElem {
    /// The supported language names and tags.
    pub fn languages() -> Vec<(&'static str, Vec<&'static str>)> {
        RAW_SYNTAXES
            .syntaxes()
            .iter()
            .map(|syntax| {
                (
                    syntax.name.as_str(),
                    syntax.file_extensions.iter().map(|s| s.as_str()).collect(),
                )
            })
            .chain([("Typst", vec!["typ"]), ("Typst (code)", vec!["typc"])])
            .collect()
    }
}

impl Synthesize for Packed<RawElem> {
    fn synthesize(&mut self, _: &mut Engine, styles: StyleChain) -> SourceResult<()> {
        let seq = self.highlight(styles);
        self.push_lines(seq);
        Ok(())
    }
}

impl Packed<RawElem> {
    #[comemo::memoize]
    fn highlight(&self, styles: StyleChain) -> Vec<Packed<RawLine>> {
        let elem = self.as_ref();
        let lines = preprocess(elem.text(), styles, self.span());

        let count = lines.len() as i64;
        let lang = elem
            .lang(styles)
            .as_ref()
            .as_ref()
            .map(|s| s.to_lowercase())
            .or(Some("txt".into()));

        let extra_syntaxes = UnsyncLazy::new(|| {
            load_syntaxes(&elem.syntaxes(styles), &elem.syntaxes_data(styles)).unwrap()
        });

        let theme = elem.theme(styles).as_ref().as_ref().map(|theme_path| {
            load_theme(theme_path, elem.theme_data(styles).as_ref().as_ref().unwrap())
                .unwrap()
        });

        let theme = theme.as_deref().unwrap_or(&RAW_THEME);
        let foreground = theme.settings.foreground.unwrap_or(synt::Color::BLACK);

        let mut seq = vec![];
        if matches!(lang.as_deref(), Some("typ" | "typst" | "typc")) {
            let text =
                lines.iter().map(|(s, _)| s.clone()).collect::<Vec<_>>().join("\n");
            let root = match lang.as_deref() {
                Some("typc") => syntax::parse_code(&text),
                _ => syntax::parse(&text),
            };

            ThemedHighlighter::new(
                &text,
                LinkedNode::new(&root),
                synt::Highlighter::new(theme),
                &mut |i, _, range, style| {
                    // Find span and start of line.
                    // Note: Dedent is already applied to the text
                    let span = lines.get(i).map_or_else(Span::detached, |l| l.1);
                    let span_offset = text[..range.start]
                        .rfind('\n')
                        .map_or(0, |i| range.start - (i + 1));
                    styled(&text[range], foreground, style, span, span_offset)
                },
                &mut |i, range, line| {
                    let span = lines.get(i).map_or_else(Span::detached, |l| l.1);
                    seq.push(
                        Packed::new(RawLine::new(
                            (i + 1) as i64,
                            count,
                            EcoString::from(&text[range]),
                            Content::sequence(line.drain(..)),
                        ))
                        .spanned(span),
                    );
                },
            )
            .highlight();
        } else if let Some((syntax_set, syntax)) = lang.and_then(|token| {
            RAW_SYNTAXES
                .find_syntax_by_token(&token)
                .map(|syntax| (&*RAW_SYNTAXES, syntax))
                .or_else(|| {
                    extra_syntaxes
                        .find_syntax_by_token(&token)
                        .map(|syntax| (&**extra_syntaxes, syntax))
                })
        }) {
            let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
            for (i, (line, line_span)) in lines.into_iter().enumerate() {
                let mut line_content = vec![];
                let mut span_offset = 0;
                for (style, piece) in highlighter
                    .highlight_line(line.as_str(), syntax_set)
                    .into_iter()
                    .flatten()
                {
                    line_content.push(styled(
                        piece,
                        foreground,
                        style,
                        line_span,
                        span_offset,
                    ));
                    span_offset += piece.len();
                }

                seq.push(
                    Packed::new(RawLine::new(
                        i as i64 + 1,
                        count,
                        line,
                        Content::sequence(line_content),
                    ))
                    .spanned(line_span),
                );
            }
        } else {
            seq.extend(lines.into_iter().enumerate().map(|(i, (line, line_span))| {
                Packed::new(RawLine::new(
                    i as i64 + 1,
                    count,
                    line.clone(),
                    TextElem::packed(line).spanned(line_span),
                ))
                .spanned(line_span)
            }));
        };

        seq
    }
}

impl Show for Packed<RawElem> {
    #[typst_macros::time(name = "raw", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let lines = self.lines().map(|v| v.as_slice()).unwrap_or_default();

        let mut seq = EcoVec::with_capacity((2 * lines.len()).saturating_sub(1));
        for (i, line) in lines.iter().enumerate() {
            if i != 0 {
                seq.push(LinebreakElem::new().pack());
            }

            seq.push(line.clone().pack());
        }

        let mut realized = Content::sequence(seq);
        if self.block(styles) {
            // Align the text before inserting it into the block.
            realized = realized.aligned(self.align(styles).into());
            realized =
                BlockElem::new().with_body(Some(realized)).pack().spanned(self.span());
        }

        Ok(realized)
    }
}

impl ShowSet for Packed<RawElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(TextElem::set_overhang(false));
        out.set(TextElem::set_hyphenate(Hyphenate(Smart::Custom(false))));
        out.set(TextElem::set_size(TextSize(Em::new(0.8).into())));
        out.set(TextElem::set_font(FontList(vec![FontFamily::new("DejaVu Sans Mono")])));
        out.set(SmartQuoteElem::set_enabled(false));
        if self.block(styles) {
            out.set(ParElem::set_shrink(false));
        }
        out
    }
}

impl LocalName for Packed<RawElem> {
    fn local_name(lang: Lang, region: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "List",
            Lang::ARABIC => "قائمة",
            Lang::BOKMÅL => "Utskrift",
            Lang::CATALAN => "Llistat",
            Lang::CHINESE if option_eq(region, "TW") => "程式",
            Lang::CHINESE => "代码",
            Lang::CZECH => "Seznam",
            Lang::DANISH => "Liste",
            Lang::DUTCH => "Listing",
            Lang::ESTONIAN => "List",
            Lang::FILIPINO => "Listahan",
            Lang::FINNISH => "Listaus",
            Lang::FRENCH => "Liste",
            Lang::GERMAN => "Listing",
            Lang::GREEK => "Παράθεση",
            Lang::ITALIAN => "Codice",
            Lang::NYNORSK => "Utskrift",
            Lang::POLISH => "Program",
            Lang::ROMANIAN => "Listă", // TODO: I dunno
            Lang::RUSSIAN => "Листинг",
            Lang::SERBIAN => "Програм",
            Lang::SLOVENIAN => "Program",
            Lang::SPANISH => "Listado",
            Lang::SWEDISH => "Listing",
            Lang::TURKISH => "Liste",
            Lang::UKRAINIAN => "Лістинг",
            Lang::VIETNAMESE => "Chương trình", // TODO: This may be wrong.
            Lang::JAPANESE => "リスト",
            Lang::ENGLISH | _ => "Listing",
        }
    }
}

impl Figurable for Packed<RawElem> {}

impl PlainText for Packed<RawElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text().get());
    }
}

/// The content of the raw text.
#[derive(Debug, Clone, Hash, PartialEq)]
pub enum RawContent {
    /// From a string.
    Text(EcoString),
    /// From lines of text.
    Lines(EcoVec<(EcoString, Span)>),
}

impl RawContent {
    /// Returns or synthesizes the text content of the raw text.
    fn get(&self) -> EcoString {
        match self.clone() {
            RawContent::Text(text) => text,
            RawContent::Lines(lines) => {
                let mut lines = lines.into_iter().map(|(s, _)| s);
                if lines.len() <= 1 {
                    lines.next().unwrap_or_default()
                } else {
                    lines.collect::<Vec<_>>().join("\n").into()
                }
            }
        }
    }
}

cast! {
    RawContent,
    self => self.get().into_value(),
    v: EcoString => Self::Text(v),
}

/// A highlighted line of raw text.
///
/// This is a helper element that is synthesized by [`raw`] elements.
///
/// It allows you to access various properties of the line, such as the line
/// number, the raw non-highlighted text, the highlighted text, and whether it
/// is the first or last line of the raw block.
#[elem(name = "line", title = "Raw Text / Code Line", Show, PlainText)]
pub struct RawLine {
    /// The line number of the raw line inside of the raw block, starts at 1.
    #[required]
    pub number: i64,

    /// The total number of lines in the raw block.
    #[required]
    pub count: i64,

    /// The line of raw text.
    #[required]
    pub text: EcoString,

    /// The highlighted raw text.
    #[required]
    pub body: Content,
}

impl Show for Packed<RawLine> {
    #[typst_macros::time(name = "raw.line", span = self.span())]
    fn show(&self, _: &mut Engine, _styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body().clone())
    }
}

impl PlainText for Packed<RawLine> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(self.text());
    }
}

/// Wrapper struct for the state required to highlight typst code.
struct ThemedHighlighter<'a> {
    /// The code being highlighted.
    code: &'a str,
    /// The current node being highlighted.
    node: LinkedNode<'a>,
    /// The highlighter.
    highlighter: synt::Highlighter<'a>,
    /// The current scopes.
    scopes: Vec<syntect::parsing::Scope>,
    /// The current highlighted line.
    current_line: Vec<Content>,
    /// The range of the current line.
    range: Range<usize>,
    /// The current line number.
    line: usize,
    /// The function to style a piece of text.
    style_fn: StyleFn<'a>,
    /// The function to append a line.
    line_fn: LineFn<'a>,
}

impl<'a> ThemedHighlighter<'a> {
    pub fn new(
        code: &'a str,
        top: LinkedNode<'a>,
        highlighter: synt::Highlighter<'a>,
        style_fn: StyleFn<'a>,
        line_fn: LineFn<'a>,
    ) -> Self {
        Self {
            code,
            node: top,
            highlighter,
            range: 0..0,
            scopes: Vec::new(),
            current_line: Vec::new(),
            line: 0,
            style_fn,
            line_fn,
        }
    }

    pub fn highlight(&mut self) {
        self.highlight_inner();

        if !self.current_line.is_empty() {
            (self.line_fn)(
                self.line,
                self.range.start..self.code.len(),
                &mut self.current_line,
            );

            self.current_line.clear();
        }
    }

    fn highlight_inner(&mut self) {
        if self.node.children().len() == 0 {
            let style = self.highlighter.style_for_stack(&self.scopes);
            let segment = &self.code[self.node.range()];

            let mut len = 0;
            for (i, line) in split_newlines(segment).into_iter().enumerate() {
                if i != 0 {
                    (self.line_fn)(
                        self.line,
                        self.range.start..self.range.end + len - 1,
                        &mut self.current_line,
                    );
                    self.range.start = self.range.end + len;
                    self.line += 1;
                }

                let offset = self.node.range().start + len;
                let token_range = offset..(offset + line.len());
                self.current_line.push((self.style_fn)(
                    self.line,
                    &self.node,
                    token_range,
                    style,
                ));

                len += line.len() + 1;
            }

            self.range.end += segment.len();
        }

        for child in self.node.children() {
            let mut scopes = self.scopes.clone();
            if let Some(tag) = crate::syntax::highlight(&child) {
                scopes.push(syntect::parsing::Scope::new(tag.tm_scope()).unwrap())
            }

            std::mem::swap(&mut scopes, &mut self.scopes);
            self.node = child;
            self.highlight_inner();
            std::mem::swap(&mut scopes, &mut self.scopes);
        }
    }
}

fn preprocess(
    text: &RawContent,
    styles: StyleChain,
    span: Span,
) -> EcoVec<(EcoString, Span)> {
    if let RawContent::Lines(lines) = text {
        if lines.iter().all(|(s, _)| !s.contains('\t')) {
            return lines.clone();
        }
    }

    let mut text = text.get();
    if text.contains('\t') {
        let tab_size = RawElem::tab_size_in(styles);
        text = align_tabs(&text, tab_size);
    }
    split_newlines(&text)
        .into_iter()
        .map(|line| (line.into(), span))
        .collect()
}

/// Style a piece of text with a syntect style.
fn styled(
    piece: &str,
    foreground: synt::Color,
    style: synt::Style,
    span: Span,
    span_offset: usize,
) -> Content {
    let mut body = TextElem::packed(piece).spanned(span);

    if span_offset > 0 {
        body = body.styled(TextElem::set_span_offset(span_offset));
    }

    if style.foreground != foreground {
        body = body.styled(TextElem::set_fill(to_typst(style.foreground).into()));
    }

    if style.font_style.contains(synt::FontStyle::BOLD) {
        body = body.strong().spanned(span);
    }

    if style.font_style.contains(synt::FontStyle::ITALIC) {
        body = body.emph().spanned(span);
    }

    if style.font_style.contains(synt::FontStyle::UNDERLINE) {
        body = body.underlined().spanned(span);
    }

    body
}

fn to_typst(synt::Color { r, g, b, a }: synt::Color) -> Color {
    Color::from_u8(r, g, b, a)
}

fn to_syn(color: Color) -> synt::Color {
    let [r, g, b, a] = color.to_rgb().to_vec4_u8();
    synt::Color { r, g, b, a }
}

/// A list of raw syntax file paths.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct SyntaxPaths(Vec<EcoString>);

cast! {
    SyntaxPaths,
    self => self.0.into_value(),
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}

impl Fold for SyntaxPaths {
    fn fold(self, outer: Self) -> Self {
        Self(self.0.fold(outer.0))
    }
}

/// Load a syntax set from a list of syntax file paths.
#[comemo::memoize]
#[typst_macros::time(name = "load syntaxes")]
fn load_syntaxes(paths: &SyntaxPaths, bytes: &[Bytes]) -> StrResult<Arc<SyntaxSet>> {
    let mut out = SyntaxSetBuilder::new();

    // We might have multiple sublime-syntax/yaml files
    for (path, bytes) in paths.0.iter().zip(bytes.iter()) {
        let src = std::str::from_utf8(bytes).map_err(FileError::from)?;
        out.add(SyntaxDefinition::load_from_str(src, false, None).map_err(|err| {
            eco_format!("failed to parse syntax file `{path}` ({err})")
        })?);
    }

    Ok(Arc::new(out.build()))
}

/// Function to parse the syntaxes argument.
/// Much nicer than having it be part of the `element` macro.
fn parse_syntaxes(
    engine: &mut Engine,
    args: &mut Args,
) -> SourceResult<(Option<SyntaxPaths>, Option<Vec<Bytes>>)> {
    let Some(Spanned { v: paths, span }) =
        args.named::<Spanned<SyntaxPaths>>("syntaxes")?
    else {
        return Ok((None, None));
    };

    // Load syntax files.
    let data = paths
        .0
        .iter()
        .map(|path| {
            let id = span.resolve_path(path).at(span)?;
            engine.world.file(id).at(span)
        })
        .collect::<SourceResult<Vec<Bytes>>>()?;

    // Check that parsing works.
    let _ = load_syntaxes(&paths, &data).at(span)?;

    Ok((Some(paths), Some(data)))
}

#[comemo::memoize]
#[typst_macros::time(name = "load theme")]
fn load_theme(path: &str, bytes: &Bytes) -> StrResult<Arc<synt::Theme>> {
    let mut cursor = std::io::Cursor::new(bytes.as_slice());

    synt::ThemeSet::load_from_reader(&mut cursor)
        .map(Arc::new)
        .map_err(|err| eco_format!("failed to parse theme file `{path}` ({err})"))
}

/// Function to parse the theme argument.
/// Much nicer than having it be part of the `element` macro.
fn parse_theme(
    engine: &mut Engine,
    args: &mut Args,
) -> SourceResult<(Option<EcoString>, Option<Bytes>)> {
    let Some(Spanned { v: path, span }) = args.named::<Spanned<EcoString>>("theme")?
    else {
        return Ok((None, None));
    };

    // Load theme file.
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;

    // Check that parsing works.
    let _ = load_theme(&path, &data).at(span)?;

    Ok((Some(path), Some(data)))
}

/// The syntect syntax definitions.
///
/// Syntax set is generated from the syntaxes from the `bat` project
/// <https://github.com/sharkdp/bat/tree/master/assets/syntaxes>
pub static RAW_SYNTAXES: Lazy<syntect::parsing::SyntaxSet> =
    Lazy::new(two_face::syntax::extra_no_newlines);

/// The default theme used for syntax highlighting.
pub static RAW_THEME: Lazy<synt::Theme> = Lazy::new(|| synt::Theme {
    name: Some("Typst Light".into()),
    author: Some("The Typst Project Developers".into()),
    settings: synt::ThemeSettings::default(),
    scopes: vec![
        item("comment", Some("#8a8a8a"), None),
        item("constant.character.escape", Some("#1d6c76"), None),
        item("markup.bold", None, Some(synt::FontStyle::BOLD)),
        item("markup.italic", None, Some(synt::FontStyle::ITALIC)),
        item("markup.underline", None, Some(synt::FontStyle::UNDERLINE)),
        item("markup.raw", Some("#818181"), None),
        item("string.other.math.typst", None, None),
        item("punctuation.definition.math", Some("#298e0d"), None),
        item("keyword.operator.math", Some("#1d6c76"), None),
        item("markup.heading, entity.name.section", None, Some(synt::FontStyle::BOLD)),
        item(
            "markup.heading.typst",
            None,
            Some(synt::FontStyle::BOLD | synt::FontStyle::UNDERLINE),
        ),
        item("punctuation.definition.list", Some("#8b41b1"), None),
        item("markup.list.term", None, Some(synt::FontStyle::BOLD)),
        item("entity.name.label, markup.other.reference", Some("#1d6c76"), None),
        item("keyword, constant.language, variable.language", Some("#d73a49"), None),
        item("storage.type, storage.modifier", Some("#d73a49"), None),
        item("constant", Some("#b60157"), None),
        item("string", Some("#298e0d"), None),
        item("entity.name, variable.function, support", Some("#4b69c6"), None),
        item("support.macro", Some("#16718d"), None),
        item("meta.annotation", Some("#301414"), None),
        item("entity.other, meta.interpolation", Some("#8b41b1"), None),
        item("meta.diff.range", Some("#8b41b1"), None),
        item("markup.inserted, meta.diff.header.to-file", Some("#298e0d"), None),
        item("markup.deleted, meta.diff.header.from-file", Some("#d73a49"), None),
    ],
});

/// Create a syntect theme item.
fn item(
    scope: &str,
    color: Option<&str>,
    font_style: Option<synt::FontStyle>,
) -> synt::ThemeItem {
    synt::ThemeItem {
        scope: scope.parse().unwrap(),
        style: synt::StyleModifier {
            foreground: color.map(|s| to_syn(s.parse::<Color>().unwrap())),
            background: None,
            font_style,
        },
    }
}

/// Replace tabs with spaces to align with multiples of `tab_size`.
fn align_tabs(text: &str, tab_size: usize) -> EcoString {
    let replacement = " ".repeat(tab_size);
    let divisor = tab_size.max(1);
    let amount = text.chars().filter(|&c| c == '\t').count();

    let mut res = EcoString::with_capacity(text.len() - amount + amount * tab_size);
    let mut column = 0;

    for grapheme in text.graphemes(true) {
        match grapheme {
            "\t" => {
                let required = tab_size - column % divisor;
                res.push_str(&replacement[..required]);
                column += required;
            }
            "\n" => {
                res.push_str(grapheme);
                column = 0;
            }
            _ => {
                res.push_str(grapheme);
                column += 1;
            }
        }
    }

    res
}
