use once_cell::sync::Lazy;
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, FontStyle, Style, StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::SyntaxSet;
use typst::syntax;

use super::{FontFamily, Hyphenate, LinebreakNode, TextNode};
use crate::layout::BlockNode;
use crate::prelude::*;

/// Raw text with optional syntax highlighting.
#[derive(Debug, Hash)]
pub struct RawNode {
    /// The raw text.
    pub text: EcoString,
    /// Whether the node is block-level.
    pub block: bool,
}

#[node(Show)]
impl RawNode {
    /// The language to syntax-highlight in.
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
            .map(Color::from)
            .unwrap_or(Color::BLACK)
            .into();

        let mut realized = if matches!(lang.as_deref(), Some("typ" | "typst" | "typc")) {
            let root = match lang.as_deref() {
                Some("typc") => syntax::parse_code(&self.text),
                _ => syntax::parse(&self.text),
            };

            let mut seq = vec![];
            syntax::highlight::highlight_themed(&root, &THEME, |range, style| {
                seq.push(styled(&self.text[range], foreground, style));
            });

            Content::sequence(seq)
        } else if let Some(syntax) =
            lang.and_then(|token| SYNTAXES.find_syntax_by_token(&token))
        {
            let mut seq = vec![];
            let mut highlighter = HighlightLines::new(syntax, &THEME);
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
        map.set(TextNode::SMART_QUOTES, false);
        map.set_family(FontFamily::new("IBM Plex Mono"), styles);

        Ok(realized.styled_with_map(map))
    }
}

/// Style a piece of text with a syntect style.
fn styled(piece: &str, foreground: Paint, style: Style) -> Content {
    let mut body = TextNode::packed(piece);

    let paint = style.foreground.into();
    if paint != foreground {
        body = body.styled(TextNode::FILL, paint);
    }

    if style.font_style.contains(FontStyle::BOLD) {
        body = body.strong();
    }

    if style.font_style.contains(FontStyle::ITALIC) {
        body = body.emph();
    }

    if style.font_style.contains(FontStyle::UNDERLINE) {
        body = body.underlined();
    }

    body
}

/// The syntect syntax definitions.
static SYNTAXES: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());

/// The default theme used for syntax highlighting.
#[rustfmt::skip]
pub static THEME: Lazy<Theme> = Lazy::new(|| Theme {
    name: Some("Typst Light".into()),
    author: Some("The Typst Project Developers".into()),
    settings: ThemeSettings::default(),
    scopes: vec![
        item("comment", Some("#8a8a8a"), None),
        item("constant.character.escape", Some("#1d6c76"), None),
        item("constant.character.shortcut", Some("#1d6c76"), None),
        item("markup.bold", None, Some(FontStyle::BOLD)),
        item("markup.italic", None, Some(FontStyle::ITALIC)),
        item("markup.underline", None, Some(FontStyle::UNDERLINE)),
        item("markup.raw", Some("#818181"), None),
        item("string.other.math.typst", None, None),
        item("punctuation.definition.math", Some("#298e0d"), None),
        item("keyword.operator.math", Some("#1d6c76"), None),
        item("markup.heading, entity.name.section", None, Some(FontStyle::BOLD)),
        item("markup.heading.typst", None, Some(FontStyle::BOLD | FontStyle::UNDERLINE)),
        item("punctuation.definition.list", Some("#8b41b1"), None),
        item("markup.list.term", None, Some(FontStyle::BOLD)),
        item("entity.name.label, markup.other.reference", Some("#1d6c76"), None),
        item("keyword, constant.language, variable.language", Some("#d73a49"), None),
        item("storage.type, storage.modifier", Some("#d73a49"), None),
        item("constant", Some("#b60157"), None),
        item("string", Some("#298e0d"), None),
        item("entity.name, variable.function, support", Some("#4b69c6"), None),
        item("support.macro", Some("#16718d"), None),
        item("meta.annotation", Some("#301414"), None),
        item("entity.other, meta.interpolation, constant.symbol.typst", Some("#8b41b1"), None),
        item("invalid", Some("#ff0000"), None),
    ],
});

/// Create a syntect theme item.
fn item(scope: &str, color: Option<&str>, font_style: Option<FontStyle>) -> ThemeItem {
    ThemeItem {
        scope: scope.parse().unwrap(),
        style: StyleModifier {
            foreground: color.map(|s| s.parse::<RgbaColor>().unwrap().into()),
            background: None,
            font_style,
        },
    }
}
