//! Text handling.

mod deco;
mod misc;
mod quote;
mod quotes;
mod raw;
mod shaping;
mod shift;

pub use self::deco::*;
pub use self::misc::*;
pub use self::quote::*;
pub use self::quotes::*;
pub use self::raw::*;
pub use self::shaping::*;
pub use self::shift::*;

use rustybuzz::Tag;
use ttf_parser::Rect;
use typst::diag::{bail, error, SourceResult};
use typst::font::{Font, FontStretch, FontStyle, FontWeight, VerticalFontMetric};

use crate::layout::ParElem;
use crate::prelude::*;

/// Hook up all text definitions.
pub(super) fn define(global: &mut Scope) {
    global.category("text");
    global.define_elem::<TextElem>();
    global.define_elem::<LinebreakElem>();
    global.define_elem::<SmartquoteElem>();
    global.define_elem::<StrongElem>();
    global.define_elem::<EmphElem>();
    global.define_elem::<SubElem>();
    global.define_elem::<SuperElem>();
    global.define_elem::<UnderlineElem>();
    global.define_elem::<OverlineElem>();
    global.define_elem::<StrikeElem>();
    global.define_elem::<HighlightElem>();
    global.define_elem::<QuoteElem>();
    global.define_elem::<RawElem>();
    global.define_func::<lower>();
    global.define_func::<upper>();
    global.define_func::<smallcaps>();
    global.define_func::<lorem>();
}

/// Customizes the look and layout of text in a variety of ways.
///
/// This function is used frequently, both with set rules and directly. While
/// the set rule is often the simpler choice, calling the `text` function
/// directly can be useful when passing text as an argument to another function.
///
/// # Example
/// ```example
/// #set text(18pt)
/// With a set rule.
///
/// #emph(text(blue)[
///   With a function call.
/// ])
/// ```
#[elem(Construct, PlainText)]
pub struct TextElem {
    /// A prioritized sequence of font families.
    ///
    /// When processing text, Typst tries all specified font families in order
    /// until it finds a font that has the necessary glyphs. In the example
    /// below, the font `Inria Serif` is preferred, but since it does not
    /// contain Arabic glyphs, the arabic text uses `Noto Sans Arabic` instead.
    ///
    /// ```example
    /// #set text(font: (
    ///   "Inria Serif",
    ///   "Noto Sans Arabic",
    /// ))
    ///
    /// This is Latin. \
    /// هذا عربي.
    ///
    /// ```
    #[default(FontList(vec![FontFamily::new("Linux Libertine")]))]
    pub font: FontList,

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
    /// #set text(font: "Inria Serif")
    /// هذا عربي
    ///
    /// #set text(fallback: false)
    /// هذا عربي
    /// ```
    #[default(true)]
    pub fallback: bool,

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
    /// [emph]($emph) function instead. This makes it easy to adapt the style
    /// later if you change your mind about how to signify the emphasis.
    ///
    /// ```example
    /// #text(font: "Linux Libertine", style: "italic")[Italic]
    /// #text(font: "DejaVu Sans", style: "oblique")[Oblique]
    /// ```
    pub style: FontStyle,

    /// The desired thickness of the font's glyphs. Accepts an integer between
    /// `{100}` and `{900}` or one of the predefined weight names. When the
    /// desired weight is not available, Typst selects the font from the family
    /// that is closest in weight.
    ///
    /// If you want to strongly emphasize your text, you should do so using the
    /// [strong]($strong) function instead. This makes it easy to adapt the
    /// style later if you change your mind about how to signify the strong
    /// emphasis.
    ///
    /// ```example
    /// #set text(font: "IBM Plex Sans")
    ///
    /// #text(weight: "light")[Light] \
    /// #text(weight: "regular")[Regular] \
    /// #text(weight: "medium")[Medium] \
    /// #text(weight: 500)[Medium] \
    /// #text(weight: "bold")[Bold]
    /// ```
    pub weight: FontWeight,

    /// The desired width of the glyphs. Accepts a ratio between `{50%}` and
    /// `{200%}`. When the desired width is not available, Typst selects the
    /// font from the family that is closest in stretch. This will only stretch
    /// the text if a condensed or expanded version of the font is available.
    ///
    /// If you want to adjust the amount of space between characters instead of
    /// stretching the glyphs itself, use the [`tracking`]($text.tracking)
    /// property instead.
    ///
    /// ```example
    /// #text(stretch: 75%)[Condensed] \
    /// #text(stretch: 100%)[Normal]
    /// ```
    pub stretch: FontStretch,

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
    #[parse(args.named_or_find("size")?)]
    #[fold]
    #[default(Abs::pt(11.0))]
    pub size: TextSize,

    /// The glyph fill paint.
    ///
    /// ```example
    /// #set text(fill: red)
    /// This text is red.
    /// ```
    #[parse({
        let paint: Option<Spanned<Paint>> = args.named_or_find("fill")?;
        if let Some(paint) = &paint {
            if let Paint::Gradient(gradient) = &paint.v {
                if gradient.relative() == Smart::Custom(Relative::Self_) {
                    bail!(
                        error!(
                            paint.span,
                            "gradients on text must be relative to the parent"
                        )
                        .with_hint("make sure to set `relative: auto` on your text fill")
                    );
                }
            }
        }
        paint.map(|paint| paint.v)
    })]
    #[default(Color::BLACK.into())]
    pub fill: Paint,

    /// The amount of space that should be added between characters.
    ///
    /// ```example
    /// #set text(tracking: 1.5pt)
    /// Distant text.
    /// ```
    #[resolve]
    pub tracking: Length,

    /// The amount of space between words.
    ///
    /// Can be given as an absolute length, but also relative to the width of
    /// the space character in the font.
    ///
    /// If you want to adjust the amount of space between characters rather than
    /// words, use the [`tracking`]($text.tracking) property instead.
    ///
    /// ```example
    /// #set text(spacing: 200%)
    /// Text with distant words.
    /// ```
    #[resolve]
    #[default(Rel::one())]
    pub spacing: Rel<Length>,

    /// An amount to shift the text baseline by.
    ///
    /// ```example
    /// A #text(baseline: 3pt)[lowered]
    /// word.
    /// ```
    #[resolve]
    pub baseline: Length,

    /// Whether certain glyphs can hang over into the margin in justified text.
    /// This can make justification visually more pleasing.
    ///
    /// ```example
    /// #set par(justify: true)
    /// This justified text has a hyphen in
    /// the paragraph's first line. Hanging
    /// the hyphen slightly into the margin
    /// results in a clearer paragraph edge.
    ///
    /// #set text(overhang: false)
    /// This justified text has a hyphen in
    /// the paragraph's first line. Hanging
    /// the hyphen slightly into the margin
    /// results in a clearer paragraph edge.
    /// ```
    #[default(true)]
    pub overhang: bool,

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
    #[default(TopEdge::Metric(TopEdgeMetric::CapHeight))]
    pub top_edge: TopEdge,

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
    #[default(BottomEdge::Metric(BottomEdgeMetric::Baseline))]
    pub bottom_edge: BottomEdge,

    /// An [ISO 639-1/2/3 language code.](https://en.wikipedia.org/wiki/ISO_639)
    ///
    /// Setting the correct language affects various parts of Typst:
    ///
    /// - The text processing pipeline can make more informed choices.
    /// - Hyphenation will use the correct patterns for the language.
    /// - [Smart quotes]($smartquote) turns into the correct quotes for the
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
    #[default(Lang::ENGLISH)]
    pub lang: Lang,

    /// An [ISO 3166-1 alpha-2 region code.](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2)
    ///
    /// This lets the text processing pipeline make more informed choices.
    pub region: Option<Region>,

    /// The OpenType writing script.
    ///
    /// The combination of `{lang}` and `{script}` determine how font features,
    /// such as glyph substitution, are implemented. Frequently the value is a
    /// modified (all-lowercase) ISO 15924 script identifier, and the `math`
    /// writing script is used for features appropriate for mathematical
    /// symbols.
    ///
    /// When set to `{auto}`, the default and recommended setting, an
    /// appropriate script is chosen for each block of characters sharing a
    /// common Unicode script property.
    ///
    /// ```example
    /// #set text(
    ///   font: "Linux Libertine",
    ///   size: 20pt,
    /// )
    ///
    /// #let scedilla = [Ş]
    /// #scedilla // S with a cedilla
    ///
    /// #set text(lang: "ro", script: "latn")
    /// #scedilla // S with a subscript comma
    ///
    /// #set text(lang: "ro", script: "grek")
    /// #scedilla // S with a cedilla
    /// ```
    pub script: Smart<WritingScript>,

    /// The dominant direction for text and inline objects. Possible values are:
    ///
    /// - `{auto}`: Automatically infer the direction from the `lang` property.
    /// - `{ltr}`: Layout text from left to right.
    /// - `{rtl}`: Layout text from right to left.
    ///
    /// When writing in right-to-left scripts like Arabic or Hebrew, you should
    /// set the [text language]($text.lang) or direction. While individual runs
    /// of text are automatically layouted in the correct direction, setting the
    /// dominant direction gives the bidirectional reordering algorithm the
    /// necessary information to correctly place punctuation and inline objects.
    /// Furthermore, setting the direction affects the alignment values `start`
    /// and `end`, which are equivalent to `left` and `right` in `ltr` text and
    /// the other way around in `rtl` text.
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
    #[resolve]
    pub dir: TextDir,

    /// Whether to hyphenate text to improve line breaking. When `{auto}`, text
    /// will be hyphenated if and only if justification is enabled.
    ///
    /// Setting the [text language]($text.lang) ensures that the correct
    /// hyphenation patterns are used.
    ///
    /// ```example
    /// #set page(width: 200pt)
    ///
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
    #[resolve]
    pub hyphenate: Hyphenate,

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
    #[default(true)]
    pub kerning: bool,

    /// Whether to apply stylistic alternates.
    ///
    /// Sometimes fonts contain alternative glyphs for the same codepoint.
    /// Setting this to `{true}` switches to these by enabling the OpenType
    /// `salt` font feature.
    ///
    /// ```example
    /// #set text(
    ///   font: "IBM Plex Sans",
    ///   size: 20pt,
    /// )
    ///
    /// 0, a, g, ß
    ///
    /// #set text(alternates: true)
    /// 0, a, g, ß
    /// ```
    #[default(false)]
    pub alternates: bool,

    /// Which stylistic set to apply. Font designers can categorize alternative
    /// glyphs forms into stylistic sets. As this value is highly font-specific,
    /// you need to consult your font to know which sets are available. When set
    /// to an integer between `{1}` and `{20}`, enables the corresponding
    /// OpenType font feature from `ss01`, ..., `ss20`.
    pub stylistic_set: Option<StylisticSet>,

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
    #[default(true)]
    pub ligatures: bool,

    /// Whether ligatures that should be used sparingly are active. Setting this
    /// to `{true}` enables the OpenType `dlig` font feature.
    #[default(false)]
    pub discretionary_ligatures: bool,

    /// Whether historical ligatures are active. Setting this to `{true}`
    /// enables the OpenType `hlig` font feature.
    #[default(false)]
    pub historical_ligatures: bool,

    /// Which kind of numbers / figures to select. When set to `{auto}`, the
    /// default numbers for the font are used.
    ///
    /// ```example
    /// #set text(font: "Noto Sans", 20pt)
    /// #set text(number-type: "lining")
    /// Number 9.
    ///
    /// #set text(number-type: "old-style")
    /// Number 9.
    /// ```
    pub number_type: Smart<NumberType>,

    /// The width of numbers / figures. When set to `{auto}`, the default
    /// numbers for the font are used.
    ///
    /// ```example
    /// #set text(font: "Noto Sans", 20pt)
    /// #set text(number-width: "proportional")
    /// A 12 B 34. \
    /// A 56 B 78.
    ///
    /// #set text(number-width: "tabular")
    /// A 12 B 34. \
    /// A 56 B 78.
    /// ```
    pub number_width: Smart<NumberWidth>,

    /// Whether to have a slash through the zero glyph. Setting this to `{true}`
    /// enables the OpenType `zero` font feature.
    ///
    /// ```example
    /// 0, #text(slashed-zero: true)[0]
    /// ```
    #[default(false)]
    pub slashed_zero: bool,

    /// Whether to turn numbers into fractions. Setting this to `{true}`
    /// enables the OpenType `frac` font feature.
    ///
    /// It is not advisable to enable this property globally as it will mess
    /// with all appearances of numbers after a slash (e.g., in URLs). Instead,
    /// enable it locally when you want a fraction.
    ///
    /// ```example
    /// 1/2 \
    /// #text(fractions: true)[1/2]
    /// ```
    #[default(false)]
    pub fractions: bool,

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
    #[fold]
    pub features: FontFeatures,

    /// Content in which all text is styled according to the other arguments.
    #[external]
    #[required]
    pub body: Content,

    /// The text.
    #[internal]
    #[required]
    pub text: EcoString,

    /// A delta to apply on the font weight.
    #[internal]
    #[fold]
    pub delta: Delta,

    /// Whether the font style should be inverted.
    #[internal]
    #[fold]
    #[default(false)]
    pub emph: Toggle,

    /// Decorative lines.
    #[internal]
    #[fold]
    pub deco: Decoration,

    /// A case transformation that should be applied to the text.
    #[internal]
    pub case: Option<Case>,

    /// Whether small capital glyphs should be used. ("smcp")
    #[internal]
    #[default(false)]
    pub smallcaps: bool,
}

impl TextElem {
    /// Create a new packed text element.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self::new(text.into()).pack()
    }
}

impl Construct for TextElem {
    fn construct(vm: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        // The text constructor is special: It doesn't create a text element.
        // Instead, it leaves the passed argument structurally unchanged, but
        // styles all text in it.
        let styles = Self::set(vm, args)?;
        let body = args.expect::<Content>("body")?;
        Ok(body.styled_with_map(styles))
    }
}

impl PlainText for TextElem {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text());
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

cast! {
    FontFamily,
    self => self.0.into_value(),
    string: EcoString => Self::new(&string),
}

/// Font family fallback list.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct FontList(pub Vec<FontFamily>);

impl IntoIterator for FontList {
    type IntoIter = std::vec::IntoIter<FontFamily>;
    type Item = FontFamily;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

cast! {
    FontList,
    self => if self.0.len() == 1 {
        self.0.into_iter().next().unwrap().0.into_value()
    } else {
        self.0.into_value()
    },
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

cast! {
    TextSize,
    self => self.0.into_value(),
    v: Length => Self(v),
}

/// Specifies the top edge of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TopEdge {
    /// An edge specified via font metrics or bounding box.
    Metric(TopEdgeMetric),
    /// An edge specified as a length.
    Length(Length),
}

impl TopEdge {
    /// Determine if the edge is specified from bounding box info.
    pub fn is_bounds(&self) -> bool {
        matches!(self, Self::Metric(TopEdgeMetric::Bounds))
    }

    /// Resolve the value of the text edge given a font's metrics.
    pub fn resolve(self, font_size: Abs, font: &Font, bbox: Option<Rect>) -> Abs {
        match self {
            TopEdge::Metric(metric) => {
                if let Ok(metric) = metric.try_into() {
                    font.metrics().vertical(metric).at(font_size)
                } else {
                    bbox.map(|bbox| (font.to_em(bbox.y_max)).at(font_size))
                        .unwrap_or_default()
                }
            }
            TopEdge::Length(length) => length.at(font_size),
        }
    }
}

cast! {
    TopEdge,
    self => match self {
        Self::Metric(metric) => metric.into_value(),
        Self::Length(length) => length.into_value(),
    },
    v: TopEdgeMetric => Self::Metric(v),
    v: Length => Self::Length(v),
}

/// Metrics that describe the top edge of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum TopEdgeMetric {
    /// The font's ascender, which typically exceeds the height of all glyphs.
    Ascender,
    /// The approximate height of uppercase letters.
    CapHeight,
    /// The approximate height of non-ascending lowercase letters.
    XHeight,
    /// The baseline on which the letters rest.
    Baseline,
    /// The top edge of the glyph's bounding box.
    Bounds,
}

impl TryInto<VerticalFontMetric> for TopEdgeMetric {
    type Error = ();

    fn try_into(self) -> Result<VerticalFontMetric, Self::Error> {
        match self {
            Self::Ascender => Ok(VerticalFontMetric::Ascender),
            Self::CapHeight => Ok(VerticalFontMetric::CapHeight),
            Self::XHeight => Ok(VerticalFontMetric::XHeight),
            Self::Baseline => Ok(VerticalFontMetric::Baseline),
            _ => Err(()),
        }
    }
}

/// Specifies the top edge of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BottomEdge {
    /// An edge specified via font metrics or bounding box.
    Metric(BottomEdgeMetric),
    /// An edge specified as a length.
    Length(Length),
}

impl BottomEdge {
    /// Determine if the edge is specified from bounding box info.
    pub fn is_bounds(&self) -> bool {
        matches!(self, Self::Metric(BottomEdgeMetric::Bounds))
    }

    /// Resolve the value of the text edge given a font's metrics.
    pub fn resolve(self, font_size: Abs, font: &Font, bbox: Option<Rect>) -> Abs {
        match self {
            BottomEdge::Metric(metric) => {
                if let Ok(metric) = metric.try_into() {
                    font.metrics().vertical(metric).at(font_size)
                } else {
                    bbox.map(|bbox| (font.to_em(bbox.y_min)).at(font_size))
                        .unwrap_or_default()
                }
            }
            BottomEdge::Length(length) => length.at(font_size),
        }
    }
}

cast! {
    BottomEdge,
    self => match self {
        Self::Metric(metric) => metric.into_value(),
        Self::Length(length) => length.into_value(),
    },
    v: BottomEdgeMetric => Self::Metric(v),
    v: Length => Self::Length(v),
}

/// Metrics that describe the bottom edge of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum BottomEdgeMetric {
    /// The baseline on which the letters rest.
    Baseline,
    /// The font's descender, which typically exceeds the depth of all glyphs.
    Descender,
    /// The bottom edge of the glyph's bounding box.
    Bounds,
}

impl TryInto<VerticalFontMetric> for BottomEdgeMetric {
    type Error = ();

    fn try_into(self) -> Result<VerticalFontMetric, Self::Error> {
        match self {
            Self::Baseline => Ok(VerticalFontMetric::Baseline),
            Self::Descender => Ok(VerticalFontMetric::Descender),
            _ => Err(()),
        }
    }
}

/// The direction of text and inline objects in their line.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextDir(pub Smart<Dir>);

cast! {
    TextDir,
    self => self.0.into_value(),
    v: Smart<Dir> => {
        if v.map_or(false, |dir| dir.axis() == Axis::Y) {
            bail!("text direction must be horizontal");
        }
        Self(v)
    },
}

impl Resolve for TextDir {
    type Output = Dir;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self.0 {
            Smart::Auto => TextElem::lang_in(styles).dir(),
            Smart::Custom(dir) => dir,
        }
    }
}

/// Whether to hyphenate text.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Hyphenate(pub Smart<bool>);

cast! {
    Hyphenate,
    self => self.0.into_value(),
    v: Smart<bool> => Self(v),
}

impl Resolve for Hyphenate {
    type Output = bool;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self.0 {
            Smart::Auto => ParElem::justify_in(styles),
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

cast! {
    StylisticSet,
    self => self.0.into_value(),
    v: i64 => match v {
        1 ..= 20 => Self::new(v as u8),
        _ => bail!("stylistic set must be between 1 and 20"),
    },
}

/// Which kind of numbers / figures to select.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum NumberType {
    /// Numbers that fit well with capital text (the OpenType `lnum`
    /// font feature).
    Lining,
    /// Numbers that fit well into a flow of upper- and lowercase text (the
    /// OpenType `onum` font feature).
    OldStyle,
}

/// The width of numbers / figures.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum NumberWidth {
    /// Numbers with glyph-specific widths (the OpenType `pnum` font feature).
    Proportional,
    /// Numbers of equal width (the OpenType `tnum` font feature).
    Tabular,
}

/// OpenType font features settings.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct FontFeatures(pub Vec<(Tag, u32)>);

cast! {
    FontFeatures,
    self => self.0
        .into_iter()
        .map(|(tag, num)| {
            let bytes = tag.to_bytes();
            let key = std::str::from_utf8(&bytes).unwrap_or_default();
            (key.into(), num.into_value())
        })
        .collect::<Dict>()
        .into_value(),
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
