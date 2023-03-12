use ttf_parser::math::MathValue;
use typst::font::{FontStyle, FontWeight};
use unicode_segmentation::UnicodeSegmentation;

use super::*;

macro_rules! scaled {
    ($ctx:expr, text: $text:ident, display: $display:ident $(,)?) => {
        match $ctx.style.size {
            MathSize::Display => scaled!($ctx, $display),
            _ => scaled!($ctx, $text),
        }
    };
    ($ctx:expr, $name:ident) => {
        $ctx.constants.$name().scaled($ctx)
    };
}

macro_rules! percent {
    ($ctx:expr, $name:ident) => {
        $ctx.constants.$name() as f64 / 100.0
    };
}

/// The context for math layout.
pub struct MathContext<'a, 'b, 'v> {
    pub vt: &'v mut Vt<'b>,
    pub regions: Regions<'static>,
    pub font: &'a Font,
    pub ttf: &'a ttf_parser::Face<'a>,
    pub table: ttf_parser::math::Table<'a>,
    pub constants: ttf_parser::math::Constants<'a>,
    pub space_width: Em,
    pub fragments: Vec<MathFragment>,
    pub map: StyleMap,
    pub style: MathStyle,
    pub size: Abs,
    outer: StyleChain<'a>,
    style_stack: Vec<(MathStyle, Abs)>,
}

impl<'a, 'b, 'v> MathContext<'a, 'b, 'v> {
    pub fn new(
        vt: &'v mut Vt<'b>,
        styles: StyleChain<'a>,
        regions: Regions,
        font: &'a Font,
        block: bool,
    ) -> Self {
        let table = font.ttf().tables().math.unwrap();
        let constants = table.constants.unwrap();
        let size = TextNode::size_in(styles);
        let ttf = font.ttf();
        let space_width = ttf
            .glyph_index(' ')
            .and_then(|id| ttf.glyph_hor_advance(id))
            .map(|advance| font.to_em(advance))
            .unwrap_or(THICK);

        let variant = variant(styles);
        Self {
            vt,
            regions: Regions::one(regions.base(), Axes::splat(false)),
            font: &font,
            ttf: font.ttf(),
            table,
            constants,
            space_width,
            fragments: vec![],
            map: StyleMap::new(),
            style: MathStyle {
                variant: MathVariant::Serif,
                size: if block { MathSize::Display } else { MathSize::Text },
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
        }
    }

    pub fn push(&mut self, fragment: impl Into<MathFragment>) {
        self.fragments.push(fragment.into());
    }

    pub fn extend(&mut self, fragments: Vec<MathFragment>) {
        self.fragments.extend(fragments);
    }

    pub fn layout_fragment(
        &mut self,
        node: &dyn LayoutMath,
    ) -> SourceResult<MathFragment> {
        let row = self.layout_fragments(node)?;
        Ok(MathRow::new(row).to_fragment(self))
    }

    pub fn layout_fragments(
        &mut self,
        node: &dyn LayoutMath,
    ) -> SourceResult<Vec<MathFragment>> {
        let prev = std::mem::take(&mut self.fragments);
        node.layout_math(self)?;
        Ok(std::mem::replace(&mut self.fragments, prev))
    }

    pub fn layout_row(&mut self, node: &dyn LayoutMath) -> SourceResult<MathRow> {
        let fragments = self.layout_fragments(node)?;
        Ok(MathRow::new(fragments))
    }

    pub fn layout_frame(&mut self, node: &dyn LayoutMath) -> SourceResult<Frame> {
        Ok(self.layout_fragment(node)?.to_frame())
    }

    pub fn layout_content(&mut self, content: &Content) -> SourceResult<Frame> {
        Ok(content
            .layout(&mut self.vt, self.outer.chain(&self.map), self.regions)?
            .into_frame())
    }

    pub fn layout_text(&mut self, node: &TextNode) -> SourceResult<()> {
        let text = node.text();
        let span = node.span();
        let mut chars = text.chars();
        if let Some(glyph) = chars
            .next()
            .filter(|_| chars.next().is_none())
            .map(|c| self.style.styled_char(c))
            .and_then(|c| GlyphFragment::try_new(self, c, span))
        {
            // A single letter that is available in the math font.
            if self.style.size == MathSize::Display
                && glyph.class == Some(MathClass::Large)
            {
                let height = scaled!(self, display_operator_min_height);
                self.push(glyph.stretch_vertical(self, height, Abs::zero()));
            } else {
                self.push(glyph);
            }
        } else if text.chars().all(|c| c.is_ascii_digit()) {
            // Numbers aren't that difficult.
            let mut fragments = vec![];
            for c in text.chars() {
                let c = self.style.styled_char(c);
                fragments.push(GlyphFragment::new(self, c, span).into());
            }
            let frame = MathRow::new(fragments).to_frame(self);
            self.push(FrameFragment::new(self, frame));
        } else {
            // Anything else is handled by Typst's standard text layout.
            let spaced = text.graphemes(true).count() > 1;
            let mut style = self.style;
            if self.style.italic == Smart::Auto {
                style = style.with_italic(false);
            }
            let text: EcoString = text.chars().map(|c| style.styled_char(c)).collect();
            let frame = self.layout_content(&TextNode::packed(text).spanned(span))?;
            self.push(
                FrameFragment::new(self, frame)
                    .with_class(MathClass::Alphabetic)
                    .with_spaced(spaced),
            );
        }

        Ok(())
    }

    pub fn styles(&self) -> StyleChain {
        self.outer.chain(&self.map)
    }

    pub fn style(&mut self, style: MathStyle) {
        self.style_stack.push((self.style, self.size));
        let base_size = TextNode::size_in(self.styles()) / self.style.size.factor(self);
        self.size = base_size * style.size.factor(self);
        self.map.set(TextNode::set_size(TextSize(self.size.into())));
        self.map
            .set(TextNode::set_style(if style.italic == Smart::Custom(true) {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            }));
        self.map.set(TextNode::set_weight(if style.bold {
            FontWeight::BOLD
        } else {
            FontWeight::REGULAR
        }));
        self.style = style;
    }

    pub fn unstyle(&mut self) {
        (self.style, self.size) = self.style_stack.pop().unwrap();
        self.map.unset();
        self.map.unset();
        self.map.unset();
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
