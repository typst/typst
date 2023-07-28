use ttf_parser::math::MathValue;
use typst::font::{FontStyle, FontWeight};
use typst::model::realize;
use unicode_segmentation::UnicodeSegmentation;

use super::*;
use crate::layout::SpanMapper;
use crate::text::{
    math_families, math_tags, BottomEdge, BottomEdgeMetric, FontFeatures, TopEdge,
    TopEdgeMetric,
};

macro_rules! scaled {
    ($ctx:expr, text: $text:ident, display: $display:ident $(,)?) => {
        match $ctx.style.size {
            MathSize::Display => scaled!($ctx, $display),
            _ => scaled!($ctx, $text),
        }
    };
    ($ctx:expr, $name:ident) => {
        $ctx.constants().$name().scaled($ctx)
    };
}

macro_rules! percent {
    ($ctx:expr, $name:ident) => {
        $ctx.constants().$name() as f64 / 100.0
    };
}

/// The context for math layout.
pub struct MathContext<'a, 'b, 'v> {
    pub vt: &'v mut Vt<'b>,
    pub regions: Regions<'static>,
    pub font: Font,
    pub space_width: Em,
    pub fragments: Vec<MathFragment>,
    pub local: Styles,
    pub style: MathStyle,
    pub size: Abs,
    outer: StyleChain<'a>,
    style_stack: Vec<(MathStyle, Abs)>,
    ssty1: Styles,
    ssty2: Styles,
}

impl<'a, 'b, 'v> MathContext<'a, 'b, 'v> {
    pub fn new(
        vt: &'v mut Vt<'b>,
        styles: StyleChain<'a>,
        regions: Regions,
        block: bool,
        span: Span,
    ) -> SourceResult<Self> {
        let Some(font) = find_math_font(vt, None, styles) else {
            bail!(span,"current font does not support math");
        };

        let size = var_size(None, styles);
        let ttf = font.ttf();
        let space_width = ttf
            .glyph_index(' ')
            .and_then(|id| ttf.glyph_hor_advance(id))
            .map(|advance| font.to_em(advance))
            .unwrap_or(THICK);

        // FIXME: There's a legacy attempt here to be smart about
        // inheriting the document's italic/bold-ness into the math.
        // But there is an inconsistency here if you set the `text`
        // properties during the equation; then they have no effect.
        let variant = math_variant(None, styles);

        let ssty_tag = ttf_parser::Tag::from_bytes(b"ssty");

        let mut ssty1 = Styles::new();
        ssty1.set(VarElem::set_features(FontFeatures(vec![(ssty_tag, 1)])));

        let mut ssty2 = Styles::new();
        ssty2.set(VarElem::set_features(FontFeatures(vec![(ssty_tag, 2)])));

        Ok(Self {
            vt,
            regions: Regions::one(regions.base(), Axes::splat(false)),
            font,
            space_width,
            fragments: vec![],
            local: Styles::new(),
            style: MathStyle {
                variant: MathVariant::Serif,
                size: if block { MathSize::Display } else { MathSize::Text },
                class: Smart::Auto,
                cramped: false,
                bold: variant.weight >= FontWeight::BOLD,
                italic: match variant.style {
                    FontStyle::Normal => Smart::Auto,
                    FontStyle::Italic | FontStyle::Oblique => Smart::Custom(true),
                },
            },
            size,
            outer: styles,
            style_stack: vec![],
            ssty1,
            ssty2,
        })
    }

    pub fn push(&mut self, fragment: impl Into<MathFragment>) {
        self.fragments.push(fragment.into());
    }

    pub fn extend(&mut self, fragments: Vec<MathFragment>) {
        self.fragments.extend(fragments);
    }

    pub fn font(&self) -> Font {
        self.font.clone()
    }

    pub fn ttf(&self) -> &ttf_parser::Face {
        self.font.ttf()
    }

    pub fn table(&self) -> ttf_parser::math::Table {
        self.font.ttf().tables().math.unwrap()
    }

    pub fn constants(&self) -> ttf_parser::math::Constants {
        self.font.ttf().tables().math.unwrap().constants.unwrap()
    }

    pub fn ssty(&'a self) -> Option<ttf_parser::gsub::AlternateSubstitution<'a>> {
        self.font.ssty()
    }

    // FIXME: This is doing needless extended computation once per glyph
    // FIXME: There are two different paths for doing substitution.
    //        One is here for singleton glyphs, and one is via the
    //        shaper (rustybuzz).  Are these in sync?
    pub fn glyphwise_tables(
        &'a self,
        elem: Option<&VarElem>,
    ) -> Option<Vec<GlyphwiseSubsts<'a>>> {
        let gsub_table = self.font.ttf().tables().gsub;

        let features = math_tags(elem, self.outer.chain(&self.local));

        gsub_table.map(|gsub| {
            features
                .into_iter()
                .filter_map(|feature| GlyphwiseSubsts::new(gsub, feature))
                .collect()
        })
    }

    pub fn update_font(
        &mut self,
        elem: Option<&VarElem>,
        span: Span,
    ) -> SourceResult<()> {
        let styles = self.outer.chain(&self.local);
        let Some(font) = find_math_font(self.vt, elem, styles) else {
            bail!(span,"current font does not support math");
        };
        self.font = font.clone();

        let ttf = self.ttf();
        self.space_width = ttf
            .glyph_index(' ')
            .and_then(|id| ttf.glyph_hor_advance(id))
            .map(|advance| font.to_em(advance))
            .unwrap_or(THICK);

        Ok(())
    }

    pub fn layout_fragment(
        &mut self,
        elem: &dyn LayoutMath,
    ) -> SourceResult<MathFragment> {
        let row = self.layout_fragments(elem)?;
        Ok(MathRow::new(row).into_fragment(self))
    }

    pub fn layout_fragments(
        &mut self,
        elem: &dyn LayoutMath,
    ) -> SourceResult<Vec<MathFragment>> {
        let prev = std::mem::take(&mut self.fragments);
        elem.layout_math(self)?;
        Ok(std::mem::replace(&mut self.fragments, prev))
    }

    pub fn layout_row(&mut self, elem: &dyn LayoutMath) -> SourceResult<MathRow> {
        let fragments = self.layout_fragments(elem)?;
        Ok(MathRow::new(fragments))
    }

    pub fn layout_frame(&mut self, elem: &dyn LayoutMath) -> SourceResult<Frame> {
        Ok(self.layout_fragment(elem)?.into_frame())
    }

    pub fn layout_content(&mut self, content: &Content) -> SourceResult<Frame> {
        Ok(content
            .layout(self.vt, self.outer.chain(&self.local), self.regions)?
            .into_frame())
    }

    pub fn layout_text(&mut self, elem: &TextElem) -> SourceResult<MathFragment> {
        let span = elem.span();
        let text = elem.text();

        let spaced = text.graphemes(true).nth(1).is_some();
        let text = TextElem::packed(text)
            .styled(TextElem::set_top_edge(TopEdge::Metric(TopEdgeMetric::Bounds)))
            .styled(TextElem::set_bottom_edge(BottomEdge::Metric(
                BottomEdgeMetric::Bounds,
            )))
            .spanned(span);
        let par = ParElem::new(vec![text]);

        let frame = par
            .layout(
                self.vt,
                self.outer.chain(&self.local),
                false,
                Size::splat(Abs::inf()),
                false,
            )?
            .into_frame();
        Ok(FrameFragment::new(self, frame)
            .with_class(MathClass::Alphabetic)
            .with_spaced(spaced)
            .into())
    }

    pub fn layout_var(&mut self, elem: &VarElem) -> SourceResult<MathFragment> {
        let span = elem.span();
        let text = elem.as_string();

        let font_prev = self.font.clone();
        let space_width_prev = self.space_width;

        let size_prev = self.size;
        self.size = var_size(Some(elem), self.styles());

        let styles = self.styles();

        if elem.font(styles) != VarElem::font_in(styles)
            || elem.base_weight(styles) != VarElem::base_weight_in(styles)
            || elem.fallback(styles) != VarElem::fallback_in(styles)
        {
            self.update_font(Some(elem), span)?;
        }

        let mut chars = text.chars();
        if let Some(mut glyph) = chars
            .next()
            .filter(|_| chars.next().is_none())
            .map(|c| self.style.styled_char(c))
            .and_then(|c| GlyphFragment::try_new(self, c, Some(elem), span))
        {
            // A single glyph in the math font. A lot of the later
            // processing seems to depend on the GlyphFragment
            // information being keep in the single glyph case, so
            // we separate this out for now.
            match self.style.size {
                MathSize::Script => {
                    glyph.make_scriptsize(self);
                }
                MathSize::ScriptScript => {
                    glyph.make_scriptscriptsize(self);
                }
                _ => {}
            }

            let fragment: MathFragment = {
                let class = self.style.class.as_custom().or(glyph.class);
                if class == Some(MathClass::Large) {
                    let mut variant = if self.style.size == MathSize::Display {
                        let height = scaled!(self, display_operator_min_height);
                        glyph.stretch_vertical(self, height, Abs::zero())
                    } else {
                        glyph.into_variant()
                    };
                    // TeXbook p 155. Large operators are always vertically centered on the axis.
                    let h = variant.frame.height();
                    variant.frame.set_baseline(h / 2.0 + scaled!(self, axis_height));
                    variant.into()
                } else {
                    glyph.into()
                }
            };
            self.size = size_prev;
            self.font = font_prev;
            self.space_width = space_width_prev;
            Ok(fragment)
        } else {
            let styles = self.styles();
            let mathstyle = self.style;

            let mut styled_text = EcoString::new();
            for c in text.chars() {
                styled_text.push(mathstyle.styled_char(c));
            }

            let lang = VarElem::lang_in(styles);
            let mut sm = SpanMapper::new();
            sm.push(styled_text.as_bytes().len(), elem.span());

            let shape = |styles| {
                crate::text::shape(
                    self.vt,
                    0,
                    styled_text.as_str(),
                    &sm,
                    styles,
                    Dir::LTR,
                    lang,
                    None,
                    Some(elem),
                )
            };

            let shaped_text = match mathstyle.size {
                MathSize::Script => {
                    let styles = styles.chain(&self.ssty1);
                    shape(styles)
                }
                MathSize::ScriptScript => {
                    let styles = styles.chain(&self.ssty2);
                    shape(styles)
                }
                _ => shape(styles),
            };

            let frame = shaped_text.build(self.vt, 0.0, Abs::zero());

            self.size = size_prev;
            self.font = font_prev;
            self.space_width = space_width_prev;
            Ok(FrameFragment::new(self, frame).into())
        }
    }

    pub fn styles(&self) -> StyleChain {
        self.outer.chain(&self.local)
    }

    pub fn realize(&mut self, content: &Content) -> SourceResult<Option<Content>> {
        realize(self.vt, content, self.outer.chain(&self.local))
    }

    pub fn style(&mut self, style: MathStyle) {
        self.style_stack.push((self.style, self.size));
        let base_size = TextElem::size_in(self.styles()) / self.style.size.factor(self);
        self.size = base_size * style.size.factor(self);
        self.local.set(TextElem::set_size(TextSize(self.size.into())));
        self.local
            .set(TextElem::set_style(if style.italic == Smart::Custom(true) {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            }));
        self.local.set(TextElem::set_weight(if style.bold {
            FontWeight::BOLD
        } else {
            // The normal weight is what we started with.
            // It's 400 for CM Regular, 450 for CM Book.
            self.font.info().variant.weight
        }));
        self.style = style;
    }

    pub fn unstyle(&mut self) {
        (self.style, self.size) = self.style_stack.pop().unwrap();
        self.local.unset();
        self.local.unset();
        self.local.unset();
    }
}

pub(super) trait Scaled {
    fn scaled(self, ctx: &MathContext) -> Abs;
}

impl Scaled for i16 {
    fn scaled(self, ctx: &MathContext) -> Abs {
        ctx.font.to_em(self).scaled(ctx)
    }
}

impl Scaled for u16 {
    fn scaled(self, ctx: &MathContext) -> Abs {
        ctx.font.to_em(self).scaled(ctx)
    }
}

impl Scaled for Em {
    fn scaled(self, ctx: &MathContext) -> Abs {
        self.at(ctx.size)
    }
}

impl Scaled for MathValue<'_> {
    fn scaled(self, ctx: &MathContext) -> Abs {
        self.value.scaled(ctx)
    }
}

fn find_math_font(vt: &Vt, elem: Option<&VarElem>, styles: StyleChain) -> Option<Font> {
    let variant = math_variant(elem, styles);
    let mut families = math_families(elem, styles);

    let world = vt.world;
    families.find_map(|family| {
        let id = world.book().select(family.as_str(), variant)?;
        let font = world.font(id)?;
        let _ = font.math()?.constants?;
        Some(font)
    })
}
