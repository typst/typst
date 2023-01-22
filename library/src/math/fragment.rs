use super::*;

#[derive(Debug, Clone)]
pub(super) enum MathFragment {
    Glyph(GlyphFragment),
    Variant(VariantFragment),
    Frame(FrameFragment),
    Spacing(Abs),
    Align,
    Linebreak,
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
            Self::Glyph(glyph) => glyph.class(),
            Self::Variant(variant) => variant.class(),
            Self::Frame(fragment) => Some(fragment.class),
            _ => None,
        }
    }

    pub fn italics_correction(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.italics_correction,
            Self::Variant(variant) => variant.italics_correction,
            _ => Abs::zero(),
        }
    }

    pub fn to_frame(self, ctx: &MathContext) -> Frame {
        match self {
            Self::Glyph(glyph) => glyph.to_frame(ctx),
            Self::Variant(variant) => variant.frame,
            Self::Frame(fragment) => fragment.frame,
            _ => Frame::new(self.size()),
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

impl From<Frame> for MathFragment {
    fn from(frame: Frame) -> Self {
        Self::Frame(FrameFragment { frame, class: MathClass::Normal, limits: false })
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GlyphFragment {
    pub id: GlyphId,
    pub c: char,
    pub font_size: Abs,
    pub width: Abs,
    pub ascent: Abs,
    pub descent: Abs,
    pub italics_correction: Abs,
}

impl GlyphFragment {
    pub fn new(ctx: &MathContext, c: char) -> Self {
        let c = ctx.style.styled_char(c);
        let id = ctx.ttf.glyph_index(c).unwrap_or_default();
        Self::with_id(ctx, c, id)
    }

    pub fn try_new(ctx: &MathContext, c: char) -> Option<Self> {
        let c = ctx.style.styled_char(c);
        let id = ctx.ttf.glyph_index(c)?;
        Some(Self::with_id(ctx, c, id))
    }

    pub fn with_id(ctx: &MathContext, c: char, id: GlyphId) -> Self {
        let advance = ctx.ttf.glyph_hor_advance(id).unwrap_or_default();
        let italics = italics_correction(ctx, id).unwrap_or_default();
        let bbox = ctx.ttf.glyph_bounding_box(id).unwrap_or(Rect {
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
        });
        Self {
            id,
            c,
            font_size: ctx.size(),
            width: advance.scaled(ctx),
            ascent: bbox.y_max.scaled(ctx),
            descent: -bbox.y_min.scaled(ctx),
            italics_correction: italics,
        }
    }

    pub fn height(&self) -> Abs {
        self.ascent + self.descent
    }

    pub fn class(&self) -> Option<MathClass> {
        unicode_math_class::class(self.c)
    }

    pub fn to_variant(&self, ctx: &MathContext) -> VariantFragment {
        VariantFragment {
            c: self.c,
            id: Some(self.id),
            frame: self.to_frame(ctx),
            italics_correction: self.italics_correction,
        }
    }

    pub fn to_frame(&self, ctx: &MathContext) -> Frame {
        let text = Text {
            font: ctx.font.clone(),
            size: self.font_size,
            fill: ctx.fill,
            lang: ctx.lang,
            glyphs: vec![Glyph {
                id: self.id.0,
                c: self.c,
                x_advance: Em::from_length(self.width, ctx.size()),
                x_offset: Em::zero(),
            }],
        };
        let size = Size::new(self.width, self.ascent + self.descent);
        let mut frame = Frame::new(size);
        frame.set_baseline(self.ascent);
        frame.push(Point::with_y(self.ascent), Element::Text(text));
        frame
    }
}

#[derive(Debug, Clone)]
pub struct VariantFragment {
    pub c: char,
    pub id: Option<GlyphId>,
    pub frame: Frame,
    pub italics_correction: Abs,
}

impl VariantFragment {
    pub fn class(&self) -> Option<MathClass> {
        unicode_math_class::class(self.c)
    }
}

#[derive(Debug, Clone)]
pub struct FrameFragment {
    pub frame: Frame,
    pub class: MathClass,
    pub limits: bool,
}

/// Look up the italics correction for a glyph.
fn italics_correction(ctx: &MathContext, id: GlyphId) -> Option<Abs> {
    Some(ctx.table.glyph_info?.italic_corrections?.get(id)?.scaled(ctx))
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
