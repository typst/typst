use indexmap::IndexMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxBuildHasher;
use typst_library::diag::{At, ParallelCollectCombinedResult, SourceResult};
use typst_library::foundations::Bytes;
use typst_library::model::PagedFormat;
use typst_syntax::{Span, VirtualPath};

use crate::{Bundle, BundleDocument, BundleFile};

/// A raw mapping from paths to bytes.
pub type VirtualFs = IndexMap<VirtualPath, Bytes, FxBuildHasher>;

/// Exports a bundle into a raw virtual file system.
#[typst_macros::time(name = "export bundle")]
pub fn export(bundle: &Bundle, options: &BundleOptions) -> SourceResult<VirtualFs> {
    bundle
        .files
        .par_iter()
        .map(|(path, file)| {
            let data = match file {
                BundleFile::Document(doc) => export_document(doc, options),
                BundleFile::Asset(bytes) => Ok(bytes.clone()),
            };
            data.map(|data| (path.clone(), data))
        })
        .collect_combined_result()
}

/// Settings for bundle export.
#[derive(Debug)]
pub struct BundleOptions<'a> {
    /// The number of pixels per point to render at when exporting a PNG.
    pub pixel_per_pt: f32,
    /// Options for exporting PDF documents.
    pub pdf: typst_pdf::PdfOptions<'a>,
}

/// Exports a single document.
fn export_document(doc: &BundleDocument, options: &BundleOptions) -> SourceResult<Bytes> {
    match doc {
        BundleDocument::Paged(doc, extras) => match extras.format {
            PagedFormat::Pdf => typst_pdf::pdf(doc, &options.pdf).map(Bytes::new),
            PagedFormat::Png => {
                typst_render::render(&doc.pages()[0], options.pixel_per_pt)
                    .encode_png()
                    .map(Bytes::new)
                    .map_err(|_| "failed to encode PNG")
                    .at(Span::detached())
            }
            PagedFormat::Svg => Ok(Bytes::from_string(typst_svg::svg(&doc.pages()[0]))),
        },
        BundleDocument::Html(doc) => typst_html::html(doc).map(Bytes::from_string),
    }
}
