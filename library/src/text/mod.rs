//! Text handling.

mod deco;
mod misc;
mod quotes;
mod raw;
mod shaping;
mod shift;
mod symbol;

pub use self::deco::*;
pub use self::misc::*;
pub use self::quotes::*;
pub use self::raw::*;
pub use self::shaping::*;
pub use self::shift::*;
pub use self::symbol::*;

use std::borrow::Cow;

use rustybuzz::Tag;
use typst::font::{FontMetrics, FontStretch, FontStyle, FontWeight, VerticalFontMetric};
use typst::util::EcoString;

use crate::layout::ParNode;
use crate::prelude::*;

/// # Text
/// Stylable text.
///
/// ## Parameters
/// - family: EcoString (positional, variadic, settable)
///   A prioritized sequence of font families.
///
/// - body: Content (positional, required)
///   Content in which all text is styled according to the other arguments.
///
/// ## Category
/// text
#[func]
#[capable]
#[derive(Clone, Hash)]
pub struct TextNode(pub EcoString);

impl TextNode {
    /// Create a new packed text node.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self(text.into()).pack()
    }
}

#[node]
impl TextNode {
    /// A prioritized sequence of font families.
    #[property(skip, referenced)]
    pub const FAMILY: FallbackList = FallbackList(vec![FontFamily::new("IBM Plex Sans")]);
    /// Whether to allow font fallback when the primary font list contains no
    /// match.
    pub const FALLBACK: bool = true;

    /// How the font is styled.
    pub const STYLE: FontStyle = FontStyle::Normal;
    /// The boldness / thickness of the font's glyphs.
    pub const WEIGHT: FontWeight = FontWeight::REGULAR;
    /// The width of the glyphs.
    pub const STRETCH: FontStretch = FontStretch::NORMAL;

    /// The size of the glyphs.
    #[property(shorthand, fold)]
    pub const SIZE: TextSize = Abs::pt(11.0);
    /// The glyph fill color.
    #[property(shorthand)]
    pub const FILL: Paint = Color::BLACK.into();
    /// The amount of space that should be added between characters.
    #[property(resolve)]
    pub const TRACKING: Length = Length::zero();
    /// The width of spaces relative to the font's space width.
    #[property(resolve)]
    pub const SPACING: Rel<Length> = Rel::one();
    /// The offset of the baseline.
    #[property(resolve)]
    pub const BASELINE: Length = Length::zero();
    /// Whether certain glyphs can hang over into the margin.
    pub const OVERHANG: bool = true;
    /// The top end of the text bounding box.
    pub const TOP_EDGE: TextEdge = TextEdge::Metric(VerticalFontMetric::CapHeight);
    /// The bottom end of the text bounding box.
    pub const BOTTOM_EDGE: TextEdge = TextEdge::Metric(VerticalFontMetric::Baseline);

    /// An ISO 639-1/2/3 language code.
    pub const LANG: Lang = Lang::ENGLISH;
    /// An ISO 3166-1 alpha-2 region code.
    pub const REGION: Option<Region> = None;
    /// The direction for text and inline objects. When `auto`, the direction is
    /// automatically inferred from the language.
    #[property(resolve)]
    pub const DIR: HorizontalDir = HorizontalDir(Smart::Auto);
    /// Whether to hyphenate text to improve line breaking. When `auto`, words
    /// will will be hyphenated if and only if justification is enabled.
    #[property(resolve)]
    pub const HYPHENATE: Hyphenate = Hyphenate(Smart::Auto);
    /// Whether to apply smart quotes.
    pub const SMART_QUOTES: bool = true;

    /// Whether to apply kerning ("kern").
    pub const KERNING: bool = true;
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
    /// Whether to have a slash through the zero glyph. ("zero")
    pub const SLASHED_ZERO: bool = false;
    /// Whether to convert fractions. ("frac")
    pub const FRACTIONS: bool = false;
    /// Raw OpenType features to apply.
    #[property(fold)]
    pub const FEATURES: FontFeatures = FontFeatures(vec![]);

    /// A delta to apply on the font weight.
    #[property(skip, fold)]
    pub const DELTA: Delta = 0;
    /// Whether the font style should be inverted.
    #[property(skip, fold)]
    pub const EMPH: Toggle = false;
    /// A case transformation that should be applied to the text.
    #[property(skip)]
    pub const CASE: Option<Case> = None;
    /// Whether small capital glyphs should be used. ("smcp")
    #[property(skip)]
    pub const SMALLCAPS: bool = false;
    /// Decorative lines.
    #[property(skip, fold)]
    pub const DECO: Decoration = vec![];

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        // The text constructor is special: It doesn't create a text node.
        // Instead, it leaves the passed argument structurally unchanged, but
        // styles all text in it.
        args.expect("body")
    }

    fn set(...) {
        if let Some(family) = args.named("family")? {
            styles.set(Self::FAMILY, family);
        } else {
            let mut count = 0;
            let mut content = false;
            for item in args.items.iter().filter(|item| item.name.is_none()) {
                if EcoString::is(&item.value) {
                    count += 1;
                } else if <Content as Cast<Spanned<Value>>>::is(&item.value) {
                    content = true;
                }
            }

            // Skip the final string if it's needed as the body.
            if constructor && !content && count > 0 {
                count -= 1;
            }

            if count > 0 {
                let mut list = Vec::with_capacity(count);
                for _ in 0..count {
                    list.push(args.find()?.unwrap());
                }

                styles.set(Self::FAMILY, FallbackList(list));
            }
        }
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "text" => Some(Value::Str(self.0.clone().into())),
            _ => None,
        }
    }
}

impl Debug for TextNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Text({:?})", self.0)
    }
}

/// A lowercased font family like "arial".
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct FontFamily(EcoString);

impl FontFamily {
    /// Create a named font family variant.
    pub fn new(string: &str) -> Self {
        Self(string.to_lowercase().into())
    }

    /// The lowercased family name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for FontFamily {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

castable! {
    FontFamily,
    string: EcoString => Self::new(&string),
}

/// Font family fallback list.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct FallbackList(pub Vec<FontFamily>);

castable! {
    FallbackList,
    family: FontFamily => Self(vec![family]),
    values: Array => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()),
}

/// The size of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextSize(pub Length);

impl Fold for TextSize {
    type Output = Abs;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.0.em.at(outer) + self.0.abs
    }
}

castable! {
    TextSize,
    v: Length => Self(v),
}

/// Specifies the bottom or top edge of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TextEdge {
    /// An edge specified using one of the well-known font metrics.
    Metric(VerticalFontMetric),
    /// An edge specified as a length.
    Length(Length),
}

impl TextEdge {
    /// Resolve the value of the text edge given a font's metrics.
    pub fn resolve(self, styles: StyleChain, metrics: &FontMetrics) -> Abs {
        match self {
            Self::Metric(metric) => metrics.vertical(metric).resolve(styles),
            Self::Length(length) => length.resolve(styles),
        }
    }
}

castable! {
    TextEdge,
    v: Length => Self::Length(v),
    /// The distance from the baseline to the ascender.
    "ascender" => Self::Metric(VerticalFontMetric::Ascender),
    /// The approximate height of uppercase letters.
    "cap-height" => Self::Metric(VerticalFontMetric::CapHeight),
    /// The approximate height of non-ascending lowercase letters.
    "x-height" => Self::Metric(VerticalFontMetric::XHeight),
    /// The baseline on which the letters rest.
    "baseline" => Self::Metric(VerticalFontMetric::Baseline),
    /// The distance from the baseline to the descender.
    "descender" => Self::Metric(VerticalFontMetric::Descender),
}

/// The direction of text and inline objects in their line.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct HorizontalDir(pub Smart<Dir>);

castable! {
    HorizontalDir,
    _: AutoValue => Self(Smart::Auto),
    dir: Dir => match dir.axis() {
        Axis::X => Self(Smart::Custom(dir)),
        Axis::Y => Err("must be horizontal")?,
    },
}

impl Resolve for HorizontalDir {
    type Output = Dir;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self.0 {
            Smart::Auto => styles.get(TextNode::LANG).dir(),
            Smart::Custom(dir) => dir,
        }
    }
}

/// Whether to hyphenate text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Hyphenate(pub Smart<bool>);

castable! {
    Hyphenate,
    _: AutoValue => Self(Smart::Auto),
    v: bool => Self(Smart::Custom(v)),
}

impl Resolve for Hyphenate {
    type Output = bool;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self.0 {
            Smart::Auto => styles.get(ParNode::JUSTIFY),
            Smart::Custom(v) => v,
        }
    }
}

/// A stylistic set in a font.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct StylisticSet(u8);

impl StylisticSet {
    /// Create a new set, clamping to 1-20.
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
    v: i64 => match v {
        1 ..= 20 => Self::new(v as u8),
        _ => Err("must be between 1 and 20")?,
    },
}

/// Which kind of numbers / figures to select.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberType {
    /// Numbers that fit well with capital text. ("lnum")
    Lining,
    /// Numbers that fit well into a flow of upper- and lowercase text. ("onum")
    OldStyle,
}

castable! {
    NumberType,
    /// Numbers that fit well with capital text.
    "lining" => Self::Lining,
    /// Numbers that fit well into a flow of upper- and lowercase text.
    "old-style" => Self::OldStyle,
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
    /// Number widths are glyph specific.
    "proportional" => Self::Proportional,
    /// All numbers are of equal width / monospaced.
    "tabular" => Self::Tabular,
}

/// OpenType font features settings.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct FontFeatures(pub Vec<(Tag, u32)>);

castable! {
    FontFeatures,
    values: Array => Self(values
        .into_iter()
        .map(|v| {
            let tag = v.cast::<EcoString>()?;
            Ok((Tag::from_bytes_lossy(tag.as_bytes()), 1))
        })
        .collect::<StrResult<_>>()?),
    values: Dict => Self(values
        .into_iter()
        .map(|(k, v)| {
            let num = v.cast::<u32>()?;
            let tag = Tag::from_bytes_lossy(k.as_bytes());
            Ok((tag, num))
        })
        .collect::<StrResult<_>>()?),
}

impl Fold for FontFeatures {
    type Output = Self;

    fn fold(mut self, outer: Self::Output) -> Self::Output {
        self.0.extend(outer.0);
        self
    }
}
