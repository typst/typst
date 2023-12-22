use std::fmt::{self, Debug, Formatter};

use smallvec::SmallVec;
use ttf_parser::gsub::AlternateSet;
use ttf_parser::{GlyphId, Rect};
use unicode_math_class::MathClass;

use crate::foundations::Smart;
use crate::introspection::{Meta, MetaElem};
use crate::layout::{Abs, Corner, Em, Frame, FrameItem, Point, Size};
use crate::math::{Limits, MathContext, MathStyle, Scaled};
use crate::syntax::Span;
use crate::text::{Font, Glyph, Lang, TextElem, TextItem};
use crate::visualize::Paint;

#[derive(Debug, Clone)]
pub enum MathFragment {
    Glyph(GlyphFragment),
    Variant(VariantFragment),
    Frame(FrameFragment),
    Spacing(Abs),
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
            Self::Spacing(amount) => *amount,
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

    pub fn class(&self) -> Option<MathClass> {
        self.style().and_then(|style| style.class.as_custom()).or(match self {
            Self::Glyph(glyph) => glyph.class,
            Self::Variant(variant) => variant.class,
            Self::Frame(fragment) => Some(fragment.class),
            _ => None,
        })
    }

    pub fn style(&self) -> Option<MathStyle> {
        match self {
            Self::Glyph(glyph) => Some(glyph.style),
            Self::Variant(variant) => Some(variant.style),
            Self::Frame(fragment) => Some(fragment.style),
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
        macro_rules! set_style_class {
            ($fragment:ident) => {
                if $fragment.style.class.is_custom() {
                    $fragment.style.class = Smart::Custom(class);
                }
            };
        }

        match self {
            Self::Glyph(glyph) => {
                glyph.class = Some(class);
                set_style_class!(glyph);
            }
            Self::Variant(variant) => {
                variant.class = Some(class);
                set_style_class!(variant);
            }
            Self::Frame(fragment) => {
                fragment.class = class;
                set_style_class!(fragment);
            }
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
        match self {
            MathFragment::Frame(frame) => {
                match self.style().and_then(|style| style.class.as_custom()) {
                    Some(MathClass::Fence) => true,
                    Some(_) => false,
                    None => frame.spaced,
                }
            }
            _ => self.class() == Some(MathClass::Fence),
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

#[derive(Clone)]
pub struct GlyphFragment {
    pub id: GlyphId,
    pub c: char,
    pub font: Font,
    pub lang: Lang,
    pub fill: Paint,
    pub shift: Abs,
    pub width: Abs,
    pub ascent: Abs,
    pub descent: Abs,
    pub italics_correction: Abs,
    pub accent_attach: Abs,
    pub style: MathStyle,
    pub font_size: Abs,
    pub class: Option<MathClass>,
    pub span: Span,
    pub meta: SmallVec<[Meta; 1]>,
    pub limits: Limits,
}

impl GlyphFragment {
    pub fn new(ctx: &MathContext, c: char, span: Span) -> Self {
        let id = ctx.ttf.glyph_index(c).unwrap_or_default();
        let id = Self::adjust_glyph_index(ctx, id);
        Self::with_id(ctx, c, id, span)
    }

    pub fn try_new(ctx: &MathContext, c: char, span: Span) -> Option<Self> {
        let c = ctx.style.styled_char(c);
        let id = ctx.ttf.glyph_index(c)?;
        let id = Self::adjust_glyph_index(ctx, id);
        Some(Self::with_id(ctx, c, id, span))
    }

    pub fn with_id(ctx: &MathContext, c: char, id: GlyphId, span: Span) -> Self {
        let class = match c {
            ':' => Some(MathClass::Relation),
            '.' | '/' | '⋯' | '⋱' | '⋰' | '⋮' => Some(MathClass::Normal),
            _ => unicode_math_class::class(c),
        };
        let mut fragment = Self {
            id,
            c,
            font: ctx.font.clone(),
            lang: TextElem::lang_in(ctx.styles()),
            fill: TextElem::fill_in(ctx.styles()).as_decoration(),
            shift: TextElem::baseline_in(ctx.styles()),
            style: ctx.style,
            font_size: ctx.size,
            width: Abs::zero(),
            ascent: Abs::zero(),
            descent: Abs::zero(),
            limits: Limits::for_char(c),
            italics_correction: Abs::zero(),
            accent_attach: Abs::zero(),
            class,
            span,
            meta: MetaElem::data_in(ctx.styles()),
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
        let italics = italics_correction(ctx, id).unwrap_or_default();
        let bbox = ctx.ttf.glyph_bounding_box(id).unwrap_or(Rect {
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
        });

        let mut width = advance.scaled(ctx);
        let accent_attach = accent_attach(ctx, id).unwrap_or((width + italics) / 2.0);

        if !is_extended_shape(ctx, id) {
            width += italics;
        }

        self.id = id;
        self.width = width;
        self.ascent = bbox.y_max.scaled(ctx);
        self.descent = -bbox.y_min.scaled(ctx);
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
            style: self.style,
            font_size: self.font_size,
            italics_correction: self.italics_correction,
            accent_attach: self.accent_attach,
            class: self.class,
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
        frame.meta_iter(self.meta);
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
    pub style: MathStyle,
    pub font_size: Abs,
    pub class: Option<MathClass>,
    pub span: Span,
    pub limits: Limits,
    pub mid_stretched: Option<bool>,
}

impl VariantFragment {
    /// Vertically adjust the fragment's frame so that it is centered
    /// on the axis.
    pub fn center_on_axis(&mut self, ctx: &MathContext) {
        let h = self.frame.height();
        self.frame.set_baseline(h / 2.0 + scaled!(ctx, axis_height));
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
    pub style: MathStyle,
    pub font_size: Abs,
    pub class: MathClass,
    pub limits: Limits,
    pub spaced: bool,
    pub base_ascent: Abs,
    pub italics_correction: Abs,
    pub accent_attach: Abs,
}

impl FrameFragment {
    pub fn new(ctx: &MathContext, mut frame: Frame) -> Self {
        let base_ascent = frame.ascent();
        let accent_attach = frame.width() / 2.0;
        frame.meta(ctx.styles(), false);
        Self {
            frame,
            font_size: ctx.size,
            style: ctx.style,
            class: MathClass::Normal,
            limits: Limits::Never,
            spaced: false,
            base_ascent,
            italics_correction: Abs::zero(),
            accent_attach,
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
}

/// Look up the italics correction for a glyph.
fn italics_correction(ctx: &MathContext, id: GlyphId) -> Option<Abs> {
    Some(ctx.table.glyph_info?.italic_corrections?.get(id)?.scaled(ctx))
}

/// Loop up the top accent attachment position for a glyph.
fn accent_attach(ctx: &MathContext, id: GlyphId) -> Option<Abs> {
    Some(ctx.table.glyph_info?.top_accent_attachments?.get(id)?.scaled(ctx))
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
    while i < kern.count() && height > kern.height(i)?.scaled(ctx) {
        i += 1;
    }

    Some(kern.kern(i)?.scaled(ctx))
}
