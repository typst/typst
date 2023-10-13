use std::hash::Hash;
use std::ops::Range;
use std::sync::Arc;

use ecow::EcoVec;
use once_cell::sync::Lazy;
use once_cell::unsync::Lazy as UnsyncLazy;
use syntect::highlighting as synt;
use syntect::parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};
use typst::diag::FileError;
use typst::eval::Bytes;
use typst::syntax::{self, is_newline, LinkedNode};
use typst::util::option_eq;
use unicode_segmentation::UnicodeSegmentation;

use super::{
    FontFamily, FontList, Hyphenate, LinebreakElem, SmartquoteElem, TextElem, TextSize,
};
use crate::layout::BlockElem;
use crate::meta::{Figurable, LocalName};
use crate::prelude::*;

// Shorthand for highlighter closures.
type StyleFn<'a> = &'a mut dyn FnMut(&LinkedNode, Range<usize>, synt::Style) -> Content;
type LineFn<'a> = &'a mut dyn FnMut(i64, Range<usize>, &mut Vec<Content>);

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
    Finalize,
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
    pub text: EcoString,

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
    /// `{"typ"}` and `{"typc"}` tags for Typst markup and Typst code,
    /// respectively.
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
    #[default(HAlign::Start)]
    pub align: HAlign,

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
        let (syntaxes, syntaxes_data) = parse_syntaxes(vm, args)?;
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
    /// can apply the foreground color yourself with the [`text`]($text)
    /// function and the background with a [filled block]($block.fill). You
    /// could also use the [`xml`]($xml) function to extract these properties
    /// from the theme.
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
        let (theme_path, theme_data) = parse_theme(vm, args)?;
        theme_path.map(Some)
    )]
    pub theme: Option<EcoString>,

    /// The raw file buffer of syntax theme file.
    #[internal]
    #[parse(theme_data.map(Some))]
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
    pub lines: Vec<Content>,
}

#[scope]
impl RawElem {
    #[elem]
    type RawLine;
}

impl RawElem {
    /// The supported language names and tags.
    pub fn languages() -> Vec<(&'static str, Vec<&'static str>)> {
        SYNTAXES
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

impl Synthesize for RawElem {
    fn synthesize(&mut self, _vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        self.push_lang(self.lang(styles));

        let mut text = self.text();
        if text.contains('\t') {
            let tab_size = RawElem::tab_size_in(styles);
            text = align_tabs(&text, tab_size);
        }

        let lang = self
            .lang(styles)
            .as_ref()
            .map(|s| s.to_lowercase())
            .or(Some("txt".into()));

        let extra_syntaxes = UnsyncLazy::new(|| {
            load_syntaxes(&self.syntaxes(styles), &self.syntaxes_data(styles)).unwrap()
        });

        let theme = self.theme(styles).map(|theme_path| {
            load_theme(theme_path, self.theme_data(styles).unwrap()).unwrap()
        });

        let theme = theme.as_deref().unwrap_or(&THEME);

        let foreground = theme.settings.foreground.unwrap_or(synt::Color::BLACK);

        let mut seq = vec![];
        if matches!(lang.as_deref(), Some("typ" | "typst" | "typc")) {
            let root = match lang.as_deref() {
                Some("typc") => syntax::parse_code(&text),
                _ => syntax::parse(&text),
            };

            ThemedHighlighter::new(
                &text,
                LinkedNode::new(&root),
                synt::Highlighter::new(theme),
                &mut |_, range, style| styled(&text[range], foreground, style),
                &mut |i, range, line| {
                    seq.push(
                        RawLine::new(
                            i + 1,
                            text.split(is_newline).count() as i64,
                            EcoString::from(&text[range]),
                            Content::sequence(line.drain(..)),
                        )
                        .pack(),
                    );
                },
            )
            .highlight();
        } else if let Some((syntax_set, syntax)) = lang.and_then(|token| {
            SYNTAXES
                .find_syntax_by_token(&token)
                .map(|syntax| (&*SYNTAXES, syntax))
                .or_else(|| {
                    extra_syntaxes
                        .find_syntax_by_token(&token)
                        .map(|syntax| (&**extra_syntaxes, syntax))
                })
        }) {
            let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
            let len = text.lines().count();
            for (i, line) in text.lines().enumerate() {
                let mut line_content = vec![];
                for (style, piece) in
                    highlighter.highlight_line(line, syntax_set).into_iter().flatten()
                {
                    line_content.push(styled(piece, foreground, style));
                }

                seq.push(
                    RawLine::new(
                        i as i64 + 1,
                        len as i64,
                        EcoString::from(line),
                        Content::sequence(line_content),
                    )
                    .pack(),
                );
            }
        } else {
            seq.extend(text.lines().map(TextElem::packed));
        };

        self.push_lines(seq);

        Ok(())
    }
}

impl Show for RawElem {
    #[tracing::instrument(name = "RawElem::show", skip_all)]
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut lines = EcoVec::with_capacity((2 * self.lines().len()).saturating_sub(1));
        for (i, line) in self.lines().into_iter().enumerate() {
            if i != 0 {
                lines.push(LinebreakElem::new().pack());
            }

            lines.push(line);
        }

        let mut realized = Content::sequence(lines);
        if self.block(styles) {
            // Align the text before inserting it into the block.
            realized = realized.aligned(self.align(styles).into());
            realized = BlockElem::new().with_body(Some(realized)).pack();
        }

        Ok(realized)
    }
}

impl Finalize for RawElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        let mut styles = Styles::new();
        styles.set(TextElem::set_overhang(false));
        styles.set(TextElem::set_hyphenate(Hyphenate(Smart::Custom(false))));
        styles.set(TextElem::set_size(TextSize(Em::new(0.8).into())));
        styles
            .set(TextElem::set_font(FontList(vec![FontFamily::new("DejaVu Sans Mono")])));
        styles.set(SmartquoteElem::set_enabled(false));
        realized.styled_with_map(styles)
    }
}

impl LocalName for RawElem {
    fn local_name(&self, lang: Lang, region: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "List",
            Lang::ARABIC => "قائمة",
            Lang::BOKMÅL => "Utskrift",
            Lang::CHINESE if option_eq(region, "TW") => "程式",
            Lang::CHINESE => "代码",
            Lang::CZECH => "Seznam",
            Lang::DANISH => "Liste",
            Lang::DUTCH => "Listing",
            Lang::FILIPINO => "Listahan",
            Lang::FINNISH => "Esimerkki",
            Lang::FRENCH => "Liste",
            Lang::GERMAN => "Listing",
            Lang::ITALIAN => "Codice",
            Lang::NYNORSK => "Utskrift",
            Lang::POLISH => "Program",
            Lang::ROMANIAN => "Listă", // TODO: I dunno
            Lang::RUSSIAN => "Листинг",
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

impl Figurable for RawElem {}

impl PlainText for RawElem {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text());
    }
}

/// A highlighted line of raw text.
///
/// This is a helper element that is synthesized by [`raw`]($raw) elements.
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

impl Show for RawLine {
    fn show(&self, _vt: &mut Vt, _styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body())
    }
}

impl PlainText for RawLine {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text());
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
    line: i64,
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
            for (i, line) in segment.split(is_newline).enumerate() {
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
                self.current_line
                    .push((self.style_fn)(&self.node, token_range, style));

                len += line.len() + 1;
            }

            self.range.end += segment.len();
        }

        for child in self.node.children() {
            let mut scopes = self.scopes.clone();
            if let Some(tag) = typst::syntax::highlight(&child) {
                scopes.push(syntect::parsing::Scope::new(tag.tm_scope()).unwrap())
            }

            std::mem::swap(&mut scopes, &mut self.scopes);
            self.node = child;
            self.highlight_inner();
            std::mem::swap(&mut scopes, &mut self.scopes);
        }
    }
}

/// Style a piece of text with a syntect style.
fn styled(piece: &str, foreground: synt::Color, style: synt::Style) -> Content {
    let mut body = TextElem::packed(piece);

    if style.foreground != foreground {
        body = body.styled(TextElem::set_fill(to_typst(style.foreground).into()));
    }

    if style.font_style.contains(synt::FontStyle::BOLD) {
        body = body.strong();
    }

    if style.font_style.contains(synt::FontStyle::ITALIC) {
        body = body.emph();
    }

    if style.font_style.contains(synt::FontStyle::UNDERLINE) {
        body = body.underlined();
    }

    body
}

fn to_typst(synt::Color { r, g, b, a }: synt::Color) -> Color {
    Color::from_u8(r, g, b, a)
}

fn to_syn(color: Color) -> synt::Color {
    let [r, g, b, a] = color.to_vec4_u8();
    synt::Color { r, g, b, a }
}

/// A list of bibliography file paths.
#[derive(Debug, Default, Clone, Hash)]
pub struct SyntaxPaths(Vec<EcoString>);

cast! {
    SyntaxPaths,
    self => self.0.into_value(),
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}

impl Fold for SyntaxPaths {
    type Output = Self;

    fn fold(mut self, outer: Self::Output) -> Self::Output {
        self.0.extend(outer.0);
        self
    }
}

/// Load a syntax set from a list of syntax file paths.
#[comemo::memoize]
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
    vm: &mut Vm,
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
            let id = vm.resolve_path(path).at(span)?;
            vm.world().file(id).at(span)
        })
        .collect::<SourceResult<Vec<Bytes>>>()?;

    // Check that parsing works.
    let _ = load_syntaxes(&paths, &data).at(span)?;

    Ok((Some(paths), Some(data)))
}

#[comemo::memoize]
fn load_theme(path: EcoString, bytes: Bytes) -> StrResult<Arc<synt::Theme>> {
    let mut cursor = std::io::Cursor::new(bytes.as_slice());

    synt::ThemeSet::load_from_reader(&mut cursor)
        .map(Arc::new)
        .map_err(|err| eco_format!("failed to parse theme file `{path}` ({err})"))
}

/// Function to parse the theme argument.
/// Much nicer than having it be part of the `element` macro.
fn parse_theme(
    vm: &mut Vm,
    args: &mut Args,
) -> SourceResult<(Option<EcoString>, Option<Bytes>)> {
    let Some(Spanned { v: path, span }) = args.named::<Spanned<EcoString>>("theme")?
    else {
        return Ok((None, None));
    };

    // Load theme file.
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;

    // Check that parsing works.
    let _ = load_theme(path.clone(), data.clone()).at(span)?;

    Ok((Some(path), Some(data)))
}

/// The syntect syntax definitions.
///
/// Code for syntax set generation is below. The `syntaxes` directory is from
/// <https://github.com/sharkdp/bat/tree/master/assets/syntaxes>
///
/// ```ignore
/// fn main() {
///     let mut builder = syntect::parsing::SyntaxSet::load_defaults_nonewlines().into_builder();
///     builder.add_from_folder("syntaxes/02_Extra", false).unwrap();
///     syntect::dumps::dump_to_file(&builder.build(), "syntect.bin").unwrap();
/// }
/// ```
///
/// The following syntaxes are disabled due to compatibility issues:
/// ```text
/// syntaxes/02_Extra/Assembly (ARM).sublime-syntax
/// syntaxes/02_Extra/Elixir/Regular Expressions (Elixir).sublime-syntax
/// syntaxes/02_Extra/JavaScript (Babel).sublime-syntax
/// syntaxes/02_Extra/LiveScript.sublime-syntax
/// syntaxes/02_Extra/PowerShell.sublime-syntax
/// syntaxes/02_Extra/SCSS_Sass/Syntaxes/Sass.sublime-syntax
/// syntaxes/02_Extra/SLS/SLS.sublime-syntax
/// syntaxes/02_Extra/VimHelp.sublime-syntax
/// syntaxes/02_Extra/cmd-help/syntaxes/cmd-help.sublime-syntax
/// ```
pub static SYNTAXES: Lazy<syntect::parsing::SyntaxSet> =
    Lazy::new(|| syntect::dumps::from_binary(include_bytes!("../../assets/syntect.bin")));

/// The default theme used for syntax highlighting.
pub static THEME: Lazy<synt::Theme> = Lazy::new(|| synt::Theme {
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
