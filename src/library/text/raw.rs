use once_cell::sync::Lazy;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Highlighter, Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

use super::{FontFamily, Hyphenate, TextNode, Toggle};
use crate::library::prelude::*;
use crate::source::SourceId;
use crate::syntax::{self, RedNode};

/// The lazily-loaded theme used for syntax highlighting.
static THEME: Lazy<Theme> =
    Lazy::new(|| ThemeSet::load_defaults().themes.remove("InspiredGitHub").unwrap());

/// The lazily-loaded syntect syntax definitions.
static SYNTAXES: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());

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
    /// The raw text's font family. Just the normal text family if `none`.
    #[property(referenced)]
    pub const FAMILY: Smart<FontFamily> = Smart::Custom(FontFamily::new("IBM Plex Mono"));

    /// The language to syntax-highlight in.
    #[property(referenced)]
    pub const LANG: Option<EcoString> = None;

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show(Self {
            text: args.expect("text")?,
            block: args.named("block")?.unwrap_or(false),
        }))
    }
}

impl Show for RawNode {
    fn encode(&self) -> Dict {
        dict! {
            "text" => Value::Str(self.text.clone()),
            "block" => Value::Bool(self.block)
        }
    }

    fn realize(&self, _: &mut Context, styles: StyleChain) -> TypResult<Content> {
        let lang = styles.get(Self::LANG).as_ref();
        let foreground = THEME
            .settings
            .foreground
            .map(Color::from)
            .unwrap_or(Color::BLACK)
            .into();

        if matches!(
            lang.map(|s| s.to_lowercase()).as_deref(),
            Some("typ" | "typst")
        ) {
            let mut seq = vec![];
            let green = crate::parse::parse(&self.text);
            let red = RedNode::from_root(green, SourceId::from_raw(0));
            let highlighter = Highlighter::new(&THEME);

            syntax::highlight_syntect(red.as_ref(), &highlighter, &mut |range, style| {
                seq.push(styled(&self.text[range], foreground, style));
            });

            Ok(Content::sequence(seq))
        } else if let Some(syntax) =
            lang.and_then(|token| SYNTAXES.find_syntax_by_token(&token))
        {
            let mut seq = vec![];
            let mut highlighter = HighlightLines::new(syntax, &THEME);
            for (i, line) in self.text.lines().enumerate() {
                if i != 0 {
                    seq.push(Content::Linebreak(false));
                }

                for (style, piece) in highlighter.highlight(line, &SYNTAXES) {
                    seq.push(styled(piece, foreground, style));
                }
            }

            Ok(Content::sequence(seq))
        } else {
            Ok(Content::Text(self.text.clone()))
        }
    }

    fn finalize(
        &self,
        _: &mut Context,
        styles: StyleChain,
        mut realized: Content,
    ) -> TypResult<Content> {
        let mut map = StyleMap::new();
        map.set(TextNode::OVERHANG, false);
        map.set(TextNode::HYPHENATE, Smart::Custom(Hyphenate(false)));
        map.set(TextNode::SMART_QUOTES, false);

        if let Smart::Custom(family) = styles.get(Self::FAMILY) {
            map.set_family(family.clone(), styles);
        }

        realized = realized.styled_with_map(map);

        if self.block {
            realized = Content::block(realized);
        }

        Ok(realized)
    }
}

/// Style a piece of text with a syntect style.
fn styled(piece: &str, foreground: Paint, style: Style) -> Content {
    let mut styles = StyleMap::new();
    let mut body = Content::Text(piece.into());

    let paint = style.foreground.into();
    if paint != foreground {
        styles.set(TextNode::FILL, paint);
    }

    if style.font_style.contains(FontStyle::BOLD) {
        styles.set(TextNode::STRONG, Toggle);
    }

    if style.font_style.contains(FontStyle::ITALIC) {
        styles.set(TextNode::EMPH, Toggle);
    }

    if style.font_style.contains(FontStyle::UNDERLINE) {
        body = body.underlined();
    }

    body.styled_with_map(styles)
}
