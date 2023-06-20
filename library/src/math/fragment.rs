use super::*;

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
        match self {
            Self::Glyph(glyph) => glyph.class,
            Self::Variant(variant) => variant.class,
            Self::Frame(fragment) => Some(fragment.class),
            _ => None,
        }
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
        match self {
            Self::Glyph(glyph) => glyph.class = Some(class),
            Self::Variant(variant) => variant.class = Some(class),
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
        match self {
            MathFragment::Frame(frame) => frame.spaced,
            _ => self.class() == Some(MathClass::Fence),
        }
    }

    pub fn italics_correction(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.italics_correction,
            Self::Variant(variant) => variant.italics_correction,
            _ => Abs::zero(),
        }
    }

    pub fn into_frame(self) -> Frame {
        match self {
            Self::Glyph(glyph) => glyph.into_frame(),
            Self::Variant(variant) => variant.frame,
            Self::Frame(fragment) => fragment.frame,
            _ => Frame::new(self.size()),
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
    pub width: Abs,
    pub ascent: Abs,
    pub descent: Abs,
    pub italics_correction: Abs,
    pub style: MathStyle,
    pub font_size: Abs,
    pub class: Option<MathClass>,
    pub span: Span,
    pub meta: Vec<Meta>,
    pub limits: Limits,
}

impl GlyphFragment {
    pub fn new(ctx: &MathContext, c: char, span: Span) -> Self {
        let id = ctx.ttf.glyph_index(c).unwrap_or_default();
        Self::with_id(ctx, c, id, span)
    }

    pub fn try_new(ctx: &MathContext, c: char, span: Span) -> Option<Self> {
        let c = ctx.style.styled_char(c);
        let id = ctx.ttf.glyph_index(c)?;
        Some(Self::with_id(ctx, c, id, span))
    }

    pub fn with_id(ctx: &MathContext, c: char, id: GlyphId, span: Span) -> Self {
        let class = match c {
            ':' => Some(MathClass::Relation),
            _ => unicode_math_class::class(c),
        };
        let mut fragment = Self {
            id,
            c,
            font: ctx.font.clone(),
            lang: TextElem::lang_in(ctx.styles()),
            fill: TextElem::fill_in(ctx.styles()),
            style: ctx.style,
            font_size: ctx.size,
            width: Abs::zero(),
            ascent: Abs::zero(),
            descent: Abs::zero(),
            limits: Limits::for_char(c),
            italics_correction: Abs::zero(),
            class,
            span,
            meta: MetaElem::data_in(ctx.styles()),
        };
        fragment.set_id(ctx, id);
        fragment
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
        if !is_extended_shape(ctx, id) {
            width += italics;
        }

        self.id = id;
        self.width = width;
        self.ascent = bbox.y_max.scaled(ctx);
        self.descent = -bbox.y_min.scaled(ctx);
        self.italics_correction = italics;
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
            class: self.class,
            span: self.span,
            limits: self.limits,
            frame: self.into_frame(),
        }
    }

    pub fn into_frame(self) -> Frame {
        let item = TextItem {
            font: self.font.clone(),
            size: self.font_size,
            fill: self.fill,
            lang: self.lang,
            text: self.c.into(),
            glyphs: vec![Glyph {
                id: self.id.0,
                x_advance: Em::from_length(self.width, self.font_size),
                x_offset: Em::zero(),
                range: 0..self.c.len_utf8() as u16,
                span: (self.span, 0),
            }],
        };
        let size = Size::new(self.width, self.ascent + self.descent);
        let mut frame = Frame::new(size);
        frame.set_baseline(self.ascent);
        frame.push(Point::with_y(self.ascent), FrameItem::Text(item));
        frame.meta_iter(self.meta);
        frame
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
    pub frame: Frame,
    pub style: MathStyle,
    pub font_size: Abs,
    pub class: Option<MathClass>,
    pub span: Span,
    pub limits: Limits,
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
}

impl FrameFragment {
    pub fn new(ctx: &MathContext, mut frame: Frame) -> Self {
        let base_ascent = frame.ascent();
        frame.meta(ctx.styles(), false);
        Self {
            frame,
            font_size: ctx.size,
            style: ctx.style,
            class: MathClass::Normal,
            limits: Limits::Never,
            spaced: false,
            base_ascent,
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
}

/// Look up the italics correction for a glyph.
fn italics_correction(ctx: &MathContext, id: GlyphId) -> Option<Abs> {
    Some(ctx.table.glyph_info?.italic_corrections?.get(id)?.scaled(ctx))
}

/// Look up the italics correction for a glyph.
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
