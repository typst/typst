use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use typst_library::diag::SourceResult;
use typst_library::foundations::Bytes;
use typst_syntax::VirtualPath;

use crate::Bundle;

/// A raw mapping from paths to bytes.
pub type VirtualFs = IndexMap<VirtualPath, Bytes, FxBuildHasher>;

/// Exports a bundle into a raw virtual file system.
#[typst_macros::time(name = "export bundle")]
#[expect(unused)]
pub fn export(bundle: &Bundle, options: &BundleOptions) -> SourceResult<VirtualFs> {
    todo!()
}

/// Settings for bundle export.
#[derive(Debug)]
pub struct BundleOptions<'a> {
    /// The number of pixels per point to render at when exporting a PNG.
    pub pixel_per_pt: f32,
    /// Options for exporting PDF documents.
    pub pdf: typst_pdf::PdfOptions<'a>,
}
