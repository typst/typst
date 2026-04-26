use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;

use ecow::{EcoString, EcoVec};

use rustc_hash::FxHashSet;
use typst_library::diag::StrResult;
use typst_library::foundations::{Content, Label, Selector};
use typst_library::introspection::{
    DocumentPosition, ElementIntrospector, ElementIntrospectorBuilder, Introspector,
    Location, PagedPosition,
};
use typst_library::layout::{Frame, FrameItem, Point, Transform};
use typst_library::model::{Destination, Numbering};
use typst_syntax::VirtualPath;
use typst_utils::NonZeroExt;

use crate::Page;

/// An introspector implementation for paged documents.
#[derive(Clone)]
pub struct PagedIntrospector {
    /// The underlying target-agnostic introspector used for most queries.
    elements: ElementIntrospector<PagedPosition>,
    /// Locations that are linked to via `FrameItem::Link`.
    frame_link_targets: FxHashSet<Location>,
    /// The number of pages in the document.
    pages: NonZeroUsize,
    /// The page numberings, indexed by page number minus 1.
    page_numberings: Vec<Option<Numbering>>,
    /// The page supplements, indexed by page number minus 1.
    page_supplements: Vec<Content>,
}

impl PagedIntrospector {
    /// Creates an introspector for a paged document.
    #[typst_macros::time(name = "introspect pages")]
    pub fn new(pages: &[Page]) -> PagedIntrospector {
        let mut builder = PagedIntrospectorBuilder::default();
        let mut page_numberings = Vec::with_capacity(pages.len());
        let mut page_supplements = Vec::with_capacity(pages.len());

        // Discover all elements.
        for (i, page) in pages.iter().enumerate() {
            let nr = NonZeroUsize::new(1 + i).unwrap();
            page_numberings.push(page.numbering.clone());
            page_supplements.push(page.supplement.clone());
            builder.discover_frame(&page.frame, Transform::identity(), &mut |point| {
                PagedPosition { page: nr, point }
            });
        }

        builder.finish(
            NonZeroUsize::new(pages.len()).unwrap_or(NonZeroUsize::ONE),
            page_numberings,
            page_supplements,
        )
    }

    /// Resolves the position of the location in the pages.
    pub fn position(&self, location: Location) -> Option<PagedPosition> {
        self.elements.position(location).copied()
    }

    /// The underlying element introspector.
    pub fn elements(&self) -> &ElementIntrospector<PagedPosition> {
        &self.elements
    }

    /// Returns the locations that the paged document links to via
    /// `FrameItem::Link`.
    pub fn frame_link_targets(&self) -> &FxHashSet<Location> {
        &self.frame_link_targets
    }
}

impl Introspector for PagedIntrospector {
    fn query(&self, selector: &Selector) -> EcoVec<Content> {
        self.elements.query(selector)
    }

    fn query_first(&self, selector: &Selector) -> Option<Content> {
        self.elements.query_first(selector)
    }

    fn query_unique(&self, selector: &Selector) -> StrResult<Content> {
        self.elements.query_unique(selector)
    }

    fn query_label(&self, label: Label) -> StrResult<&Content> {
        self.elements.query_label(label)
    }

    fn query_labelled(&self) -> EcoVec<Content> {
        self.elements.query_labelled()
    }

    fn query_count_before(&self, selector: &Selector, end: Location) -> usize {
        self.elements.query_count_before(selector, end)
    }

    fn label_count(&self, label: Label) -> usize {
        self.elements.label_count(label)
    }

    fn locator(&self, key: u128, base: Location) -> Option<Location> {
        self.elements.locator(key, base)
    }

    fn pages(&self, _: Location) -> Option<NonZeroUsize> {
        Some(self.pages)
    }

    fn page(&self, location: Location) -> Option<NonZeroUsize> {
        self.elements.position(location).map(|pos| pos.page)
    }

    fn position(&self, location: Location) -> Option<DocumentPosition> {
        self.elements.position(location).copied().map(DocumentPosition::Paged)
    }

    fn page_numbering(&self, location: Location) -> Option<&Numbering> {
        let page = self.page(location)?;
        self.page_numberings
            .get(page.get() - 1)
            .and_then(|slot| slot.as_ref())
    }

    fn page_supplement(&self, location: Location) -> Option<&Content> {
        let page = self.page(location)?;
        self.page_supplements.get(page.get() - 1)
    }

    fn anchor(&self, _: Location) -> Option<&EcoString> {
        None
    }

    fn document(&self, _: Location) -> Option<Location> {
        None
    }

    fn path(&self, _: Location) -> Option<&VirtualPath> {
        None
    }
}

impl Debug for PagedIntrospector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("PagedIntrospector(..)")
    }
}

/// Builds the introspector.
#[derive(Default)]
struct PagedIntrospectorBuilder {
    elements: ElementIntrospectorBuilder<PagedPosition>,
    frame_link_targets: FxHashSet<Location>,
}

impl PagedIntrospectorBuilder {
    /// Build a complete introspector with all acceleration structures from a
    /// list of top-level pairs.
    fn finish(
        self,
        pages: NonZeroUsize,
        page_numberings: Vec<Option<Numbering>>,
        page_supplements: Vec<Content>,
    ) -> PagedIntrospector {
        PagedIntrospector {
            elements: self.elements.finalize(),
            frame_link_targets: self.frame_link_targets,
            pages,
            page_numberings,
            page_supplements,
        }
    }

    /// Discovers introspectibles in a frame.
    fn discover_frame<F>(&mut self, frame: &Frame, ts: Transform, to_pos: &mut F)
    where
        F: FnMut(Point) -> PagedPosition,
    {
        for (pos, item) in frame.items() {
            match item {
                FrameItem::Tag(tag) => {
                    self.elements.discover_tag(tag, to_pos(pos.transform(ts)));
                }
                FrameItem::Group(group) => {
                    let ts = ts
                        .pre_concat(Transform::translate(pos.x, pos.y))
                        .pre_concat(group.transform);

                    if let Some(parent) = group.parent {
                        self.elements.start_insertion();
                        self.discover_frame(&group.frame, ts, to_pos);
                        self.elements.end_insertion(parent.location);
                    } else {
                        self.discover_frame(&group.frame, ts, to_pos);
                    }
                }
                FrameItem::Link(dest, _) => {
                    if let Destination::Location(loc) = dest {
                        self.frame_link_targets.insert(*loc);
                    }
                }
                FrameItem::Text(..) | FrameItem::Shape(..) | FrameItem::Image(..) => {}
            }
        }
    }
}
