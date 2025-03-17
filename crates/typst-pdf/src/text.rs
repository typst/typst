use std::ops::Range;
use std::sync::Arc;

use bytemuck::TransparentWrapper;
use krilla::surface::{Location, Surface};
use krilla::text::GlyphId;
use typst_library::diag::{bail, SourceResult};
use typst_library::layout::{Abs, Size};
use typst_library::text::{Font, Glyph, TextItem};
use typst_library::visualize::FillRule;
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::paint;
use crate::util::{display_font, AbsExt, TransformExt};

pub(crate) fn handle_text(
    fc: &mut FrameContext,
    t: &TextItem,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) -> SourceResult<()> {
    *gc.languages.entry(t.lang).or_insert(0) += t.glyphs.len();

    let font = convert_font(gc, t.font.clone())?;
    let fill = paint::convert_fill(
        gc,
        &t.fill,
        FillRule::NonZero,
        true,
        surface,
        fc.state(),
        Size::zero(),
    )?;
    let text = t.text.as_str();
    let size = t.size;
    let glyphs: &[PdfGlyph] = TransparentWrapper::wrap_slice(t.glyphs.as_slice());

    surface.push_transform(&fc.state().transform().to_krilla());
    surface.set_fill(fill);
    surface.fill_glyphs(
        krilla::geom::Point::from_xy(0.0, 0.0),
        glyphs,
        font.clone(),
        text,
        size.to_f32(),
        false,
    );

    if let Some(stroke) = t
        .stroke
        .as_ref()
        .map(|s| paint::convert_stroke(gc, s, true, surface, fc.state(), Size::zero()))
    {
        let stroke = stroke?;

        surface.set_stroke(stroke);
        surface.stroke_glyphs(
            krilla::geom::Point::from_xy(0.0, 0.0),
            glyphs,
            font,
            text,
            size.to_f32(),
            // TODO: What if only stroke?
            true,
        );
    }

    surface.pop();

    Ok(())
}

fn convert_font(
    gc: &mut GlobalContext,
    typst_font: Font,
) -> SourceResult<krilla::text::Font> {
    if let Some(font) = gc.fonts_forward.get(&typst_font) {
        Ok(font.clone())
    } else {
        let font_data: Arc<dyn AsRef<[u8]> + Send + Sync> =
            Arc::new(typst_font.data().clone());
        let font =
            match krilla::text::Font::new(font_data.into(), typst_font.index(), true) {
                None => {
                    let font_str = display_font(&typst_font);
                    bail!(Span::detached(), "failed to process font {font_str}");
                }
                Some(f) => f,
            };

        gc.fonts_forward.insert(typst_font.clone(), font.clone());
        gc.fonts_backward.insert(font.clone(), typst_font.clone());

        Ok(font)
    }
}

#[derive(TransparentWrapper)]
#[repr(transparent)]
struct PdfGlyph(Glyph);

impl krilla::text::Glyph for PdfGlyph {
    fn glyph_id(&self) -> GlyphId {
        GlyphId::new(self.0.id as u32)
    }

    fn text_range(&self) -> Range<usize> {
        self.0.range.start as usize..self.0.range.end as usize
    }

    fn x_advance(&self, size: f32) -> f32 {
        self.0.x_advance.at(Abs::raw(size as f64)).to_raw() as f32
    }

    fn x_offset(&self, size: f32) -> f32 {
        self.0.x_offset.at(Abs::raw(size as f64)).to_raw() as f32
    }

    fn y_offset(&self, _: f32) -> f32 {
        0.0
    }

    fn y_advance(&self, _: f32) -> f32 {
        0.0
    }

    fn location(&self) -> Option<Location> {
        Some(self.0.span.0.into_raw().get())
    }
}
