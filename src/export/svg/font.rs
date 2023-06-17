use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use crate::font::Font;
use crate::geom::Axes;
use crate::image::Image;
use ttf_parser::GlyphId;

use super::hash::{make_item_hash, HashedTrait, StaticHash128};
use super::path2d::SvgPath2DBuilder;

/// IGlyphProvider extracts the font data from the font.
/// Note (Possibly block unsafe): If a [`Font`] is dummy (lazy loaded),
///   it will block current thread and fetch the font data from the server.
pub trait IGlyphProvider {
    /// With font with glyph id, return the svg document data.
    /// Note: The returned data is possibly compressed.
    /// See [`FontGlyphProvider::svg_glyph`] for the default implementation.
    fn svg_glyph(&self, font: &Font, id: GlyphId) -> Option<Arc<[u8]>>;

    /// With font with glyph id, return the bitmap image data.
    /// Optionally, with given ppem, return the best fit bitmap image.
    /// Return the best quality bitmap image if ppem is [`std::u16::MAX`].
    /// See [`FontGlyphProvider::bitmap_glyph`] for the default implementation.
    fn bitmap_glyph(
        &self,
        font: &Font,
        id: GlyphId,
        ppem: u16,
    ) -> Option<(Image, i16, i16)>;

    /// With font with glyph id, return the outline path data.
    /// The returned data is in Path2D format.
    /// See [`FontGlyphProvider::outline_glyph`] for the default implementation.
    fn outline_glyph(&self, font: &Font, id: GlyphId) -> Option<String>;
}

/// Wrapper of [`IGlyphProvider`] with [`Hash`] implementation,
///   which is used for [`comemo::memoize`].
#[derive(Clone)]
pub struct GlyphProvider(Arc<HashedTrait<dyn IGlyphProvider>>);

impl GlyphProvider {
    pub fn new<T>(provider: T) -> Self
    where
        T: IGlyphProvider + Hash + 'static,
    {
        let hash = make_item_hash(&provider);
        let provider = Box::new(provider);
        Self(Arc::new(HashedTrait::<dyn IGlyphProvider>::new(hash, provider)))
    }
}

impl Deref for GlyphProvider {
    type Target = dyn IGlyphProvider;

    fn deref(&self) -> &Self::Target {
        (*self.0.as_ref()).deref()
    }
}

impl Hash for GlyphProvider {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(self.0.get_hash());
    }
}

impl Default for GlyphProvider {
    fn default() -> Self {
        Self::new(FontGlyphProvider::default())
    }
}

/// The default [`IGlyphProvider`] implementation.
/// It uses the local font data to extract the glyph data.
#[derive(Default, Hash)]
pub struct FontGlyphProvider {}

impl IGlyphProvider for FontGlyphProvider {
    /// See [`IGlyphProvider::svg_glyph`] for more information.
    fn svg_glyph(&self, font: &Font, id: GlyphId) -> Option<Arc<[u8]>> {
        let font_face = font.ttf();

        Some(font_face.glyph_svg_image(id)?.into())
    }

    /// See [`IGlyphProvider::bitmap_glyph`] for more information.
    /// Note: It converts the data into [`typst::image::Image`] and introduces overhead.
    fn bitmap_glyph(
        &self,
        font: &Font,
        id: GlyphId,
        ppem: u16,
    ) -> Option<(Image, i16, i16)> {
        let font_face = font.ttf();

        let raster = font_face.glyph_raster_image(id, ppem)?;

        // convert to typst's image format
        let glyph_image = Image::new_with_size(
            raster.data.into(),
            raster.format.into(),
            None,
            Axes::new(raster.width as u32, raster.height as u32),
        )
        .ok()?;

        Some((glyph_image, raster.x, raster.y))
    }

    /// See [`IGlyphProvider::outline_glyph`] for more information.
    fn outline_glyph(&self, font: &Font, id: GlyphId) -> Option<String> {
        let font_face = font.ttf();

        // todo: handling no such glyph
        let mut builder = SvgPath2DBuilder(String::new());
        font_face.outline_glyph(id, &mut builder)?;
        Some(builder.0)
    }
}
