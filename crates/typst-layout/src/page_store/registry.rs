//! Registries for resolving font and image references during deserialization.

use rustc_hash::FxHashMap;
use typst_library::text::Font;
use typst_library::visualize::Image;

use super::types::SFontRef;

/// Registry mapping font references (hash + index) to Font objects.
/// Built during serialization by collecting all fonts encountered.
/// Used during deserialization to reconstruct TextItem objects.
pub struct FontRegistry {
    fonts: FxHashMap<(u128, u32), Font>,
}

impl FontRegistry {
    pub fn new() -> Self {
        Self { fonts: FxHashMap::default() }
    }

    pub fn register(&mut self, font: &Font) -> SFontRef {
        let hash = typst_utils::hash128(font.data());
        let index = font.index();
        let key = (hash, index);
        self.fonts.entry(key).or_insert_with(|| font.clone());
        SFontRef { data_hash: hash, index }
    }

    pub fn resolve(&self, font_ref: &SFontRef) -> Option<Font> {
        self.fonts.get(&(font_ref.data_hash, font_ref.index)).cloned()
    }
}

/// Registry mapping image hashes to Image objects.
pub struct ImageRegistry {
    images: FxHashMap<u128, Image>,
}

impl ImageRegistry {
    pub fn new() -> Self {
        Self { images: FxHashMap::default() }
    }

    pub fn register(&mut self, image: &Image) -> u128 {
        let hash = typst_utils::hash128(image);
        self.images.entry(hash).or_insert_with(|| image.clone());
        hash
    }

    pub fn resolve(&self, hash: u128) -> Option<Image> {
        self.images.get(&hash).cloned()
    }
}
