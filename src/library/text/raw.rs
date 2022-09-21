use once_cell::sync::Lazy;
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, FontStyle, Style, StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::SyntaxSet;

use super::{FontFamily, Hyphenate, TextNode};
use crate::library::layout::BlockSpacing;
use crate::library::prelude::*;
use crate::parse::TokenMode;
use crate::syntax;

/// Monospaced text with optional syntax highlighting.
#[derive(Debug, Hash)]
pub struct RawNode {
    /// The raw text.
    pub text: EcoString,
    /// Whether the node is block-level.
    pub block: bool,
}

#[node(showable)]
impl RawNode {
    /// The language to syntax-highlight in.
    #[property(referenced)]
    pub const LANG: Option<EcoString> = None;
    /// The raw text's font family.
    #[property(referenced)]
    pub const FAMILY: FontFamily = FontFamily::new("IBM Plex Mono");
    /// The spacing above block-level raw.
    #[property(resolve, shorthand(around))]
    pub const ABOVE: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing below block-level raw.
    #[property(resolve, shorthand(around))]
    pub const BELOW: Option<BlockSpacing> = Some(Ratio::one().into());

    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Content::show(Self {
            text: args.expect("text")?,
            block: args.named("block")?.unwrap_or(false),
        }))
    }
}

impl Show for RawNode {
    fn unguard(&self, _: Selector) -> ShowNode {
        Self { text: self.text.clone(), ..*self }.pack()
    }

    fn encode(&self, styles: StyleChain) -> Dict {
        dict! {
           "text" => Value::Str(self.text.clone().into()),
           "block" => Value::Bool(self.block),
           "lang" => match styles.get(Self::LANG) {
               Some(lang) => Value::Str(lang.clone().into()),
               None => Value::None,
           },
        }
    }

    fn realize(
        &self,
        _: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Content> {
        let lang = styles.get(Self::LANG).as_ref().map(|s| s.to_lowercase());
        let foreground = THEME
            .settings
            .foreground
            .map(Color::from)
            .unwrap_or(Color::BLACK)
            .into();

        let mut realized = if matches!(lang.as_deref(), Some("typ" | "typst" | "typc")) {
            let mode = match lang.as_deref() {
                Some("typc") => TokenMode::Code,
                _ => TokenMode::Markup,
            };

            let mut seq = vec![];
            syntax::highlight_themed(&self.text, mode, &THEME, |piece, style| {
                seq.push(styled(piece, foreground, style));
            });

            Content::sequence(seq)
        } else if let Some(syntax) =
            lang.and_then(|token| SYNTAXES.find_syntax_by_token(&token))
        {
            let mut seq = vec![];
            let mut highlighter = HighlightLines::new(syntax, &THEME);
            for (i, line) in self.text.lines().enumerate() {
                if i != 0 {
                    seq.push(Content::Linebreak { justified: false });
                }

                for (style, piece) in
                    highlighter.highlight_line(line, &SYNTAXES).into_iter().flatten()
                {
                    seq.push(styled(piece, foreground, style));
                }
            }

            Content::sequence(seq)
        } else {
            Content::Text(self.text.clone())
        };

        if self.block {
            realized = Content::block(realized);
        }

        Ok(realized)
    }

    fn finalize(
        &self,
        _: Tracked<dyn World>,
        styles: StyleChain,
        mut realized: Content,
    ) -> SourceResult<Content> {
        let mut map = StyleMap::new();
        map.set_family(styles.get(Self::FAMILY).clone(), styles);
        map.set(TextNode::OVERHANG, false);
        map.set(TextNode::HYPHENATE, Smart::Custom(Hyphenate(false)));
        map.set(TextNode::SMART_QUOTES, false);

        if self.block {
            realized = realized.spaced(styles.get(Self::ABOVE), styles.get(Self::BELOW));
        }

        Ok(realized.styled_with_map(map).role(Role::Code))
    }
}

/// Style a piece of text with a syntect style.
fn styled(piece: &str, foreground: Paint, style: Style) -> Content {
    let mut body = Content::Text(piece.into());

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
        item("markup.bold", None, Some(FontStyle::BOLD)),
        item("markup.italic", None, Some(FontStyle::ITALIC)),
        item("markup.heading, entity.name.section", None, Some(FontStyle::BOLD)),
        item("markup.heading.typst", None, Some(FontStyle::BOLD | FontStyle::UNDERLINE)),
        item("markup.raw", Some("#818181"), None),
        item("markup.list", Some("#8b41b1"), None),
        item("comment", Some("#8a8a8a"), None),
        item("punctuation.shortcut", Some("#1d6c76"), None),
        item("constant.character.escape", Some("#1d6c76"), None),
        item("entity.name.label, markup.other.reference", Some("#1d6c76"), None),
        item("keyword, constant.language, variable.language", Some("#d73a49"), None),
        item("storage.type, storage.modifier", Some("#d73a49"), None),
        item("entity.other", Some("#8b41b1"), None),
        item("entity.name, variable.function, support", Some("#4b69c6"), None),
        item("support.macro", Some("#16718d"), None),
        item("meta.annotation", Some("#301414"), None),
        item("constant", Some("#b60157"), None),
        item("string", Some("#298e0d"), None),
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
