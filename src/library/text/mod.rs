//! Text handling and paragraph layout.

mod deco;
mod link;
mod par;
mod quotes;
mod raw;
mod repeat;
mod shaping;
mod shift;

pub use deco::*;
pub use link::*;
pub use par::*;
pub use quotes::*;
pub use raw::*;
pub use repeat::*;
pub use shaping::*;
pub use shift::*;

use std::borrow::Cow;

use rustybuzz::Tag;

use crate::font::{FontMetrics, FontStretch, FontStyle, FontWeight, VerticalFontMetric};
use crate::library::prelude::*;
use crate::util::EcoString;

/// A single run of text with the same style.
#[derive(Hash)]
pub struct TextNode;

#[node]
impl TextNode {
    /// A prioritized sequence of font families.
    #[property(skip, referenced)]
    pub const FAMILY: Vec<FontFamily> = vec![FontFamily::new("IBM Plex Sans")];
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
    pub const DIR: Smart<HorizontalDir> = Smart::Auto;
    /// Whether to hyphenate text to improve line breaking. When `auto`, words
    /// will will be hyphenated if and only if justification is enabled.
    #[property(resolve)]
    pub const HYPHENATE: Smart<Hyphenate> = Smart::Auto;
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
    pub const FEATURES: Vec<(Tag, u32)> = vec![];

    /// Whether the font weight should be increased by 300.
    #[property(skip, fold)]
    pub const BOLD: Toggle = false;
    /// Whether the font style should be inverted.
    #[property(skip, fold)]
    pub const ITALIC: Toggle = false;
    /// A case transformation that should be applied to the text.
    #[property(skip)]
    pub const CASE: Option<Case> = None;
    /// Whether small capital glyphs should be used. ("smcp")
    #[property(skip)]
    pub const SMALLCAPS: bool = false;
    /// A destination the text should be linked to.
    #[property(skip, referenced)]
    pub const LINK: Option<Destination> = None;
    /// Decorative lines.
    #[property(skip, fold)]
    pub const DECO: Decoration = vec![];

    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
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
                } else if Content::is(&item.value) {
                    content = true;
                }
            }

            // Skip the final string if it's needed as the body.
            if constructor && !content && count > 0 {
                count -= 1;
            }

            if count > 0 {
                let mut list = Vec::with_capacity(count);
                for _ in 0 .. count {
                    list.push(args.find()?.unwrap());
                }

                styles.set(Self::FAMILY, list);
            }
        }
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
    Expected: "string",
    Value::Str(string) => Self::new(&string),
}

castable! {
    Vec<FontFamily>,
    Expected: "string or array of strings",
    Value::Str(string) => vec![FontFamily::new(&string)],
    Value::Array(values) => values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .map(|string: EcoString| FontFamily::new(&string))
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
    Expected: "ratio",
    Value::Ratio(v) => Self::from_ratio(v.get() as f32),
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

castable!(TextSize: Length);

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
    Expected: "string or length",
    Value::Length(v) => Self::Length(v),
    Value::Str(string) => Self::Metric(match string.as_str() {
        "ascender" => VerticalFontMetric::Ascender,
        "cap-height" => VerticalFontMetric::CapHeight,
        "x-height" => VerticalFontMetric::XHeight,
        "baseline" => VerticalFontMetric::Baseline,
        "descender" => VerticalFontMetric::Descender,
        _ => Err("unknown font metric")?,
    }),
}

castable! {
    Lang,
    Expected: "string",
    Value::Str(string) => Self::from_str(&string)
        .ok_or("expected two or three letter language code (ISO 639-1/2/3)")?,
}

castable! {
    Region,
    Expected: "string",
    Value::Str(string) => Self::from_str(&string)
        .ok_or("expected two letter region code (ISO 3166-1 alpha-2)")?,
}

/// The direction of text and inline objects in their line.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct HorizontalDir(pub Dir);

castable! {
    HorizontalDir,
    Expected: "direction",
    @dir: Dir => match dir.axis() {
        Axis::X => Self(*dir),
        Axis::Y => Err("must be horizontal")?,
    },
}

impl Resolve for Smart<HorizontalDir> {
    type Output = Dir;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self {
            Smart::Auto => styles.get(TextNode::LANG).dir(),
            Smart::Custom(dir) => dir.0,
        }
    }
}

/// Whether to hyphenate text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Hyphenate(pub bool);

castable!(Hyphenate: bool);

impl Resolve for Smart<Hyphenate> {
    type Output = bool;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self {
            Smart::Auto => styles.get(ParNode::JUSTIFY),
            Smart::Custom(v) => v.0,
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
    /// Numbers that fit well into a flow of upper- and lowercase text. ("onum")
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

impl Fold for Vec<(Tag, u32)> {
    type Output = Self;

    fn fold(mut self, outer: Self::Output) -> Self::Output {
        self.extend(outer);
        self
    }
}

/// Convert a string or content to lowercase.
pub fn lower(_: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    case(Case::Lower, args)
}

/// Convert a string or content to uppercase.
pub fn upper(_: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    case(Case::Upper, args)
}

/// Change the case of text.
fn case(case: Case, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect("string or content")?;
    Ok(match v {
        Value::Str(v) => Value::Str(case.apply(&v).into()),
        Value::Content(v) => Value::Content(v.styled(TextNode::CASE, Some(case))),
        v => bail!(span, "expected string or content, found {}", v.type_name()),
    })
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
    /// Apply the case to a string.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Upper => text.to_uppercase(),
            Self::Lower => text.to_lowercase(),
        }
    }
}

/// Display text in small capitals.
pub fn smallcaps(_: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    let body: Content = args.expect("content")?;
    Ok(Value::Content(body.styled(TextNode::SMALLCAPS, true)))
}

/// A toggle that turns on and off alternatingly if folded.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Toggle;

impl Fold for Toggle {
    type Output = bool;

    fn fold(self, outer: Self::Output) -> Self::Output {
        !outer
    }
}

impl Fold for Decoration {
    type Output = Vec<Self>;

    fn fold(self, mut outer: Self::Output) -> Self::Output {
        outer.insert(0, self);
        outer
    }
}

/// Strong text, rendered in boldface by default.
#[derive(Debug, Hash)]
pub struct StrongNode(pub Content);

#[node(showable)]
impl StrongNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Content::show(Self(args.expect("body")?)))
    }
}

impl Show for StrongNode {
    fn unguard(&self, sel: Selector) -> ShowNode {
        Self(self.0.unguard(sel)).pack()
    }

    fn encode(&self, _: StyleChain) -> Dict {
        dict! { "body" => Value::Content(self.0.clone()) }
    }

    fn realize(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(self.0.clone().styled(TextNode::BOLD, Toggle))
    }
}

/// Emphasized text, rendered with an italic font by default.
#[derive(Debug, Hash)]
pub struct EmphNode(pub Content);

#[node(showable)]
impl EmphNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Content::show(Self(args.expect("body")?)))
    }
}

impl Show for EmphNode {
    fn unguard(&self, sel: Selector) -> ShowNode {
        Self(self.0.unguard(sel)).pack()
    }

    fn encode(&self, _: StyleChain) -> Dict {
        dict! { "body" => Value::Content(self.0.clone()) }
    }

    fn realize(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(self.0.clone().styled(TextNode::ITALIC, Toggle))
    }
}
