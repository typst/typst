//! Text handling.

mod case;
mod deco;
mod font;
mod item;
mod lang;
mod linebreak;
#[path = "lorem.rs"]
mod lorem_;
mod raw;
mod shift;
#[path = "smallcaps.rs"]
mod smallcaps_;
mod smartquote;
mod space;

pub use self::case::*;
pub use self::deco::*;
pub use self::font::*;
pub use self::item::*;
pub use self::lang::*;
pub use self::linebreak::*;
pub use self::lorem_::*;
pub use self::raw::*;
pub use self::shift::*;
pub use self::smallcaps_::*;
pub use self::smartquote::*;
pub use self::space::*;

use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::str::FromStr;
use std::sync::LazyLock;

use ecow::{eco_format, EcoString};
use icu_properties::sets::CodePointSetData;
use icu_provider::AsDeserializingBufferProvider;
use icu_provider_blob::BlobDataProvider;
use rustybuzz::Feature;
use smallvec::SmallVec;
use ttf_parser::Tag;
use typst_syntax::Spanned;
use typst_utils::singleton;

use crate::diag::{bail, warning, HintedStrResult, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, dict, elem, Args, Array, Cast, Construct, Content, Dict, Fold, IntoValue,
    NativeElement, Never, NoneValue, Packed, PlainText, Regex, Repr, Resolve, Scope, Set,
    Smart, StyleChain,
};
use crate::layout::{Abs, Axis, Dir, Em, Length, Ratio, Rel};
use crate::math::{EquationElem, MathSize};
use crate::visualize::{Color, Paint, RelativeTo, Stroke};
use crate::World;

/// Hook up all `text` definitions.
pub(super) fn define(global: &mut Scope) {
    global.start_category(crate::Category::Text);
    global.define_elem::<TextElem>();
    global.define_elem::<LinebreakElem>();
    global.define_elem::<SmartQuoteElem>();
    global.define_elem::<SubElem>();
    global.define_elem::<SuperElem>();
    global.define_elem::<UnderlineElem>();
    global.define_elem::<OverlineElem>();
    global.define_elem::<StrikeElem>();
    global.define_elem::<HighlightElem>();
    global.define_elem::<SmallcapsElem>();
    global.define_elem::<RawElem>();
    global.define_func::<lower>();
    global.define_func::<upper>();
    global.define_func::<lorem>();
    global.reset_category();
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
#[elem(Debug, Construct, PlainText, Repr)]
pub struct TextElem {
    /// A font family descriptor or priority list of font family descriptors.
    ///
    /// A font family descriptor can be a plain string representing the family
    /// name or a dictionary with the following keys:
    ///
    /// - `name` (required): The font family name.
    /// - `covers` (optional): Defines the Unicode codepoints for which the
    ///   family shall be used. This can be:
    ///   - A predefined coverage set:
    ///     - `{"latin-in-cjk"}` covers all codepoints except for those which
    ///       exist in Latin fonts, but should preferrably be taken from CJK
    ///       fonts.
    ///   - A [regular expression]($regex) that defines exactly which codepoints
    ///     shall be covered. Accepts only the subset of regular expressions
    ///     which consist of exactly one dot, letter, or character class.
    ///
    /// When processing text, Typst tries all specified font families in order
    /// until it finds a font that has the necessary glyphs. In the example
    /// below, the font `Inria Serif` is preferred, but since it does not
    /// contain Arabic glyphs, the arabic text uses `Noto Sans Arabic` instead.
    ///
    /// The collection of available fonts differs by platform:
    ///
    /// - In the web app, you can see the list of available fonts by clicking on
    ///   the "Ag" button. You can provide additional fonts by uploading `.ttf`
    ///   or `.otf` files into your project. They will be discovered
    ///   automatically. The priority is: project fonts > server fonts.
    ///
    /// - Locally, Typst uses your installed system fonts or embedded fonts in
    ///   the CLI, which are `Libertinus Serif`, `New Computer Modern`,
    ///   `New Computer Modern Math`, and `DejaVu Sans Mono`. In addition, you
    ///   can use the `--font-path` argument or `TYPST_FONT_PATHS` environment
    ///   variable to add directories that should be scanned for fonts. The
    ///   priority is: `--font-paths` > system fonts > embedded fonts. Run
    ///   `typst fonts` to see the fonts that Typst has discovered on your
    ///   system. Note that you can pass the `--ignore-system-fonts` parameter
    ///   to the CLI to ensure Typst won't search for system fonts.
    ///
    /// ```example
    /// #set text(font: "PT Sans")
    /// This is sans-serif.
    ///
    /// #set text(font: (
    ///   "Inria Serif",
    ///   "Noto Sans Arabic",
    /// ))
    ///
    /// This is Latin. \
    /// هذا عربي.
    ///
    /// // Change font only for numbers.
    /// #set text(font: (
    ///   (name: "PT Sans", covers: regex("[0-9]")),
    ///   "Libertinus Serif"
    /// ))
    ///
    /// The number 123.
    ///
    /// // Mix Latin and CJK fonts.
    /// #set text(font: (
    ///   (name: "Inria Serif", covers: "latin-in-cjk"),
    ///   "Noto Serif CJK SC"
    /// ))
    /// 分别设置“中文”和English字体
    /// ```
    #[parse({
        let font_list: Option<Spanned<FontList>> = args.named("font")?;
        if let Some(list) = &font_list {
            check_font_list(engine, list);
        }
        font_list.map(|font_list| font_list.v)
    })]
    #[default(FontList(vec![FontFamily::new("Libertinus Serif")]))]
    #[ghost]
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
    #[ghost]
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
    /// If you want to emphasize your text, you should do so using the [emph]
    /// function instead. This makes it easy to adapt the style later if you
    /// change your mind about how to signify the emphasis.
    ///
    /// ```example
    /// #text(font: "Libertinus Serif", style: "italic")[Italic]
    /// #text(font: "DejaVu Sans", style: "oblique")[Oblique]
    /// ```
    #[ghost]
    pub style: FontStyle,

    /// The desired thickness of the font's glyphs. Accepts an integer between
    /// `{100}` and `{900}` or one of the predefined weight names. When the
    /// desired weight is not available, Typst selects the font from the family
    /// that is closest in weight.
    ///
    /// If you want to strongly emphasize your text, you should do so using the
    /// [strong] function instead. This makes it easy to adapt the style later
    /// if you change your mind about how to signify the strong emphasis.
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
    #[ghost]
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
    #[ghost]
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
    #[default(TextSize(Abs::pt(11.0).into()))]
    #[ghost]
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
            if paint.v.relative() == Smart::Custom(RelativeTo::Self_) {
                bail!(
                    paint.span,
                    "gradients and tilings on text must be relative to the parent";
                    hint: "make sure to set `relative: auto` on your text fill"
                );
            }
        }
        paint.map(|paint| paint.v)
    })]
    #[default(Color::BLACK.into())]
    #[ghost]
    pub fill: Paint,

    /// How to stroke the text.
    ///
    /// ```example
    /// #text(stroke: 0.5pt + red)[Stroked]
    /// ```
    #[ghost]
    pub stroke: Option<Stroke>,

    /// The amount of space that should be added between characters.
    ///
    /// ```example
    /// #set text(tracking: 1.5pt)
    /// Distant text.
    /// ```
    #[ghost]
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
    #[default(Rel::one())]
    #[ghost]
    pub spacing: Rel<Length>,

    /// Whether to automatically insert spacing between CJK and Latin characters.
    ///
    /// ```example
    /// #set text(cjk-latin-spacing: auto)
    /// 第4章介绍了基本的API。
    ///
    /// #set text(cjk-latin-spacing: none)
    /// 第4章介绍了基本的API。
    /// ```
    #[ghost]
    pub cjk_latin_spacing: Smart<Option<Never>>,

    /// An amount to shift the text baseline by.
    ///
    /// ```example
    /// A #text(baseline: 3pt)[lowered]
    /// word.
    /// ```
    #[ghost]
    pub baseline: Length,

    /// Whether certain glyphs can hang over into the margin in justified text.
    /// This can make justification visually more pleasing.
    ///
    /// ```example
    /// #set page(width: 220pt)
    ///
    /// #set par(justify: true)
    /// This justified text has a hyphen in
    /// the paragraph's second line. Hanging
    /// the hyphen slightly into the margin
    /// results in a clearer paragraph edge.
    ///
    /// #set text(overhang: false)
    /// This justified text has a hyphen in
    /// the paragraph's second line. Hanging
    /// the hyphen slightly into the margin
    /// results in a clearer paragraph edge.
    /// ```
    #[default(true)]
    #[ghost]
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
    #[ghost]
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
    #[ghost]
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
    #[ghost]
    pub lang: Lang,

    /// An [ISO 3166-1 alpha-2 region code.](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2)
    ///
    /// This lets the text processing pipeline make more informed choices.
    #[ghost]
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
    ///   font: "Libertinus Serif",
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
    #[ghost]
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
    /// output, please get in touch with us through the
    /// [Forum](https://forum.typst.app/),
    /// [Discord server](https://discord.gg/2uDybryKPe),
    /// or our [contact form](https://typst.app/contact).
    ///
    /// ```example
    /// #set text(dir: rtl)
    /// هذا عربي.
    /// ```
    #[ghost]
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
    #[ghost]
    pub hyphenate: Smart<bool>,

    /// The "cost" of various choices when laying out text. A higher cost means
    /// the layout engine will make the choice less often. Costs are specified
    /// as a ratio of the default cost, so `{50%}` will make text layout twice
    /// as eager to make a given choice, while `{200%}` will make it half as
    /// eager.
    ///
    /// Currently, the following costs can be customized:
    /// - `hyphenation`: splitting a word across multiple lines
    /// - `runt`: ending a paragraph with a line with a single word
    /// - `widow`: leaving a single line of paragraph on the next page
    /// - `orphan`: leaving single line of paragraph on the previous page
    ///
    /// Hyphenation is generally avoided by placing the whole word on the next
    /// line, so a higher hyphenation cost can result in awkward justification
    /// spacing. Note: Hyphenation costs will only be applied when the
    /// [`linebreaks`]($par.linebreaks) are set to "optimized". (For example
    /// by default implied by [`justify`]($par.justify).)
    ///
    /// Runts are avoided by placing more or fewer words on previous lines, so a
    /// higher runt cost can result in more awkward in justification spacing.
    ///
    /// Text layout prevents widows and orphans by default because they are
    /// generally discouraged by style guides. However, in some contexts they
    /// are allowed because the prevention method, which moves a line to the
    /// next page, can result in an uneven number of lines between pages. The
    /// `widow` and `orphan` costs allow disabling these modifications.
    /// (Currently, `{0%}` allows widows/orphans; anything else, including the
    /// default of `{100%}`, prevents them. More nuanced cost specification for
    /// these modifications is planned for the future.)
    ///
    /// ```example
    /// #set text(hyphenate: true, size: 11.4pt)
    /// #set par(justify: true)
    ///
    /// #lorem(10)
    ///
    /// // Set hyphenation to ten times the normal cost.
    /// #set text(costs: (hyphenation: 1000%))
    ///
    /// #lorem(10)
    /// ```
    #[fold]
    #[ghost]
    pub costs: Costs,

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
    #[ghost]
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
    #[ghost]
    pub alternates: bool,

    /// Which stylistic sets to apply. Font designers can categorize alternative
    /// glyphs forms into stylistic sets. As this value is highly font-specific,
    /// you need to consult your font to know which sets are available.
    ///
    /// This can be set to an integer or an array of integers, all
    /// of which must be between `{1}` and `{20}`, enabling the
    /// corresponding OpenType feature(s) from `ss01` to `ss20`.
    /// Setting this to `{none}` will disable all stylistic sets.
    ///
    /// ```example
    /// #set text(font: "IBM Plex Serif")
    /// ß vs #text(stylistic-set: 5)[ß] \
    /// 10 years ago vs #text(stylistic-set: (1, 2, 3))[10 years ago]
    /// ```
    #[ghost]
    pub stylistic_set: StylisticSets,

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
    #[ghost]
    pub ligatures: bool,

    /// Whether ligatures that should be used sparingly are active. Setting this
    /// to `{true}` enables the OpenType `dlig` font feature.
    #[default(false)]
    #[ghost]
    pub discretionary_ligatures: bool,

    /// Whether historical ligatures are active. Setting this to `{true}`
    /// enables the OpenType `hlig` font feature.
    #[default(false)]
    #[ghost]
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
    #[ghost]
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
    #[ghost]
    pub number_width: Smart<NumberWidth>,

    /// Whether to have a slash through the zero glyph. Setting this to `{true}`
    /// enables the OpenType `zero` font feature.
    ///
    /// ```example
    /// 0, #text(slashed-zero: true)[0]
    /// ```
    #[default(false)]
    #[ghost]
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
    #[ghost]
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
    #[ghost]
    pub features: FontFeatures,

    /// Content in which all text is styled according to the other arguments.
    #[external]
    #[required]
    pub body: Content,

    /// The text.
    #[required]
    pub text: EcoString,

    /// The offset of the text in the text syntax node referenced by this
    /// element's span.
    #[internal]
    #[ghost]
    pub span_offset: usize,

    /// A delta to apply on the font weight.
    #[internal]
    #[fold]
    #[ghost]
    pub delta: WeightDelta,

    /// Whether the font style should be inverted.
    #[internal]
    #[fold]
    #[default(ItalicToggle(false))]
    #[ghost]
    pub emph: ItalicToggle,

    /// Decorative lines.
    #[internal]
    #[fold]
    #[ghost]
    pub deco: SmallVec<[Decoration; 1]>,

    /// A case transformation that should be applied to the text.
    #[internal]
    #[ghost]
    pub case: Option<Case>,

    /// Whether small capital glyphs should be used. ("smcp", "c2sc")
    #[internal]
    #[ghost]
    pub smallcaps: Option<Smallcaps>,

    /// The configuration for superscripts or subscripts, if one of them is
    /// enabled.
    #[internal]
    #[ghost]
    pub shift_settings: Option<ShiftSettings>,
}

impl TextElem {
    /// Create a new packed text element.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self::new(text.into()).pack()
    }
}

impl Debug for TextElem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Text({})", self.text)
    }
}

impl Repr for TextElem {
    fn repr(&self) -> EcoString {
        eco_format!("[{}]", self.text)
    }
}

impl Construct for TextElem {
    fn construct(engine: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        // The text constructor is special: It doesn't create a text element.
        // Instead, it leaves the passed argument structurally unchanged, but
        // styles all text in it.
        let styles = Self::set(engine, args)?;
        let body = args.expect::<Content>("body")?;
        Ok(body.styled_with_map(styles))
    }
}

impl PlainText for Packed<TextElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text);
    }
}

/// A lowercased font family like "arial".
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct FontFamily {
    // The name of the font family
    name: EcoString,
    // A regex that defines the Unicode codepoints supported by the font.
    covers: Option<Covers>,
}

impl FontFamily {
    /// Create a named font family variant.
    pub fn new(string: &str) -> Self {
        Self::with_coverage(string, None)
    }

    /// Create a font family by name and optional Unicode coverage.
    pub fn with_coverage(string: &str, covers: Option<Covers>) -> Self {
        Self { name: string.to_lowercase().into(), covers }
    }

    /// The lowercased family name.
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// The user-set coverage of the font family.
    pub fn covers(&self) -> Option<&Regex> {
        self.covers.as_ref().map(|covers| covers.as_regex())
    }
}

cast! {
    FontFamily,
    self => self.name.into_value(),
    string: EcoString => Self::new(&string),
    mut v: Dict => {
        let ret = Self::with_coverage(
            &v.take("name")?.cast::<EcoString>()?,
            v.take("covers").ok().map(|v| v.cast()).transpose()?
        );
        v.finish(&["name", "covers"])?;
        ret
    },
}

/// Defines which codepoints a font family will be used for.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Covers {
    /// Covers all codepoints except those used both in Latin and CJK fonts.
    LatinInCjk,
    /// Covers the set of codepoints for which the regex matches.
    Regex(Regex),
}

impl Covers {
    /// Retrieve the regex for the coverage.
    pub fn as_regex(&self) -> &Regex {
        match self {
            Self::LatinInCjk => singleton!(
                Regex,
                Regex::new(
                    "[^\u{00B7}\u{2013}\u{2014}\u{2018}\u{2019}\
                       \u{201C}\u{201D}\u{2025}-\u{2027}\u{2E3A}]"
                )
                .unwrap()
            ),
            Self::Regex(regex) => regex,
        }
    }
}

cast! {
    Covers,
    self => match self {
        Self::LatinInCjk => "latin-in-cjk".into_value(),
        Self::Regex(regex) => regex.into_value(),
    },

    /// Covers all codepoints except those used both in Latin and CJK fonts.
    "latin-in-cjk" => Covers::LatinInCjk,

    regex: Regex => {
        let ast = regex_syntax::ast::parse::Parser::new().parse(regex.as_str());
        match ast {
            Ok(
                regex_syntax::ast::Ast::ClassBracketed(..)
                | regex_syntax::ast::Ast::ClassUnicode(..)
                | regex_syntax::ast::Ast::ClassPerl(..)
                | regex_syntax::ast::Ast::Dot(..)
                | regex_syntax::ast::Ast::Literal(..),
            ) => {}
            _ => bail!(
                "coverage regex may only use dot, letters, and character classes";
                hint: "the regex is applied to each letter individually"
            ),
        }
        Covers::Regex(regex)
    },
}

/// Font family fallback list.
///
/// Must contain at least one font.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct FontList(pub Vec<FontFamily>);

impl FontList {
    pub fn new(fonts: Vec<FontFamily>) -> StrResult<Self> {
        if fonts.is_empty() {
            bail!("font fallback list must not be empty")
        } else {
            Ok(Self(fonts))
        }
    }
}

impl<'a> IntoIterator for &'a FontList {
    type IntoIter = std::slice::Iter<'a, FontFamily>;
    type Item = &'a FontFamily;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

cast! {
    FontList,
    self => if self.0.len() == 1 {
        self.0.into_iter().next().unwrap().name.into_value()
    } else {
        self.0.into_value()
    },
    family: FontFamily => Self(vec![family]),
    values: Array => Self::new(values.into_iter().map(|v| v.cast()).collect::<HintedStrResult<_>>()?)?,
}

/// Resolve a prioritized iterator over the font families.
pub fn families(styles: StyleChain<'_>) -> impl Iterator<Item = &'_ FontFamily> + Clone {
    let fallbacks = singleton!(Vec<FontFamily>, {
        [
            "libertinus serif",
            "twitter color emoji",
            "noto color emoji",
            "apple color emoji",
            "segoe ui emoji",
        ]
        .into_iter()
        .map(FontFamily::new)
        .collect()
    });

    let tail = if styles.get(TextElem::fallback) { fallbacks.as_slice() } else { &[] };
    styles.get_ref(TextElem::font).into_iter().chain(tail.iter())
}

/// Resolve the font variant.
pub fn variant(styles: StyleChain) -> FontVariant {
    let mut variant = FontVariant::new(
        styles.get(TextElem::style),
        styles.get(TextElem::weight),
        styles.get(TextElem::stretch),
    );

    let WeightDelta(delta) = styles.get(TextElem::delta);
    variant.weight = variant
        .weight
        .thicken(delta.clamp(i16::MIN as i64, i16::MAX as i64) as i16);

    if styles.get(TextElem::emph).0 {
        variant.style = match variant.style {
            FontStyle::Normal => FontStyle::Italic,
            FontStyle::Italic => FontStyle::Normal,
            FontStyle::Oblique => FontStyle::Normal,
        }
    }

    variant
}

/// The size of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextSize(pub Length);

impl Fold for TextSize {
    fn fold(self, outer: Self) -> Self {
        // Multiply the two linear functions.
        Self(Length {
            em: Em::new(self.0.em.get() * outer.0.em.get()),
            abs: self.0.em.get() * outer.0.abs + self.0.abs,
        })
    }
}

impl Resolve for TextSize {
    type Output = Abs;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        let factor = match styles.get(EquationElem::size) {
            MathSize::Display | MathSize::Text => 1.0,
            MathSize::Script => styles.get(EquationElem::script_scale).0 as f64 / 100.0,
            MathSize::ScriptScript => {
                styles.get(EquationElem::script_scale).1 as f64 / 100.0
            }
        };
        factor * self.0.resolve(styles)
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
        if v.is_custom_and(|dir| dir.axis() == Axis::Y) {
            bail!("text direction must be horizontal");
        }
        Self(v)
    },
}

impl Resolve for TextDir {
    type Output = Dir;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self.0 {
            Smart::Auto => styles.get(TextElem::lang).dir(),
            Smart::Custom(dir) => dir,
        }
    }
}

/// A set of stylistic sets to enable.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct StylisticSets(u32);

impl StylisticSets {
    /// Converts this set into a Typst array of values.
    pub fn into_array(self) -> Array {
        self.sets().map(IntoValue::into_value).collect()
    }

    /// Returns whether this set contains a particular stylistic set.
    pub fn has(self, ss: u8) -> bool {
        self.0 & (1 << (ss as u32)) != 0
    }

    /// Returns an iterator over all stylistic sets to enable.
    pub fn sets(self) -> impl Iterator<Item = u8> {
        (1..=20).filter(move |i| self.has(*i))
    }
}

cast! {
    StylisticSets,
    self => self.into_array().into_value(),
    _: NoneValue => Self(0),
    v: i64 => match v {
        1 ..= 20 => Self(1 << (v as u32)),
        _ => bail!("stylistic set must be between 1 and 20"),
    },
    v: Vec<i64> => {
        let mut flags = 0;
        for i in v {
            match i {
                1 ..= 20 => flags |= 1 << (i as u32),
                _ => bail!("stylistic set must be between 1 and 20"),
            }
        }
        Self(flags)
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
        .collect::<HintedStrResult<_>>()?),
    values: Dict => Self(values
        .into_iter()
        .map(|(k, v)| {
            let num = v.cast::<u32>()?;
            let tag = Tag::from_bytes_lossy(k.as_bytes());
            Ok((tag, num))
        })
        .collect::<HintedStrResult<_>>()?),
}

impl Fold for FontFeatures {
    fn fold(self, outer: Self) -> Self {
        Self(self.0.fold(outer.0))
    }
}

/// Collect the OpenType features to apply.
pub fn features(styles: StyleChain) -> Vec<Feature> {
    let mut tags = vec![];
    let mut feat = |tag: &[u8; 4], value: u32| {
        tags.push(Feature::new(Tag::from_bytes(tag), value, ..));
    };

    // Features that are on by default in Harfbuzz are only added if disabled.
    if !styles.get(TextElem::kerning) {
        feat(b"kern", 0);
    }

    // Features that are off by default in Harfbuzz are only added if enabled.
    if let Some(sc) = styles.get(TextElem::smallcaps) {
        feat(b"smcp", 1);
        if sc == Smallcaps::All {
            feat(b"c2sc", 1);
        }
    }

    if styles.get(TextElem::alternates) {
        feat(b"salt", 1);
    }

    for set in styles.get(TextElem::stylistic_set).sets() {
        let storage = [b's', b's', b'0' + set / 10, b'0' + set % 10];
        feat(&storage, 1);
    }

    if !styles.get(TextElem::ligatures) {
        feat(b"liga", 0);
        feat(b"clig", 0);
    }

    if styles.get(TextElem::discretionary_ligatures) {
        feat(b"dlig", 1);
    }

    if styles.get(TextElem::historical_ligatures) {
        feat(b"hlig", 1);
    }

    match styles.get(TextElem::number_type) {
        Smart::Auto => {}
        Smart::Custom(NumberType::Lining) => feat(b"lnum", 1),
        Smart::Custom(NumberType::OldStyle) => feat(b"onum", 1),
    }

    match styles.get(TextElem::number_width) {
        Smart::Auto => {}
        Smart::Custom(NumberWidth::Proportional) => feat(b"pnum", 1),
        Smart::Custom(NumberWidth::Tabular) => feat(b"tnum", 1),
    }

    if styles.get(TextElem::slashed_zero) {
        feat(b"zero", 1);
    }

    if styles.get(TextElem::fractions) {
        feat(b"frac", 1);
    }

    match styles.get(EquationElem::size) {
        MathSize::Script => feat(b"ssty", 1),
        MathSize::ScriptScript => feat(b"ssty", 2),
        _ => {}
    }

    for (tag, value) in styles.get_cloned(TextElem::features).0 {
        tags.push(Feature::new(tag, value, ..))
    }

    tags
}

/// Process the language and region of a style chain into a
/// rustybuzz-compatible BCP 47 language.
pub fn language(styles: StyleChain) -> rustybuzz::Language {
    let mut bcp: EcoString = styles.get(TextElem::lang).as_str().into();
    if let Some(region) = styles.get(TextElem::region) {
        bcp.push('-');
        bcp.push_str(region.as_str());
    }
    rustybuzz::Language::from_str(&bcp).unwrap()
}

/// A toggle that turns on and off alternatingly if folded.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ItalicToggle(pub bool);

impl Fold for ItalicToggle {
    fn fold(self, outer: Self) -> Self {
        Self(self.0 ^ outer.0)
    }
}

/// A delta that is summed up when folded.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct WeightDelta(pub i64);

impl Fold for WeightDelta {
    fn fold(self, outer: Self) -> Self {
        Self(outer.0 + self.0)
    }
}

/// Costs for various layout decisions.
///
/// Costs are updated (prioritizing the later value) when folded.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub struct Costs {
    hyphenation: Option<Ratio>,
    runt: Option<Ratio>,
    widow: Option<Ratio>,
    orphan: Option<Ratio>,
}

impl Costs {
    #[must_use]
    pub fn hyphenation(&self) -> Ratio {
        self.hyphenation.unwrap_or(Ratio::one())
    }

    #[must_use]
    pub fn runt(&self) -> Ratio {
        self.runt.unwrap_or(Ratio::one())
    }

    #[must_use]
    pub fn widow(&self) -> Ratio {
        self.widow.unwrap_or(Ratio::one())
    }

    #[must_use]
    pub fn orphan(&self) -> Ratio {
        self.orphan.unwrap_or(Ratio::one())
    }
}

impl Fold for Costs {
    #[inline]
    fn fold(self, outer: Self) -> Self {
        Self {
            hyphenation: self.hyphenation.or(outer.hyphenation),
            runt: self.runt.or(outer.runt),
            widow: self.widow.or(outer.widow),
            orphan: self.orphan.or(outer.orphan),
        }
    }
}

cast! {
    Costs,
    self => dict![
        "hyphenation" => self.hyphenation(),
        "runt" => self.runt(),
        "widow" => self.widow(),
        "orphan" => self.orphan(),
    ].into_value(),
    mut v: Dict => {
        let ret = Self {
            hyphenation: v.take("hyphenation").ok().map(|v| v.cast()).transpose()?,
            runt: v.take("runt").ok().map(|v| v.cast()).transpose()?,
            widow: v.take("widow").ok().map(|v| v.cast()).transpose()?,
            orphan: v.take("orphan").ok().map(|v| v.cast()).transpose()?,
        };
        v.finish(&["hyphenation", "runt", "widow", "orphan"])?;
        ret
    },
}

/// Whether a codepoint is Unicode `Default_Ignorable`.
pub fn is_default_ignorable(c: char) -> bool {
    /// The set of Unicode default ignorables.
    static DEFAULT_IGNORABLE_DATA: LazyLock<CodePointSetData> = LazyLock::new(|| {
        icu_properties::sets::load_default_ignorable_code_point(
            &BlobDataProvider::try_new_from_static_blob(typst_assets::icu::ICU)
                .unwrap()
                .as_deserializing(),
        )
        .unwrap()
    });
    DEFAULT_IGNORABLE_DATA.as_borrowed().contains(c)
}

/// Checks for font families that are not available.
fn check_font_list(engine: &mut Engine, list: &Spanned<FontList>) {
    let book = engine.world.book();
    for family in &list.v {
        match book.select_family(family.as_str()).next() {
            Some(index) => {
                if book
                    .info(index)
                    .is_some_and(|x| x.flags.contains(FontFlags::VARIABLE))
                {
                    engine.sink.warn(warning!(
                        list.span,
                        "variable fonts are not currently supported and may render incorrectly";
                        hint: "try installing a static version of \"{}\" instead", family.as_str()
                    ))
                }
            }
            None => engine.sink.warn(warning!(
                list.span,
                "unknown font family: {}",
                family.as_str(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_elem_size() {
        assert_eq!(std::mem::size_of::<TextElem>(), std::mem::size_of::<EcoString>());
    }
}
