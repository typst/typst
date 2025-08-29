use std::ops::Range;
use std::sync::Arc;

use bytemuck::TransparentWrapper;
use krilla::surface::{Location, Surface};
use krilla::text::GlyphId;
use typst_library::diag::{SourceResult, bail};
use typst_library::layout::Size;
use typst_library::text::{Font, Glyph, TextItem};
use typst_library::visualize::FillRule;
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::util::{AbsExt, TransformExt, display_font};
use crate::{paint, tags};

#[typst_macros::time(name = "handle text")]
pub(crate) fn handle_text(
    fc: &mut FrameContext,
    t: &TextItem,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) -> SourceResult<()> {
    let mut handle = tags::text(gc, fc, surface, t);
    let surface = handle.surface();

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
    let stroke =
        if let Some(stroke) = t.stroke.as_ref().map(|s| {
            paint::convert_stroke(gc, s, true, surface, fc.state(), Size::zero())
        }) {
            Some(stroke?)
        } else {
            None
        };
    let text = t.text.as_str();
    let size = t.size;
    let glyphs: &[PdfGlyph] = TransparentWrapper::wrap_slice(t.glyphs.as_slice());

    surface.push_transform(&fc.state().transform().to_krilla());
    surface.set_fill(Some(fill));
    surface.set_stroke(stroke);
    surface.draw_glyphs(
        krilla::geom::Point::from_xy(0.0, 0.0),
        glyphs,
        font.clone(),
        text,
        size.to_f32(),
        false,
    );

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
        let font = build_font(typst_font.clone())?;

        gc.fonts_forward.insert(typst_font.clone(), font.clone());
        gc.fonts_backward.insert(font.clone(), typst_font.clone());

        Ok(font)
    }
}

#[comemo::memoize]
fn build_font(typst_font: Font) -> SourceResult<krilla::text::Font> {
    let font_data: Arc<dyn AsRef<[u8]> + Send + Sync> =
        Arc::new(typst_font.data().clone());

    match krilla::text::Font::new(font_data.into(), typst_font.index()) {
        None => {
            let font_str = display_font(&typst_font);
            bail!(Span::detached(), "failed to process font {font_str}");
        }
        Some(f) => Ok(f),
    }
}

#[derive(TransparentWrapper, Debug)]
#[repr(transparent)]
struct PdfGlyph(Glyph);

impl krilla::text::Glyph for PdfGlyph {
    #[inline(always)]
    fn glyph_id(&self) -> GlyphId {
        GlyphId::new(self.0.id as u32)
    }

    #[inline(always)]
    fn text_range(&self) -> Range<usize> {
        self.0.range.start as usize..self.0.range.end as usize
    }

    #[inline(always)]
    fn x_advance(&self, size: f32) -> f32 {
        // Don't use `Em::at`, because it contains an expensive check whether the result is finite.
        self.0.x_advance.get() as f32 * size
    }

    #[inline(always)]
    fn x_offset(&self, size: f32) -> f32 {
        // Don't use `Em::at`, because it contains an expensive check whether the result is finite.
        self.0.x_offset.get() as f32 * size
    }

    #[inline(always)]
    fn y_offset(&self, size: f32) -> f32 {
        // Don't use `Em::at`, because it contains an expensive check whether the result is finite.
        self.0.y_offset.get() as f32 * size
    }

    #[inline(always)]
    fn y_advance(&self, size: f32) -> f32 {
        // Don't use `Em::at`, because it contains an expensive check whether the result is finite.
        self.0.y_advance.get() as f32 * size
    }

    fn location(&self) -> Option<Location> {
        Some(self.0.span.0.into_raw())
    }
}
