#![allow(clippy::too_many_arguments)]
use std::cell::Cell;
use std::ops::MulAssign;

use bumpalo::{Bump, boxed::Box as BumpBox, collections::Vec as BumpVec};
use ecow::EcoString;
use typst_syntax::Span;
use typst_utils::{Get, default_math_class};
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;

use super::preprocess::preprocess;
use crate::foundations::{Content, Packed, Smart, StyleChain};
use crate::introspection::Tag;
use crate::layout::{Abs, Axes, Axis, BoxElem, Em, FixedAlignment, PlaceElem, Rel};
use crate::math::{
    Augment, CancelAngle, EquationElem, LeftRightAlternator, Limits, MathSize,
};
use crate::visualize::FixedStroke;

/// The top-level item in the math IR.
#[derive(Debug)]
pub enum MathItem<'a> {
    /// A layoutable component with associated properties and styles.
    Component(MathComponent<'a>),
    /// Explicit spacing. The boolean indicates whether the spacing is weak.
    Spacing(Abs, bool),
    /// A regular space.
    Space,
    /// A line break.
    Linebreak,
    /// An alignment point.
    Align,
    /// An introspection tag.
    Tag(Tag),
}

impl<'a> From<MathComponent<'a>> for MathItem<'a> {
    fn from(comp: MathComponent<'a>) -> MathItem<'a> {
        MathItem::Component(comp)
    }
}

impl<'a> MathItem<'a> {
    /// Returns the limit placement configuration for this item.
    pub(crate) fn limits(&self) -> Limits {
        match self {
            Self::Component(comp) => comp.props.limits,
            _ => Limits::Never,
        }
    }

    /// Returns the math class of this item.
    pub(crate) fn class(&self) -> MathClass {
        match self {
            Self::Component(comp) => comp.props.class,
            Self::Spacing(_, _) | Self::Space | Self::Linebreak => MathClass::Space,
            Self::Align | Self::Tag(_) => MathClass::Special,
        }
    }

    /// Returns the effective math class on the right side of this item.
    ///
    /// For fenced items with a closing delimiter, this returns the closing
    /// class instead of the item's overall class.
    pub(crate) fn rclass(&self) -> MathClass {
        match self {
            Self::Component(MathComponent { kind: MathKind::Fenced(fence), .. })
                if fence.close.is_some() =>
            {
                MathClass::Closing
            }
            _ => self.class(),
        }
    }

    /// Returns the effective math class on the left side of this item.
    ///
    /// For fenced items with an opening delimiter, this returns the opening
    /// class instead of the item's overall class.
    pub(crate) fn lclass(&self) -> MathClass {
        match self {
            Self::Component(MathComponent { kind: MathKind::Fenced(fence), .. })
                if fence.open.is_some() =>
            {
                MathClass::Opening
            }
            _ => self.class(),
        }
    }

    /// Returns the math size of this item, if it is a component.
    pub(crate) fn size(&self) -> Option<MathSize> {
        match self {
            Self::Component(comp) => Some(comp.props.size),
            _ => None,
        }
    }

    /// Whether this item should have explicit spaces around it.
    pub(crate) fn is_spaced(&self) -> bool {
        if self.class() == MathClass::Fence {
            return true;
        }

        if let Self::Component(comp) = self
            && comp.props.spaced
            && matches!(comp.props.class, MathClass::Normal | MathClass::Alphabetic)
        {
            true
        } else {
            false
        }
    }

    /// Whether this item should be ignored for spacing calculations.
    pub(crate) fn is_ignorant(&self) -> bool {
        match self {
            Self::Component(comp) => comp.props.ignorant,
            Self::Tag(_) => true,
            _ => false,
        }
    }

    /// Returns the source span of this item.
    pub fn span(&self) -> Span {
        match self {
            Self::Component(comp) => comp.props.span,
            _ => Span::detached(),
        }
    }

    /// Returns the style chain of this item, if it is a component.
    pub fn styles(&self) -> Option<StyleChain<'a>> {
        match self {
            Self::Component(comp) => Some(comp.styles),
            _ => None,
        }
    }

    /// Returns whether this glyph has been stretched as a middle delimiter.
    pub fn mid_stretched(&self) -> Option<bool> {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.mid_stretched.get()
        } else {
            None
        }
    }

    /// Whether this item contains multiple lines.
    pub fn is_multiline(&self) -> bool {
        let items = self.as_slice();
        let len = items.len();
        for (i, item) in items.iter().enumerate() {
            let is_last = i == len - 1;

            match item {
                // If it's a linebreak and not the last item, it counts.
                MathItem::Linebreak if !is_last => return true,
                MathItem::Component(MathComponent {
                    kind: MathKind::Fenced(fence),
                    ..
                }) => {
                    // Check for linebreak in the middle of the body, e.g.
                    // `(a \ b)`.
                    if fence.body.is_multiline() {
                        return true;
                    }

                    // The above check leaves out `(a \ )` and `(a \`, in the
                    // former case it should always count, but in the latter
                    // case it should only count if this isn't the last item.
                    if fence.body.ends_with_linebreak()
                        && (fence.close.is_some() || !is_last)
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }

    /// Whether this item ends with a line break.
    fn ends_with_linebreak(&self) -> bool {
        match self.as_slice().last() {
            Some(MathItem::Linebreak) => true,
            Some(MathItem::Component(MathComponent {
                kind: MathKind::Fenced(fence),
                ..
            })) if fence.close.is_none() => fence.body.ends_with_linebreak(),
            _ => false,
        }
    }

    /// Returns the inner items if this is a group, or a slice containing
    /// just this item otherwise.
    pub fn as_slice(&self) -> &[MathItem<'a>] {
        if let MathItem::Component(comp) = self
            && let MathKind::Group(group) = &comp.kind
        {
            &group.items
        } else {
            core::slice::from_ref(self)
        }
    }

    /// Sets the limit placement configuration for this item.
    pub(crate) fn set_limits(&mut self, limits: Limits) {
        if let Self::Component(comp) = self {
            comp.props.limits = limits;
        }
    }

    /// Sets the math class of this item.
    pub(crate) fn set_class(&mut self, class: MathClass) {
        if let Self::Component(comp) = self {
            comp.props.class = class;
        }
    }

    /// Sets the left spacing for this item.
    pub(crate) fn set_lspace(&mut self, lspace: Option<Em>) {
        if let Self::Component(comp) = self {
            comp.props.lspace = lspace;
        }
    }

    /// Sets the right spacing for this item.
    pub(crate) fn set_rspace(&mut self, rspace: Option<Em>) {
        if let Self::Component(comp) = self {
            comp.props.rspace = rspace;
        }
    }

    /// Sets whether this glyph has been stretched as a middle delimiter.
    pub(crate) fn set_mid_stretched(&self, mid_stretched: Option<bool>) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.mid_stretched.set(mid_stretched);
        }
    }

    /// Sets the stretch configuration for this glyph.
    pub(crate) fn set_stretch(&self, stretch: Stretch) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.replace(stretch);
        }
    }

    /// Updates the vertical stretch info for this glyph.
    pub(crate) fn set_y_stretch(&self, info: StretchInfo) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.update(|stretch| stretch.with_y(info));
        }
    }

    /// Updates the stretch info for both axes of this glyph.
    pub(crate) fn update_stretch(&self, info: StretchInfo) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.update(|stretch| stretch.update(info));
        }
    }

    /// Sets the reference size for relative stretching on the given axis.
    pub fn set_stretch_relative_to(&self, relative_to: Abs, axis: Axis) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.update(|stretch| stretch.relative_to(relative_to, axis));
        }
    }

    /// Sets the font size to use for short-fall calculations on the given axis.
    pub fn set_stretch_font_size(&self, font_size: Abs, axis: Axis) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.update(|stretch| stretch.font_size(font_size, axis));
        }
    }

    /// Enables the flac OpenType feature for this glyph.
    pub fn set_flac(&self) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.flac.set(true);
        }
    }
}

/// A generic component that bundles a specific math item kind with common
/// properties and styles.
#[derive(Debug)]
pub struct MathComponent<'a> {
    /// The specific kind of math item.
    pub kind: MathKind<'a>,
    /// The properties attached to this component.
    pub props: MathProperties,
    /// The item's styles.
    pub styles: StyleChain<'a>,
}

/// The specific kind of a layoutable math item.
///
/// Recursive or large variants are boxed (allocated in a bump arena).
#[derive(Debug)]
pub enum MathKind<'a> {
    /// A group of math items laid out horizontally.
    Group(GroupItem<'a>),
    /// A radical (square root or nth root).
    Radical(BumpBox<'a, RadicalItem<'a>>),
    /// An item enclosed in delimiters.
    Fenced(BumpBox<'a, FencedItem<'a>>),
    /// A vertical fraction.
    Fraction(BumpBox<'a, FractionItem<'a>>),
    /// An inline skewed fraction.
    SkewedFraction(BumpBox<'a, SkewedFractionItem<'a>>),
    /// A 2D collection of math items laid out as a table/matrix.
    Table(BumpBox<'a, TableItem<'a>>),
    /// A base with scripts (subscripts/superscripts) and/or limits attached.
    Scripts(BumpBox<'a, ScriptsItem<'a>>),
    /// A base with an accent mark above or below.
    Accent(BumpBox<'a, AccentItem<'a>>),
    /// A base with a line overlaid.
    Cancel(BumpBox<'a, CancelItem<'a>>),
    /// A base with a line drawn above or below.
    Line(BumpBox<'a, LineItem<'a>>),
    /// Grouped prime symbols.
    Primes(BumpBox<'a, PrimesItem<'a>>),
    /// A text string.
    Text(TextItem<'a>),
    /// A single glyph (grapheme cluster).
    Glyph(BumpBox<'a, GlyphItem>),
    /// Inline content.
    Box(BoxItem<'a>),
    /// External content that needs to be laid out separately.
    External(ExternalItem<'a>),
}

/// Shared properties for all layoutable math components.
#[derive(Debug, Copy, Clone)]
pub struct MathProperties {
    /// How attachments should be positioned.
    pub(crate) limits: Limits,
    /// The math class.
    pub class: MathClass,
    /// The current math size.
    pub size: MathSize,
    /// Whether this item should be ignored for spacing calculations.
    pub ignorant: bool,
    /// Whether this item should have explicit spaces around it.
    pub(crate) spaced: bool,
    /// The amount of spacing to the left of this item.
    pub lspace: Option<Em>,
    /// The amount of spacing to the right of this item.
    pub rspace: Option<Em>,
    /// The source span.
    pub span: Span,
}

impl MathProperties {
    /// Creates default properties from the given styles.
    ///
    /// This gets both the math class and size from the styles.
    pub fn default(styles: StyleChain) -> MathProperties {
        Self {
            limits: Limits::Never,
            class: styles.get(EquationElem::class).unwrap_or(MathClass::Normal),
            size: styles.get(EquationElem::size),
            ignorant: false,
            spaced: false,
            lspace: None,
            rspace: None,
            span: Span::detached(),
        }
    }

    /// Creates properties with an explicit class, avoiding the style lookup.
    fn with_explicit_class(styles: StyleChain, class: MathClass) -> MathProperties {
        Self {
            limits: Limits::Never,
            class,
            size: styles.get(EquationElem::size),
            ignorant: false,
            spaced: false,
            lspace: None,
            rspace: None,
            span: Span::detached(),
        }
    }

    /// Creates properties with explicit limits and class, avoiding style lookups.
    fn with_explicit_limits_and_class(
        styles: StyleChain,
        limits: Limits,
        class: MathClass,
    ) -> MathProperties {
        Self {
            limits,
            class,
            size: styles.get(EquationElem::size),
            ignorant: false,
            spaced: false,
            lspace: None,
            rspace: None,
            span: Span::detached(),
        }
    }

    /// Sets whether this item should be ignored for spacing calculations.
    fn with_ignorant(mut self, ignorant: bool) -> Self {
        self.ignorant = ignorant;
        self
    }

    /// Sets whether this item should have explicit spaces around it.
    fn with_spaced(mut self, spaced: bool) -> Self {
        self.spaced = spaced;
        self
    }

    /// Sets the source span for this item.
    fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }
}

/// A group of math items laid out horizontally.
#[derive(Debug)]
pub struct GroupItem<'a> {
    /// The items in the group.
    pub items: BumpBox<'a, [MathItem<'a>]>,
}

impl<'a> GroupItem<'a> {
    /// Creates a new group item from the given items.
    ///
    /// The items are preprocessed to calculate spacing between them. The
    /// `closing_exists` parameter indicates whether a closing delimiter
    /// will follow the group of items.
    pub(crate) fn create<I>(
        items: I,
        closing_exists: bool,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a>
    where
        I: IntoIterator<Item = MathItem<'a>>,
        I::IntoIter: ExactSizeIterator,
    {
        let props = MathProperties::default(styles);
        let kind =
            MathKind::Group(Self { items: preprocess(items, bump, closing_exists) });
        MathComponent { kind, props, styles }.into()
    }
}

/// A radical (square root or nth root).
#[derive(Debug)]
pub struct RadicalItem<'a> {
    /// The item under the radical symbol.
    pub radicand: MathItem<'a>,
    /// The index for nth roots. `None` for square roots.
    pub index: Option<MathItem<'a>>,
    /// The radical symbol.
    ///
    /// Only used in paged export.
    pub sqrt: MathItem<'a>,
}

impl<'a> RadicalItem<'a> {
    /// Creates a new radical item.
    pub(crate) fn create(
        radicand: MathItem<'a>,
        index: Option<MathItem<'a>>,
        sqrt: MathItem<'a>,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind =
            MathKind::Radical(BumpBox::new_in(Self { radicand, index, sqrt }, bump));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

/// An item enclosed in delimiters.
#[derive(Debug)]
pub struct FencedItem<'a> {
    /// The optional opening delimiter.
    pub open: Option<MathItem<'a>>,
    /// The optional closing delimiter.
    pub close: Option<MathItem<'a>>,
    /// The item between the delimiters.
    pub body: MathItem<'a>,
    /// How the target height for the delimiters should be calculated.
    ///
    /// If true, the height for each body item is two times the maximum of its
    /// ascent and descent. If false, the height for each body item is simply
    /// its height.
    ///
    /// Only used in paged export.
    pub balanced: bool,
}

impl<'a> FencedItem<'a> {
    /// Creates a new fenced item.
    pub(crate) fn create(
        open: Option<MathItem<'a>>,
        close: Option<MathItem<'a>>,
        body: MathItem<'a>,
        balanced: bool,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind =
            MathKind::Fenced(BumpBox::new_in(Self { open, close, body, balanced }, bump));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

/// A vertical fraction.
#[derive(Debug)]
pub struct FractionItem<'a> {
    /// The item in the top part of the fraction.
    pub numerator: MathItem<'a>,
    /// The item in the bottom part of the fraction.
    pub denominator: MathItem<'a>,
    /// Whether to draw a fraction line between the numerator and denominator.
    pub line: bool,
    /// The amount of padding added before and after the fraction.
    pub padding: Em,
}

impl<'a> FractionItem<'a> {
    /// Creates a new fraction item.
    pub(crate) fn create(
        numerator: MathItem<'a>,
        denominator: MathItem<'a>,
        line: bool,
        padding: Em,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind = MathKind::Fraction(BumpBox::new_in(
            Self { numerator, denominator, line, padding },
            bump,
        ));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

/// An inline skewed fraction.
#[derive(Debug)]
pub struct SkewedFractionItem<'a> {
    /// The item in the top-left part of the fraction.
    pub numerator: MathItem<'a>,
    /// The item in the bottom-right part of the fraction.
    pub denominator: MathItem<'a>,
    /// The fraction slash symbol.
    ///
    /// Only used in paged export.
    pub slash: MathItem<'a>,
}

impl<'a> SkewedFractionItem<'a> {
    /// Creates a new skewed fraction item.
    pub(crate) fn create(
        numerator: MathItem<'a>,
        denominator: MathItem<'a>,
        slash: MathItem<'a>,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind = MathKind::SkewedFraction(BumpBox::new_in(
            Self { numerator, denominator, slash },
            bump,
        ));
        let props = MathProperties::default(styles);
        MathComponent { kind, props, styles }.into()
    }
}

/// A 2D collection of math items laid out as a table/matrix.
#[derive(Debug)]
pub struct TableItem<'a> {
    /// The cells of the table, organized by row.
    pub cells: BumpBox<'a, [BumpBox<'a, [MathItem<'a>]>]>,
    /// The gap between rows and columns.
    pub gap: Axes<Rel<Abs>>,
    /// Optional augmentation lines to draw.
    pub augment: Option<Augment<Abs>>,
    /// The alignment for cells.
    pub align: FixedAlignment,
    /// How to perform left/right alternation for alignment.
    pub alternator: LeftRightAlternator,
}

impl<'a> TableItem<'a> {
    /// Creates a new table item.
    pub(crate) fn create(
        cells: Vec<Vec<MathItem<'a>>>,
        gap: Axes<Rel<Abs>>,
        augment: Option<Augment<Abs>>,
        align: FixedAlignment,
        alternator: LeftRightAlternator,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let cells = BumpVec::from_iter_in(
            cells
                .into_iter()
                .map(|row| BumpVec::from_iter_in(row, bump).into_boxed_slice()),
            bump,
        )
        .into_boxed_slice();
        let kind = MathKind::Table(BumpBox::new_in(
            Self { cells, gap, augment, align, alternator },
            bump,
        ));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

/// A base with scripts (subscripts/superscripts) and/or limits attached.
#[derive(Debug)]
pub struct ScriptsItem<'a> {
    /// The base item.
    pub base: MathItem<'a>,
    /// The top attachment (limit above).
    pub top: Option<MathItem<'a>>,
    /// The bottom attachment (limit below).
    pub bottom: Option<MathItem<'a>>,
    /// The top-left attachment (pre-superscript).
    pub top_left: Option<MathItem<'a>>,
    /// The bottom-left attachment (pre-subscript).
    pub bottom_left: Option<MathItem<'a>>,
    /// The top-right attachment (post-superscript).
    pub top_right: Option<MathItem<'a>>,
    /// The bottom-right attachment (post-subscript).
    pub bottom_right: Option<MathItem<'a>>,
}

impl<'a> ScriptsItem<'a> {
    /// Creates a new scripts item.
    ///
    /// The resulting item inherits its math class from the base.
    pub(crate) fn create(
        base: MathItem<'a>,
        top: Option<MathItem<'a>>,
        bottom: Option<MathItem<'a>>,
        top_left: Option<MathItem<'a>>,
        bottom_left: Option<MathItem<'a>>,
        top_right: Option<MathItem<'a>>,
        bottom_right: Option<MathItem<'a>>,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props = MathProperties::with_explicit_class(styles, base.class());
        let kind = MathKind::Scripts(BumpBox::new_in(
            Self {
                base,
                top,
                bottom,
                top_left,
                bottom_left,
                top_right,
                bottom_right,
            },
            bump,
        ));
        MathComponent { kind, props, styles }.into()
    }
}

/// A base with an accent mark above or below.
#[derive(Debug)]
pub struct AccentItem<'a> {
    /// The base item.
    pub base: MathItem<'a>,
    /// The accent mark item.
    pub accent: MathItem<'a>,
    /// Whether this is a top or bottom accent.
    pub position: Position,
    /// Whether the item's width should include the accent's width.
    ///
    /// Only used in paged export.
    pub exact_frame_width: bool,
}

impl<'a> AccentItem<'a> {
    /// Creates a new accent item.
    ///
    /// The resulting item inherits its math class from the base.
    pub(crate) fn create(
        base: MathItem<'a>,
        accent: MathItem<'a>,
        position: Position,
        exact_frame_width: bool,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props = MathProperties::with_explicit_class(styles, base.class());
        let kind = MathKind::Accent(BumpBox::new_in(
            Self { base, accent, position, exact_frame_width },
            bump,
        ));
        MathComponent { kind, props, styles }.into()
    }
}

/// A base with a line overlaid.
#[derive(Debug)]
pub struct CancelItem<'a> {
    /// The base item.
    pub base: MathItem<'a>,
    /// The length of the line.
    pub length: Rel<Abs>,
    /// The stroke for the line.
    pub stroke: FixedStroke,
    /// Whether a cross (two lines) is drawn instead of a single line.
    pub cross: bool,
    /// Whether to invert the angle of the first line.
    pub invert_first_line: bool,
    /// The angle of the line.
    pub angle: Smart<CancelAngle>,
}

impl<'a> CancelItem<'a> {
    /// Creates a new cancel item.
    ///
    /// The resulting item inherits its math class from the base.
    pub(crate) fn create(
        base: MathItem<'a>,
        length: Rel<Abs>,
        stroke: FixedStroke,
        cross: bool,
        invert_first_line: bool,
        angle: Smart<CancelAngle>,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props =
            MathProperties::with_explicit_class(styles, base.class()).with_span(span);
        let kind = MathKind::Cancel(BumpBox::new_in(
            Self {
                base,
                length,
                stroke,
                cross,
                invert_first_line,
                angle,
            },
            bump,
        ));
        MathComponent { kind, props, styles }.into()
    }
}

/// A base with a line drawn above or below.
#[derive(Debug)]
pub struct LineItem<'a> {
    /// The base item.
    pub base: MathItem<'a>,
    /// Whether the line is drawn above or below the base.
    pub position: Position,
}

impl<'a> LineItem<'a> {
    /// Creates a new line item.
    ///
    /// The resulting item inherits its math class from the base.
    pub(crate) fn create(
        base: MathItem<'a>,
        position: Position,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props =
            MathProperties::with_explicit_class(styles, base.class()).with_span(span);
        let kind = MathKind::Line(BumpBox::new_in(Self { base, position }, bump));
        MathComponent { kind, props, styles }.into()
    }
}

/// Grouped prime symbols.
///
/// This is for more than four prime symbols, since there are only dedicated
/// Unicode codepoints up to four.
#[derive(Debug)]
pub struct PrimesItem<'a> {
    /// The prime symbol item.
    pub prime: MathItem<'a>,
    /// The number of primes to display. Always at least five.
    pub count: usize,
}

impl<'a> PrimesItem<'a> {
    /// Creates a new primes item.
    pub(crate) fn create(
        prime: MathItem<'a>,
        count: usize,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind = MathKind::Primes(BumpBox::new_in(Self { prime, count }, bump));
        let props = MathProperties::default(styles);
        MathComponent { kind, props, styles }.into()
    }
}

/// A text string.
#[derive(Debug)]
pub struct TextItem<'a> {
    /// The text content.
    pub text: &'a str,
}

impl<'a> TextItem<'a> {
    /// Creates a new text item.
    ///
    /// The `line` parameter indicates that the text does not contain a newline
    /// and is not a number. If true, then the resulting item is spaced and has
    /// alphabetic math class.
    pub(crate) fn create(
        text: EcoString,
        line: bool,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let text = bump.alloc_str(&text);
        let kind = MathKind::Text(Self { text });
        let props = if line {
            MathProperties::with_explicit_class(styles, MathClass::Alphabetic)
                .with_spaced(true)
        } else {
            MathProperties::default(styles)
        }
        .with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

/// A single glyph (grapheme cluster).
#[derive(Debug)]
pub struct GlyphItem {
    /// The text content.
    pub text: EcoString,
    /// How the glyph should be stretched.
    pub stretch: Cell<Stretch>,
    /// Whether this glyph has been stretched as a middle delimiter.
    pub mid_stretched: Cell<Option<bool>>,
    /// Whether to apply the flac OpenType feature.
    pub flac: Cell<bool>,
}

impl GlyphItem {
    /// Creates a new glyph item.
    ///
    /// The `dtls` parameter indicates that a dotless character was converted
    /// to its non-dotless version.
    pub(crate) fn create<'a>(
        text: EcoString,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        assert!(text.graphemes(true).count() == 1);

        let c = text.chars().next().unwrap();

        let default_class = default_math_class(c);
        let limits = Limits::for_char_with_class(c, default_class);
        let class = styles
            .get(EquationElem::class)
            .or(default_class)
            .unwrap_or(MathClass::Normal);

        let kind = MathKind::Glyph(BumpBox::new_in(
            Self {
                text,
                stretch: Cell::new(Stretch::new()),
                mid_stretched: Cell::new(None),
                flac: Cell::new(false),
            },
            bump,
        ));
        let props = MathProperties::with_explicit_limits_and_class(styles, limits, class)
            .with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

/// Inline content.
#[derive(Debug)]
pub struct BoxItem<'a> {
    /// The [`BoxElem`] to layout.
    pub elem: &'a Packed<BoxElem>,
}

impl<'a> BoxItem<'a> {
    /// Creates a new box item.
    ///
    /// The resulting item is spaced.
    pub(crate) fn create(
        elem: &'a Packed<BoxElem>,
        styles: StyleChain<'a>,
    ) -> MathItem<'a> {
        let kind = MathKind::Box(Self { elem });
        let props = MathProperties::default(styles).with_spaced(true);
        MathComponent { kind, props, styles }.into()
    }
}

/// External content that needs to be laid out separately.
#[derive(Debug)]
pub struct ExternalItem<'a> {
    /// The content to layout externally.
    pub content: &'a Content,
}

impl<'a> ExternalItem<'a> {
    /// Creates a new external item.
    ///
    /// The resulting item is spaced and, if the content is a [`PlaceElem`], is
    /// ignorant.
    pub(crate) fn create(content: &'a Content, styles: StyleChain<'a>) -> MathItem<'a> {
        let kind = MathKind::External(Self { content });
        let props = MathProperties::default(styles)
            .with_spaced(true)
            .with_ignorant(content.is::<PlaceElem>());
        MathComponent { kind, props, styles }.into()
    }
}

/// Stretch configuration for a glyph on both axes.
#[derive(Debug, Clone, Copy)]
pub struct Stretch(Axes<Option<StretchInfo>>);

impl Stretch {
    /// Creates a new empty stretch configuration.
    pub(crate) fn new() -> Self {
        Self(Axes::splat(None))
    }

    /// Adds horizontal stretch information.
    pub(crate) fn with_x(mut self, info: StretchInfo) -> Self {
        self.0.x = Some(info);
        self
    }

    /// Adds vertical stretch information.
    pub(crate) fn with_y(mut self, info: StretchInfo) -> Self {
        self.0.y = Some(info);
        self
    }

    /// Updates stretch info for both axes, combining with existing info.
    pub(crate) fn update(mut self, info: StretchInfo) -> Self {
        match &mut self.0.x {
            Some(val) => *val *= info,
            None => self.0.x = Some(info),
        }
        match &mut self.0.y {
            Some(val) => *val *= info,
            None => self.0.y = Some(info),
        }
        self
    }

    /// Sets the reference size for relative stretching on the given axis.
    ///
    /// Only sets the value if not already set.
    pub(crate) fn relative_to(mut self, relative_to: Abs, axis: Axis) -> Self {
        if let Some(info) = self.0.get_mut(axis)
            && info.relative_to.is_none()
        {
            info.relative_to = Some(relative_to);
        }
        self
    }

    /// Sets the font size for short-fall calculations on the given axis.
    ///
    /// Only sets the value if not already set.
    pub(crate) fn font_size(mut self, font_size: Abs, axis: Axis) -> Self {
        if let Some(info) = self.0.get_mut(axis)
            && info.font_size.is_none()
        {
            info.font_size = Some(font_size);
        }
        self
    }

    /// Returns the stretch info for the given axis, if any.
    pub fn resolve(mut self, axis: Axis) -> Option<StretchInfo> {
        if let Some(info) = self.0.get_mut(axis)
            && let Some(buffer) = info.buffer
        {
            // Sort out the buffer before returning the info to use.
            if info.relative_to.is_some() {
                info.target = buffer;
            } else {
                info.target = Rel::new(
                    info.target.rel * buffer.rel,
                    buffer.rel.of(info.target.abs) + buffer.abs,
                );
            }
        }
        self.0.get(axis)
    }
}

/// Information about how to stretch a glyph on one axis.
#[derive(Debug, Clone, Copy)]
pub struct StretchInfo {
    /// The target size to stretch to.
    pub target: Rel<Abs>,
    /// A buffer to store the latest stretch added, in case it needs to be
    /// relative to something else.
    buffer: Option<Rel<Abs>>,
    /// The short-fall amount for glyph assembly.
    pub short_fall: Em,
    /// The reference size for relative targets.
    ///
    /// Only used in paged export.
    pub relative_to: Option<Abs>,
    /// The font size to use for short-fall.
    ///
    /// Only used in paged export.
    pub font_size: Option<Abs>,
}

impl StretchInfo {
    /// Creates new stretch info with the given target and short-fall.
    pub(crate) fn new(target: Rel<Abs>, short_fall: Em) -> Self {
        Self {
            target,
            buffer: None,
            short_fall,
            relative_to: None,
            font_size: None,
        }
    }
}

impl MulAssign for StretchInfo {
    fn mul_assign(&mut self, rhs: Self) {
        if let Some(buffer) = self.buffer {
            self.target = Rel::new(
                self.target.rel * buffer.rel,
                buffer.rel.of(self.target.abs) + buffer.abs,
            );
        }
        self.buffer = Some(rhs.target);
        self.short_fall = rhs.short_fall;
    }
}

/// A marker representing the positioning of something above or below a base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Placed above the base.
    Above,
    /// Placed below the base.
    Below,
}
