use crate::export::svg::{
    font::GlyphProvider,
    ir::{GlyphItem, ImageGlyphItem, OutlineGlyphItem},
    utils::ToCssExt,
    vector::{codegen::render_image, lowering::GlyphLowerBuilder},
};

pub struct GlyphRenderTask {
    pub glyph_provider: GlyphProvider,
}

impl GlyphRenderTask {
    pub fn render_glyph(
        &mut self,
        glyph_id: &str,
        glyph_item: &GlyphItem,
    ) -> Option<String> {
        let gp = &self.glyph_provider;
        Self::render_glyph_inner(gp, glyph_id, glyph_item)
    }

    #[comemo::memoize]
    pub fn render_glyph_pure(glyph_id: &str, glyph_item: &GlyphItem) -> Option<String> {
        Self::render_glyph_pure_inner(glyph_id, glyph_item)
    }

    #[comemo::memoize]
    fn render_glyph_inner(
        gp: &GlyphProvider,
        glyph_id: &str,
        glyph_item: &GlyphItem,
    ) -> Option<String> {
        if matches!(glyph_item, GlyphItem::Raw(..)) {
            return Self::render_glyph_pure_inner(
                glyph_id,
                &GlyphLowerBuilder::new(gp).lower_glyph(glyph_item)?,
            );
        }

        Self::render_glyph_pure_inner(glyph_id, glyph_item)
    }

    fn render_glyph_pure_inner(glyph_id: &str, glyph_item: &GlyphItem) -> Option<String> {
        match glyph_item {
            GlyphItem::Image(image_glyph) => {
                Self::render_image_glyph(glyph_id, image_glyph)
            }
            GlyphItem::Outline(outline_glyph) => {
                Self::render_outline_glyph(glyph_id, outline_glyph)
            }
            GlyphItem::Raw(..) => unreachable!(),
        }
    }

    /// Render an image glyph into the svg text.
    fn render_image_glyph(glyph_id: &str, ig: &ImageGlyphItem) -> Option<String> {
        let img = render_image(&ig.image.image, ig.image.size);

        let ts = ig.ts.to_css();
        let symbol_def = format!(
            r#"<symbol overflow="visible" id="{}" class="image_glyph"><g transform="{}">{}</g></symbol>"#,
            glyph_id, ts, img
        );
        Some(symbol_def)
    }

    /// Render an outline glyph into svg text. This is the "normal" case.
    fn render_outline_glyph(
        glyph_id: &str,
        outline_glyph: &OutlineGlyphItem,
    ) -> Option<String> {
        let symbol_def = format!(
            r#"<symbol overflow="visible" id="{}" class="outline_glyph"><path d="{}"/></symbol>"#,
            glyph_id, outline_glyph.d
        );
        Some(symbol_def)
    }
}
