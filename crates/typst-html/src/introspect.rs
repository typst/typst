use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;

use ecow::{EcoString, EcoVec};
use rustc_hash::{FxHashMap, FxHashSet};
use typst_library::diag::StrResult;
use typst_library::foundations::{Content, Label, Selector};
use typst_library::introspection::{
    DocumentPosition, ElementIntrospector, ElementIntrospectorBuilder, HtmlPosition,
    Introspector, Location,
};
use typst_library::layout::{Frame, FrameItem, Point, Transform};
use typst_library::model::{Destination, LinkElem, Numbering};
use typst_syntax::VirtualPath;

use crate::{HtmlNode, HtmlSliceExt, tag};

/// An introspector implementation for HTML documents.
#[derive(Clone)]
pub struct HtmlIntrospector {
    /// The underlying target-agnostic introspector used for most queries.
    elements: ElementIntrospector<HtmlPosition>,
    /// Locations that are linked to via `FrameItem::Link`.
    frame_link_targets: FxHashSet<Location>,
    /// Maps from element locations to assigned HTML link anchors. This is used
    /// to support intra-doc links.
    anchors: FxHashMap<Location, EcoString>,
}

impl HtmlIntrospector {
    /// Creates an introspector for an HTML document.
    #[typst_macros::time(name = "introspect html")]
    pub fn new(output: &[HtmlNode]) -> HtmlIntrospector {
        let mut builder = HtmlIntrospectorBuilder::default();
        builder.discover_nodes(output, &mut EcoVec::new());
        builder.finish()
    }

    /// The underlying element introspector.
    pub fn elements(&self) -> &ElementIntrospector<HtmlPosition> {
        &self.elements
    }

    /// Resolves the position in the DOM of an element.
    pub fn position(&self, location: Location) -> Option<HtmlPosition> {
        self.elements.position(location).cloned()
    }

    /// Computes all locations that are referenced by intra-doc links of any
    /// kind.
    pub fn link_targets(&self) -> FxHashSet<Location> {
        LinkElem::find_destinations(self)
            .chain(self.frame_link_targets.iter().copied())
            .collect()
    }

    /// Returns the locations that the HTML document links to via
    /// `FrameItem::Link`.
    pub fn frame_link_targets(&self) -> &FxHashSet<Location> {
        &self.frame_link_targets
    }

    /// Enriches an existing introspector with HTML link anchors, which were
    /// assigned to the DOM in a post-processing step.
    pub fn set_anchors(&mut self, anchors: FxHashMap<Location, EcoString>) {
        self.anchors = anchors;
    }
}

impl Introspector for HtmlIntrospector {
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
        None
    }

    fn page(&self, _: Location) -> Option<NonZeroUsize> {
        None
    }

    fn position(&self, location: Location) -> Option<DocumentPosition> {
        self.position(location).map(DocumentPosition::Html)
    }

    fn page_numbering(&self, _: Location) -> Option<&Numbering> {
        None
    }

    fn page_supplement(&self, _: Location) -> Option<&Content> {
        None
    }

    fn anchor(&self, location: Location) -> Option<&EcoString> {
        self.anchors.get(&location)
    }

    fn document(&self, _: Location) -> Option<Location> {
        None
    }

    fn path(&self, _: Location) -> Option<&VirtualPath> {
        None
    }
}

impl Debug for HtmlIntrospector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("HtmlIntrospector(..)")
    }
}

/// Constructs the [`HtmlIntrospector`].
#[derive(Default)]
struct HtmlIntrospectorBuilder {
    elements: ElementIntrospectorBuilder<HtmlPosition>,
    frame_link_targets: FxHashSet<Location>,
}

impl HtmlIntrospectorBuilder {
    /// Returns the resulting introspector.
    fn finish(self) -> HtmlIntrospector {
        HtmlIntrospector {
            elements: self.elements.finalize(),
            frame_link_targets: self.frame_link_targets,
            anchors: FxHashMap::default(),
        }
    }

    /// Discovers introspectibles in a collection of HTML nodes.
    fn discover_nodes(
        &mut self,
        nodes: &[HtmlNode],
        current_position: &mut EcoVec<usize>,
    ) {
        for (node, dom_index) in nodes.iter_with_dom_indices() {
            match node {
                HtmlNode::Tag(tag) => {
                    current_position.push(dom_index);
                    self.elements
                        .discover_tag(tag, HtmlPosition::new(current_position.clone()));
                    current_position.pop();
                }
                HtmlNode::Text(_, _) => {}
                HtmlNode::Element(elem) => {
                    let is_root = elem.tag == tag::html;
                    if !is_root {
                        current_position.push(dom_index);
                    }

                    if let Some(parent) = elem.parent {
                        self.elements.start_insertion();
                        self.discover_nodes(&elem.children, current_position);
                        self.elements.end_insertion(parent);
                    } else {
                        self.discover_nodes(&elem.children, current_position);
                    }

                    if !is_root {
                        current_position.pop();
                    }
                }
                HtmlNode::Frame(frame) => {
                    current_position.push(dom_index);
                    self.discover_frame(
                        &frame.inner,
                        Transform::identity(),
                        &mut |point| {
                            HtmlPosition::new(current_position.clone()).in_frame(point)
                        },
                    );
                    current_position.pop();
                }
            }
        }
    }

    /// Discovers introspectibles in a frame.
    fn discover_frame<F>(&mut self, frame: &Frame, ts: Transform, to_pos: &mut F)
    where
        F: FnMut(Point) -> HtmlPosition,
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
