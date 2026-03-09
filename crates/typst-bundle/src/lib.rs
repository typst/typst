//! Multi-file output for Typst.

#[path = "export.rs"]
mod export_;
mod introspect;

use crate::introspect::BundleIntrospector;

pub use self::export_::{BundleOptions, VirtualFs, export};

use std::sync::Arc;

use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use typst_html::HtmlDocument;
use typst_layout::PagedDocument;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Bytes, Content, Output, StyleChain, Target};
use typst_library::introspection::Introspector;
use typst_library::model::{Document, DocumentInfo, PagedFormat};
use typst_syntax::VirtualPath;

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

    #[expect(unused)]
    fn create(
        engine: &mut Engine,
        content: &Content,
        styles: StyleChain,
    ) -> SourceResult<Self> {
        todo!()
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

/// A document in one of the supported output formats, resulting from a
/// `document` element.
#[derive(Debug, Clone)]
pub enum BundleDocument {
    /// A document in one of the paged formats.
    Paged(Box<PagedDocument>, PagedExtras),
    /// A document in the HTML format.
    Html(Box<HtmlDocument>),
}

impl Document for BundleDocument {
    fn info(&self) -> &DocumentInfo {
        match self {
            BundleDocument::Paged(doc, _) => doc.info(),
            BundleDocument::Html(doc) => doc.info(),
        }
    }
}

/// Extra data relevant for exporting a paged document in a bundle.
#[derive(Debug, Clone, Hash)]
pub struct PagedExtras {
    /// The format to export in.
    pub format: PagedFormat,
}
