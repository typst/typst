use ecow::EcoString;
use either::Either;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use rustc_hash::{FxHashMap, FxHashSet};
use typst_layout::PagedDocument;
use typst_library::introspection::Location;
use typst_library::model::AnchorGenerator;
use typst_syntax::VirtualPath;

use crate::{BundleDocument, Item};

/// Creates link anchors for all linked-to elements.
///
/// The `targets` map should contain the locations of all elements that are
/// linked to sharded by the path of the document they are contained in.
///
/// Returns the assigned anchors for all locations in the sets in `targets`.
/// The anchors are local to the file. Also creates empty anchors for the
/// documents and assets themselves, so that they become linkable, too.
pub fn create_link_anchors(
    items: &mut [Item],
    targets: &FxHashMap<&VirtualPath, FxHashSet<Location>>,
) -> FxHashMap<Location, EcoString> {
    let empty = FxHashSet::default();
    let mut anchors: FxHashMap<Location, EcoString> = items
        .par_iter_mut()
        .flat_map_iter(|item| {
            let Item::Document(path, doc, _) = item else {
                return Either::Right([].iter().cloned());
            };
            let targets = targets.get(path).unwrap_or(&empty);
            match doc {
                BundleDocument::Html(doc) => Either::Left(
                    // Mutates the DOM in place to insert IDs as necessary.
                    typst_html::create_link_anchors(
                        doc.as_mut(),
                        &targets.iter().copied().collect(),
                    )
                    .into_iter(),
                ),
                BundleDocument::Paged(doc, options) => {
                    // Mutates the export options so that named destinations
                    // are generated (if supported by the format).
                    options.anchors = create_paged_link_anchors(doc, targets);
                    Either::Right(options.anchors.iter().cloned())
                }
            }
        })
        .collect();

    // Create empty anchors for assets and documents so that we can also link
    // directly to them.
    for item in items {
        let loc = match item {
            Item::Tag(_) => continue,
            Item::Asset(.., loc) => loc,
            Item::Document(.., loc) => loc,
        };
        anchors.insert(*loc, EcoString::new());
    }

    anchors
}

/// Creates link anchors for a paged document.
///
/// The `targets` set should contain the locations of all elements in the paged
/// document that are linked to from somewhere.
fn create_paged_link_anchors(
    doc: &PagedDocument,
    targets: &FxHashSet<Location>,
) -> Vec<(Location, EcoString)> {
    let elements = doc.introspector().elements();

    let mut generator = AnchorGenerator::new(doc.introspector().as_ref());
    let mut anchors = Vec::new();
    let mut targets: Vec<_> = targets.iter().copied().collect();
    targets.sort_by_key(|loc| elements.loc_index(loc));

    for target in targets {
        if let Some(elem) = elements.get_by_loc(&target) {
            let anchor = generator.identify(elem.label());
            anchors.push((target, anchor));
        }
    }

    anchors
}
