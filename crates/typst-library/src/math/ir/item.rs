use bumpalo::Bump;
use ecow::EcoString;
use typst_syntax::Span;
use typst_utils::default_math_class;
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;

use crate::foundations::{Content, Packed, Smart, StyleChain};
use crate::introspection::Tag;
use crate::layout::{
    Abs, Axes, Axis, BoxElem, Em, FixedAlignment, FrameModifiers, PlaceElem, Rel,
};
use crate::math::{
    Augment, CancelAngle, EquationElem, LeftRightAlternator, Limits, MathRun, MathSize,
};
use crate::text::TextElem;
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
    pub fn limits(&self) -> Limits {
        match self {
            Self::Component(comp) => comp.props.limits,
            _ => Limits::Never,
        }
    }

    pub fn class(&self) -> MathClass {
        match self {
            Self::Component(comp) => comp.props.class,
            Self::Spacing(_, _) | Self::Space | Self::Linebreak => MathClass::Space,
            Self::Align | Self::Tag(_) => MathClass::Special,
        }
    }

    pub fn rclass(&self) -> MathClass {
        match self {
            Self::Component(MathComponent { kind: MathKind::Fenced(fence), .. })
                if fence.close.is_some() =>
            {
                MathClass::Closing
            }
            _ => self.class(),
        }
    }

    pub fn lclass(&self) -> MathClass {
        match self {
            Self::Component(MathComponent { kind: MathKind::Fenced(fence), .. })
                if fence.open.is_some() =>
            {
                MathClass::Opening
            }
            _ => self.class(),
        }
    }

    pub fn size(&self) -> Option<MathSize> {
        match self {
            Self::Component(comp) => Some(comp.props.size),
            _ => None,
        }
    }

    pub fn is_spaced(&self) -> bool {
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

    pub fn is_ignorant(&self) -> bool {
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

    pub fn get_mid_stretched(&self) -> Option<bool> {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.mid_stretched
        } else {
            None
        }
    }

    pub fn get_stretch(&self) -> Option<(Rel<Abs>, Option<Axis>)> {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &comp.kind
        {
            glyph.stretch
        } else {
            None
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

    pub(crate) fn set_mid_stretched(&mut self, mid_stretched: Option<bool>) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &mut comp.kind
        {
            glyph.mid_stretched = mid_stretched;
        }
    }

    pub(crate) fn set_stretch(&mut self, stretch: Option<(Rel<Abs>, Option<Axis>)>) {
        if let Self::Component(comp) = self
            && let MathKind::Glyph(glyph) = &mut comp.kind
        {
            glyph.stretch = stretch;
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
#[derive(Debug, Clone)]
pub enum MathKind<'a> {
    // Inline variants. GlyphItem (~48 bytes I think) is the threshold.
    Group(GroupItem<'a>),
    Line(LineItem<'a>),
    Primes(PrimesItem<'a>),
    Glyph(GlyphItem),
    Text(TextItem),
    External(ExternalItem<'a>),
    Box(BoxItem<'a>),
    // Boxed variants.
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
    pub limits: Limits,
    pub class: MathClass,
    pub size: MathSize,
    pub ignorant: bool,
    pub spaced: bool,
    pub lspace: Option<Em>,
    pub rspace: Option<Em>,
    pub font_size: Abs,
    pub modifiers: FrameModifiers,
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
            modifiers: FrameModifiers::get_in(styles),
            font_size: styles.resolve(TextElem::size),
            span: Span::detached(),
        }
    }

    fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    fn with_class(mut self, class: MathClass) -> Self {
        self.class = class;
        self
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
    pub items: MathRun<'a>,
}

impl<'a> GroupItem<'a> {
    pub(crate) fn create(run: MathRun<'a>) -> MathItem<'a> {
        let props = MathProperties::default(run.styles);
        let kind = MathKind::Group(Self { items: run.clone() });
        MathComponent { kind, props, styles: run.styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct RadicalItem<'a> {
    pub radicand: MathRun<'a>,
    pub index: Option<MathRun<'a>>,
    pub sqrt: MathRun<'a>,
}

impl<'a> RadicalItem<'a> {
    pub(crate) fn create(
        radicand: MathRun<'a>,
        index: Option<MathRun<'a>>,
        sqrt: MathRun<'a>,
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
    pub open: Option<MathRun<'a>>,
    pub close: Option<MathRun<'a>>,
    pub body: MathRun<'a>,
    pub balanced: bool,
    pub target: Rel<Abs>,
    pub short_fall: Em,
}

impl<'a> FencedItem<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        open: Option<MathRun<'a>>,
        close: Option<MathRun<'a>>,
        body: MathRun<'a>,
        balanced: bool,
        short_fall: Em,
        target: Rel<Abs>,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind = MathKind::Fenced(bump.alloc(Self {
            open,
            close,
            body,
            balanced,
            target,
            short_fall,
        }));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct FractionItem<'a> {
    pub numerator: MathRun<'a>,
    pub denominator: MathRun<'a>,
    pub line: bool,
}

impl<'a> FractionItem<'a> {
    pub(crate) fn create(
        numerator: MathRun<'a>,
        denominator: MathRun<'a>,
        line: bool,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind = MathKind::Fraction(bump.alloc(Self { numerator, denominator, line }));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct SkewedFractionItem<'a> {
    pub numerator: MathRun<'a>,
    pub denominator: MathRun<'a>,
    pub slash: MathRun<'a>,
}

impl<'a> SkewedFractionItem<'a> {
    pub(crate) fn create(
        numerator: MathRun<'a>,
        denominator: MathRun<'a>,
        slash: MathRun<'a>,
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
    pub cells: Vec<Vec<MathRun<'a>>>,
    pub gap: Axes<Rel<Abs>>,
    pub augment: Option<Augment<Abs>>,
    pub align: FixedAlignment,
    pub alternator: LeftRightAlternator,
}

impl<'a> TableItem<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        cells: Vec<Vec<MathRun<'a>>>,
        gap: Axes<Rel<Abs>>,
        augment: Option<Augment<Abs>>,
        align: FixedAlignment,
        alternator: LeftRightAlternator,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let kind =
            MathKind::Table(bump.alloc(Self { cells, gap, augment, align, alternator }));
        let props = MathProperties::default(styles).with_span(span);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct ScriptsItem<'a> {
    pub base: MathRun<'a>,
    pub top: Option<MathRun<'a>>,
    pub bottom: Option<MathRun<'a>>,
    pub top_left: Option<MathRun<'a>>,
    pub bottom_left: Option<MathRun<'a>>,
    pub top_right: Option<MathRun<'a>>,
    pub bottom_right: Option<MathRun<'a>>,
    pub base_target: Option<Rel<Abs>>,
}

impl<'a> ScriptsItem<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        base: MathRun<'a>,
        top: Option<MathRun<'a>>,
        bottom: Option<MathRun<'a>>,
        top_left: Option<MathRun<'a>>,
        bottom_left: Option<MathRun<'a>>,
        top_right: Option<MathRun<'a>>,
        bottom_right: Option<MathRun<'a>>,
        base_target: Option<Rel<Abs>>,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props = MathProperties::default(styles).with_class(base.class());
        let kind = MathKind::Scripts(bump.alloc(Self {
            base,
            top,
            bottom,
            top_left,
            bottom_left,
            top_right,
            bottom_right,
            base_target,
        }));
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct AccentItem<'a> {
    pub base: MathRun<'a>,
    pub accent: MathRun<'a>,
    pub is_bottom: bool,
    pub target: Rel<Abs>,
    pub short_fall: Em,
    pub exact_frame_width: bool,
}

impl<'a> AccentItem<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        base: MathRun<'a>,
        accent: MathRun<'a>,
        is_bottom: bool,
        target: Rel<Abs>,
        short_fall: Em,
        exact_frame_width: bool,
        styles: StyleChain<'a>,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props = MathProperties::default(styles).with_class(base.class());
        let kind = MathKind::Accent(bump.alloc(Self {
            base,
            accent,
            is_bottom,
            target,
            short_fall,
            exact_frame_width,
        }));
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct CancelItem<'a> {
    pub base: MathRun<'a>,
    pub length: Rel<Abs>,
    pub stroke: FixedStroke,
    pub cross: bool,
    pub invert_first_line: bool,
    pub angle: Smart<CancelAngle>,
}

impl<'a> CancelItem<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        base: MathRun<'a>,
        length: Rel<Abs>,
        stroke: FixedStroke,
        cross: bool,
        invert_first_line: bool,
        angle: Smart<CancelAngle>,
        styles: StyleChain<'a>,
        span: Span,
        bump: &'a Bump,
    ) -> MathItem<'a> {
        let props = MathProperties::default(styles)
            .with_class(base.class())
            .with_span(span);
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
    pub base: MathRun<'a>,
    pub under: bool,
}

impl<'a> LineItem<'a> {
    pub(crate) fn create(
        base: MathRun<'a>,
        under: bool,
        styles: StyleChain<'a>,
        span: Span,
    ) -> MathItem<'a> {
        let props = MathProperties::default(styles)
            .with_class(base.class())
            .with_span(span);
        let kind = MathKind::Line(Self { base, under });
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct PrimesItem<'a> {
    pub prime: MathRun<'a>,
    pub count: usize,
}

impl<'a> PrimesItem<'a> {
    pub(crate) fn create(
        prime: MathRun<'a>,
        count: usize,
        styles: StyleChain<'a>,
    ) -> MathItem<'a> {
        let kind = MathKind::Primes(Self { prime, count });
        let props = MathProperties::default(styles);
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct TextItem {
    pub text: EcoString,
}

impl TextItem {
    pub(crate) fn create<'a>(
        text: EcoString,
        line: bool,
        styles: StyleChain<'a>,
        span: Span,
    ) -> MathItem<'a> {
        let kind = MathKind::Text(Self { text });
        let mut props = MathProperties::default(styles).with_span(span);
        if line {
            props = props.with_class(MathClass::Alphabetic).with_spaced(true);
        }
        MathComponent { kind, props, styles }.into()
    }
}

#[derive(Debug, Clone)]
pub struct GlyphItem {
    pub text: EcoString,
    pub stretch: Option<(Rel<Abs>, Option<Axis>)>,
    pub mid_stretched: Option<bool>,
}

impl GlyphItem {
    pub(crate) fn create<'a>(
        text: EcoString,
        styles: StyleChain<'a>,
        span: Span,
    ) -> MathItem<'a> {
        assert!(text.graphemes(true).count() == 1);

        let c = text.chars().next().unwrap();

        let limits = Limits::for_char(c);
        let class = styles
            .get(EquationElem::class)
            .or_else(|| default_math_class(c))
            .unwrap_or(MathClass::Normal);

        let kind = MathKind::Glyph(Self { text, stretch: None, mid_stretched: None });
        let props = MathProperties::default(styles)
            .with_limits(limits)
            .with_class(class)
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
