//! Multi-file output for Typst.

#[path = "export.rs"]
mod export_;
mod format;
mod introspect;
mod link;

pub use self::export_::{BundleOptions, VirtualFs, export};
pub use self::format::{AssetData, AssetElem, BundleFormat, FORMAT};

use std::collections::hash_map::Entry;
use std::sync::Arc;

use comemo::{Tracked, TrackedMut};
use ecow::{EcoString, EcoVec};
use indexmap::IndexMap;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rustc_hash::{FxBuildHasher, FxHashMap};
use typst_html::css::StylesheetData;
use typst_html::{HtmlDocument, HtmlElement, css};
use typst_layout::PagedDocument;
use typst_library::diag::{At, CollectCombinedResult, SourceResult, bail, error};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::format::DocumentFormatOptions;
use typst_library::foundations::{
    BundlePath, Bytes, Content, Output, Packed, StyleChain, Target, TargetElem,
};
use typst_library::introspection::{
    Introspector, Location, Locator, SplitLocator, Tag, TagElem,
};
use typst_library::model::{
    Document, DocumentElem, DocumentFormat, DocumentInfo, PagedFormat,
};
use typst_library::routines::{Arenas, Pair, RealizationKind};
use typst_library::{Feature, Library, World};
use typst_syntax::VirtualPath;
use typst_utils::{LazyHash, Protected};

use crate::introspect::BundleIntrospector;

/// A collection of files resulting from compilation.
///
/// In the `bundle` target, Typst can emit multiple documents and assets from a
/// single Typst project.
///
/// The `Bundle` is the output of compilation and is to the `bundle` output
/// format what the `PagedDocument` is to `pdf`, `png`, and `svg` outputs.
#[derive(Debug, Clone)]
pub struct Bundle {
    /// The files in the bundle.
    pub files: Arc<IndexMap<VirtualPath, BundleFile, FxBuildHasher>>,
    /// An introspector for the whole bundle.
    ///
    /// The whole bundle is subject to one large introspection loop (as opposed
    /// to each document iterating separately). They can introspect each other
    /// and all contribute to this one introspector.
    pub introspector: Arc<BundleIntrospector>,
}

impl Output for Bundle {
    fn introspector(&self) -> &dyn Introspector {
        self.introspector.as_ref()
    }

    fn target() -> Target {
        Target::Bundle
    }

    fn create(
        engine: &mut Engine,
        content: &Content,
        styles: StyleChain,
    ) -> SourceResult<Self> {
        bundle(engine, content, styles)
    }
}

/// A single file in the bundle.
#[derive(Debug, Clone)]
pub enum BundleFile {
    /// A document in one of the supported output formats, resulting from a
    /// `document` element.
    Document(BundleDocument),
    /// Raw file data, resulting from an `asset` element.
    Asset(Bytes),
}

impl BundleFile {
    pub fn as_document_mut(&mut self) -> Option<&mut BundleDocument> {
        match self {
            Self::Document(v) => Some(v),
            Self::Asset(_) => None,
        }
    }
}

/// A document in one of the supported output formats, resulting from a
/// `document` element.
#[derive(Debug, Clone)]
pub enum BundleDocument {
    /// A document in one of the paged formats.
    Paged(Box<PagedDocument>, PagedExtras),
    /// A document in the HTML format.
    Html(Box<HtmlDocument>),
}

impl BundleDocument {
    pub fn as_html_mut(&mut self) -> Option<&mut HtmlDocument> {
        match self {
            Self::Html(v) => Some(v),
            Self::Paged(..) => None,
        }
    }
}

impl Document for BundleDocument {
    fn info(&self) -> &DocumentInfo {
        match self {
            BundleDocument::Paged(doc, _) => doc.info(),
            BundleDocument::Html(doc) => doc.info(),
        }
    }

    fn options(&self) -> &DocumentFormatOptions {
        match self {
            BundleDocument::Paged(doc, _) => doc.options(),
            BundleDocument::Html(doc) => doc.options(),
        }
    }
}

/// Extra data relevant for exporting a paged document in a bundle.
#[derive(Debug, Clone, Hash)]
pub struct PagedExtras {
    /// The format to export in.
    pub format: PagedFormat,
    /// Named anchors that should be exported, so that cross-document links can
    /// jump to a precise location.
    ///
    /// Not all export targets support this (e.g. PNG), in which case it can
    /// simply be ignored.
    pub anchors: Vec<(Location, EcoString)>,
}

/// Produces a bundle from content.
///
/// This first performs root-level bundle realization and then compiles the
/// individual documents (in parallel).
#[typst_macros::time]
pub fn bundle(
    engine: &mut Engine,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<Bundle> {
    bundle_impl(
        engine.world,
        engine.library,
        engine.introspector.into_raw(),
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        styles,
    )
}

/// The internal implementation of `bundle`.
#[comemo::memoize]
#[expect(clippy::too_many_arguments)]
fn bundle_impl(
    world: Tracked<dyn World + '_>,
    library: &LazyHash<Library>,
    introspector: Tracked<dyn Introspector + '_>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<Bundle> {
    let introspector = Protected::from_raw(introspector);
    let mut locator = Locator::root().split();
    let mut engine = Engine {
        library,
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route).unnested(),
    };

    // Mark the external styles as "outside" so that they are valid at the page
    // level.
    let styles = styles.to_map().outside();
    let styles = StyleChain::new(&styles);

    let arenas = Arenas::default();
    let children = (engine.library.routines.realize)(
        RealizationKind::Bundle,
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    let children = collect(&children, &mut engine, &mut locator)?;

    let mut items = engine
        .parallelize(children, |engine, child| -> SourceResult<_> {
            Ok(match child {
                Child::Tag(tag) => Item::Tag(tag.clone()),
                Child::Asset(asset) => Item::Asset(
                    asset.path.clone().into_inner(),
                    asset.data.0.clone(),
                    asset.location().unwrap(),
                ),
                Child::Document(document, styles, locator) => Item::Document(
                    document.path.clone().into_inner(),
                    compile_document(engine, document, styles, locator)?,
                    document.location().unwrap(),
                ),
            })
        })
        .collect_combined_result::<Vec<_>>()?;

    let mut introspector = BundleIntrospector::new(&items);
    let targets = introspector.link_targets();
    let anchors = crate::link::create_link_anchors(&mut items, &targets);
    introspector.set_anchors(anchors);

    let mut files = IndexMap::default();
    for item in items {
        match item {
            Item::Tag(_) => {}
            Item::Asset(path, bytes, _) => {
                files.insert(path, BundleFile::Asset(bytes));
            }
            Item::Document(path, doc, _) => {
                files.insert(path, BundleFile::Document(doc));
            }
        }
    }

    // Resolve an external stylesheet.
    resolve_external_stylesheet(&mut engine, &mut files)?;

    Ok(Bundle {
        files: Arc::new(files),
        introspector: Arc::new(introspector),
    })
}

/// Resolve an external stylesheet for HTML documents that have set
/// [`typst_html::HtmlStyleLocation::External`].
fn resolve_external_stylesheet(
    engine: &mut Engine,
    files: &mut IndexMap<VirtualPath, BundleFile, FxBuildHasher>,
) -> SourceResult<()> {
    let mut groups: IndexMap<_, (StylesheetData, Vec<&mut HtmlElement>), FxBuildHasher> =
        IndexMap::default();

    // Filter documents that use an external stylesheet, group them by the
    // stylesheet location, and merge stylesheet data per group.
    let html_docs = files
        .values_mut()
        .filter_map(|file| file.as_document_mut()?.as_html_mut());
    for doc in html_docs {
        if let Some(css) = doc.external_css() {
            let (_, root_elems) = groups
                .entry(css.path.clone())
                .and_modify(|(data, _)| data.merge(&css.data))
                .or_insert((css.data.clone(), Vec::new()));
            root_elems.push(doc.root_mut());
        }
    }

    // Resolve the external stylesheet.
    let stylesheets = groups
        .into_par_iter()
        .filter_map(|(path, (data, ref mut root_elems))| {
            let stylesheet = css::resolve_stylesheet(root_elems, &data);
            if stylesheet.is_empty() {
                return None;
            }

            // Insert links to the external stylesheet.
            let path = path.map(BundlePath::into_inner);
            for root in root_elems {
                css::insert_external_stylesheet_link(root, &path.v);
            }

            Some((path, stylesheet))
        })
        .collect::<Vec<_>>();

    // Insert into the bundle.
    for (path, stylesheet) in stylesheets {
        if files.contains_key(&path.v) {
            engine.sink.delayed_error(error!(
                path.span,
                "cannot emit external stylesheet \
             because the path `{}` already exists in the bundle",
                path.v.get_without_slash(),
            ));
        } else {
            files.insert(path.v, BundleFile::Asset(Bytes::from_string(stylesheet)));
        }
    }

    Ok(())
}

/// Something that can result from bundle realization.
enum Child<'a> {
    Tag(&'a Tag),
    Asset(&'a Packed<AssetElem>),
    Document(&'a Packed<DocumentElem>, StyleChain<'a>, Locator<'a>),
}

/// The processed version of a [`Child`].
enum Item {
    Tag(Tag),
    Asset(VirtualPath, Bytes, Location),
    Document(VirtualPath, BundleDocument, Location),
}

/// Collects all documents and assets in the bundle.
fn collect<'a>(
    children: &'a [Pair<'a>],
    engine: &mut Engine,
    locator: &mut SplitLocator<'a>,
) -> SourceResult<Vec<Child<'a>>> {
    let mut items = Vec::new();
    let mut errors = EcoVec::new();
    let mut seen = FxHashMap::default();

    for (elem, styles) in children {
        let path = if let Some(elem) = elem.to_packed::<TagElem>() {
            items.push(Child::Tag(&elem.tag));
            continue;
        } else if let Some(elem) = elem.to_packed::<AssetElem>() {
            items.push(Child::Asset(elem));
            elem.path.as_ref()
        } else if let Some(elem) = elem.to_packed::<DocumentElem>() {
            items.push(Child::Document(elem, *styles, locator.next(&elem.span())));
            elem.path.as_ref()
        } else {
            errors.push(error!(
                elem.span(), "{} is not allowed at the top-level in bundle export",
                elem.func().name();
                hint: "try wrapping the content in a `document` instead";
            ));
            continue;
        };

        match seen.entry(path) {
            Entry::Vacant(entry) => {
                entry.insert(elem.span());
            }
            Entry::Occupied(entry) => {
                engine.sink.delayed_error(error!(
                    elem.span(), "path `{}` occurs multiple times in the bundle",
                    path.get_without_slash();
                    hint: "{} paths must be unique in the bundle",
                    elem.func().name();
                    hint[*entry.get()]: "path is already in use here";
                ));
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(items)
}

/// Compiles a single document.
fn compile_document<'a>(
    engine: &mut Engine,
    document: &'a Packed<DocumentElem>,
    styles: StyleChain<'a>,
    locator: Locator<'a>,
) -> SourceResult<BundleDocument> {
    let format = document.determine_format(styles).at(document.span())?;
    let target = TargetElem::target.set(format.target()).wrap();
    let styles = styles.chain(&target);
    Ok(match format {
        DocumentFormat::Paged(format) => {
            let doc = typst_layout::layout_document_for_bundle(
                engine,
                &document.body,
                locator,
                styles,
            )?;

            let num_pages = doc.pages().len();
            if num_pages != 1 && matches!(format, PagedFormat::Png | PagedFormat::Svg) {
                engine.sink.delayed_error(error!(
                    document.span(),
                    "expected document to have a single page";
                    hint: "the document resulted in {num_pages} pages";
                    hint: "documents exported to an image format only support a single page";
                ));
            }

            BundleDocument::Paged(
                Box::new(doc),
                PagedExtras { format, anchors: Vec::new() },
            )
        }
        DocumentFormat::Html => {
            if !engine.library.features.is_enabled(Feature::Html) {
                bail!(
                    document.span(),
                    "html export is only available when the `html` feature is enabled";
                    hint: "html export is under active development and incomplete";
                    hint: "to enable both bundle and html export, pass `--features bundle,html`";
                );
            }

            let doc = typst_html::html_document_for_bundle(
                engine,
                &document.body,
                locator,
                styles,
            )?;
            BundleDocument::Html(Box::new(doc))
        }
    })
}
