mod glyph;

pub(crate) use self::glyph::GlyphFragment;

use std::fmt::Debug;

use ttf_parser::GlyphId;
use typst_library::foundations::StyleChain;
use typst_library::introspection::Tag;
use typst_library::layout::{Abs, Axis, Corner, Em, Frame, FrameItem, Point, Size};
use typst_library::math::MathSize;
use typst_library::math::ir::MathProperties;
use typst_library::text::{FontInstance, TextElem};
use typst_library::visualize::{FixedStroke, Paint};
use typst_utils::Get;
use unicode_math_class::MathClass;

use self::glyph::kern_at_height;
use super::MathContext;
use crate::modifiers::{FrameModifiers, FrameModify};

#[derive(Debug, Clone)]
pub enum MathFragment {
    Glyph(GlyphFragment),
    Frame(FrameFragment),
    Space(Abs),
    Tag(Tag),
}

impl MathFragment {
    pub fn size(&self) -> Size {
        match self {
            Self::Glyph(glyph) => glyph.size,
            Self::Frame(fragment) => fragment.frame.size(),
            Self::Space(amount) => Size::with_x(*amount),
            _ => Size::zero(),
        }
    }

    pub fn width(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.size.x,
            Self::Frame(fragment) => fragment.frame.width(),
            Self::Space(amount) => *amount,
            _ => Abs::zero(),
        }
    }

    pub fn height(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.size.y,
            Self::Frame(fragment) => fragment.frame.height(),
            _ => Abs::zero(),
        }
    }

    pub fn ascent(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.ascent(),
            Self::Frame(fragment) => fragment.frame.ascent(),
            _ => Abs::zero(),
        }
    }

    pub fn descent(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.descent(),
            Self::Frame(fragment) => fragment.frame.descent(),
            _ => Abs::zero(),
        }
    }

    pub fn stroke(&self) -> Option<FixedStroke> {
        match self {
            Self::Glyph(glyph) => glyph.item.stroke.clone(),
            _ => None,
        }
    }

    pub fn base_ascent(&self) -> Abs {
        match self {
            Self::Frame(fragment) => fragment.base_ascent,
            _ => self.ascent(),
        }
    }

    pub fn base_descent(&self) -> Abs {
        match self {
            Self::Frame(fragment) => fragment.base_descent,
            _ => self.descent(),
        }
    }

    pub fn class(&self) -> MathClass {
        match self {
            Self::Glyph(glyph) => glyph.class,
            Self::Frame(fragment) => fragment.class,
            Self::Space(_) => MathClass::Space,
            Self::Tag(_) => MathClass::Special,
        }
    }

    pub fn math_size(&self) -> Option<MathSize> {
        match self {
            Self::Glyph(glyph) => Some(glyph.math_size),
            Self::Frame(fragment) => Some(fragment.math_size),
            _ => None,
        }
    }

    #[inline]
    pub fn font(&self, ctx: &MathContext, styles: StyleChain) -> (FontInstance, Abs) {
        (
            match self {
                Self::Glyph(glyph) => glyph.item.font.clone(),
                _ => ctx.font().clone(),
            },
            self.font_size().unwrap_or_else(|| styles.resolve(TextElem::size)),
        )
    }

    fn font_size(&self) -> Option<Abs> {
        match self {
            Self::Glyph(glyph) => Some(glyph.item.size),
            Self::Frame(fragment) => Some(fragment.font_size),
            _ => None,
        }
    }

    pub fn is_stretchable(&self, axis: Axis) -> bool {
        match self {
            Self::Glyph(glyph) => glyph.stretchable_axes.get(axis),
            _ => false,
        }
    }

    pub fn is_text_like(&self) -> bool {
        match self {
            Self::Glyph(glyph) => !glyph.extended_shape,
            MathFragment::Frame(frame) => frame.text_like,
            _ => false,
        }
    }

    pub fn italics_correction(&self) -> Abs {
        match self {
            Self::Glyph(glyph) => glyph.italics_correction,
            Self::Frame(fragment) => fragment.italics_correction,
            _ => Abs::zero(),
        }
    }

    pub fn accent_attach(&self) -> (Abs, Abs) {
        match self {
            Self::Glyph(glyph) => glyph.accent_attach,
            Self::Frame(fragment) => fragment.accent_attach,
            _ => (self.width() / 2.0, self.width() / 2.0),
        }
    }

    pub fn into_frame(self) -> Frame {
        match self {
            Self::Glyph(glyph) => glyph.into_frame(),
            Self::Frame(fragment) => fragment.frame,
            Self::Tag(tag) => {
                let mut frame = Frame::soft(Size::zero());
                frame.push(Point::zero(), FrameItem::Tag(tag));
                frame
            }
            _ => Frame::soft(self.size()),
        }
    }

    pub fn fill(&self) -> Option<Paint> {
        match self {
            Self::Glyph(glyph) => Some(glyph.item.fill.clone()),
            _ => None,
        }
    }

    /// If no kern table is provided for a corner, a kerning amount of zero is
    /// assumed.
    pub fn kern_at_height(&self, corner: Corner, height: Abs) -> Abs {
        match self {
            Self::Glyph(glyph) => {
                // For glyph assemblies we pick either the start or end glyph
                // depending on the corner.
                let is_vertical =
                    glyph.item.glyphs.iter().all(|glyph| glyph.y_advance != Em::zero());
                let glyph_index = match (is_vertical, corner) {
                    (true, Corner::TopLeft | Corner::TopRight) => {
                        glyph.item.glyphs.len() - 1
                    }
                    (false, Corner::TopRight | Corner::BottomRight) => {
                        glyph.item.glyphs.len() - 1
                    }
                    _ => 0,
                };

                kern_at_height(
                    &glyph.item.font,
                    GlyphId(glyph.item.glyphs[glyph_index].id),
                    corner,
                    Em::from_abs(height, glyph.item.size),
                )
                .unwrap_or_default()
                .at(glyph.item.size)
            }
            _ => Abs::zero(),
        }
    }
}

impl From<GlyphFragment> for MathFragment {
    fn from(glyph: GlyphFragment) -> Self {
        Self::Glyph(glyph)
    }
}

impl From<FrameFragment> for MathFragment {
    fn from(fragment: FrameFragment) -> Self {
        Self::Frame(fragment)
    }
}

#[derive(Debug, Clone)]
pub struct FrameFragment {
    frame: Frame,
    font_size: Abs,
    class: MathClass,
    math_size: MathSize,
    base_ascent: Abs,
    base_descent: Abs,
    italics_correction: Abs,
    accent_attach: (Abs, Abs),
    text_like: bool,
}

impl FrameFragment {
    pub fn new(props: &MathProperties, styles: StyleChain, frame: Frame) -> Self {
        let base_ascent = frame.ascent();
        let base_descent = frame.descent();
        let accent_attach = frame.width() / 2.0;
        let modifiers = FrameModifiers::get_in(styles);
        Self {
            frame: frame.modified(&modifiers),
            font_size: styles.resolve(TextElem::size),
            class: props.class(),
            math_size: props.size,
            base_ascent,
            base_descent,
            italics_correction: Abs::zero(),
            accent_attach: (accent_attach, accent_attach),
            text_like: false,
        }
    }

    pub fn with_base_ascent(self, base_ascent: Abs) -> Self {
        Self { base_ascent, ..self }
    }

    pub fn with_base_descent(self, base_descent: Abs) -> Self {
        Self { base_descent, ..self }
    }

    pub fn with_italics_correction(self, italics_correction: Abs) -> Self {
        Self { italics_correction, ..self }
    }

    pub fn with_accent_attach(self, accent_attach: (Abs, Abs)) -> Self {
        Self { accent_attach, ..self }
    }

    pub fn with_text_like(self, text_like: bool) -> Self {
        Self { text_like, ..self }
    }
}
