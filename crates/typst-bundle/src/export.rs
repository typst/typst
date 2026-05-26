use comemo::{Track, Tracked};
use ecow::EcoString;
use indexmap::IndexMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxBuildHasher;
use typst_html::{Html, HtmlElement, HtmlOptions};
use typst_layout::PagedDocument;
use typst_library::diag::{At, ParallelCollectCombinedResult, SourceResult};
use typst_library::format::Complete;
use typst_library::foundations::Bytes;
use typst_library::introspection::Location;
use typst_library::model::{Document, LateLinkResolver, PagedFormat};
use typst_pdf::PdfOptions;
use typst_render::{Png, RenderOptions};
use typst_svg::{Svg, SvgOptions};
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
#[derive(Debug, Default)]
pub struct BundleOptions {
    /// Options for exporting HTML documents.
    pub html: HtmlOptions,
    /// Options for exporting PDF documents.
    pub pdf: PdfOptions,
    /// Options for exporting PNG documents.
    pub png: RenderOptions,
    /// Options for exporting SVG documents.
    pub svg: SvgOptions,
}

/// Exports a single document.
fn export_document(
    doc: &BundleDocument,
    options: &BundleOptions,
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Bytes> {
    match doc {
        BundleDocument::Paged(doc, extras) => match &extras.format {
            PagedFormat::Pdf => {
                export_pdf(doc, &options.pdf, &extras.anchors, link_resolver)
            }
            PagedFormat::Png => export_png(doc, &options.png),
            PagedFormat::Svg => {
                export_svg(doc, &options.svg, &extras.anchors, link_resolver)
            }
        },
        BundleDocument::Html(doc) => {
            let options = options.html.resolve(doc.options().get::<Html>());
            export_html(doc.root(), &options, link_resolver)
        }
    }
}

/// Exports a PDF document.
#[comemo::memoize]
#[typst_macros::time(name = "export pdf")]
fn export_pdf(
    doc: &PagedDocument,
    options: &PdfOptions,
    anchors: &[(Location, EcoString)],
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Bytes> {
    typst_pdf::pdf_in_bundle(doc, options, anchors, link_resolver).map(Bytes::new)
}

/// Exports a PNG document.
#[comemo::memoize]
#[typst_macros::time(name = "export png")]
fn export_png(doc: &PagedDocument, options: &RenderOptions) -> SourceResult<Bytes> {
    let options = options.resolve(doc.options().get::<Png>());
    typst_render::render(&doc.pages()[0], &options)
        .encode_png()
        .map(Bytes::new)
        .map_err(|_| "failed to encode PNG")
        .at(Span::detached())
}

/// Exports an SVG document.
#[comemo::memoize]
#[typst_macros::time(name = "export svg")]
fn export_svg(
    doc: &PagedDocument,
    options: &SvgOptions,
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
    let options = options.resolve(doc.options().get::<Svg>());
    Ok(Bytes::from_string(typst_svg::svg_in_bundle(
        &doc.pages()[0],
        &options,
        &anchors,
        link_resolver,
    )))
}

/// Exports an HTML document.
///
/// This function takes the root element rather than the document because it
/// doesn't need the metadata or introspector and this way, it can be memoized.
/// Bringing the HTML introspector across the memoization boundary is a little
/// trickier than the paged one because the HTML document is mutated after being
/// built (for linking), which means it's not 100% derived from the document.
#[comemo::memoize]
#[typst_macros::time(name = "export html")]
fn export_html(
    root: &HtmlElement,
    options: &HtmlOptions<Complete>,
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Bytes> {
    typst_html::html_in_bundle(root, options, link_resolver).map(Bytes::from_string)
}
