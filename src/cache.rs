//! Caching of compilation artifacts.

use crate::font::FontCache;
use crate::image::ImageCache;
#[cfg(feature = "layout-cache")]
use crate::layout::LayoutCache;
use crate::loading::Loader;

/// Caches compilation artifacts.
pub struct Cache {
    /// Caches parsed font faces.
    pub font: FontCache,
    /// Caches decoded images.
    pub image: ImageCache,
    /// Caches layouting artifacts.
    #[cfg(feature = "layout-cache")]
    pub layout: LayoutCache,
}

impl Cache {
    /// Create a new, empty cache.
    pub fn new(loader: &dyn Loader) -> Self {
        Self {
            font: FontCache::new(loader),
            image: ImageCache::new(),
            #[cfg(feature = "layout-cache")]
            layout: LayoutCache::new(),
        }
    }
}
