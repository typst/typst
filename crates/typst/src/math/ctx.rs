use std::f64::consts::SQRT_2;

use ecow::EcoString;
use rustybuzz::Feature;
use ttf_parser::gsub::{AlternateSubstitution, SingleSubstitution, SubstitutionSubtable};
use ttf_parser::math::MathValue;
use ttf_parser::opentype_layout::LayoutTable;
use ttf_parser::GlyphId;
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{Content, Packed, StyleChain};
use crate::layout::{Abs, Axes, BoxElem, Em, Frame, LayoutMultiple, Regions, Size};
use crate::math::{
    scaled_font_size, styled_char, EquationElem, FrameFragment, GlyphFragment,
    LayoutMath, MathFragment, MathRun, MathSize, THICK,
};
use crate::model::ParElem;
use crate::syntax::{is_newline, Span};
use crate::text::{
    features, BottomEdge, BottomEdgeMetric, Font, TextElem, TextSize, TopEdge,
    TopEdgeMetric,
};

macro_rules! scaled {
    ($ctx:expr, $styles:expr, text: $text:ident, display: $display:ident $(,)?) => {
        match $crate::math::EquationElem::size_in($styles) {
            $crate::math::MathSize::Display => scaled!($ctx, $styles, $display),
            _ => scaled!($ctx, $styles, $text),
        }
    };
    ($ctx:expr, $styles:expr, $name:ident) => {
        $ctx.constants
            .$name()
            .scaled($ctx, $crate::math::scaled_font_size($ctx, $styles))
    };
}

macro_rules! percent {
    ($ctx:expr, $name:ident) => {
        $ctx.constants.$name() as f64 / 100.0
    };
}

/// The context for math layout.
pub struct MathContext<'a, 'b, 'v> {
    // External.
    pub engine: &'v mut Engine<'b>,
    pub regions: Regions<'static>,
    // Font-related.
    pub font: &'a Font,
    pub ttf: &'a ttf_parser::Face<'a>,
    pub table: ttf_parser::math::Table<'a>,
    pub constants: ttf_parser::math::Constants<'a>,
    pub ssty_table: Option<ttf_parser::gsub::AlternateSubstitution<'a>>,
    pub glyphwise_tables: Option<Vec<GlyphwiseSubsts<'a>>>,
    pub space_width: Em,
    // Mutable.
    pub fragments: Vec<MathFragment>,
}

impl<'a, 'b, 'v> MathContext<'a, 'b, 'v> {
    pub fn new(
        engine: &'v mut Engine<'b>,
        styles: StyleChain<'a>,
        regions: Regions,
        font: &'a Font,
    ) -> Self {
        let math_table = font.ttf().tables().math.unwrap();
        let gsub_table = font.ttf().tables().gsub;
        let constants = math_table.constants.unwrap();

        let ssty_table = gsub_table
            .and_then(|gsub| {
                gsub.features
                    .find(ttf_parser::Tag::from_bytes(b"ssty"))
                    .and_then(|feature| feature.lookup_indices.get(0))
                    .and_then(|index| gsub.lookups.get(index))
            })
            .and_then(|ssty| ssty.subtables.get::<SubstitutionSubtable>(0))
            .and_then(|ssty| match ssty {
                SubstitutionSubtable::Alternate(alt_glyphs) => Some(alt_glyphs),
                _ => None,
            });

        let features = features(styles);
        let glyphwise_tables = gsub_table.map(|gsub| {
            features
                .into_iter()
                .filter_map(|feature| GlyphwiseSubsts::new(gsub, feature))
                .collect()
        });

        let ttf = font.ttf();
        let space_width = ttf
            .glyph_index(' ')
            .and_then(|id| ttf.glyph_hor_advance(id))
            .map(|advance| font.to_em(advance))
            .unwrap_or(THICK);

        Self {
            engine,
            regions: Regions::one(regions.base(), Axes::splat(false)),
            font,
            ttf: font.ttf(),
            table: math_table,
            constants,
            ssty_table,
            glyphwise_tables,
            space_width,
            fragments: vec![],
        }
    }

    pub fn push(&mut self, fragment: impl Into<MathFragment>) {
        self.fragments.push(fragment.into());
    }

    pub fn extend(&mut self, fragments: Vec<MathFragment>) {
        self.fragments.extend(fragments);
    }

    /// Layout the given element and return the resulting [`MathFragment`]s.
    pub fn layout_into_fragments(
        &mut self,
        elem: &dyn LayoutMath,
        styles: StyleChain,
    ) -> SourceResult<Vec<MathFragment>> {
        // The element's layout_math() changes the fragments held in this
        // MathContext object, but for convenience this function shouldn't change
        // them, so we restore the MathContext's fragments after obtaining the
        // layout result.
        let prev = std::mem::take(&mut self.fragments);
        elem.layout_math(self, styles)?;
        Ok(std::mem::replace(&mut self.fragments, prev))
    }

    /// Layout the given element and return the result as a [`MathRun`].
    pub fn layout_into_run(
        &mut self,
        elem: &dyn LayoutMath,
        styles: StyleChain,
    ) -> SourceResult<MathRun> {
        Ok(MathRun::new(self.layout_into_fragments(elem, styles)?))
    }

    /// Layout the given element and return the result as a
    /// unified [`MathFragment`].
    pub fn layout_into_fragment(
        &mut self,
        elem: &dyn LayoutMath,
        styles: StyleChain,
    ) -> SourceResult<MathFragment> {
        Ok(self.layout_into_run(elem, styles)?.into_fragment(self, styles))
    }

    /// Layout the given element and return the result as a [`Frame`].
    pub fn layout_into_frame(
        &mut self,
        elem: &dyn LayoutMath,
        styles: StyleChain,
    ) -> SourceResult<Frame> {
        Ok(self.layout_into_fragment(elem, styles)?.into_frame())
    }

    /// Layout the given [`BoxElem`] into a [`Frame`].
    pub fn layout_box(
        &mut self,
        boxed: &Packed<BoxElem>,
        styles: StyleChain,
    ) -> SourceResult<Frame> {
        let local =
            TextElem::set_size(TextSize(scaled_font_size(self, styles).into())).wrap();
        boxed.layout(self.engine, styles.chain(&local), self.regions)
    }

    /// Layout the given [`Content`] into a [`Frame`].
    pub fn layout_content(
        &mut self,
        content: &Content,
        styles: StyleChain,
    ) -> SourceResult<Frame> {
        let local =
            TextElem::set_size(TextSize(scaled_font_size(self, styles).into())).wrap();
        Ok(content
            .layout(self.engine, styles.chain(&local), self.regions)?
            .into_frame())
    }

    /// Layout the given [`TextElem`] into a [`MathFragment`].
    pub fn layout_text(
        &mut self,
        elem: &Packed<TextElem>,
        styles: StyleChain,
    ) -> SourceResult<MathFragment> {
        let text = elem.text();
        let span = elem.span();
        let mut chars = text.chars();
        let math_size = EquationElem::size_in(styles);
        let fragment = if let Some(mut glyph) = chars
            .next()
            .filter(|_| chars.next().is_none())
            .map(|c| styled_char(styles, c, true))
            .and_then(|c| GlyphFragment::try_new(self, styles, c, span))
        {
            // A single letter that is available in the math font.
            match math_size {
                MathSize::Script => {
                    glyph.make_scriptsize(self);
                }
                MathSize::ScriptScript => {
                    glyph.make_scriptscriptsize(self);
                }
                _ => (),
            }

            if glyph.class == MathClass::Large {
                let mut variant = if math_size == MathSize::Display {
                    let height = scaled!(self, styles, display_operator_min_height)
                        .max(SQRT_2 * glyph.height());
                    glyph.stretch_vertical(self, height, Abs::zero())
                } else {
                    glyph.into_variant()
                };
                // TeXbook p 155. Large operators are always vertically centered on the axis.
                variant.center_on_axis(self);
                variant.into()
            } else {
                glyph.into()
            }
        } else if text.chars().all(|c| c.is_ascii_digit() || c == '.') {
            // Numbers aren't that difficult.
            let mut fragments = vec![];
            for c in text.chars() {
                let c = styled_char(styles, c, false);
                fragments.push(GlyphFragment::new(self, styles, c, span).into());
            }
            let frame = MathRun::new(fragments).into_frame(self, styles);
            FrameFragment::new(self, styles, frame).with_text_like(true).into()
        } else {
            let local = [
                TextElem::set_top_edge(TopEdge::Metric(TopEdgeMetric::Bounds)),
                TextElem::set_bottom_edge(BottomEdge::Metric(BottomEdgeMetric::Bounds)),
                TextElem::set_size(TextSize(scaled_font_size(self, styles).into())),
            ]
            .map(|p| p.wrap());

            // Anything else is handled by Typst's standard text layout.
            let styles = styles.chain(&local);
            let text: EcoString =
                text.chars().map(|c| styled_char(styles, c, false)).collect();
            if text.contains(is_newline) {
                let mut fragments = vec![];
                for (i, piece) in text.split(is_newline).enumerate() {
                    if i != 0 {
                        fragments.push(MathFragment::Linebreak);
                    }
                    if !piece.is_empty() {
                        fragments
                            .push(self.layout_complex_text(piece, span, styles)?.into());
                    }
                }
                let mut frame = MathRun::new(fragments).into_frame(self, styles);
                let axis = scaled!(self, styles, axis_height);
                frame.set_baseline(frame.height() / 2.0 + axis);
                FrameFragment::new(self, styles, frame).into()
            } else {
                self.layout_complex_text(&text, span, styles)?.into()
            }
        };
        Ok(fragment)
    }

    /// Layout the given text string into a [`FrameFragment`].
    fn layout_complex_text(
        &mut self,
        text: &str,
        span: Span,
        styles: StyleChain,
    ) -> SourceResult<FrameFragment> {
        // There isn't a natural width for a paragraph in a math environment;
        // because it will be placed somewhere probably not at the left margin
        // it will overflow. So emulate an `hbox` instead and allow the paragraph
        // to extend as far as needed.
        let spaced = text.graphemes(true).nth(1).is_some();
        let text = TextElem::packed(text).spanned(span);
        let par = ParElem::new(vec![text]);
        let frame = Packed::new(par)
            .spanned(span)
            .layout(self.engine, styles, false, Size::splat(Abs::inf()), false)?
            .into_frame();

        Ok(FrameFragment::new(self, styles, frame)
            .with_class(MathClass::Alphabetic)
            .with_text_like(true)
            .with_spaced(spaced))
    }
}

pub(super) trait Scaled {
    fn scaled(self, ctx: &MathContext, font_size: Abs) -> Abs;
}

impl Scaled for i16 {
    fn scaled(self, ctx: &MathContext, font_size: Abs) -> Abs {
        ctx.font.to_em(self).at(font_size)
    }
}

impl Scaled for u16 {
    fn scaled(self, ctx: &MathContext, font_size: Abs) -> Abs {
        ctx.font.to_em(self).at(font_size)
    }
}

impl Scaled for MathValue<'_> {
    fn scaled(self, ctx: &MathContext, font_size: Abs) -> Abs {
        self.value.scaled(ctx, font_size)
    }
}

/// An OpenType substitution table that is applicable to glyph-wise substitutions.
pub enum GlyphwiseSubsts<'a> {
    Single(SingleSubstitution<'a>),
    Alternate(AlternateSubstitution<'a>, u32),
}

impl<'a> GlyphwiseSubsts<'a> {
    pub fn new(gsub: LayoutTable<'a>, feature: Feature) -> Option<Self> {
        let table = gsub
            .features
            .find(ttf_parser::Tag(feature.tag.0))
            .and_then(|feature| feature.lookup_indices.get(0))
            .and_then(|index| gsub.lookups.get(index))?;
        let table = table.subtables.get::<SubstitutionSubtable>(0)?;
        match table {
            SubstitutionSubtable::Single(single_glyphs) => {
                Some(Self::Single(single_glyphs))
            }
            SubstitutionSubtable::Alternate(alt_glyphs) => {
                Some(Self::Alternate(alt_glyphs, feature.value))
            }
            _ => None,
        }
    }

    pub fn try_apply(&self, glyph_id: GlyphId) -> Option<GlyphId> {
        match self {
            Self::Single(single) => match single {
                SingleSubstitution::Format1 { coverage, delta } => coverage
                    .get(glyph_id)
                    .map(|_| GlyphId(glyph_id.0.wrapping_add(*delta as u16))),
                SingleSubstitution::Format2 { coverage, substitutes } => {
                    coverage.get(glyph_id).and_then(|idx| substitutes.get(idx))
                }
            },
            Self::Alternate(alternate, value) => alternate
                .coverage
                .get(glyph_id)
                .and_then(|idx| alternate.alternate_sets.get(idx))
                .and_then(|set| set.alternates.get(*value as u16)),
        }
    }

    pub fn apply(&self, glyph_id: GlyphId) -> GlyphId {
        self.try_apply(glyph_id).unwrap_or(glyph_id)
    }
}
