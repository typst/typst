#![allow(clippy::too_many_arguments)]
use std::cell::Cell;
use std::ops::{Deref, DerefMut, MulAssign};

use bumpalo::Bump;
use ecow::EcoString;
use smallvec::SmallVec;
use typst_syntax::Span;
use typst_utils::{Get, default_math_class};
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;

use crate::foundations::{Content, Packed, Smart, StyleChain};
use crate::introspection::Tag;
use crate::layout::{Abs, Axes, Axis, BoxElem, Em, FixedAlignment, PlaceElem, Rel};
use crate::math::{
    Augment, CancelAngle, EquationElem, LeftRightAlternator, Limits, MEDIUM, MathSize,
    THICK, THIN,
};
use crate::routines::Arenas;
use crate::visualize::FixedStroke;

/// The top-level item in the math IR.
#[derive(Debug, Clone)]
pub enum MathItem<'a> {
    // A layoutable component with properties.
    Component(MathComponent<'a>),
    // Special, non-component items.
    Spacing(Abs, bool),
    Space,
    Linebreak,
    Align,
    Tag(Tag),
}

impl<'a> From<MathComponent<'a>> for MathItem<'a> {
    fn from(comp: MathComponent<'a>) -> MathItem<'a> {
        MathItem::Component(comp)
    }
}

impl<'a> MathItem<'a> {
    pub(crate) fn limits(&self) -> Limits {
        match self {
            Self::Component(comp) => comp.props.limits,
            _ => Limits::Never,
        }
    }

    pub(crate) fn class(&self) -> MathClass {
        match self {
            Self::Component(comp) => comp.props.class,
            Self::Spacing(_, _) | Self::Space | Self::Linebreak => MathClass::Space,
            Self::Align | Self::Tag(_) => MathClass::Special,
        }
    }

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

    pub(crate) fn size(&self) -> Option<MathSize> {
        match self {
            Self::Component(comp) => Some(comp.props.size),
            _ => None,
        }
    }

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

    pub(crate) fn is_ignorant(&self) -> bool {
        match self {
            Self::Component(comp) => comp.props.ignorant,
            Self::Tag(_) => true,
            _ => false,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Self::Component(comp) => comp.props.span,
            _ => Span::detached(),
        }
    }

    pub fn styles(&self) -> Option<StyleChain<'a>> {
        match self {
            Self::Component(comp) => Some(comp.styles),
            _ => None,
        }
    }

    pub fn mid_stretched(&self) -> Option<bool> {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.mid_stretched.get()
        } else {
            None
        }
    }

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
            group.items
        } else {
            core::slice::from_ref(self)
        }
    }

    pub(crate) fn set_limits(&mut self, limits: Limits) {
        if let Self::Component(comp) = self {
            comp.props.limits = limits;
        }
    }

    pub(crate) fn set_class(&mut self, class: MathClass) {
        if let Self::Component(comp) = self {
            comp.props.class = class;
        }
    }

    pub(crate) fn set_lspace(&mut self, lspace: Option<Em>) {
        if let Self::Component(comp) = self {
            comp.props.lspace = lspace;
        }
    }

    pub(crate) fn set_rspace(&mut self, rspace: Option<Em>) {
        if let Self::Component(comp) = self {
            comp.props.rspace = rspace;
        }
    }

    pub(crate) fn set_mid_stretched(&self, mid_stretched: Option<bool>) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.mid_stretched.set(mid_stretched);
        }
    }

    pub(crate) fn set_stretch(&self, stretch: Stretch) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.replace(stretch);
        }
    }

    pub(crate) fn set_y_stretch(&self, info: StretchInfo) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.update(|stretch| stretch.with_y(info));
        }
    }

    pub(crate) fn update_stretch(&self, info: StretchInfo) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.update(|stretch| stretch.update(info));
        }
    }

    pub fn set_stretch_relative_to(&self, relative_to: Abs, axis: Axis) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.update(|stretch| stretch.relative_to(relative_to, axis));
        }
    }

    pub fn set_stretch_font_size(&self, font_size: Abs, axis: Axis) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch.update(|stretch| stretch.font_size(font_size, axis));
        }
    }

    pub fn set_flac(&self) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.flac.set(true);
        }
    }
}

/// A generic component that bundles a specific item with common properties.
#[derive(Debug, Clone)]
pub struct MathComponent<'a> {
    /// The specific item.
    pub kind: MathKind<'a>,
    /// The properties attached to this component.
    pub props: MathProperties,
    /// The item's styles.
    pub styles: StyleChain<'a>,
}

/// A layoutable math item.
///
/// Recursive or large variants are boxed (allocated in bump arena).
#[derive(Debug, Clone)]
pub enum MathKind<'a> {
    Group(GroupItem<'a>),
    Text(TextItem<'a>),
    External(ExternalItem<'a>),
    Box(BoxItem<'a>),
    Glyph(&'a GlyphItem),
    Line(&'a LineItem<'a>),
    Primes(&'a PrimesItem<'a>),
    Radical(&'a RadicalItem<'a>),
    Fenced(&'a FencedItem<'a>),
    Fraction(&'a FractionItem<'a>),
    SkewedFraction(&'a SkewedFractionItem<'a>),
    Table(&'a TableItem<'a>),
    Scripts(&'a ScriptsItem<'a>),
    Accent(&'a AccentItem<'a>),
    Cancel(&'a CancelItem<'a>),
}

/// Shared properties for layoutable components.
#[derive(Debug, Clone)]
pub struct MathProperties {
    pub(crate) limits: Limits,
    pub class: MathClass,
    pub size: MathSize,
    pub(crate) ignorant: bool,
    pub(crate) spaced: bool,
    pub lspace: Option<Em>,
    pub rspace: Option<Em>,
    pub span: Span,
}

impl MathProperties {
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

    /// Creates properties with an explicit class, avoiding the style lookup for class.
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

    fn with_ignorant(mut self, ignorant: bool) -> Self {
        self.ignorant = ignorant;
        self
    }

    fn with_spaced(mut self, spaced: bool) -> Self {
        self.spaced = spaced;
        self
    }

    fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }
}

#[derive(Debug, Clone)]
pub struct GroupItem<'a> {
    pub items: &'a [MathItem<'a>],
}

impl<'a> GroupItem<'a> {
    pub(crate) fn create<I>(
        items: I,
        closing_exists: bool,
        styles: StyleChain<'a>,
        arenas: &'a Arenas,
    ) -> MathItem<'a>
    where
        I: IntoIterator<Item = MathItem<'a>>,
        I::IntoIter: ExactSizeIterator,
    {
        let props = MathProperties::default(styles);
        let kind =
            MathKind::Group(Self { items: preprocess(items, arenas, closing_exists) });
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct RadicalItem<'a> {
    pub radicand: MathItem<'a>,
    pub index: Option<MathItem<'a>>,
    pub sqrt: MathItem<'a>,
}

impl<'a> RadicalItem<'a> {
    pub(crate) fn create(
        radicand: MathItem<'a>,
        index: Option<MathItem<'a>>,
        sqrt: MathItem<'a>,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind = MathKind::Radical(bump.alloc(Self { radicand, index, sqrt }));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct FencedItem<'a> {
    pub open: Option<MathItem<'a>>,
    pub close: Option<MathItem<'a>>,
    pub body: MathItem<'a>,
    pub balanced: bool,
}

impl<'a> FencedItem<'a> {
    pub(crate) fn create(
        open: Option<MathItem<'a>>,
        close: Option<MathItem<'a>>,
        body: MathItem<'a>,
        balanced: bool,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind = MathKind::Fenced(bump.alloc(Self { open, close, body, balanced }));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct FractionItem<'a> {
    pub numerator: MathItem<'a>,
    pub denominator: MathItem<'a>,
    pub line: bool,
    pub around: Em,
}

impl<'a> FractionItem<'a> {
    pub(crate) fn create(
        numerator: MathItem<'a>,
        denominator: MathItem<'a>,
        line: bool,
        around: Em,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind =
            MathKind::Fraction(bump.alloc(Self { numerator, denominator, line, around }));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct SkewedFractionItem<'a> {
    pub numerator: MathItem<'a>,
    pub denominator: MathItem<'a>,
    pub slash: MathItem<'a>,
}

impl<'a> SkewedFractionItem<'a> {
    pub(crate) fn create(
        numerator: MathItem<'a>,
        denominator: MathItem<'a>,
        slash: MathItem<'a>,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind =
            MathKind::SkewedFraction(bump.alloc(Self { numerator, denominator, slash }));
        let props = MathProperties::default(styles);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct TableItem<'a> {
    /// By row.
    pub cells: &'a [&'a [MathItem<'a>]],
    pub gap: Axes<Rel<Abs>>,
    pub augment: Option<Augment<Abs>>,
    pub align: FixedAlignment,
    pub alternator: LeftRightAlternator,
}

impl<'a> TableItem<'a> {
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
        let cells = bump.alloc_slice_fill_iter(cells.into_iter().map(|row| {
            let row_slice = bump.alloc_slice_fill_iter(row);
            row_slice as &[MathItem<'a>]
        }));
        let kind =
            MathKind::Table(bump.alloc(Self { cells, gap, augment, align, alternator }));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct ScriptsItem<'a> {
    pub base: MathItem<'a>,
    pub top: Option<MathItem<'a>>,
    pub bottom: Option<MathItem<'a>>,
    pub top_left: Option<MathItem<'a>>,
    pub bottom_left: Option<MathItem<'a>>,
    pub top_right: Option<MathItem<'a>>,
    pub bottom_right: Option<MathItem<'a>>,
}

impl<'a> ScriptsItem<'a> {
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
        let kind = MathKind::Scripts(bump.alloc(Self {
            base,
            top,
            bottom,
            top_left,
            bottom_left,
            top_right,
            bottom_right,
        }));
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct AccentItem<'a> {
    pub base: MathItem<'a>,
    pub accent: MathItem<'a>,
    pub is_bottom: bool,
    pub exact_frame_width: bool,
}

impl<'a> AccentItem<'a> {
    pub(crate) fn create(
        base: MathItem<'a>,
        accent: MathItem<'a>,
        is_bottom: bool,
        exact_frame_width: bool,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props = MathProperties::with_explicit_class(styles, base.class());
        let kind = MathKind::Accent(bump.alloc(Self {
            base,
            accent,
            is_bottom,
            exact_frame_width,
        }));
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct CancelItem<'a> {
    pub base: MathItem<'a>,
    pub length: Rel<Abs>,
    pub stroke: FixedStroke,
    pub cross: bool,
    pub invert_first_line: bool,
    pub angle: Smart<CancelAngle>,
}

impl<'a> CancelItem<'a> {
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
        let kind = MathKind::Cancel(bump.alloc(Self {
            base,
            length,
            stroke,
            cross,
            invert_first_line,
            angle,
        }));
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct LineItem<'a> {
    pub base: MathItem<'a>,
    pub under: bool,
}

impl<'a> LineItem<'a> {
    pub(crate) fn create(
        base: MathItem<'a>,
        under: bool,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props =
            MathProperties::with_explicit_class(styles, base.class()).with_span(span);
        let kind = MathKind::Line(bump.alloc(Self { base, under }));
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct PrimesItem<'a> {
    pub prime: MathItem<'a>,
    pub count: usize,
}

impl<'a> PrimesItem<'a> {
    pub(crate) fn create(
        prime: MathItem<'a>,
        count: usize,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind = MathKind::Primes(bump.alloc(Self { prime, count }));
        let props = MathProperties::default(styles);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct TextItem<'a> {
    pub text: &'a str,
}

impl<'a> TextItem<'a> {
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
            // Avoid class lookup when we're going to override it anyway.
            MathProperties::with_explicit_class(styles, MathClass::Alphabetic)
                .with_spaced(true)
        } else {
            MathProperties::default(styles)
        }
        .with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Stretch(Axes<Option<StretchInfo>>);

impl Stretch {
    pub(crate) fn new() -> Self {
        Self(Axes::splat(None))
    }

    pub(crate) fn with_x(mut self, info: StretchInfo) -> Self {
        self.0.x = Some(info);
        self
    }

    pub(crate) fn with_y(mut self, info: StretchInfo) -> Self {
        self.0.y = Some(info);
        self
    }

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

    pub(crate) fn relative_to(mut self, relative_to: Abs, axis: Axis) -> Self {
        if let Some(info) = self.0.get_mut(axis)
            && info.relative_to.is_none()
        {
            info.relative_to = Some(relative_to);
        }
        self
    }

    pub(crate) fn font_size(mut self, font_size: Abs, axis: Axis) -> Self {
        if let Some(info) = self.0.get_mut(axis)
            && info.font_size.is_none()
        {
            info.font_size = Some(font_size);
        }
        self
    }

    pub fn resolve(self, axis: Axis) -> Option<StretchInfo> {
        self.0.get(axis)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StretchInfo {
    pub target: Rel<Abs>,
    pub short_fall: Em,
    // resolved later
    pub relative_to: Option<Abs>,
    pub font_size: Option<Abs>,
}

impl StretchInfo {
    pub(crate) fn new(target: Rel<Abs>, short_fall: Em) -> Self {
        Self {
            target,
            short_fall,
            relative_to: None,
            font_size: None,
        }
    }
}

impl MulAssign for StretchInfo {
    fn mul_assign(&mut self, rhs: Self) {
        self.target = Rel::new(
            self.target.rel * rhs.target.rel,
            rhs.target.rel.of(self.target.abs) + rhs.target.abs,
        );
        self.short_fall = rhs.short_fall;
    }
}

#[derive(Debug, Clone)]
pub struct GlyphItem {
    pub text: EcoString,
    pub stretch: Cell<Stretch>,
    pub mid_stretched: Cell<Option<bool>>,
    pub flac: Cell<bool>,
    pub dtls: bool,
}

impl GlyphItem {
    pub(crate) fn create<'a>(
        text: EcoString,
        dtls: bool,
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

        let kind = MathKind::Glyph(bump.alloc(Self {
            text,
            stretch: Cell::new(Stretch::new()),
            mid_stretched: Cell::new(None),
            flac: Cell::new(false),
            dtls,
        }));
        let props = MathProperties::with_explicit_limits_and_class(styles, limits, class)
            .with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct BoxItem<'a> {
    pub elem: &'a Packed<BoxElem>,
}

impl<'a> BoxItem<'a> {
    pub(crate) fn create(
        elem: &'a Packed<BoxElem>,
        styles: StyleChain<'a>,
    ) -> MathItem<'a> {
        let kind = MathKind::Box(Self { elem });
        let props = MathProperties::default(styles).with_spaced(true);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct ExternalItem<'a> {
    pub content: &'a Content,
}

impl<'a> ExternalItem<'a> {
    pub(crate) fn create(content: &'a Content, styles: StyleChain<'a>) -> MathItem<'a> {
        let kind = MathKind::External(Self { content });
        let props = MathProperties::default(styles)
            .with_spaced(true)
            .with_ignorant(content.is::<PlaceElem>());
        MathComponent { kind, props, styles }.into()
    }
}

/// Takes the given [`MathItem`]s and do some basic processing.
///
/// The behavior of spacing around alignment points is subtle and differs from
/// the `align` environment in amsmath. The current policy is:
/// > always put the correct spacing between fragments separated by an
/// > alignment point, and always put the space on the left of the alignment
/// > point
fn preprocess<'a, I>(items: I, arenas: &'a Arenas, closing: bool) -> &'a [MathItem<'a>]
where
    I: IntoIterator<Item = MathItem<'a>>,
    I::IntoIter: ExactSizeIterator,
{
    let iter = items.into_iter();
    let mut resolved = MathBuffer::with_capacity(iter.len());
    let iter = iter.peekable();

    let mut last: Option<usize> = None;
    let mut space: Option<MathItem> = None;

    for mut item in iter {
        match item {
            // Tags don't affect layout.
            MathItem::Tag(_) => {
                resolved.push(item);
                continue;
            }

            // Keep space only if supported by spaced items.
            MathItem::Space => {
                if last.is_some() {
                    space = Some(item);
                }
                continue;
            }

            // Explicit spacing disables automatic spacing.
            MathItem::Spacing(width, weak) => {
                last = None;
                space = None;

                if weak {
                    let Some(resolved_last) = resolved.last_mut() else { continue };
                    if let MathItem::Spacing(prev, true) = resolved_last {
                        *prev = (*prev).max(width);
                        continue;
                    }
                }

                resolved.push(item);
                continue;
            }

            // Alignment points are resolved later.
            MathItem::Align => {
                resolved.push(item);
                continue;
            }

            // New line, new things.
            MathItem::Linebreak => {
                resolved.push(item);
                space = None;
                last = None;
                continue;
            }

            _ => {}
        }

        // Convert variable operators into binary operators if something
        // precedes them and they are not preceded by a operator or comparator.
        if item.class() == MathClass::Vary
            && matches!(
                last.map(|i| resolved[i].class()),
                Some(
                    MathClass::Normal
                        | MathClass::Alphabetic
                        | MathClass::Closing
                        | MathClass::Fence
                )
            )
        {
            item.set_class(MathClass::Binary);
        }

        // Insert spacing between the last and this non-ignorant item.
        if !item.is_ignorant() {
            if let Some(i) = last
                && let Some(s) = spacing(&mut resolved[i], space.take(), &mut item)
            {
                resolved.insert(i + 1, s);
            }

            last = Some(resolved.len());
        }

        resolved.push(item);
    }

    // Apply closing punctuation spacing if applicable.
    if closing
        && let Some(item) = resolved.last_mut()
        && item.rclass() == MathClass::Punctuation
        && item.size().is_none_or(|s| s > MathSize::Script)
    {
        item.set_rspace(Some(THIN))
    } else if let Some(idx) = resolved.last_index()
        && let MathItem::Spacing(_, true) = resolved.0[idx]
    {
        resolved.0.remove(idx);
    }

    arenas.bump.alloc_slice_fill_iter(resolved.0)
}

/// Create the spacing between two items in a given style.
fn spacing<'a>(
    l: &mut MathItem,
    space: Option<MathItem<'a>>,
    r: &mut MathItem,
) -> Option<MathItem<'a>> {
    use MathClass::*;

    let script = |f: &MathItem| f.size().is_some_and(|s| s <= MathSize::Script);

    match (l.rclass(), r.lclass()) {
        // No spacing before punctuation; thin spacing after punctuation, unless
        // in script size.
        (_, Punctuation) => {}
        (Punctuation, _) if !script(l) => l.set_rspace(Some(THIN)),

        // No spacing after opening delimiters and before closing delimiters.
        (Opening, _) | (_, Closing) => {}

        // Thick spacing around relations, unless followed by a another relation
        // or in script size.
        (Relation, Relation) => {}
        (Relation, _) if !script(l) => l.set_rspace(Some(THICK)),
        (_, Relation) if !script(r) => r.set_lspace(Some(THICK)),

        // Medium spacing around binary operators, unless in script size.
        (Binary, _) if !script(l) => l.set_rspace(Some(MEDIUM)),
        (_, Binary) if !script(r) => r.set_lspace(Some(MEDIUM)),

        // Thin spacing around large operators, unless to the left of
        // an opening delimiter. TeXBook, p170
        (Large, Opening | Fence) => {}
        (Large, _) => l.set_rspace(Some(THIN)),

        (_, Large) => r.set_lspace(Some(THIN)),

        // Spacing around spaced frames.
        _ if (l.is_spaced() || r.is_spaced()) => return space,

        _ => {}
    };

    None
}

/// A wrapper around `SmallVec<[MathItem; 8]>` that ignores [`MathItem::Tag`]s
/// in some access methods.
struct MathBuffer<'a>(SmallVec<[MathItem<'a>; 8]>);

impl<'a> MathBuffer<'a> {
    fn with_capacity(size: usize) -> Self {
        Self(SmallVec::with_capacity(size))
    }

    /// Returns a mutable reference to the last non-Tag fragment.
    fn last_mut(&mut self) -> Option<&mut MathItem<'a>> {
        self.0.iter_mut().rev().find(|f| !matches!(f, MathItem::Tag(_)))
    }

    /// Returns the physical index of the last non-Tag fragment.
    fn last_index(&self) -> Option<usize> {
        self.0.iter().rposition(|f| !matches!(f, MathItem::Tag(_)))
    }
}

impl<'a> Deref for MathBuffer<'a> {
    type Target = SmallVec<[MathItem<'a>; 8]>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for MathBuffer<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
