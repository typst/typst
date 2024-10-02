use std::fmt::{self, Debug, Formatter};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, scope, Args, Cast, Construct, Content, NativeElement, Packed, Set, Smart,
    StyleVec, Unlabellable,
};
use crate::introspection::{Count, CounterUpdate, Locatable};
use crate::layout::{Em, HAlignment, Length, OuterHAlignment};
use crate::model::Numbering;
use crate::utils::singleton;

/// Arranges text, spacing and inline-level elements into a paragraph.
///
/// Although this function is primarily used in set rules to affect paragraph
/// properties, it can also be used to explicitly render its argument onto a
/// paragraph of its own.
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
#[elem(scope, title = "Paragraph", Debug, Construct)]
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
    /// to `-{0.2em}` to get a baseline gap of exactly `{2em}`. The exact
    /// distribution of the top- and bottom-edge values affects the bounds of
    /// the first and last line.
    #[resolve]
    #[ghost]
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
    #[resolve]
    #[ghost]
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
    #[ghost]
    #[default(false)]
    pub justify: bool,

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
    #[ghost]
    pub linebreaks: Smart<Linebreaks>,

    /// The indent the first line of a paragraph should have.
    ///
    /// Only the first line of a consecutive paragraph will be indented (not
    /// the first one in a block or on the page).
    ///
    /// By typographic convention, paragraph breaks are indicated either by some
    /// space between paragraphs or by indented first lines. Consider reducing
    /// the [paragraph spacing]($block.spacing) to the [`leading`]($par.leading)
    /// when using this property (e.g. using `[#set par(spacing: 0.65em)]`).
    #[ghost]
    pub first_line_indent: Length,

    /// The indent all but the first line of a paragraph should have.
    #[ghost]
    #[resolve]
    pub hanging_indent: Length,

    /// Indicates whether an overflowing line should be shrunk.
    ///
    /// This property is set to `false` on raw blocks, because shrinking a line
    /// could visually break the indentation.
    #[ghost]
    #[internal]
    #[default(true)]
    pub shrink: bool,

    /// The contents of the paragraph.
    #[external]
    #[required]
    pub body: Content,

    /// The paragraph's children.
    #[internal]
    #[variadic]
    pub children: StyleVec,
}

#[scope]
impl ParElem {
    #[elem]
    type ParLine;
}

impl Construct for ParElem {
    fn construct(engine: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        // The paragraph constructor is special: It doesn't create a paragraph
        // element. Instead, it just ensures that the passed content lives in a
        // separate paragraph and styles it.
        let styles = Self::set(engine, args)?;
        let body = args.expect::<Content>("body")?;
        Ok(Content::sequence([
            ParbreakElem::shared().clone(),
            body.styled_with_map(styles),
            ParbreakElem::shared().clone(),
        ]))
    }
}

impl Debug for ParElem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Par ")?;
        self.children.fmt(f)
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
/// This element is exclusively used for line number configuration and cannot
/// be placed.
///
/// ```example
/// >>> #set page(margin: (left: 3em))
/// #set par.line(numbering: "1")
///
/// Roses are red. \
/// Violets are blue. \
/// Typst is there for you.
/// ```
#[elem(name = "line", title = "Paragraph Line", keywords = ["line numbering"], Construct, Locatable)]
pub struct ParLine {
    /// How to number each line. Accepts a
    /// [numbering pattern or function]($numbering).
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
    /// The default of `auto` will provide a smart default where numbers grow
    /// horizontally away from the text, considering the margin they're in and
    /// the current text direction.
    ///
    /// ```example
    /// >>> #set page(margin: (left: 3em))
    /// #set par.line(numbering: "I", number-align: left)
    ///
    /// Hello world! \
    /// Today is a beautiful day \
    /// For exploring the world.
    /// ```
    #[ghost]
    pub number_align: Smart<HAlignment>,

    /// The margin at which line numbers appear.
    ///
    /// ```example
    /// >>> #set page(margin: (right: 3em))
    /// #set par.line(numbering: "1", number-margin: right)
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
    ///   number-clearance: 4pt
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
    /// Possible options are `"document"`, indicating the line number counter
    /// is never reset, or `"page"`, indicating it is reset on every page.
    ///
    /// ```example
    /// >>> #set page(margin: (left: 3em))
    /// #set par.line(
    ///   numbering: "1",
    ///   numbering-scope: "page"
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
#[derive(Debug, Cast, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineNumberingScope {
    /// Indicates the line number counter spans the whole document, that is,
    /// is never automatically reset.
    Document,
    /// Indicates the line number counter should be reset at the start of every
    /// new page.
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
