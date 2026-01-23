use comemo::{Track, Tracked};
use ecow::EcoString;
use indexmap::IndexMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxBuildHasher;
use typst_html::HtmlDocument;
use typst_layout::PagedDocument;
use typst_library::diag::{At, ParallelCollectCombinedResult, SourceResult};
use typst_library::foundations::Bytes;
use typst_library::introspection::Location;
use typst_library::model::{LateLinkResolver, PagedFormat};
use typst_pdf::PdfOptions;
use typst_syntax::{Span, VirtualPath};
use typst_utils::Scalar;

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
                BundleFile::Document(doc) => {
                    let link_resolver =
                        LateLinkResolver::new(Some(path), bundle.introspector.as_ref());
                    export_document(doc, options, link_resolver.track())
                }
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
fn export_document(
    doc: &BundleDocument,
    options: &BundleOptions,
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Bytes> {
    match doc {
        BundleDocument::Paged(doc, extras) => match extras.format {
            PagedFormat::Pdf => {
                export_pdf(doc, &options.pdf, &extras.anchors, link_resolver)
            }
            PagedFormat::Png => export_png(doc, Scalar::new(options.pixel_per_pt as _)),
            PagedFormat::Svg => export_svg(doc, &extras.anchors, link_resolver),
        },
        BundleDocument::Html(doc) => export_html(doc, link_resolver),
    }
}

/// Exports a PDF document.
fn export_pdf(
    doc: &PagedDocument,
    options: &PdfOptions,
    anchors: &[(Location, EcoString)],
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Bytes> {
    typst_pdf::pdf_in_bundle(doc, options, anchors, link_resolver).map(Bytes::new)
}

/// Exports a PNG document.
fn export_png(doc: &PagedDocument, pixel_per_pt: Scalar) -> SourceResult<Bytes> {
    typst_render::render(&doc.pages()[0], pixel_per_pt.get() as f32)
        .encode_png()
        .map(Bytes::new)
        .map_err(|_| "failed to encode PNG")
        .at(Span::detached())
}

/// Exports an SVG document.
fn export_svg(
    doc: &PagedDocument,
    anchors: &[(Location, EcoString)],
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Bytes> {
    let anchors = anchors
        .iter()
        .filter_map(|(loc, name)| {
            // We only support a single page at the moment and all anchor
            // location should point into it, so it's safe to extract just the
            // point using the document's introspector.
            let point = doc.introspector().position(*loc)?.point;
            Some((point, name.clone()))
        })
        .collect::<Vec<_>>();
    Ok(Bytes::from_string(typst_svg::svg_in_bundle(
        &doc.pages()[0],
        &anchors,
        link_resolver,
    )))
}

/// Exports an HTML document.
fn export_html(
    doc: &HtmlDocument,
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Bytes> {
    typst_html::html_in_bundle(doc, link_resolver).map(Bytes::from_string)
}
