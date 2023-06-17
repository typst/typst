pub(crate) mod glyph;
pub(crate) use glyph::GlyphRenderTask;

#[cfg(feature = "flat-vector")]
pub(crate) mod dynamic_layout;
#[cfg(feature = "flat-vector")]
pub(crate) mod flat;
#[cfg(feature = "flat-vector")]
pub(crate) mod incremental;
