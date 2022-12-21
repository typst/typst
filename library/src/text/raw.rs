use once_cell::sync::Lazy;
use syntect::highlighting as synt;
use typst::syntax::{self, LinkedNode};

use super::{FontFamily, Hyphenate, LinebreakNode, SmartQuoteNode, TextNode};
use crate::layout::BlockNode;
use crate::prelude::*;

/// # Raw Text / Code
/// Raw text with optional syntax highlighting.
///
/// Displays the text verbatim and in a monospace font. This is typically used
/// to embed computer code into your document.
///
/// ## Syntax
/// This function also has dedicated syntax. You can enclose text in 1 or 3+
/// backticks (`` ` ``) to make it raw. Two backticks produce empty raw text.
/// When you use three or more backticks, you can additionally specify a
/// language tag for syntax highlighting directly after the opening backticks.
/// Within raw blocks, everything is rendered as is, in particular, there are no
/// escape sequences.
///
/// ## Example
/// ````
/// Adding `rbx` to `rcx` gives
/// the desired result.
///
/// ```rust
/// fn main() {
///     println!("Hello World!");
/// }
/// ```
/// ````
///
/// ## Parameters
/// - text: EcoString (positional, required)
///   The raw text.
///
///   You can also use raw blocks creatively to create custom syntaxes for
///   your automations.
///
///   ### Example
///   ````
///   // Parse numbers in raw blocks with the
///   // `mydsl` tag and sum them up.
///   #show raw.where(lang: "mydsl"): it => {
///     let sum = 0
///     for part in it.text.split("+") {
///       sum += int(part.trim())
///     }
///     sum
///   }
///
///   ```mydsl
///   1 + 2 + 3 + 4 + 5
///   ```
///   ````
///
/// - block: bool (named)
///   Whether the raw text is displayed as a separate block.
///
///   ### Example
///   ````
///   // Display inline code in a small box
///   // that retains the correct baseline.
///   #show raw.where(block: false): rect.with(
///     fill: luma(240),
///     inset: (x: 3pt),
///     outset: (y: 3pt),
///     radius: 2pt,
///   )
///
///   // Display block code in a larger box
///   // with more padding.
///   #show raw.where(block: true): rect.with(
///     fill: luma(240),
///     inset: 10pt,
///     radius: 4pt,
///   )
///
///   With `rg`, you can search through your files quickly.
///
///   ```bash
///   rg "Hello World"
///   ```
///   ````
///
/// ## Category
/// text
#[func]
#[capable(Show, Prepare)]
#[derive(Debug, Hash)]
pub struct RawNode {
    /// The raw text.
    pub text: EcoString,
    /// Whether the raw text is displayed as a separate block.
    pub block: bool,
}

#[node]
impl RawNode {
    /// The language to syntax-highlight in.
    ///
    /// Apart from typical language tags known from Markdown, this supports the
    /// `{"typ"}` and `{"typc"}` tags for Typst markup and Typst code,
    /// respectively.
    ///
    /// # Example
    /// ````
    /// ```typ
    /// This is *Typst!*
    /// ```
    /// ````
    #[property(referenced)]
    pub const LANG: Option<EcoString> = None;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            text: args.expect("text")?,
            block: args.named("block")?.unwrap_or(false),
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "text" => Some(Value::Str(self.text.clone().into())),
            "block" => Some(Value::Bool(self.block)),
            _ => None,
        }
    }
}

impl Prepare for RawNode {
    fn prepare(&self, _: &mut Vt, mut this: Content, styles: StyleChain) -> Content {
        this.push_field(
            "lang",
            match styles.get(Self::LANG) {
                Some(lang) => Value::Str(lang.clone().into()),
                None => Value::None,
            },
        );
        this
    }
}

impl Show for RawNode {
    fn show(&self, _: &mut Vt, _: &Content, styles: StyleChain) -> SourceResult<Content> {
        let lang = styles.get(Self::LANG).as_ref().map(|s| s.to_lowercase());
        let foreground = THEME
            .settings
            .foreground
            .map(to_typst)
            .map_or(Color::BLACK, Color::from)
            .into();

        let mut realized = if matches!(lang.as_deref(), Some("typ" | "typst" | "typc")) {
            let root = match lang.as_deref() {
                Some("typc") => syntax::parse_code(&self.text),
                _ => syntax::parse(&self.text),
            };

            let mut seq = vec![];
            let highlighter = synt::Highlighter::new(&THEME);
            highlight_themed(
                &LinkedNode::new(&root),
                vec![],
                &highlighter,
                &mut |node, style| {
                    seq.push(styled(&self.text[node.range()], foreground, style));
                },
            );

            Content::sequence(seq)
        } else if let Some(syntax) =
            lang.and_then(|token| SYNTAXES.find_syntax_by_token(&token))
        {
            let mut seq = vec![];
            let mut highlighter = syntect::easy::HighlightLines::new(syntax, &THEME);
            for (i, line) in self.text.lines().enumerate() {
                if i != 0 {
                    seq.push(LinebreakNode { justify: false }.pack());
                }

                for (style, piece) in
                    highlighter.highlight_line(line, &SYNTAXES).into_iter().flatten()
                {
                    seq.push(styled(piece, foreground, style));
                }
            }

            Content::sequence(seq)
        } else {
            TextNode::packed(self.text.clone())
        };

        if self.block {
            realized = BlockNode(realized).pack();
        }

        let mut map = StyleMap::new();
        map.set(TextNode::OVERHANG, false);
        map.set(TextNode::HYPHENATE, Hyphenate(Smart::Custom(false)));
        map.set(SmartQuoteNode::ENABLED, false);
        map.set_family(FontFamily::new("IBM Plex Mono"), styles);

        Ok(realized.styled_with_map(map))
    }
}

/// Highlight a syntax node in a theme by calling `f` with ranges and their
/// styles.
fn highlight_themed<F>(
    node: &LinkedNode,
    scopes: Vec<syntect::parsing::Scope>,
    highlighter: &synt::Highlighter,
    f: &mut F,
) where
    F: FnMut(&LinkedNode, synt::Style),
{
    if node.children().len() == 0 {
        let style = highlighter.style_for_stack(&scopes);
        f(node, style);
        return;
    }

    for child in node.children() {
        let mut scopes = scopes.clone();
        if let Some(tag) = typst::ide::highlight(&child) {
            scopes.push(syntect::parsing::Scope::new(tag.tm_scope()).unwrap())
        }
        highlight_themed(&child, scopes, highlighter, f);
    }
}

/// Style a piece of text with a syntect style.
fn styled(piece: &str, foreground: Paint, style: synt::Style) -> Content {
    let mut body = TextNode::packed(piece);

    let paint = to_typst(style.foreground).into();
    if paint != foreground {
        body = body.styled(TextNode::FILL, paint);
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

fn to_typst(synt::Color { r, g, b, a }: synt::Color) -> RgbaColor {
    RgbaColor { r, g, b, a }
}

fn to_syn(RgbaColor { r, g, b, a }: RgbaColor) -> synt::Color {
    synt::Color { r, g, b, a }
}

/// The syntect syntax definitions.
static SYNTAXES: Lazy<syntect::parsing::SyntaxSet> =
    Lazy::new(|| syntect::parsing::SyntaxSet::load_defaults_newlines());

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
        item("invalid", Some("#ff0000"), None),
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
            foreground: color.map(|s| to_syn(s.parse::<RgbaColor>().unwrap())),
            background: None,
            font_style,
        },
    }
}
