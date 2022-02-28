mod deco;
mod link;
mod par;
mod raw;
mod shaping;

pub use deco::*;
pub use link::*;
pub use par::*;
pub use raw::*;
pub use shaping::*;

use std::borrow::Cow;
use std::ops::BitXor;

use ttf_parser::Tag;

use crate::font::{Face, FontStretch, FontStyle, FontWeight, VerticalFontMetric};
use crate::library::prelude::*;
use crate::util::EcoString;

/// A single run of text with the same style.
#[derive(Hash)]
pub struct TextNode;

#[class]
impl TextNode {
    /// A prioritized sequence of font families.
    #[variadic]
    pub const FAMILY: Vec<FontFamily> = vec![FontFamily::SansSerif];
    /// The serif font family/families.
    pub const SERIF: Vec<NamedFamily> = vec![NamedFamily::new("IBM Plex Serif")];
    /// The sans-serif font family/families.
    pub const SANS_SERIF: Vec<NamedFamily> = vec![NamedFamily::new("IBM Plex Sans")];
    /// The monospace font family/families.
    pub const MONOSPACE: Vec<NamedFamily> = vec![NamedFamily::new("IBM Plex Mono")];
    /// Whether to allow font fallback when the primary font list contains no
    /// match.
    pub const FALLBACK: bool = true;

    /// How the font is styled.
    pub const STYLE: FontStyle = FontStyle::Normal;
    /// The boldness / thickness of the font's glyphs.
    pub const WEIGHT: FontWeight = FontWeight::REGULAR;
    /// The width of the glyphs.
    pub const STRETCH: FontStretch = FontStretch::NORMAL;
    /// The glyph fill color.
    #[shorthand]
    pub const FILL: Paint = Color::BLACK.into();

    /// The size of the glyphs.
    #[shorthand]
    #[fold(Linear::compose)]
    pub const SIZE: Linear = Length::pt(11.0).into();
    /// The amount of space that should be added between characters.
    pub const TRACKING: Em = Em::zero();
    /// The top end of the text bounding box.
    pub const TOP_EDGE: VerticalFontMetric = VerticalFontMetric::CapHeight;
    /// The bottom end of the text bounding box.
    pub const BOTTOM_EDGE: VerticalFontMetric = VerticalFontMetric::Baseline;

    /// Whether to apply kerning ("kern").
    pub const KERNING: bool = true;
    /// Whether small capital glyphs should be used. ("smcp")
    pub const SMALLCAPS: bool = false;
    /// Whether to apply stylistic alternates. ("salt")
    pub const ALTERNATES: bool = false;
    /// Which stylistic set to apply. ("ss01" - "ss20")
    pub const STYLISTIC_SET: Option<StylisticSet> = None;
    /// Whether standard ligatures are active. ("liga", "clig")
    pub const LIGATURES: bool = true;
    /// Whether ligatures that should be used sparingly are active. ("dlig")
    pub const DISCRETIONARY_LIGATURES: bool = false;
    /// Whether historical ligatures are active. ("hlig")
    pub const HISTORICAL_LIGATURES: bool = false;
    /// Which kind of numbers / figures to select.
    pub const NUMBER_TYPE: Smart<NumberType> = Smart::Auto;
    /// The width of numbers / figures.
    pub const NUMBER_WIDTH: Smart<NumberWidth> = Smart::Auto;
    /// How to position numbers.
    pub const NUMBER_POSITION: NumberPosition = NumberPosition::Normal;
    /// Whether to have a slash through the zero glyph. ("zero")
    pub const SLASHED_ZERO: bool = false;
    /// Whether to convert fractions. ("frac")
    pub const FRACTIONS: bool = false;
    /// Raw OpenType features to apply.
    pub const FEATURES: Vec<(Tag, u32)> = vec![];

    /// Whether the font weight should be increased by 300.
    #[skip]
    #[fold(bool::bitxor)]
    pub const STRONG: bool = false;
    /// Whether the the font style should be inverted.
    #[skip]
    #[fold(bool::bitxor)]
    pub const EMPH: bool = false;
    /// Whether a monospace font should be preferred.
    #[skip]
    pub const MONOSPACED: bool = false;
    /// The case transformation that should be applied to the next.
    #[skip]
    pub const CASE: Option<Case> = None;
    /// Decorative lines.
    #[skip]
    #[fold(|a, b| a.into_iter().chain(b).collect())]
    pub const LINES: Vec<Decoration> = vec![];
    /// An URL the text should link to.
    #[skip]
    pub const LINK: Option<EcoString> = None;

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Template> {
        // The text constructor is special: It doesn't create a text node.
        // Instead, it leaves the passed argument structurally unchanged, but
        // styles all text in it.
        args.expect("body")
    }
}

/// Strong text, rendered in boldface.
#[derive(Debug, Hash)]
pub struct StrongNode(pub Template);

#[class]
impl StrongNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Template> {
        Ok(Template::show(Self(args.expect("body")?)))
    }
}

impl Show for StrongNode {
    fn show(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Template> {
        Ok(styles
            .show(self, ctx, [Value::Template(self.0.clone())])?
            .unwrap_or_else(|| self.0.clone().styled(TextNode::STRONG, true)))
    }
}

/// Emphasized text, rendered with an italic face.
#[derive(Debug, Hash)]
pub struct EmphNode(pub Template);

#[class]
impl EmphNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Template> {
        Ok(Template::show(Self(args.expect("body")?)))
    }
}

impl Show for EmphNode {
    fn show(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Template> {
        Ok(styles
            .show(self, ctx, [Value::Template(self.0.clone())])?
            .unwrap_or_else(|| self.0.clone().styled(TextNode::EMPH, true)))
    }
}

/// A generic or named font family.
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum FontFamily {
    /// A family that has "serifs", small strokes attached to letters.
    Serif,
    /// A family in which glyphs do not have "serifs", small attached strokes.
    SansSerif,
    /// A family in which (almost) all glyphs are of equal width.
    Monospace,
    /// A specific font family like "Arial".
    Named(NamedFamily),
}

impl Debug for FontFamily {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Serif => f.pad("serif"),
            Self::SansSerif => f.pad("sans-serif"),
            Self::Monospace => f.pad("monospace"),
            Self::Named(s) => s.fmt(f),
        }
    }
}

dynamic! {
    FontFamily: "font family",
    Value::Str(string) => Self::Named(NamedFamily::new(&string)),
}

castable! {
    Vec<FontFamily>,
    Expected: "string, generic family or array thereof",
    Value::Str(string) => vec![FontFamily::Named(NamedFamily::new(&string))],
    Value::Array(values) => {
        values.into_iter().filter_map(|v| v.cast().ok()).collect()
    },
    @family: FontFamily => vec![family.clone()],
}

/// A specific font family like "Arial".
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct NamedFamily(EcoString);

impl NamedFamily {
    /// Create a named font family variant.
    pub fn new(string: &str) -> Self {
        Self(string.to_lowercase().into())
    }

    /// The lowercased family name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for NamedFamily {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

castable! {
    Vec<NamedFamily>,
    Expected: "string or array of strings",
    Value::Str(string) => vec![NamedFamily::new(&string)],
    Value::Array(values) => values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .map(|string: EcoString| NamedFamily::new(&string))
        .collect(),
}

castable! {
    FontStyle,
    Expected: "string",
    Value::Str(string) => match string.as_str() {
        "normal" => Self::Normal,
        "italic" => Self::Italic,
        "oblique" => Self::Oblique,
        _ => Err(r#"expected "normal", "italic" or "oblique""#)?,
    },
}

castable! {
    FontWeight,
    Expected: "integer or string",
    Value::Int(v) => Value::Int(v)
        .cast::<usize>()?
        .try_into()
        .map_or(Self::BLACK, Self::from_number),
    Value::Str(string) => match string.as_str() {
        "thin" => Self::THIN,
        "extralight" => Self::EXTRALIGHT,
        "light" => Self::LIGHT,
        "regular" => Self::REGULAR,
        "medium" => Self::MEDIUM,
        "semibold" => Self::SEMIBOLD,
        "bold" => Self::BOLD,
        "extrabold" => Self::EXTRABOLD,
        "black" => Self::BLACK,
        _ => Err("unknown font weight")?,
    },
}

castable! {
    FontStretch,
    Expected: "relative",
    Value::Relative(v) => Self::from_ratio(v.get() as f32),
}

castable! {
    Em,
    Expected: "float",
    Value::Float(v) => Self::new(v),
}

castable! {
    VerticalFontMetric,
    Expected: "linear or string",
    Value::Length(v) => Self::Linear(v.into()),
    Value::Relative(v) => Self::Linear(v.into()),
    Value::Linear(v) => Self::Linear(v),
    Value::Str(string) => match string.as_str() {
        "ascender" => Self::Ascender,
        "cap-height" => Self::CapHeight,
        "x-height" => Self::XHeight,
        "baseline" => Self::Baseline,
        "descender" => Self::Descender,
        _ => Err("unknown font metric")?,
    },
}

/// A stylistic set in a font face.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct StylisticSet(u8);

impl StylisticSet {
    /// Creates a new set, clamping to 1-20.
    pub fn new(index: u8) -> Self {
        Self(index.clamp(1, 20))
    }

    /// Get the value, guaranteed to be 1-20.
    pub fn get(self) -> u8 {
        self.0
    }
}

castable! {
    StylisticSet,
    Expected: "integer",
    Value::Int(v) => match v {
        1 ..= 20 => Self::new(v as u8),
        _ => Err("must be between 1 and 20")?,
    },
}

/// Which kind of numbers / figures to select.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberType {
    /// Numbers that fit well with capital text. ("lnum")
    Lining,
    /// Numbers that fit well into flow of upper- and lowercase text. ("onum")
    OldStyle,
}

castable! {
    NumberType,
    Expected: "string",
    Value::Str(string) => match string.as_str() {
        "lining" => Self::Lining,
        "old-style" => Self::OldStyle,
        _ => Err(r#"expected "lining" or "old-style""#)?,
    },
}

/// The width of numbers / figures.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberWidth {
    /// Number widths are glyph specific. ("pnum")
    Proportional,
    /// All numbers are of equal width / monospaced. ("tnum")
    Tabular,
}

castable! {
    NumberWidth,
    Expected: "string",
    Value::Str(string) => match string.as_str() {
        "proportional" => Self::Proportional,
        "tabular" => Self::Tabular,
        _ => Err(r#"expected "proportional" or "tabular""#)?,
    },
}

/// How to position numbers.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberPosition {
    /// Numbers are positioned on the same baseline as text.
    Normal,
    /// Numbers are smaller and placed at the bottom. ("subs")
    Subscript,
    /// Numbers are smaller and placed at the top. ("sups")
    Superscript,
}

castable! {
    NumberPosition,
    Expected: "string",
    Value::Str(string) => match string.as_str() {
        "normal" => Self::Normal,
        "subscript" => Self::Subscript,
        "superscript" => Self::Superscript,
        _ => Err(r#"expected "normal", "subscript" or "superscript""#)?,
    },
}

castable! {
    Vec<(Tag, u32)>,
    Expected: "array of strings or dictionary mapping tags to integers",
    Value::Array(values) => values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .map(|string: EcoString| (Tag::from_bytes_lossy(string.as_bytes()), 1))
        .collect(),
    Value::Dict(values) => values
        .into_iter()
        .filter_map(|(k, v)| {
            let tag = Tag::from_bytes_lossy(k.as_bytes());
            let num = v.cast::<i64>().ok()?.try_into().ok()?;
            Some((tag, num))
        })
        .collect(),
}

/// A case transformation on text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Case {
    /// Everything is uppercased.
    Upper,
    /// Everything is lowercased.
    Lower,
}

impl Case {
    /// Apply the case to a string of text.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Upper => text.to_uppercase(),
            Self::Lower => text.to_lowercase(),
        }
    }
}
