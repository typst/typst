use std::cell::LazyCell;
use std::ops::Range;
use std::sync::{Arc, LazyLock};

use comemo::Tracked;
use ecow::{EcoString, EcoVec};
use syntect::highlighting::{self as synt};
use syntect::parsing::{ParseSyntaxError, SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};
use typst_syntax::{LinkedNode, Span, Spanned, split_newlines};
use typst_utils::ManuallyHash;
use unicode_segmentation::UnicodeSegmentation;

use super::Lang;
use crate::World;
use crate::diag::{
    LineCol, LoadError, LoadResult, LoadedWithin, ReportPos, SourceResult,
};
use crate::engine::Engine;
use crate::foundations::{
    Bytes, Content, Derived, OneOrMultiple, Packed, PlainText, ShowSet, Smart,
    StyleChain, Styles, Synthesize, Target, TargetElem, cast, elem, scope,
};
use crate::introspection::{Locatable, Tagged};
use crate::layout::{Em, HAlignment};
use crate::loading::{DataSource, Load};
use crate::model::{Figurable, ParElem};
use crate::routines::Routines;
use crate::text::{FontFamily, FontList, LocalName, TextElem, TextSize};
use crate::visualize::Color;

/// Raw text with optional syntax highlighting.
///
/// Displays the text verbatim and in a monospace font. This is typically used
/// to embed computer code into your document.
///
/// Note that text given to this element cannot contain arbitrary formatting,
/// such as `[*strong*]` or `[_emphasis_]`, as it is displayed verbatim. If
/// you'd like to display any kind of content with a monospace font, instead of
/// using [`raw`], you should change its font to a monospace font using the
/// [`text`]($text) function.
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
/// You can also construct a [`raw`] element programmatically from a string (and
/// provide the language tag via the optional [`lang`]($raw.lang) argument).
/// ```example
/// #raw("fn " + "main() {}", lang: "rust")
/// ```
///
/// # Syntax
/// This function also has dedicated syntax. You can enclose text in 1 or 3+
/// backticks (`` ` ``) to make it raw. Two backticks produce empty raw text.
/// This works both in markup and code.
///
/// When you use three or more backticks, you can additionally specify a
/// language tag for syntax highlighting directly after the opening backticks.
/// Within raw blocks, everything (except for the language tag, if applicable)
/// is rendered as is, in particular, there are no escape sequences.
///
/// Directly following the three or more opening backticks is the language tag
/// until the first whitespace or backtick. If your text starts with something
/// that looks like an identifier, but no syntax highlighting is needed, start
/// the text with a single space (which will be trimmed) or use the single
/// backtick syntax. If your text should start or end with a backtick, put a
/// space before or after it (it will be trimmed).
///
/// If no syntax highlighting is available by default for your specified
/// language tag (or if you want to override the built-in definition), you may
/// provide a custom syntax specification file to the
/// [`syntaxes`]($raw.syntaxes) field.
///
/// # Styling
/// By default, the `raw` element uses the `DejaVu Sans Mono` font (included
/// with Typst), with a smaller font size of `{0.8em}` (that is, 80% of
/// the global font size). This is because monospace fonts tend to be visually
/// larger than non-monospace fonts.
///
/// You can customize these properties with show-set rules:
///
/// ````example
/// // Switch to Cascadia Code for both
/// // inline and block raw.
/// #show raw: set text(font: "Cascadia Code")
///
/// // Reset raw blocks to the same size as normal text,
/// // but keep inline raw at the reduced size.
/// #show raw.where(block: true): set text(1em / 0.8)
///
/// Now using the `Cascadia Code` font for raw text.
/// Here's some Python code. It looks larger now:
///
/// ```py
/// def python():
///   return 5 + 5
/// ```
/// ````
///
/// In addition, you can customize the syntax highlighting colors by setting
/// a custom theme through the [`theme`]($raw.theme) field.
///
/// For complete customization of the appearance of a raw block, a show rule
/// on [`raw.line`]($raw.line) could be helpful, such as to add line numbers.
///
/// Note that, in raw text, typesetting features like
/// [hyphenation]($text.hyphenate), [overhang]($text.overhang),
/// [CJK-Latin spacing]($text.cjk-latin-spacing) (and
/// [justification]($par.justify) for [raw blocks]($raw.block)) will be
/// disabled by default.
#[elem(
    scope,
    title = "Raw Text / Code",
    Synthesize,
    Locatable,
    Tagged,
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
    /// ````example:"Implementing a DSL using raw and show rules"
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
    /// `{"typ"}`, `{"typc"}`, and `{"typm"}` tags for
    /// [Typst markup]($reference/syntax/#markup),
    /// [Typst code]($reference/syntax/#code), and
    /// [Typst math]($reference/syntax/#math), respectively.
    ///
    /// ````example
    /// ```typ
    /// This is *Typst!*
    /// ```
    ///
    /// This is ```typ also *Typst*```, but inline!
    /// ````
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

    /// Additional syntax definitions to load. The syntax definitions should be
    /// in the [`sublime-syntax` file format](https://www.sublimetext.com/docs/syntax.html).
    ///
    /// You can pass any of the following values:
    ///
    /// - A path string to load a syntax file from the given path. For more
    ///   details about paths, see the [Paths section]($syntax/#paths).
    /// - Raw bytes from which the syntax should be decoded.
    /// - An array where each item is one of the above.
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
    #[parse(match args.named("syntaxes")? {
        Some(sources) => Some(RawSyntax::load(engine.world, sources)?),
        None => None,
    })]
    #[fold]
    pub syntaxes: Derived<OneOrMultiple<DataSource>, Vec<RawSyntax>>,

    /// The theme to use for syntax highlighting. Themes should be in the
    /// [`tmTheme` file format](https://www.sublimetext.com/docs/color_schemes_tmtheme.html).
    ///
    /// You can pass any of the following values:
    ///
    /// - `{none}`: Disables syntax highlighting.
    /// - `{auto}`: Highlights with Typst's default theme.
    /// - A path string to load a theme file from the given path. For more
    ///   details about paths, see the [Paths section]($syntax/#paths).
    /// - Raw bytes from which the theme should be decoded.
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
    #[parse(match args.named::<Spanned<Smart<Option<DataSource>>>>("theme")? {
        Some(Spanned { v: Smart::Custom(Some(source)), span }) => Some(Smart::Custom(
            Some(RawTheme::load(engine.world, Spanned::new(source, span))?)
        )),
        Some(Spanned { v: Smart::Custom(None), .. }) => Some(Smart::Custom(None)),
        Some(Spanned { v: Smart::Auto, .. }) => Some(Smart::Auto),
        None => None,
    })]
    pub theme: Smart<Option<Derived<DataSource, RawTheme>>>,

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
            .chain([
                ("Typst", vec!["typ"]),
                ("Typst (code)", vec!["typc"]),
                ("Typst (math)", vec!["typm"]),
            ])
            .collect()
    }
}

impl Synthesize for Packed<RawElem> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let seq = self.highlight(engine.routines, styles);
        self.lines = Some(seq);
        Ok(())
    }
}

impl Packed<RawElem> {
    #[comemo::memoize]
    fn highlight(&self, routines: &Routines, styles: StyleChain) -> Vec<Packed<RawLine>> {
        let elem = self.as_ref();
        let lines = preprocess(&elem.text, styles, self.span());

        let count = lines.len() as i64;
        let lang = elem
            .lang
            .get_ref(styles)
            .as_ref()
            .map(|s| s.to_lowercase())
            .or(Some("txt".into()));

        let non_highlighted_result = |lines: EcoVec<(EcoString, Span)>| {
            lines.into_iter().enumerate().map(|(i, (line, line_span))| {
                Packed::new(RawLine::new(
                    i as i64 + 1,
                    count,
                    line.clone(),
                    TextElem::packed(line).spanned(line_span),
                ))
                .spanned(line_span)
            })
        };

        let syntaxes = LazyCell::new(|| elem.syntaxes.get_cloned(styles));
        let theme: &synt::Theme = match elem.theme.get_ref(styles) {
            Smart::Auto => &RAW_THEME,
            Smart::Custom(Some(theme)) => theme.derived.get(),
            Smart::Custom(None) => return non_highlighted_result(lines).collect(),
        };

        let foreground = theme.settings.foreground.unwrap_or(synt::Color::BLACK);
        let target = styles.get(TargetElem::target);

        let mut seq = vec![];
        if matches!(lang.as_deref(), Some("typ" | "typst" | "typc" | "typm")) {
            let text =
                lines.iter().map(|(s, _)| s.clone()).collect::<Vec<_>>().join("\n");
            let root = match lang.as_deref() {
                Some("typc") => typst_syntax::parse_code(&text),
                Some("typm") => typst_syntax::parse_math(&text),
                _ => typst_syntax::parse(&text),
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
                    styled(
                        routines,
                        target,
                        &text[range],
                        foreground,
                        style,
                        span,
                        span_offset,
                    )
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
            // Prefer user-provided syntaxes over built-in ones.
            syntaxes
                .derived
                .iter()
                .map(|syntax| syntax.get())
                .chain(std::iter::once(&*RAW_SYNTAXES))
                .find_map(|set| {
                    set.find_syntax_by_token(&token).map(|syntax| (set, syntax))
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
                        routines,
                        target,
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
            seq.extend(non_highlighted_result(lines));
        };

        seq
    }
}

impl ShowSet for Packed<RawElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(TextElem::overhang, false);
        out.set(TextElem::lang, Lang::ENGLISH);
        out.set(TextElem::hyphenate, Smart::Custom(false));
        out.set(TextElem::size, TextSize(Em::new(0.8).into()));
        out.set(TextElem::font, FontList(vec![FontFamily::new("DejaVu Sans Mono")]));
        out.set(TextElem::cjk_latin_spacing, Smart::Custom(None));
        if self.block.get(styles) {
            out.set(ParElem::justify, false);
        }
        out
    }
}

impl LocalName for Packed<RawElem> {
    const KEY: &'static str = "raw";
}

impl Figurable for Packed<RawElem> {}

impl PlainText for Packed<RawElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text.get());
    }
}

/// The content of the raw text.
#[derive(Debug, Clone, Hash)]
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

impl PartialEq for RawContent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RawContent::Text(a), RawContent::Text(b)) => a == b,
            (lines @ RawContent::Lines(_), RawContent::Text(text))
            | (RawContent::Text(text), lines @ RawContent::Lines(_)) => {
                *text == lines.get()
            }
            (RawContent::Lines(a), RawContent::Lines(b)) => Iterator::eq(
                a.iter().map(|(line, _)| line),
                b.iter().map(|(line, _)| line),
            ),
        }
    }
}

cast! {
    RawContent,
    self => self.get().into_value(),
    v: EcoString => Self::Text(v),
}

/// A loaded syntax.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct RawSyntax(Arc<ManuallyHash<SyntaxSet>>);

impl RawSyntax {
    /// Load syntaxes from sources.
    fn load(
        world: Tracked<dyn World + '_>,
        sources: Spanned<OneOrMultiple<DataSource>>,
    ) -> SourceResult<Derived<OneOrMultiple<DataSource>, Vec<RawSyntax>>> {
        let loaded = sources.load(world)?;
        let list = loaded
            .iter()
            .map(|data| Self::decode(&data.data).within(data))
            .collect::<SourceResult<_>>()?;
        Ok(Derived::new(sources.v, list))
    }

    /// Decode a syntax from a loaded source.
    #[comemo::memoize]
    #[typst_macros::time(name = "load syntaxes")]
    fn decode(bytes: &Bytes) -> LoadResult<RawSyntax> {
        let str = bytes.as_str()?;

        let syntax = SyntaxDefinition::load_from_str(str, false, None)
            .map_err(format_syntax_error)?;

        let mut builder = SyntaxSetBuilder::new();
        builder.add(syntax);

        Ok(RawSyntax(Arc::new(ManuallyHash::new(
            builder.build(),
            typst_utils::hash128(bytes),
        ))))
    }

    /// Return the underlying syntax set.
    fn get(&self) -> &SyntaxSet {
        self.0.as_ref()
    }
}

fn format_syntax_error(error: ParseSyntaxError) -> LoadError {
    let pos = syntax_error_pos(&error);
    LoadError::new(pos, "failed to parse syntax", error)
}

fn syntax_error_pos(error: &ParseSyntaxError) -> ReportPos {
    match error {
        ParseSyntaxError::InvalidYaml(scan_error) => {
            let m = scan_error.marker();
            ReportPos::full(
                m.index()..m.index(),
                LineCol::one_based(m.line(), m.col() + 1),
            )
        }
        _ => ReportPos::None,
    }
}

/// A loaded syntect theme.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct RawTheme(Arc<ManuallyHash<synt::Theme>>);

impl RawTheme {
    /// Load a theme from a data source.
    fn load(
        world: Tracked<dyn World + '_>,
        source: Spanned<DataSource>,
    ) -> SourceResult<Derived<DataSource, Self>> {
        let loaded = source.load(world)?;
        let theme = Self::decode(&loaded.data).within(&loaded)?;
        Ok(Derived::new(source.v, theme))
    }

    /// Decode a theme from bytes.
    #[comemo::memoize]
    fn decode(bytes: &Bytes) -> LoadResult<RawTheme> {
        let mut cursor = std::io::Cursor::new(bytes.as_slice());
        let theme =
            synt::ThemeSet::load_from_reader(&mut cursor).map_err(format_theme_error)?;
        Ok(RawTheme(Arc::new(ManuallyHash::new(theme, typst_utils::hash128(bytes)))))
    }

    /// Get the underlying syntect theme.
    pub fn get(&self) -> &synt::Theme {
        self.0.as_ref()
    }
}

fn format_theme_error(error: syntect::LoadingError) -> LoadError {
    let pos = match &error {
        syntect::LoadingError::ParseSyntax(err, _) => syntax_error_pos(err),
        _ => ReportPos::None,
    };
    LoadError::new(pos, "failed to parse theme", error)
}

/// A highlighted line of raw text.
///
/// This is a helper element that is synthesized by [`raw`] elements.
///
/// It allows you to access various properties of the line, such as the line
/// number, the raw non-highlighted text, the highlighted text, and whether it
/// is the first or last line of the raw block.
#[elem(name = "line", title = "Raw Text / Code Line", Tagged, PlainText)]
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

impl PlainText for Packed<RawLine> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text);
    }
}

/// Wrapper struct for the state required to highlight Typst code.
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

// Shorthands for highlighter closures.
type StyleFn<'a> =
    &'a mut dyn FnMut(usize, &LinkedNode, Range<usize>, synt::Style) -> Content;
type LineFn<'a> = &'a mut dyn FnMut(usize, Range<usize>, &mut Vec<Content>);

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
            if let Some(tag) = typst_syntax::highlight(&child) {
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
    if let RawContent::Lines(lines) = text
        && lines.iter().all(|(s, _)| !s.contains('\t'))
    {
        return lines.clone();
    }

    let mut text = text.get();
    if text.contains('\t') {
        let tab_size = styles.get(RawElem::tab_size);
        text = align_tabs(&text, tab_size);
    }
    split_newlines(&text)
        .into_iter()
        .map(|line| (line.into(), span))
        .collect()
}

/// Style a piece of text with a syntect style.
fn styled(
    routines: &Routines,
    target: Target,
    piece: &str,
    foreground: synt::Color,
    style: synt::Style,
    span: Span,
    span_offset: usize,
) -> Content {
    let mut body = TextElem::packed(piece).spanned(span);

    if span_offset > 0 {
        body = body.set(TextElem::span_offset, span_offset);
    }

    if style.foreground != foreground {
        let color = to_typst(style.foreground);
        body = match target {
            Target::Html => (routines.html_span_filled)(body, color),
            Target::Paged => body.set(TextElem::fill, color.into()),
        };
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
    let (r, g, b, a) = color.to_rgb().into_format::<u8, u8>().into_components();
    synt::Color { r, g, b, a }
}

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
        let c = grapheme.parse::<char>();
        if c == Ok('\t') {
            let required = tab_size - column % divisor;
            res.push_str(&replacement[..required]);
            column += required;
        } else if c.is_ok_and(typst_syntax::is_newline) || grapheme == "\r\n" {
            res.push_str(grapheme);
            column = 0;
        } else {
            res.push_str(grapheme);
            column += 1;
        }
    }

    res
}

/// The syntect syntax definitions.
///
/// Syntax set is generated from the syntaxes from the `bat` project
/// <https://github.com/sharkdp/bat/tree/master/assets/syntaxes>
pub static RAW_SYNTAXES: LazyLock<syntect::parsing::SyntaxSet> =
    LazyLock::new(two_face::syntax::extra_no_newlines);

/// The default theme used for syntax highlighting.
pub static RAW_THEME: LazyLock<synt::Theme> = LazyLock::new(|| synt::Theme {
    name: Some("Typst Light".into()),
    author: Some("The Typst Project Developers".into()),
    settings: synt::ThemeSettings::default(),
    scopes: vec![
        item("comment", Some("#74747c"), None),
        item("constant.character.escape", Some("#1d6c76"), None),
        item("markup.bold", None, Some(synt::FontStyle::BOLD)),
        item("markup.italic", None, Some(synt::FontStyle::ITALIC)),
        item("markup.underline", None, Some(synt::FontStyle::UNDERLINE)),
        item("markup.raw", Some("#6b6b6f"), None),
        item("string.other.math.typst", None, None),
        item("punctuation.definition.math", Some("#198810"), None),
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
        item("keyword, constant.language, variable.language", Some("#d73948"), None),
        item("storage.type, storage.modifier", Some("#d73948"), None),
        item("constant", Some("#b60157"), None),
        item("string", Some("#198810"), None),
        item("entity.name, variable.function, support", Some("#4b69c6"), None),
        item("support.macro", Some("#16718d"), None),
        item("meta.annotation", Some("#301414"), None),
        item("entity.other, meta.interpolation", Some("#8b41b1"), None),
        item("meta.diff.range", Some("#8b41b1"), None),
        item("markup.inserted, meta.diff.header.to-file", Some("#198810"), None),
        item("markup.deleted, meta.diff.header.from-file", Some("#d73948"), None),
        item("meta.mapping.key.json string.quoted.double.json", Some("#4b69c6"), None),
        item("meta.mapping.value.json string.quoted.double.json", Some("#198810"), None),
    ],
});
