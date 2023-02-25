//! Text handling.

mod deco;
mod misc;
mod quotes;
mod raw;
mod shaping;
mod shift;

pub use self::deco::*;
pub use self::misc::*;
pub use self::quotes::*;
pub use self::raw::*;
pub use self::shaping::*;
pub use self::shift::*;

use std::borrow::Cow;

use rustybuzz::Tag;
use typst::font::{FontMetrics, FontStretch, FontStyle, FontWeight, VerticalFontMetric};

use crate::layout::ParNode;
use crate::prelude::*;

/// # Text
/// Customize the look and layout of text in a variety of ways.
///
/// This function is used often, both with set rules and directly. While the set
/// rule is often the simpler choice, calling the text function directly can be
/// useful when passing text as an argument to another function.
///
/// ## Example
/// ```example
/// #set text(18pt)
/// With a set rule.
///
/// #emph(text(blue)[
///   With a function call.
/// ])
/// ```
///
/// ## Parameters
/// - family: `FallbackList` (positional, named, variadic, settable)
///   A prioritized sequence of font families.
///
///   When processing text, Typst tries all specified font families in order
///   until it finds a font that has the necessary glyphs. In the example below,
///   the font `Inria Serif` is preferred, but since it does not contain Arabic
///   glyphs, the arabic text uses `Noto Sans Arabic` instead.
///
///   ```example
///   #set text(
///     "Inria Serif",
///     "Noto Sans Arabic",
///   )
///
///   This is Latin. \
///   هذا عربي.
///
///   ```
///
/// - body: `Content` (positional, required)
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
    pub const FAMILY: FallbackList =
        FallbackList(vec![FontFamily::new("Linux Libertine")]);

    /// Whether to allow last resort font fallback when the primary font list
    /// contains no match. This lets Typst search through all available fonts
    /// for the most similar one that has the necessary glyphs.
    ///
    /// _Note:_ Currently, there are no warnings when fallback is disabled and
    /// no glyphs are found. Instead, your text shows up in the form of "tofus":
    /// Small boxes that indicate the lack of an appropriate glyph. In the
    /// future, you will be able to instruct Typst to issue warnings so you know
    /// something is up.
    ///
    /// ```example
    /// #set text(family: "Inria Serif")
    /// هذا عربي
    ///
    /// #set text(fallback: false)
    /// هذا عربي
    /// ```
    pub const FALLBACK: bool = true;

    /// The desired font style.
    ///
    /// When an italic style is requested and only an oblique one is available,
    /// it is used. Similarly, the other way around, an italic style can stand
    /// in for an oblique one.  When neither an italic nor an oblique style is
    /// available, Typst selects the normal style. Since most fonts are only
    /// available either in an italic or oblique style, the difference between
    /// italic and oblique style is rarely observable.
    ///
    /// If you want to emphasize your text, you should do so using the
    /// [emph]($func/emph) function instead. This makes it easy to adapt the
    /// style later if you change your mind about how to signify the emphasis.
    ///
    /// ```example
    /// #text("Linux Libertine", style: "italic")[Italic]
    /// #text("DejaVu Sans", style: "oblique")[Oblique]
    /// ```
    pub const STYLE: FontStyle = FontStyle::Normal;

    /// The desired thickness of the font's glyphs. Accepts an integer between
    /// `{100}` and `{900}` or one of the predefined weight names. When the
    /// desired weight is not available, Typst selects the font from the family
    /// that is closest in weight.
    ///
    /// If you want to strongly emphasize your text, you should do so using the
    /// [strong]($func/strong) function instead. This makes it easy to adapt the
    /// style later if you change your mind about how to signify the strong
    /// emphasis.
    ///
    /// ```example
    /// #text(weight: "light")[Light] \
    /// #text(weight: "regular")[Regular] \
    /// #text(weight: "medium")[Medium] \
    /// #text(weight: 500)[Medium] \
    /// #text(weight: "bold")[Bold]
    /// ```
    pub const WEIGHT: FontWeight = FontWeight::REGULAR;

    /// The desired width of the glyphs. Accepts a ratio between `{50%}` and
    /// `{200%}`. When the desired weight is not available, Typst selects the
    /// font from the family that is closest in stretch.
    ///
    /// ```example
    /// #text(stretch: 75%)[Condensed] \
    /// #text(stretch: 100%)[Normal]
    /// ```
    pub const STRETCH: FontStretch = FontStretch::NORMAL;

    /// The size of the glyphs. This value forms the basis of the `em` unit:
    /// `{1em}` is equivalent to the font size.
    ///
    /// You can also give the font size itself in `em` units. Then, it is
    /// relative to the previous font size.
    ///
    /// ```example
    /// #set text(size: 20pt)
    /// very #text(1.5em)[big] text
    /// ```
    #[property(shorthand, fold)]
    pub const SIZE: TextSize = Abs::pt(11.0);

    /// The glyph fill color.
    ///
    /// ```example
    /// #set text(fill: red)
    /// This text is red.
    /// ```
    #[property(shorthand)]
    pub const FILL: Paint = Color::BLACK.into();

    /// The amount of space that should be added between characters.
    ///
    /// ```example
    /// #set text(tracking: 1.5pt)
    /// Distant text.
    /// ```
    #[property(resolve)]
    pub const TRACKING: Length = Length::zero();

    /// The amount of space between words.
    ///
    /// Can be given as an absolute length, but also relative to the width of
    /// the space character in the font.
    ///
    /// ```example
    /// #set text(spacing: 200%)
    /// Text with distant words.
    /// ```
    #[property(resolve)]
    pub const SPACING: Rel<Length> = Rel::one();

    /// An amount to shift the text baseline by.
    ///
    /// ```example
    /// A #text(baseline: 3pt)[lowered]
    /// word.
    /// ```
    #[property(resolve)]
    pub const BASELINE: Length = Length::zero();

    /// Whether certain glyphs can hang over into the margin in justified text.
    /// This can make justification visually more pleasing.
    ///
    /// ```example
    /// #set par(justify: true)
    /// In this particular text, the
    /// justification produces a hyphen
    /// in the first line. Letting this
    /// hyphen hang slightly into the
    /// margin makes for a clear
    /// paragraph edge.
    ///
    /// #set text(overhang: false)
    /// In this particular text, the
    /// justification produces a hyphen
    /// in the first line. This time the
    /// hyphen does not hang into the
    /// margin, making the paragraph's
    /// edge less clear.
    /// ```
    pub const OVERHANG: bool = true;

    /// The top end of the conceptual frame around the text used for layout and
    /// positioning. This affects the size of containers that hold text.
    ///
    /// ```example
    /// #set rect(inset: 0pt)
    /// #set text(size: 20pt)
    ///
    /// #set text(top-edge: "ascender")
    /// #rect(fill: aqua)[Typst]
    ///
    /// #set text(top-edge: "cap-height")
    /// #rect(fill: aqua)[Typst]
    /// ```
    pub const TOP_EDGE: TextEdge = TextEdge::Metric(VerticalFontMetric::CapHeight);

    /// The bottom end of the conceptual frame around the text used for layout
    /// and positioning. This affects the size of containers that hold text.
    ///
    /// ```example
    /// #set rect(inset: 0pt)
    /// #set text(size: 20pt)
    ///
    /// #set text(bottom-edge: "baseline")
    /// #rect(fill: aqua)[Typst]
    ///
    /// #set text(bottom-edge: "descender")
    /// #rect(fill: aqua)[Typst]
    /// ```
    pub const BOTTOM_EDGE: TextEdge = TextEdge::Metric(VerticalFontMetric::Baseline);

    /// An [ISO 639-1/2/3 language code.](https://en.wikipedia.org/wiki/ISO_639)
    ///
    /// Setting the correct language affects various parts of Typst:
    ///
    /// - The text processing pipeline can make more informed choices.
    /// - Hyphenation will use the correct patterns for the language.
    /// - [Smart quotes]($func/smartquote) turns into the correct quotes for the
    ///   language.
    /// - And all other things which are language-aware.
    ///
    /// ```example
    /// #set text(lang: "de")
    /// #outline()
    ///
    /// = Einleitung
    /// In diesem Dokument, ...
    /// ```
    pub const LANG: Lang = Lang::ENGLISH;

    /// An [ISO 3166-1 alpha-2 region code.](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2)
    ///
    /// This lets the text processing pipeline make more informed choices.
    pub const REGION: Option<Region> = None;

    /// The dominant direction for text and inline objects. Possible values are:
    ///
    /// - `{auto}`: Automatically infer the direction from the `lang` property.
    /// - `{ltr}`: Layout text from left to right.
    /// - `{rtl}`: Layout text from right to left.
    ///
    /// When writing in right-to-left scripts like Arabic or Hebrew, you should
    /// set the [text language]($func/text.lang) or direction. While individual
    /// runs of text are automatically layouted in the correct direction,
    /// setting the dominant direction gives the bidirectional reordering
    /// algorithm the necessary information to correctly place punctuation and
    /// inline objects. Furthermore, setting the direction affects the alignment
    /// values `start` and `end`, which are equivalent to `left` and `right` in
    /// `ltr` text and the other way around in `rtl` text.
    ///
    /// If you set this to `rtl` and experience bugs or in some way bad looking
    /// output, please do get in touch with us through the
    /// [contact form](https://typst.app/contact) or our
    /// [Discord server]($community/#discord)!
    ///
    /// ```example
    /// #set text(dir: rtl)
    /// هذا عربي.
    /// ```
    #[property(resolve)]
    pub const DIR: HorizontalDir = HorizontalDir(Smart::Auto);

    /// Whether to hyphenate text to improve line breaking. When `{auto}`, text
    /// will be hyphenated if and only if justification is enabled.
    ///
    /// Setting the [text language]($func/text.lang) ensures that the correct
    /// hyphenation patterns are used.
    ///
    /// ```example
    /// #set par(justify: true)
    /// This text illustrates how
    /// enabling hyphenation can
    /// improve justification.
    ///
    /// #set text(hyphenate: false)
    /// This text illustrates how
    /// enabling hyphenation can
    /// improve justification.
    /// ```
    #[property(resolve)]
    pub const HYPHENATE: Hyphenate = Hyphenate(Smart::Auto);

    /// Whether to apply kerning.
    ///
    /// When enabled, specific letter pairings move closer together or further
    /// apart for a more visually pleasing result. The example below
    /// demonstrates how decreasing the gap between the "T" and "o" results in a
    /// more natural look. Setting this to `{false}` disables kerning by turning
    /// off the OpenType `kern` font feature.
    ///
    /// ```example
    /// #set text(size: 25pt)
    /// Totally
    ///
    /// #set text(kerning: false)
    /// Totally
    /// ```
    pub const KERNING: bool = true;

    /// Whether to apply stylistic alternates.
    ///
    /// Sometimes fonts contain alternative glyphs for the same codepoint.
    /// Setting this to `{true}` switches to these by enabling the OpenType
    /// `salt` font feature.
    ///
    /// ```example
    /// #set text(size: 20pt)
    /// 0, a, g, ß
    ///
    /// #set text(alternates: true)
    /// 0, a, g, ß
    /// ```
    pub const ALTERNATES: bool = false;

    /// Which stylistic set to apply. Font designers can categorize alternative
    /// glyphs forms into stylistic sets. As this value is highly font-specific,
    /// you need to consult your font to know which sets are available. When set
    /// to an integer between `{1}` and `{20}`, enables the corresponding
    /// OpenType font feature from `ss01`, ..., `ss20`.
    pub const STYLISTIC_SET: Option<StylisticSet> = None;

    /// Whether standard ligatures are active.
    ///
    /// Certain letter combinations like "fi" are often displayed as a single
    /// merged glyph called a _ligature._ Setting this to `{false}` disables
    /// these ligatures by turning off the OpenType `liga` and `clig` font
    /// features.
    ///
    /// ```example
    /// #set text(size: 20pt)
    /// A fine ligature.
    ///
    /// #set text(ligatures: false)
    /// A fine ligature.
    /// ```
    pub const LIGATURES: bool = true;

    /// Whether ligatures that should be used sparingly are active. Setting this
    /// to `{true}` enables the OpenType `dlig` font feature.
    pub const DISCRETIONARY_LIGATURES: bool = false;

    /// Whether historical ligatures are active. Setting this to `{true}`
    /// enables the OpenType `hlig` font feature.
    pub const HISTORICAL_LIGATURES: bool = false;

    /// Which kind of numbers / figures to select. When set to `{auto}`, the
    /// default numbers for the font are used.
    ///
    /// ```example
    /// #set text(20pt, "Noto Sans")
    /// #set text(number-type: "lining")
    /// Number 9.
    ///
    /// #set text(number-type: "old-style")
    /// Number 9.
    /// ```
    pub const NUMBER_TYPE: Smart<NumberType> = Smart::Auto;

    /// The width of numbers / figures. When set to `{auto}`, the default
    /// numbers for the font are used.
    ///
    /// ```example
    /// #set text(20pt, "Noto Sans")
    /// #set text(number-width: "proportional")
    /// A 12 B 34. \
    /// A 56 B 78.
    ///
    /// #set text(number-width: "tabular")
    /// A 12 B 34. \
    /// A 56 B 78.
    /// ```
    pub const NUMBER_WIDTH: Smart<NumberWidth> = Smart::Auto;

    /// Whether to have a slash through the zero glyph. Setting this to `{true}`
    /// enables the OpenType `zero` font feature.
    ///
    /// ```example
    /// 0, #text(slashed-zero: true)[0]
    /// ```
    pub const SLASHED_ZERO: bool = false;

    /// Whether to turns numbers into fractions. Setting this to `{true}`
    /// enables the OpenType `frac` font feature.
    ///
    /// ```example
    /// 1/2 \
    /// #text(fractions: true)[1/2]
    /// ```
    pub const FRACTIONS: bool = false;

    /// Raw OpenType features to apply.
    ///
    /// - If given an array of strings, sets the features identified by the
    ///   strings to `{1}`.
    /// - If given a dictionary mapping to numbers, sets the features
    ///   identified by the keys to the values.
    ///
    /// ```example
    /// // Enable the `frac` feature manually.
    /// #set text(features: ("frac",))
    /// 1/2
    /// ```
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
    values: Array => Self(values.into_iter().map(|v| v.cast()).collect::<StrResult<_>>()?),
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
    /// The font's ascender, which typically exceeds the height of all glyphs.
    "ascender" => Self::Metric(VerticalFontMetric::Ascender),
    /// The approximate height of uppercase letters.
    "cap-height" => Self::Metric(VerticalFontMetric::CapHeight),
    /// The approximate height of non-ascending lowercase letters.
    "x-height" => Self::Metric(VerticalFontMetric::XHeight),
    /// The baseline on which the letters rest.
    "baseline" => Self::Metric(VerticalFontMetric::Baseline),
    /// The font's ascender, which typically exceeds the depth of all glyphs.
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
    /// Numbers that fit well with capital text (the OpenType `lnum`
    /// font feature).
    "lining" => Self::Lining,
    /// Numbers that fit well into a flow of upper- and lowercase text (the
    /// OpenType `onum` font feature).
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
    /// Numbers with glyph-specific widths (the OpenType `pnum` font feature).
    "proportional" => Self::Proportional,
    /// Numbers of equal width (the OpenType `tnum` font feature).
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
