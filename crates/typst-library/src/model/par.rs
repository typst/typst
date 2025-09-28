use typst_utils::singleton;

use crate::diag::{SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Args, Cast, Construct, Content, Dict, NativeElement, Packed, Smart, Unlabellable,
    Value, cast, dict, elem, scope,
};
use crate::introspection::{Count, CounterUpdate, Locatable};
use crate::layout::{Em, HAlignment, Length, OuterHAlignment, Rel};
use crate::model::Numbering;

/// A logical subdivison of textual content.
///
/// Typst automatically collects _inline-level_ elements into paragraphs.
/// Inline-level elements include [text], [horizontal spacing]($h),
/// [boxes]($box), and [inline equations]($math.equation).
///
/// To separate paragraphs, use a blank line (or an explicit [`parbreak`]).
/// Paragraphs are also automatically interrupted by any block-level element
/// (like [`block`], [`place`], or anything that shows itself as one of these).
///
/// The `par` element is primarily used in set rules to affect paragraph
/// properties, but it can also be used to explicitly display its argument as a
/// paragraph of its own. Then, the paragraph's body may not contain any
/// block-level content.
///
/// # Boxes and blocks
/// As explained above, usually paragraphs only contain inline-level content.
/// However, you can integrate any kind of block-level content into a paragraph
/// by wrapping it in a [`box`].
///
/// Conversely, you can separate inline-level content from a paragraph by
/// wrapping it in a [`block`]. In this case, it will not become part of any
/// paragraph at all. Read the following section for an explanation of why that
/// matters and how it differs from just adding paragraph breaks around the
/// content.
///
/// # What becomes a paragraph?
/// When you add inline-level content to your document, Typst will automatically
/// wrap it in paragraphs. However, a typical document also contains some text
/// that is not semantically part of a paragraph, for example in a heading or
/// caption.
///
/// The rules for when Typst wraps inline-level content in a paragraph are as
/// follows:
///
/// - All text at the root of a document is wrapped in paragraphs.
///
/// - Text in a container (like a `block`) is only wrapped in a paragraph if the
///   container holds any block-level content. If all of the contents are
///   inline-level, no paragraph is created.
///
/// In the laid-out document, it's not immediately visible whether text became
/// part of a paragraph. However, it is still important for various reasons:
///
/// - Certain paragraph styling like `first-line-indent` will only apply to
///   proper paragraphs, not any text. Similarly, `par` show rules of course
///   only trigger on paragraphs.
///
/// - A proper distinction between paragraphs and other text helps people who
///   rely on assistive technologies (such as screen readers) navigate and
///   understand the document properly. Currently, this only applies to HTML
///   export since Typst does not yet output accessible PDFs, but support for
///   this is planned for the near future.
///
/// - HTML export will generate a `<p>` tag only for paragraphs.
///
/// When creating custom reusable components, you can and should take charge
/// over whether Typst creates paragraphs. By wrapping text in a [`block`]
/// instead of just adding paragraph breaks around it, you can force the absence
/// of a paragraph. Conversely, by adding a [`parbreak`] after some content in a
/// container, you can force it to become a paragraph even if it's just one
/// word. This is, for example, what [non-`tight`]($list.tight) lists do to
/// force their items to become paragraphs.
///
/// # Example
/// ```example
/// #set par(
///   first-line-indent: 1em,
///   spacing: 0.65em,
///   justify: true,
/// )
///
/// We proceed by contradiction.
/// Suppose that there exists a set
/// of positive integers $a$, $b$, and
/// $c$ that satisfies the equation
/// $a^n + b^n = c^n$ for some
/// integer value of $n > 2$.
///
/// Without loss of generality,
/// let $a$ be the smallest of the
/// three integers. Then, we ...
/// ```
#[elem(scope, title = "Paragraph")]
pub struct ParElem {
    /// The spacing between lines.
    ///
    /// Leading defines the spacing between the [bottom edge]($text.bottom-edge)
    /// of one line and the [top edge]($text.top-edge) of the following line. By
    /// default, these two properties are up to the font, but they can also be
    /// configured manually with a text set rule.
    ///
    /// By setting top edge, bottom edge, and leading, you can also configure a
    /// consistent baseline-to-baseline distance. You could, for instance, set
    /// the leading to `{1em}`, the top-edge to `{0.8em}`, and the bottom-edge
    /// to `{-0.2em}` to get a baseline gap of exactly `{2em}`. The exact
    /// distribution of the top- and bottom-edge values affects the bounds of
    /// the first and last line.
    ///
    /// ```preview
    /// // Color palette
    /// #let c = (
    ///   par-line: aqua.transparentize(60%),
    ///   leading-line: blue,
    ///   leading-text: blue.darken(20%),
    ///   spacing-line: orange.mix(red).darken(15%),
    ///   spacing-text: orange.mix(red).darken(20%),
    /// )
    ///
    /// // A sample text for measuring font metrics.
    /// #let sample-text = [A]
    ///
    /// // Number of lines in each paragraph
    /// #let n-lines = (4, 4, 2)
    /// #let annotated-lines = (4, 8)
    ///
    /// // The wide margin is for annotations
    /// #set page(width: 350pt, margin: (x: 20%))
    ///
    /// #context {
    ///   let text-height = measure(sample-text).height
    ///   let line-height = text-height + par.leading.to-absolute()
    ///
    ///   let jumps = n-lines
    ///     .map(n => ((text-height,) * n).intersperse(par.leading))
    ///     .intersperse(par.spacing)
    ///     .flatten()
    ///
    ///   place(grid(
    ///     ..jumps
    ///       .enumerate()
    ///       .map(((i, h)) => if calc.even(i) {
    ///         // Draw a stripe for the line
    ///         block(height: h, width: 100%, fill: c.par-line)
    ///       } else {
    ///         // Put an annotation for the gap
    ///         let sw(a, b) = if h == par.leading { a } else { b }
    ///
    ///         align(end, block(
    ///           height: h,
    ///           outset: (right: sw(0.5em, 1em)),
    ///           stroke: (
    ///             left: none,
    ///             rest: 0.5pt + sw(c.leading-line, c.spacing-line),
    ///           ),
    ///           if i / 2 <= sw(..annotated-lines) {
    ///             place(horizon, dx: 1.3em, text(
    ///               0.8em,
    ///               sw(c.leading-text, c.spacing-text),
    ///               sw([leading], [spacing]),
    ///             ))
    ///           },
    ///         ))
    ///       })
    ///   ))
    ///
    ///   // Mark top and bottom edges
    ///   place(
    ///     // pos: top/bottom edge
    ///     // dy: Δy to the last mark
    ///     // kind: leading/spacing
    ///     for (pos, dy, kind) in (
    ///       (bottom, text-height, "leading"),
    ///       (top, par.leading, "leading"),
    ///       (bottom, (n-lines.first() - 1) * line-height - par.leading, "spacing"),
    ///       (top, par.spacing, "spacing"),
    ///     ) {
    ///       v(dy)
    ///
    ///       let c-text = c.at(kind + "-text")
    ///       let c-line = c.at(kind + "-line")
    ///
    ///       place(end, box(
    ///         height: 0pt,
    ///         grid(
    ///           columns: 2,
    ///           column-gutter: 0.2em,
    ///           align: pos,
    ///           move(
    ///             // Compensate optical illusion
    ///             dy: if pos == top { -0.2em } else { 0.05em },
    ///             text(0.8em, c-text)[#repr(pos) edge],
    ///           ),
    ///           line(length: 1em, stroke: 0.5pt + c-line),
    ///         ),
    ///       ))
    ///     },
    ///   )
    /// }
    ///
    /// #set par(justify: true)
    /// #set text(luma(25%), overhang: false)
    /// #show ". ": it => it + parbreak()
    /// #lorem(55)
    /// ```
    #[default(Em::new(0.65).into())]
    pub leading: Length,

    /// The spacing between paragraphs.
    ///
    /// Just like leading, this defines the spacing between the bottom edge of a
    /// paragraph's last line and the top edge of the next paragraph's first
    /// line.
    ///
    /// When a paragraph is adjacent to a [`block`] that is not a paragraph,
    /// that block's [`above`]($block.above) or [`below`]($block.below) property
    /// takes precedence over the paragraph spacing. Headings, for instance,
    /// reduce the spacing below them by default for a better look.
    #[default(Em::new(1.2).into())]
    pub spacing: Length,

    /// Whether to justify text in its line.
    ///
    /// Hyphenation will be enabled for justified paragraphs if the
    /// [text function's `hyphenate` property]($text.hyphenate) is set to
    /// `{auto}` and the current language is known.
    ///
    /// Note that the current [alignment]($align.alignment) still has an effect
    /// on the placement of the last line except if it ends with a
    /// [justified line break]($linebreak.justify).
    #[default(false)]
    pub justify: bool,

    /// Microtypographical settings that are used during justification.
    pub justification_limits: Option<Smart<JustificationLimits>>,

    /// How to determine line breaks.
    ///
    /// When this property is set to `{auto}`, its default value, optimized line
    /// breaks will be used for justified paragraphs. Enabling optimized line
    /// breaks for ragged paragraphs may also be worthwhile to improve the
    /// appearance of the text.
    ///
    /// ```example
    /// #set page(width: 207pt)
    /// #set par(linebreaks: "simple")
    /// Some texts feature many longer
    /// words. Those are often exceedingly
    /// challenging to break in a visually
    /// pleasing way.
    ///
    /// #set par(linebreaks: "optimized")
    /// Some texts feature many longer
    /// words. Those are often exceedingly
    /// challenging to break in a visually
    /// pleasing way.
    /// ```
    pub linebreaks: Smart<Linebreaks>,

    /// The indent the first line of a paragraph should have.
    ///
    /// By default, only the first line of a consecutive paragraph will be
    /// indented (not the first one in the document or container, and not
    /// paragraphs immediately following other block-level elements).
    ///
    /// If you want to indent all paragraphs instead, you can pass a dictionary
    /// containing the `amount` of indent as a length and the pair
    /// `{all: true}`. When `all` is omitted from the dictionary, it defaults to
    /// `{false}`.
    ///
    /// By typographic convention, paragraph breaks are indicated either by some
    /// space between paragraphs or by indented first lines. Consider
    /// - reducing the [paragraph `spacing`]($par.spacing) to the
    ///   [`leading`]($par.leading) using `{set par(spacing: 0.65em)}`
    /// - increasing the [block `spacing`]($block.spacing) (which inherits the
    ///   paragraph spacing by default) to the original paragraph spacing using
    ///   `{set block(spacing: 1.2em)}`
    ///
    /// ```example
    /// #set block(spacing: 1.2em)
    /// #set par(
    ///   first-line-indent: 1.5em,
    ///   spacing: 0.65em,
    /// )
    ///
    /// The first paragraph is not affected
    /// by the indent.
    ///
    /// But the second paragraph is.
    ///
    /// #line(length: 100%)
    ///
    /// #set par(first-line-indent: (
    ///   amount: 1.5em,
    ///   all: true,
    /// ))
    ///
    /// Now all paragraphs are affected
    /// by the first line indent.
    ///
    /// Even the first one.
    /// ```
    pub first_line_indent: FirstLineIndent,

    /// The indent that all but the first line of a paragraph should have.
    ///
    /// ```example
    /// #set par(hanging-indent: 1em)
    ///
    /// #lorem(15)
    /// ```
    pub hanging_indent: Length,

    /// The contents of the paragraph.
    #[required]
    pub body: Content,
}

#[scope]
impl ParElem {
    #[elem]
    type ParLine;
}

/// Configuration for microtypographical settings to be used during
/// justification.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash)]
pub struct JustificationLimits {
    /// Minimum allowable word spacing.
    pub word_min: Rel,
    /// Maximum allowable word spacing.
    pub word_max: Rel,
    /// Minimum allowable glyph spacing.
    pub glyph_min: Rel,
    /// Maximum allowable glyph spacing.
    pub glyph_max: Rel,
}

cast! {
    JustificationLimits,
    self => Value::Dict(self.into()),
    mut dict: Dict => {
        let mut word: Dict = dict.take("word")?.cast()?;
        let word_min = word.take("min")?.cast()?;
        let word_max = word.take("max")?.cast()?;

        let mut glyph: Dict = dict.take("glyph")?.cast()?;
        let glyph_min = glyph.take("min")?.cast()?;
        let glyph_max = glyph.take("max")?.cast()?;

        dict.finish(&["word", "glyph"])?;
        Self {
            word_min,
            word_max,
            glyph_min,
            glyph_max
        }
    },
}

impl From<JustificationLimits> for Dict {
    fn from(justification_limits: JustificationLimits) -> Self {
        dict! {
            "word" => dict! {
                "min" => justification_limits.word_min,
                "max" => justification_limits.word_max,
            },
            "glyph" => dict! {
                "min" => justification_limits.glyph_min,
                "max" => justification_limits.glyph_max,
            },
        }
    }
}

/// How to determine line breaks in a paragraph.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Linebreaks {
    /// Determine the line breaks in a simple first-fit style.
    Simple,
    /// Optimize the line breaks for the whole paragraph.
    ///
    /// Typst will try to produce more evenly filled lines of text by
    /// considering the whole paragraph when calculating line breaks.
    Optimized,
}

/// Configuration for first line indent.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash)]
pub struct FirstLineIndent {
    /// The amount of indent.
    pub amount: Length,
    /// Whether to indent all paragraphs, not just consecutive ones.
    pub all: bool,
}

cast! {
    FirstLineIndent,
    self => Value::Dict(self.into()),
    amount: Length => Self { amount, all: false },
    mut dict: Dict => {
        let amount = dict.take("amount")?.cast()?;
        let all = dict.take("all").ok().map(|v| v.cast()).transpose()?.unwrap_or(false);
        dict.finish(&["amount", "all"])?;
        Self { amount, all }
    },
}

impl From<FirstLineIndent> for Dict {
    fn from(indent: FirstLineIndent) -> Self {
        dict! {
            "amount" => indent.amount,
            "all" => indent.all,
        }
    }
}

/// A paragraph break.
///
/// This starts a new paragraph. Especially useful when used within code like
/// [for loops]($scripting/#loops). Multiple consecutive
/// paragraph breaks collapse into a single one.
///
/// # Example
/// ```example
/// #for i in range(3) {
///   [Blind text #i: ]
///   lorem(5)
///   parbreak()
/// }
/// ```
///
/// # Syntax
/// Instead of calling this function, you can insert a blank line into your
/// markup to create a paragraph break.
#[elem(title = "Paragraph Break", Unlabellable)]
pub struct ParbreakElem {}

impl ParbreakElem {
    /// Get the globally shared paragraph element.
    pub fn shared() -> &'static Content {
        singleton!(Content, ParbreakElem::new().pack())
    }
}

impl Unlabellable for Packed<ParbreakElem> {}

/// A paragraph line.
///
/// This element is exclusively used for line number configuration through set
/// rules and cannot be placed.
///
/// The [`numbering`]($par.line.numbering) option is used to enable line
/// numbers by specifying a numbering format.
///
/// ```example
/// >>> #set page(margin: (left: 3em))
/// #set par.line(numbering: "1")
///
/// Roses are red. \
/// Violets are blue. \
/// Typst is there for you.
/// ```
///
/// The `numbering` option takes either a predefined
/// [numbering pattern]($numbering) or a function returning styled content. You
/// can disable line numbers for text inside certain elements by setting the
/// numbering to `{none}` using show-set rules.
///
/// ```example
/// >>> #set page(margin: (left: 3em))
/// // Styled red line numbers.
/// #set par.line(
///   numbering: n => text(red)[#n]
/// )
///
/// // Disable numbers inside figures.
/// #show figure: set par.line(
///   numbering: none
/// )
///
/// Roses are red. \
/// Violets are blue.
///
/// #figure(
///   caption: [Without line numbers.]
/// )[
///   Lorem ipsum \
///   dolor sit amet
/// ]
///
/// The text above is a sample \
/// originating from distant times.
/// ```
///
/// This element exposes further options which may be used to control other
/// aspects of line numbering, such as its [alignment]($par.line.number-align)
/// or [margin]($par.line.number-margin). In addition, you can control whether
/// the numbering is reset on each page through the
/// [`numbering-scope`]($par.line.numbering-scope) option.
#[elem(name = "line", title = "Paragraph Line", keywords = ["line numbering"], Construct, Locatable)]
pub struct ParLine {
    /// How to number each line. Accepts a
    /// [numbering pattern or function]($numbering) taking a single number.
    ///
    /// ```example
    /// >>> #set page(margin: (left: 3em))
    /// #set par.line(numbering: "I")
    ///
    /// Roses are red. \
    /// Violets are blue. \
    /// Typst is there for you.
    /// ```
    #[ghost]
    pub numbering: Option<Numbering>,

    /// The alignment of line numbers associated with each line.
    ///
    /// The default of `{auto}` indicates a smart default where numbers grow
    /// horizontally away from the text, considering the margin they're in and
    /// the current text direction.
    ///
    /// ```example
    /// >>> #set page(margin: (left: 3em))
    /// #set par.line(
    ///   numbering: "I",
    ///   number-align: left,
    /// )
    ///
    /// Hello world! \
    /// Today is a beautiful day \
    /// For exploring the world.
    /// ```
    #[ghost]
    pub number_align: Smart<HAlignment>,

    /// The margin at which line numbers appear.
    ///
    /// _Note:_ In a multi-column document, the line numbers for paragraphs
    /// inside the last column will always appear on the `{end}` margin (right
    /// margin for left-to-right text and left margin for right-to-left),
    /// regardless of this configuration. That behavior cannot be changed at
    /// this moment.
    ///
    /// ```example
    /// >>> #set page(margin: (right: 3em))
    /// #set par.line(
    ///   numbering: "1",
    ///   number-margin: right,
    /// )
    ///
    /// = Report
    /// - Brightness: Dark, yet darker
    /// - Readings: Negative
    /// ```
    #[ghost]
    #[default(OuterHAlignment::Start)]
    pub number_margin: OuterHAlignment,

    /// The distance between line numbers and text.
    ///
    /// The default value of `{auto}` results in a clearance that is adaptive to
    /// the page width and yields reasonable results in most cases.
    ///
    /// ```example
    /// >>> #set page(margin: (left: 3em))
    /// #set par.line(
    ///   numbering: "1",
    ///   number-clearance: 4pt,
    /// )
    ///
    /// Typesetting \
    /// Styling \
    /// Layout
    /// ```
    #[ghost]
    #[default]
    pub number_clearance: Smart<Length>,

    /// Controls when to reset line numbering.
    ///
    /// _Note:_ The line numbering scope must be uniform across each page run (a
    /// page run is a sequence of pages without an explicit pagebreak in
    /// between). For this reason, set rules for it should be defined before any
    /// page content, typically at the very start of the document.
    ///
    /// ```example
    /// >>> #set page(margin: (left: 3em))
    /// #set par.line(
    ///   numbering: "1",
    ///   numbering-scope: "page",
    /// )
    ///
    /// First line \
    /// Second line
    /// #pagebreak()
    /// First line again \
    /// Second line again
    /// ```
    #[ghost]
    #[default(LineNumberingScope::Document)]
    pub numbering_scope: LineNumberingScope,
}

impl Construct for ParLine {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

/// Possible line numbering scope options, indicating how often the line number
/// counter should be reset.
///
/// Note that, currently, manually resetting the line number counter is not
/// supported.
#[derive(Debug, Cast, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineNumberingScope {
    /// Indicates that the line number counter spans the whole document, i.e.,
    /// it's never automatically reset.
    Document,
    /// Indicates that the line number counter should be reset at the start of
    /// every new page.
    Page,
}

/// A marker used to indicate the presence of a line.
///
/// This element is added to each line in a paragraph and later searched to
/// find out where to add line numbers.
#[elem(Construct, Locatable, Count)]
pub struct ParLineMarker {
    #[internal]
    #[required]
    pub numbering: Numbering,

    #[internal]
    #[required]
    pub number_align: Smart<HAlignment>,

    #[internal]
    #[required]
    pub number_margin: OuterHAlignment,

    #[internal]
    #[required]
    pub number_clearance: Smart<Length>,
}

impl Construct for ParLineMarker {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

impl Count for Packed<ParLineMarker> {
    fn update(&self) -> Option<CounterUpdate> {
        // The line counter must be updated manually by the root flow.
        None
    }
}
