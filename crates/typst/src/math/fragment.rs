use std::fmt::{self, Debug, Formatter};

use smallvec::SmallVec;
use ttf_parser::gsub::AlternateSet;
use ttf_parser::{GlyphId, Rect};
use unicode_math_class::MathClass;

use crate::foundations::StyleChain;
use crate::layout::{Abs, Corner, Em, Frame, FrameItem, HideElem, Point, Size};
use crate::math::{
    scaled_font_size, EquationElem, Limits, MathContext, MathSize, Scaled,
};
use crate::model::{Destination, LinkElem};
use crate::syntax::Span;
use crate::text::{Font, Glyph, Lang, Region, TextElem, TextItem};
use crate::visualize::Paint;

#[derive(Debug, Clone)]
pub enum MathFragment {
    Glyph(GlyphFragment),
    Variant(VariantFragment),
    Frame(FrameFragment),
    Spacing(SpacingFragment),
    Space(Abs),
    Linebreak,
    Align,
}

impl MathFragment {
    pub fn size(&self) -> Size {
        Size::new(self.width(), self.height())
    }

    pub fn width(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.width,
            Self::Variant(variant) => variant.frame.width(),
            Self::Frame(fragment) => fragment.frame.width(),
            Self::Spacing(spacing) => spacing.width,
            Self::Space(amount) => *amount,
            _ => Abs::zero(),
        }
    }

    pub fn height(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.height(),
            Self::Variant(variant) => variant.frame.height(),
            Self::Frame(fragment) => fragment.frame.height(),
            _ => Abs::zero(),
        }
    }

    pub fn ascent(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.ascent,
            Self::Variant(variant) => variant.frame.ascent(),
            Self::Frame(fragment) => fragment.frame.baseline(),
            _ => Abs::zero(),
        }
    }

    pub fn descent(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.descent,
            Self::Variant(variant) => variant.frame.descent(),
            Self::Frame(fragment) => fragment.frame.descent(),
            _ => Abs::zero(),
        }
    }

    pub fn class(&self) -> MathClass {
        match self {
            Self::Glyph(glyph) => glyph.class,
            Self::Variant(variant) => variant.class,
            Self::Frame(fragment) => fragment.class,
            Self::Spacing(_) => MathClass::Space,
            Self::Space(_) => MathClass::Space,
            Self::Linebreak => MathClass::Space,
            Self::Align => MathClass::Special,
        }
    }

    pub fn math_size(&self) -> Option<MathSize> {
        match self {
            Self::Glyph(glyph) => Some(glyph.math_size),
            Self::Variant(variant) => Some(variant.math_size),
            Self::Frame(fragment) => Some(fragment.math_size),
            _ => None,
        }
    }

    pub fn font_size(&self) -> Option<Abs> {
        match self {
            Self::Glyph(glyph) => Some(glyph.font_size),
            Self::Variant(variant) => Some(variant.font_size),
            Self::Frame(fragment) => Some(fragment.font_size),
            _ => None,
        }
    }

    pub fn set_class(&mut self, class: MathClass) {
        match self {
            Self::Glyph(glyph) => glyph.class = class,
            Self::Variant(variant) => variant.class = class,
            Self::Frame(fragment) => fragment.class = class,
            _ => {}
        }
    }

    pub fn set_limits(&mut self, limits: Limits) {
        match self {
            Self::Glyph(glyph) => glyph.limits = limits,
            Self::Variant(variant) => variant.limits = limits,
            Self::Frame(fragment) => fragment.limits = limits,
            _ => {}
        }
    }

    pub fn is_spaced(&self) -> bool {
        self.class() == MathClass::Fence
            || match self {
                MathFragment::Frame(frame) => {
                    frame.spaced
                        && matches!(
                            frame.class,
                            MathClass::Normal | MathClass::Alphabetic
                        )
                }
                _ => false,
            }
    }

    pub fn is_text_like(&self) -> bool {
        match self {
            Self::Glyph(_) | Self::Variant(_) => self.class() != MathClass::Large,
            MathFragment::Frame(frame) => frame.text_like,
            _ => false,
        }
    }

    pub fn italics_correction(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.italics_correction,
            Self::Variant(variant) => variant.italics_correction,
            Self::Frame(fragment) => fragment.italics_correction,
            _ => Abs::zero(),
        }
    }

    pub fn accent_attach(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.accent_attach,
            Self::Variant(variant) => variant.accent_attach,
            Self::Frame(fragment) => fragment.accent_attach,
            _ => self.width() / 2.0,
        }
    }

    pub fn into_frame(self) -> Frame {
        match self {
            Self::Glyph(glyph) => glyph.into_frame(),
            Self::Variant(variant) => variant.frame,
            Self::Frame(fragment) => fragment.frame,
            _ => Frame::soft(self.size()),
        }
    }

    pub fn limits(&self) -> Limits {
        match self {
            MathFragment::Glyph(glyph) => glyph.limits,
            MathFragment::Variant(variant) => variant.limits,
            MathFragment::Frame(fragment) => fragment.limits,
            _ => Limits::Never,
        }
    }
}

impl From<GlyphFragment> for MathFragment {
    fn from(glyph: GlyphFragment) -> Self {
        Self::Glyph(glyph)
    }
}

impl From<VariantFragment> for MathFragment {
    fn from(variant: VariantFragment) -> Self {
        Self::Variant(variant)
    }
}

impl From<FrameFragment> for MathFragment {
    fn from(fragment: FrameFragment) -> Self {
        Self::Frame(fragment)
    }
}

impl From<SpacingFragment> for MathFragment {
    fn from(fragment: SpacingFragment) -> Self {
        Self::Spacing(fragment)
    }
}

#[derive(Clone)]
pub struct GlyphFragment {
    pub id: GlyphId,
    pub c: char,
    pub font: Font,
    pub lang: Lang,
    pub region: Option<Region>,
    pub fill: Paint,
    pub shift: Abs,
    pub width: Abs,
    pub ascent: Abs,
    pub descent: Abs,
    pub italics_correction: Abs,
    pub accent_attach: Abs,
    pub font_size: Abs,
    pub class: MathClass,
    pub math_size: MathSize,
    pub span: Span,
    pub dests: SmallVec<[Destination; 1]>,
    pub hidden: bool,
    pub limits: Limits,
}

impl GlyphFragment {
    pub fn new(ctx: &MathContext, styles: StyleChain, c: char, span: Span) -> Self {
        let id = ctx.ttf.glyph_index(c).unwrap_or_default();
        let id = Self::adjust_glyph_index(ctx, id);
        Self::with_id(ctx, styles, c, id, span)
    }

    pub fn try_new(
        ctx: &MathContext,
        styles: StyleChain,
        c: char,
        span: Span,
    ) -> Option<Self> {
        let id = ctx.ttf.glyph_index(c)?;
        let id = Self::adjust_glyph_index(ctx, id);
        Some(Self::with_id(ctx, styles, c, id, span))
    }

    pub fn with_id(
        ctx: &MathContext,
        styles: StyleChain,
        c: char,
        id: GlyphId,
        span: Span,
    ) -> Self {
        let class = EquationElem::class_in(styles)
            .or_else(|| match c {
                ':' => Some(MathClass::Relation),
                '.' | '/' | '⋯' | '⋱' | '⋰' | '⋮' => Some(MathClass::Normal),
                _ => unicode_math_class::class(c),
            })
            .unwrap_or(MathClass::Normal);

        let mut fragment = Self {
            id,
            c,
            font: ctx.font.clone(),
            lang: TextElem::lang_in(styles),
            region: TextElem::region_in(styles),
            fill: TextElem::fill_in(styles).as_decoration(),
            shift: TextElem::baseline_in(styles),
            font_size: scaled_font_size(ctx, styles),
            math_size: EquationElem::size_in(styles),
            width: Abs::zero(),
            ascent: Abs::zero(),
            descent: Abs::zero(),
            limits: Limits::for_char(c),
            italics_correction: Abs::zero(),
            accent_attach: Abs::zero(),
            class,
            span,
            dests: LinkElem::dests_in(styles),
            hidden: HideElem::hidden_in(styles),
        };
        fragment.set_id(ctx, id);
        fragment
    }

    /// Apply GSUB substitutions.
    fn adjust_glyph_index(ctx: &MathContext, id: GlyphId) -> GlyphId {
        if let Some(glyphwise_tables) = &ctx.glyphwise_tables {
            glyphwise_tables.iter().fold(id, |id, table| table.apply(id))
        } else {
            id
        }
    }

    /// Sets element id and boxes in appropriate way without changing other
    /// styles. This is used to replace the glyph with a stretch variant.
    pub fn set_id(&mut self, ctx: &MathContext, id: GlyphId) {
        let advance = ctx.ttf.glyph_hor_advance(id).unwrap_or_default();
        let italics = italics_correction(ctx, id, self.font_size).unwrap_or_default();
        let bbox = ctx.ttf.glyph_bounding_box(id).unwrap_or(Rect {
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
        });

        let mut width = advance.scaled(ctx, self.font_size);
        let accent_attach =
            accent_attach(ctx, id, self.font_size).unwrap_or((width + italics) / 2.0);

        if !is_extended_shape(ctx, id) {
            width += italics;
        }

        self.id = id;
        self.width = width;
        self.ascent = bbox.y_max.scaled(ctx, self.font_size);
        self.descent = -bbox.y_min.scaled(ctx, self.font_size);
        self.italics_correction = italics;
        self.accent_attach = accent_attach;
    }

    pub fn height(&self) -> Abs {
        self.ascent + self.descent
    }

    pub fn into_variant(self) -> VariantFragment {
        VariantFragment {
            c: self.c,
            id: Some(self.id),
            font_size: self.font_size,
            italics_correction: self.italics_correction,
            accent_attach: self.accent_attach,
            class: self.class,
            math_size: self.math_size,
            span: self.span,
            limits: self.limits,
            frame: self.into_frame(),
            mid_stretched: None,
        }
    }

    pub fn into_frame(self) -> Frame {
        let item = TextItem {
            font: self.font.clone(),
            size: self.font_size,
            fill: self.fill,
            lang: self.lang,
            region: self.region,
            text: self.c.into(),
            stroke: None,
            glyphs: vec![Glyph {
                id: self.id.0,
                x_advance: Em::from_length(self.width, self.font_size),
                x_offset: Em::zero(),
                range: 0..self.c.len_utf8() as u16,
                span: (self.span, 0),
            }],
        };
        let size = Size::new(self.width, self.ascent + self.descent);
        let mut frame = Frame::soft(size);
        frame.set_baseline(self.ascent);
        frame.push(Point::with_y(self.ascent + self.shift), FrameItem::Text(item));
        frame.post_process_raw(self.dests, self.hidden);
        frame
    }

    pub fn make_scriptsize(&mut self, ctx: &MathContext) {
        let alt_id =
            script_alternatives(ctx, self.id).and_then(|alts| alts.alternates.get(0));

        if let Some(alt_id) = alt_id {
            self.set_id(ctx, alt_id);
        }
    }

    pub fn make_scriptscriptsize(&mut self, ctx: &MathContext) {
        let alts = script_alternatives(ctx, self.id);
        let alt_id = alts
            .and_then(|alts| alts.alternates.get(1).or_else(|| alts.alternates.get(0)));

        if let Some(alt_id) = alt_id {
            self.set_id(ctx, alt_id);
        }
    }
}

impl Debug for GlyphFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphFragment({:?})", self.c)
    }
}

#[derive(Clone)]
pub struct VariantFragment {
    pub c: char,
    pub id: Option<GlyphId>,
    pub italics_correction: Abs,
    pub accent_attach: Abs,
    pub frame: Frame,
    pub font_size: Abs,
    pub class: MathClass,
    pub math_size: MathSize,
    pub span: Span,
    pub limits: Limits,
    pub mid_stretched: Option<bool>,
}

impl VariantFragment {
    /// Vertically adjust the fragment's frame so that it is centered
    /// on the axis.
    pub fn center_on_axis(&mut self, ctx: &MathContext) {
        let h = self.frame.height();
        let axis = ctx.constants.axis_height().scaled(ctx, self.font_size);
        self.frame.set_baseline(h / 2.0 + axis);
    }
}

impl Debug for VariantFragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "VariantFragment({:?})", self.c)
    }
}

#[derive(Debug, Clone)]
pub struct FrameFragment {
    pub frame: Frame,
    pub font_size: Abs,
    pub class: MathClass,
    pub math_size: MathSize,
    pub limits: Limits,
    pub spaced: bool,
    pub base_ascent: Abs,
    pub italics_correction: Abs,
    pub accent_attach: Abs,
    pub text_like: bool,
}

impl FrameFragment {
    pub fn new(ctx: &MathContext, styles: StyleChain, mut frame: Frame) -> Self {
        let base_ascent = frame.ascent();
        let accent_attach = frame.width() / 2.0;
        frame.post_process(styles);
        Self {
            frame,
            font_size: scaled_font_size(ctx, styles),
            class: EquationElem::class_in(styles).unwrap_or(MathClass::Normal),
            math_size: EquationElem::size_in(styles),
            limits: Limits::Never,
            spaced: false,
            base_ascent,
            italics_correction: Abs::zero(),
            accent_attach,
            text_like: false,
        }
    }

    pub fn with_class(self, class: MathClass) -> Self {
        Self { class, ..self }
    }

    pub fn with_limits(self, limits: Limits) -> Self {
        Self { limits, ..self }
    }

    pub fn with_spaced(self, spaced: bool) -> Self {
        Self { spaced, ..self }
    }

    pub fn with_base_ascent(self, base_ascent: Abs) -> Self {
        Self { base_ascent, ..self }
    }

    pub fn with_italics_correction(self, italics_correction: Abs) -> Self {
        Self { italics_correction, ..self }
    }

    pub fn with_accent_attach(self, accent_attach: Abs) -> Self {
        Self { accent_attach, ..self }
    }

    pub fn with_text_like(self, text_like: bool) -> Self {
        Self { text_like, ..self }
    }
}

#[derive(Debug, Clone)]
pub struct SpacingFragment {
    pub width: Abs,
    pub weak: bool,
}

/// Look up the italics correction for a glyph.
fn italics_correction(ctx: &MathContext, id: GlyphId, font_size: Abs) -> Option<Abs> {
    Some(
        ctx.table
            .glyph_info?
            .italic_corrections?
            .get(id)?
            .scaled(ctx, font_size),
    )
}

/// Loop up the top accent attachment position for a glyph.
fn accent_attach(ctx: &MathContext, id: GlyphId, font_size: Abs) -> Option<Abs> {
    Some(
        ctx.table
            .glyph_info?
            .top_accent_attachments?
            .get(id)?
            .scaled(ctx, font_size),
    )
}

/// Look up the script/scriptscript alternates for a glyph
fn script_alternatives<'a>(
    ctx: &MathContext<'a, '_, '_>,
    id: GlyphId,
) -> Option<AlternateSet<'a>> {
    ctx.ssty_table.and_then(|ssty| {
        ssty.coverage.get(id).and_then(|index| ssty.alternate_sets.get(index))
    })
}

/// Look up whether a glyph is an extended shape.
fn is_extended_shape(ctx: &MathContext, id: GlyphId) -> bool {
    ctx.table
        .glyph_info
        .and_then(|info| info.extended_shapes)
        .and_then(|info| info.get(id))
        .is_some()
}

/// Look up a kerning value at a specific corner and height.
///
/// This can be integrated once we've found a font that actually provides this
/// data.
#[allow(unused)]
fn kern_at_height(
    ctx: &MathContext,
    font_size: Abs,
    id: GlyphId,
    corner: Corner,
    height: Abs,
) -> Option<Abs> {
    let kerns = ctx.table.glyph_info?.kern_infos?.get(id)?;
    let kern = match corner {
        Corner::TopLeft => kerns.top_left,
        Corner::TopRight => kerns.top_right,
        Corner::BottomRight => kerns.bottom_right,
        Corner::BottomLeft => kerns.bottom_left,
    }?;

    let mut i = 0;
    while i < kern.count() && height > kern.height(i)?.scaled(ctx, font_size) {
        i += 1;
    }

    Some(kern.kern(i)?.scaled(ctx, font_size))
}
