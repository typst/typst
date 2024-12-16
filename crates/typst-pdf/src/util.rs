//! Convert basic primitive types from typst to krilla.

use krilla::page::{NumberingStyle, PageLabel};
use std::num::NonZeroUsize;
use typst_library::layout::{Abs, Point, Size, Transform};
use typst_library::model::Numbering;
use typst_library::text::Font;
use typst_library::visualize::{FillRule, LineCap, LineJoin};

pub(crate) trait SizeExt {
    fn as_krilla(&self) -> krilla::geom::Size;
}

impl SizeExt for Size {
    fn as_krilla(&self) -> krilla::geom::Size {
        krilla::geom::Size::from_wh(self.x.to_f32(), self.y.to_f32()).unwrap()
    }
}

pub(crate) trait PointExt {
    fn as_krilla(&self) -> krilla::geom::Point;
}

impl PointExt for Point {
    fn as_krilla(&self) -> krilla::geom::Point {
        krilla::geom::Point::from_xy(self.x.to_f32(), self.y.to_f32())
    }
}

pub(crate) trait LineCapExt {
    fn as_krilla(&self) -> krilla::path::LineCap;
}

impl LineCapExt for LineCap {
    fn as_krilla(&self) -> krilla::path::LineCap {
        match self {
            LineCap::Butt => krilla::path::LineCap::Butt,
            LineCap::Round => krilla::path::LineCap::Round,
            LineCap::Square => krilla::path::LineCap::Square,
        }
    }
}

pub(crate) trait LineJoinExt {
    fn as_krilla(&self) -> krilla::path::LineJoin;
}

impl LineJoinExt for LineJoin {
    fn as_krilla(&self) -> krilla::path::LineJoin {
        match self {
            LineJoin::Miter => krilla::path::LineJoin::Miter,
            LineJoin::Round => krilla::path::LineJoin::Round,
            LineJoin::Bevel => krilla::path::LineJoin::Bevel,
        }
    }
}

pub(crate) trait TransformExt {
    fn as_krilla(&self) -> krilla::geom::Transform;
}

impl TransformExt for Transform {
    fn as_krilla(&self) -> krilla::geom::Transform {
        krilla::geom::Transform::from_row(
            self.sx.get() as f32,
            self.ky.get() as f32,
            self.kx.get() as f32,
            self.sy.get() as f32,
            self.tx.to_f32(),
            self.ty.to_f32(),
        )
    }
}

pub(crate) trait FillRuleExt {
    fn as_krilla(&self) -> krilla::path::FillRule;
}

impl FillRuleExt for FillRule {
    fn as_krilla(&self) -> krilla::path::FillRule {
        match self {
            FillRule::NonZero => krilla::path::FillRule::NonZero,
            FillRule::EvenOdd => krilla::path::FillRule::EvenOdd,
        }
    }
}

/// Additional methods for [`Abs`].
pub(crate) trait AbsExt {
    /// Convert an to a number of points.
    fn to_f32(self) -> f32;
}

impl AbsExt for Abs {
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
    }
}

pub(crate) trait PageLabelExt {
    fn generate(numbering: &Numbering, number: usize) -> Option<PageLabel>;
    fn arabic(number: usize) -> PageLabel;
}

impl PageLabelExt for PageLabel {
    /// Create a new `PageLabel` from a `Numbering` applied to a page
    /// number.
    fn generate(numbering: &Numbering, number: usize) -> Option<PageLabel> {
        {
            let Numbering::Pattern(pat) = numbering else {
                return None;
            };

            let (prefix, kind) = pat.pieces.first()?;

            // If there is a suffix, we cannot use the common style optimisation,
            // since PDF does not provide a suffix field.
            let style = if pat.suffix.is_empty() {
                use krilla::page::NumberingStyle as Style;
                use typst_library::model::NumberingKind as Kind;
                match kind {
                    Kind::Arabic => Some(Style::Arabic),
                    Kind::LowerRoman => Some(Style::LowerRoman),
                    Kind::UpperRoman => Some(Style::UpperRoman),
                    Kind::LowerLatin if number <= 26 => Some(Style::LowerAlpha),
                    Kind::LowerLatin if number <= 26 => Some(Style::UpperAlpha),
                    _ => None,
                }
            } else {
                None
            };

            // Prefix and offset depend on the style: If it is supported by the PDF
            // spec, we use the given prefix and an offset. Otherwise, everything
            // goes into prefix.
            let prefix = if style.is_none() {
                Some(pat.apply(&[number]))
            } else {
                (!prefix.is_empty()).then(|| prefix.clone())
            };

            let offset = style.and(NonZeroUsize::new(number));
            Some(PageLabel::new(style, prefix.map(|s| s.to_string()), offset))
        }
    }

    /// Creates an arabic page label with the specified page number.
    /// For example, this will display page label `11` when given the page
    /// number 11.
    fn arabic(number: usize) -> PageLabel {
        PageLabel::new(Some(NumberingStyle::Arabic), None, NonZeroUsize::new(number))
    }
}

pub(crate) fn font_to_str(font: &Font) -> String {
    let font_family = &font.info().family;
    let font_variant = font.info().variant;
    format!("{} ({:?})", font_family, font_variant)
}
