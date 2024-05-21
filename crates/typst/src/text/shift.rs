use ecow::EcoString;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, SequenceElem, Show, StyleChain};
use crate::layout::{Em, Length};
use crate::text::{variant, SpaceElem, TextElem, TextSize};
use crate::World;

/// Renders text in subscript.
///
/// The text is rendered smaller and its baseline is lowered.
///
/// # Example
/// ```example
/// Revenue#sub[yearly]
/// ```
#[elem(title = "Subscript", Show)]
pub struct SubElem {
    /// Whether to prefer the dedicated subscript characters of the font.
    ///
    /// If this is enabled, Typst first tries to transform the text to subscript
    /// codepoints. If that fails, it falls back to rendering lowered and shrunk
    /// normal letters.
    ///
    /// ```example
    /// N#sub(typographic: true)[1]
    /// N#sub(typographic: false)[1]
    /// ```
    #[default(true)]
    pub typographic: bool,

    /// The baseline shift for synthetic subscripts. Does not apply if
    /// `typographic` is true and the font has subscript codepoints for the
    /// given `body`.
    #[default(Em::new(0.2).into())]
    pub baseline: Length,

    /// The font size for synthetic subscripts. Does not apply if
    /// `typographic` is true and the font has subscript codepoints for the
    /// given `body`.
    #[default(TextSize(Em::new(0.6).into()))]
    pub size: TextSize,

    /// The text to display in subscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SubElem> {
    #[typst_macros::time(name = "sub", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body().clone();
        let mut transformed = None;
        if self.typographic(styles) {
            if let Some(text) = search_text(&body, true) {
                if is_shapable(engine, &text, styles) {
                    transformed = Some(TextElem::packed(text));
                }
            }
        };

        Ok(transformed.unwrap_or_else(|| {
            body.styled(TextElem::set_baseline(self.baseline(styles)))
                .styled(TextElem::set_size(self.size(styles)))
        }))
    }
}

/// Renders text in superscript.
///
/// The text is rendered smaller and its baseline is raised.
///
/// # Example
/// ```example
/// 1#super[st] try!
/// ```
#[elem(title = "Superscript", Show)]
pub struct SuperElem {
    /// Whether to prefer the dedicated superscript characters of the font.
    ///
    /// If this is enabled, Typst first tries to transform the text to
    /// superscript codepoints. If that fails, it falls back to rendering
    /// raised and shrunk normal letters.
    ///
    /// ```example
    /// N#super(typographic: true)[1]
    /// N#super(typographic: false)[1]
    /// ```
    #[default(true)]
    pub typographic: bool,

    /// The baseline shift for synthetic superscripts. Does not apply if
    /// `typographic` is true and the font has superscript codepoints for the
    /// given `body`.
    #[default(Em::new(-0.5).into())]
    pub baseline: Length,

    /// The font size for synthetic superscripts. Does not apply if
    /// `typographic` is true and the font has superscript codepoints for the
    /// given `body`.
    #[default(TextSize(Em::new(0.6).into()))]
    pub size: TextSize,

    /// The text to display in superscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SuperElem> {
    #[typst_macros::time(name = "super", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body().clone();
        let mut transformed = None;
        if self.typographic(styles) {
            if let Some(text) = search_text(&body, false) {
                if is_shapable(engine, &text, styles) {
                    transformed = Some(TextElem::packed(text));
                }
            }
        };

        Ok(transformed.unwrap_or_else(|| {
            body.styled(TextElem::set_baseline(self.baseline(styles)))
                .styled(TextElem::set_size(self.size(styles)))
        }))
    }
}

/// Find and transform the text contained in `content` to the given script kind
/// if and only if it only consists of `Text`, `Space`, and `Empty` leafs.
fn search_text(content: &Content, sub: bool) -> Option<EcoString> {
    if content.is::<SpaceElem>() {
        Some(' '.into())
    } else if let Some(elem) = content.to_packed::<TextElem>() {
        convert_script(elem.text(), sub)
    } else if let Some(sequence) = content.to_packed::<SequenceElem>() {
        let mut full = EcoString::new();
        for item in &sequence.children {
            match search_text(item, sub) {
                Some(text) => full.push_str(&text),
                None => return None,
            }
        }
        Some(full)
    } else {
        None
    }
}

/// Checks whether the first retrievable family contains all code points of the
/// given string.
fn is_shapable(engine: &Engine, text: &str, styles: StyleChain) -> bool {
    let world = engine.world;
    for family in TextElem::font_in(styles) {
        if let Some(font) = world
            .book()
            .select(family.as_str(), variant(styles))
            .and_then(|id| world.font(id))
        {
            return text.chars().all(|c| font.ttf().glyph_index(c).is_some());
        }
    }

    false
}

/// Convert a string to sub- or superscript codepoints if all characters
/// can be mapped to such a codepoint.
fn convert_script(text: &str, sub: bool) -> Option<EcoString> {
    let mut result = EcoString::with_capacity(text.len());
    let converter = if sub { to_subscript_codepoint } else { to_superscript_codepoint };

    for c in text.chars() {
        match converter(c) {
            Some(c) => result.push(c),
            None => return None,
        }
    }

    Some(result)
}

/// Convert a character to its corresponding Unicode superscript.
fn to_superscript_codepoint(c: char) -> Option<char> {
    char::from_u32(match c {
        '0' => 0x2070,
        '1' => 0x00B9,
        '2' => 0x00B2,
        '3' => 0x00B3,
        '4'..='9' => 0x2070 + (c as u32 + 4 - '4' as u32),
        '+' => 0x207A,
        '-' => 0x207B,
        '=' => 0x207C,
        '(' => 0x207D,
        ')' => 0x207E,
        'n' => 0x207F,
        'i' => 0x2071,
        ' ' => 0x0020,
        _ => return None,
    })
}

/// Convert a character to its corresponding Unicode subscript.
fn to_subscript_codepoint(c: char) -> Option<char> {
    char::from_u32(match c {
        '0' => 0x2080,
        '1'..='9' => 0x2080 + (c as u32 - '0' as u32),
        '+' => 0x208A,
        '-' => 0x208B,
        '=' => 0x208C,
        '(' => 0x208D,
        ')' => 0x208E,
        'a' => 0x2090,
        'e' => 0x2091,
        'o' => 0x2092,
        'x' => 0x2093,
        'h' => 0x2095,
        'k' => 0x2096,
        'l' => 0x2097,
        'm' => 0x2098,
        'n' => 0x2099,
        'p' => 0x209A,
        's' => 0x209B,
        't' => 0x209C,
        ' ' => 0x0020,
        _ => return None,
    })
}
