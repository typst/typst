use smallvec::smallvec;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Show, Smart, StyleChain, TargetElem,
};
use crate::html::{tag, HtmlElem};
use crate::layout::{Abs, Corners, Length, Rel, Sides};
use crate::text::{BottomEdge, BottomEdgeMetric, TextElem, TopEdge, TopEdgeMetric};
use crate::visualize::{Color, FixedStroke, Paint, Stroke};

/// Underlines text.
///
/// # Example
/// ```example
/// This is #underline[important].
/// ```
#[elem(Show)]
pub struct UnderlineElem {
    /// How to [stroke] the line.
    ///
    /// If set to `{auto}`, takes on the text's color and a thickness defined in
    /// the current font.
    ///
    /// ```example
    /// Take #underline(
    ///   stroke: 1.5pt + red,
    ///   offset: 2pt,
    ///   [care],
    /// )
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Smart<Stroke>,

    /// The position of the line relative to the baseline, read from the font
    /// tables if `{auto}`.
    ///
    /// ```example
    /// #underline(offset: 5pt)[
    ///   The Tale Of A Faraway Line I
    /// ]
    /// ```
    #[resolve]
    pub offset: Smart<Length>,

    /// The amount by which to extend the line beyond (or within if negative)
    /// the content.
    ///
    /// ```example
    /// #align(center,
    ///   underline(extent: 2pt)[Chapter 1]
    /// )
    /// ```
    #[resolve]
    pub extent: Length,

    /// Whether the line skips sections in which it would collide with the
    /// glyphs.
    ///
    /// ```example
    /// This #underline(evade: true)[is great].
    /// This #underline(evade: false)[is less great].
    /// ```
    #[default(true)]
    pub evade: bool,

    /// Whether the line is placed behind the content it underlines.
    ///
    /// ```example
    /// #set underline(stroke: (thickness: 1em, paint: maroon, cap: "round"))
    /// #underline(background: true)[This is stylized.] \
    /// #underline(background: false)[This is partially hidden.]
    /// ```
    #[default(false)]
    pub background: bool,

    /// The content to underline.
    #[required]
    pub body: Content,
}

impl Show for Packed<UnderlineElem> {
    #[typst_macros::time(name = "underline", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body.clone();
        Ok(if TargetElem::target_in(styles).is_html() {
            HtmlElem::new(tag::u)
                .with_body(Some(body))
                .pack()
                .spanned(self.span())
        } else {
            body.styled(TextElem::set_deco(smallvec![Decoration {
                line: DecoLine::Underline {
                    stroke: self.stroke(styles).unwrap_or_default(),
                    offset: self.offset(styles),
                    evade: self.evade(styles),
                    background: self.background(styles),
                },
                extent: self.extent(styles),
            }]))
        })
    }
}

/// Adds a line over text.
///
/// # Example
/// ```example
/// #overline[A line over text.]
/// ```
#[elem(Show)]
pub struct OverlineElem {
    /// How to [stroke] the line.
    ///
    /// If set to `{auto}`, takes on the text's color and a thickness defined in
    /// the current font.
    ///
    /// ```example
    /// #set text(fill: olive)
    /// #overline(
    ///   stroke: green.darken(20%),
    ///   offset: -12pt,
    ///   [The Forest Theme],
    /// )
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Smart<Stroke>,

    /// The position of the line relative to the baseline. Read from the font
    /// tables if `{auto}`.
    ///
    /// ```example
    /// #overline(offset: -1.2em)[
    ///   The Tale Of A Faraway Line II
    /// ]
    /// ```
    #[resolve]
    pub offset: Smart<Length>,

    /// The amount by which to extend the line beyond (or within if negative)
    /// the content.
    ///
    /// ```example
    /// #set overline(extent: 4pt)
    /// #set underline(extent: 4pt)
    /// #overline(underline[Typography Today])
    /// ```
    #[resolve]
    pub extent: Length,

    /// Whether the line skips sections in which it would collide with the
    /// glyphs.
    ///
    /// ```example
    /// #overline(
    ///   evade: false,
    ///   offset: -7.5pt,
    ///   stroke: 1pt,
    ///   extent: 3pt,
    ///   [Temple],
    /// )
    /// ```
    #[default(true)]
    pub evade: bool,

    /// Whether the line is placed behind the content it overlines.
    ///
    /// ```example
    /// #set overline(stroke: (thickness: 1em, paint: maroon, cap: "round"))
    /// #overline(background: true)[This is stylized.] \
    /// #overline(background: false)[This is partially hidden.]
    /// ```
    #[default(false)]
    pub background: bool,

    /// The content to add a line over.
    #[required]
    pub body: Content,
}

impl Show for Packed<OverlineElem> {
    #[typst_macros::time(name = "overline", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone().styled(TextElem::set_deco(smallvec![Decoration {
            line: DecoLine::Overline {
                stroke: self.stroke(styles).unwrap_or_default(),
                offset: self.offset(styles),
                evade: self.evade(styles),
                background: self.background(styles),
            },
            extent: self.extent(styles),
        }])))
    }
}

/// Strikes through text.
///
/// # Example
/// ```example
/// This is #strike[not] relevant.
/// ```
#[elem(title = "Strikethrough", Show)]
pub struct StrikeElem {
    /// How to [stroke] the line.
    ///
    /// If set to `{auto}`, takes on the text's color and a thickness defined in
    /// the current font.
    ///
    /// _Note:_ Please don't use this for real redaction as you can still copy
    /// paste the text.
    ///
    /// ```example
    /// This is #strike(stroke: 1.5pt + red)[very stricken through]. \
    /// This is #strike(stroke: 10pt)[redacted].
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Smart<Stroke>,

    /// The position of the line relative to the baseline. Read from the font
    /// tables if `{auto}`.
    ///
    /// This is useful if you are unhappy with the offset your font provides.
    ///
    /// ```example
    /// #set text(font: "Inria Serif")
    /// This is #strike(offset: auto)[low-ish]. \
    /// This is #strike(offset: -3.5pt)[on-top].
    /// ```
    #[resolve]
    pub offset: Smart<Length>,

    /// The amount by which to extend the line beyond (or within if negative)
    /// the content.
    ///
    /// ```example
    /// This #strike(extent: -2pt)[skips] parts of the word.
    /// This #strike(extent: 2pt)[extends] beyond the word.
    /// ```
    #[resolve]
    pub extent: Length,

    /// Whether the line is placed behind the content.
    ///
    /// ```example
    /// #set strike(stroke: red)
    /// #strike(background: true)[This is behind.] \
    /// #strike(background: false)[This is in front.]
    /// ```
    #[default(false)]
    pub background: bool,

    /// The content to strike through.
    #[required]
    pub body: Content,
}

impl Show for Packed<StrikeElem> {
    #[typst_macros::time(name = "strike", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone().styled(TextElem::set_deco(smallvec![Decoration {
            // Note that we do not support evade option for strikethrough.
            line: DecoLine::Strikethrough {
                stroke: self.stroke(styles).unwrap_or_default(),
                offset: self.offset(styles),
                background: self.background(styles),
            },
            extent: self.extent(styles),
        }])))
    }
}

/// Highlights text with a background color.
///
/// # Example
/// ```example
/// This is #highlight[important].
/// ```
#[elem(Show)]
pub struct HighlightElem {
    /// The color to highlight the text with.
    ///
    /// ```example
    /// This is #highlight(
    ///   fill: blue
    /// )[highlighted with blue].
    /// ```
    #[default(Some(Color::from_u8(0xFF, 0xFD, 0x11, 0xA1).into()))]
    pub fill: Option<Paint>,

    /// The highlight's border color. See the
    /// [rectangle's documentation]($rect.stroke) for more details.
    ///
    /// ```example
    /// This is a #highlight(
    ///   stroke: fuchsia
    /// )[stroked highlighting].
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Sides<Option<Option<Stroke>>>,

    /// The top end of the background rectangle.
    ///
    /// ```example
    /// #set highlight(top-edge: "ascender")
    /// #highlight[a] #highlight[aib]
    ///
    /// #set highlight(top-edge: "x-height")
    /// #highlight[a] #highlight[aib]
    /// ```
    #[default(TopEdge::Metric(TopEdgeMetric::Ascender))]
    pub top_edge: TopEdge,

    /// The bottom end of the background rectangle.
    ///
    /// ```example
    /// #set highlight(bottom-edge: "descender")
    /// #highlight[a] #highlight[ap]
    ///
    /// #set highlight(bottom-edge: "baseline")
    /// #highlight[a] #highlight[ap]
    /// ```
    #[default(BottomEdge::Metric(BottomEdgeMetric::Descender))]
    pub bottom_edge: BottomEdge,

    /// The amount by which to extend the background to the sides beyond
    /// (or within if negative) the content.
    ///
    /// ```example
    /// A long #highlight(extent: 4pt)[background].
    /// ```
    #[resolve]
    pub extent: Length,

    /// How much to round the highlight's corners. See the
    /// [rectangle's documentation]($rect.radius) for more details.
    ///
    /// ```example
    /// Listen #highlight(
    ///   radius: 5pt, extent: 2pt
    /// )[carefully], it will be on the test.
    /// ```
    #[resolve]
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// The content that should be highlighted.
    #[required]
    pub body: Content,
}

impl Show for Packed<HighlightElem> {
    #[typst_macros::time(name = "highlight", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone().styled(TextElem::set_deco(smallvec![Decoration {
            line: DecoLine::Highlight {
                fill: self.fill(styles),
                stroke: self
                    .stroke(styles)
                    .unwrap_or_default()
                    .map(|stroke| stroke.map(Stroke::unwrap_or_default)),
                top_edge: self.top_edge(styles),
                bottom_edge: self.bottom_edge(styles),
                radius: self.radius(styles).unwrap_or_default(),
            },
            extent: self.extent(styles),
        }])))
    }
}

/// A text decoration.
///
/// Can be positioned over, under, or on top of text, or highlight the text with
/// a background.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Decoration {
    pub line: DecoLine,
    pub extent: Abs,
}

/// A kind of decorative line.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(clippy::large_enum_variant)]
pub enum DecoLine {
    Underline {
        stroke: Stroke<Abs>,
        offset: Smart<Abs>,
        evade: bool,
        background: bool,
    },
    Strikethrough {
        stroke: Stroke<Abs>,
        offset: Smart<Abs>,
        background: bool,
    },
    Overline {
        stroke: Stroke<Abs>,
        offset: Smart<Abs>,
        evade: bool,
        background: bool,
    },
    Highlight {
        fill: Option<Paint>,
        stroke: Sides<Option<FixedStroke>>,
        top_edge: TopEdge,
        bottom_edge: BottomEdge,
        radius: Corners<Rel<Abs>>,
    },
}
