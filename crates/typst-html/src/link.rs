use std::collections::VecDeque;
use std::sync::Arc;

use ecow::{EcoString, EcoVec, eco_vec};
use rustc_hash::{FxHashMap, FxHashSet};
use typst_library::foundations::Label;
use typst_library::introspection::{DocumentPosition, InnerHtmlPosition, Location, Tag};
use typst_library::layout::{Frame, FrameItem, Point};
use typst_library::model::AnchorGenerator;

use crate::{HtmlDocument, HtmlElement, HtmlNode, attr, tag};

/// Attaches IDs to nodes produced by link targets to make them linkable.
///
/// The `targets` set should contain the locations of all elements in the HTML
/// document that are linked to from somewhere.
///
/// May produce `<span>`s for link targets that turned into text nodes or no
/// nodes at all. See the [`LinkElem`](typst_library::model::LinkElem)
/// documentation for more details.
///
/// Anchor ID generation attempts to use existing HTML element IDs and Typst
/// labels to generate human-readable fragment names. If a label occurs multiple
/// times, it's disambiguated with a suffix. This disambiguation is per
/// document, even in bundle output. It uses the document's own introspector.
pub fn create_link_anchors(
    document: &mut HtmlDocument,
    targets: &FxHashSet<Location>,
) -> FxHashMap<Location, EcoString> {
    if targets.is_empty() {
        // Nothing to do.
        return FxHashMap::default();
    }

    // Assign IDs to all link targets.
    let mut work = Work::new();
    let introspector = Arc::clone(document.introspector());
    traverse(
        &mut work,
        targets,
        &mut AnchorGenerator::new(introspector.as_ref()),
        &mut document.root_mut().children,
    );
    work.ids
}

/// Traverses a list of nodes.
fn traverse(
    work: &mut Work,
    targets: &FxHashSet<Location>,
    generator: &mut AnchorGenerator<'_>,
    nodes: &mut EcoVec<HtmlNode>,
) {
    let mut i = 0;
    while i < nodes.len() {
        let node = &mut nodes.make_mut()[i];
        match node {
            // When visiting a start tag, we check whether the element needs an
            // ID and if so, add it to the queue, so that its first child node
            // receives an ID.
            HtmlNode::Tag(Tag::Start(elem, loc, _)) => {
                let loc = *loc;
                if targets.contains(&loc) {
                    work.enqueue(loc, elem.label());
                }
            }

            // When we reach an end tag, we check whether it closes an element
            // that is still in our queue. If so, that means the element
            // produced no nodes and we need to insert an empty span.
            HtmlNode::Tag(Tag::End(loc, _, _)) => {
                work.remove(*loc, |label| {
                    let mut element = HtmlElement::new(tag::span);
                    let id = generator.assign(&mut element, label);
                    nodes.insert(i + 1, HtmlNode::Element(element));
                    id
                });
            }

            // When visiting an element and the queue is non-empty, we assign an
            // ID. Then, we traverse its children.
            HtmlNode::Element(element) => {
                work.drain(|label| generator.assign(element, label));
                traverse(work, targets, generator, &mut element.children);
            }

            // When visiting text and the queue is non-empty, we generate a span
            // and assign an ID.
            HtmlNode::Text(..) => {
                work.drain(|label| {
                    let mut element =
                        HtmlElement::new(tag::span).with_children(eco_vec![node.clone()]);
                    let id = generator.assign(&mut element, label);
                    *node = HtmlNode::Element(element);
                    id
                });
            }

            // When visiting a frame and the queue is non-empty, we assign an
            // ID to it (will be added to the resulting SVG element).
            HtmlNode::Frame(frame) => {
                work.drain(|label| {
                    frame.id.get_or_insert_with(|| generator.identify(label)).clone()
                });
                traverse_frame(
                    work,
                    targets,
                    generator,
                    &frame.inner,
                    &mut frame.anchors,
                );
            }
        }

        i += 1;
    }
}

/// Traverses a frame embedded in HTML.
fn traverse_frame(
    work: &mut Work,
    targets: &FxHashSet<Location>,
    generator: &mut AnchorGenerator<'_>,
    frame: &Frame,
    anchors: &mut EcoVec<(Point, EcoString)>,
) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Tag(Tag::Start(elem, loc, _)) => {
                let loc = *loc;
                if targets.contains(&loc)
                    && let Some(DocumentPosition::Html(position)) =
                        generator.introspector().position(loc)
                    && let Some(InnerHtmlPosition::Frame(point)) = position.details()
                {
                    let id = generator.identify(elem.label());
                    work.ids.insert(loc, id.clone());
                    anchors.push((*point, id));
                }
            }
            FrameItem::Group(group) => {
                traverse_frame(work, targets, generator, &group.frame, anchors);
            }
            _ => {}
        }
    }
}

/// Keeps track of the work to be done during ID generation.
struct Work {
    /// The locations and labels of elements we need to assign an ID to right
    /// now.
    queue: VecDeque<(Location, Option<Label>)>,
    /// The resulting mapping from element location's to HTML IDs.
    ids: FxHashMap<Location, EcoString>,
}

impl Work {
    /// Sets up.
    fn new() -> Self {
        Self { queue: VecDeque::new(), ids: FxHashMap::default() }
    }

    /// Marks the element with the given location and label as in need of an
    /// ID. A subsequent call to `drain` will call `f`.
    fn enqueue(&mut self, loc: Location, label: Option<Label>) {
        self.queue.push_back((loc, label))
    }

    /// If one or multiple elements are in need of an ID, calls `f` to generate
    /// an ID and apply it to the current node with `f`, and then establishes a
    /// mapping from the elements' locations to that ID.
    fn drain(&mut self, f: impl FnOnce(Option<Label>) -> EcoString) {
        if let Some(&(_, label)) = self.queue.front() {
            let id = f(label);
            for (loc, _) in self.queue.drain(..) {
                self.ids.insert(loc, id.clone());
            }
        }
    }

    /// Similar to `drain`, but only for a specific given location.
    fn remove(&mut self, loc: Location, f: impl FnOnce(Option<Label>) -> EcoString) {
        if let Some(i) = self.queue.iter().position(|&(l, _)| l == loc) {
            let (_, label) = self.queue.remove(i).unwrap();
            let id = f(label);
            self.ids.insert(loc, id.clone());
        }
    }
}

trait AnchorGeneratorExt {
    /// Assigns an ID to an element or reuses an existing ID.
    fn assign(&mut self, element: &mut HtmlElement, label: Option<Label>) -> EcoString;
}

impl AnchorGeneratorExt for AnchorGenerator<'_> {
    fn assign(&mut self, element: &mut HtmlElement, label: Option<Label>) -> EcoString {
        element.attrs.get(attr::id).cloned().unwrap_or_else(|| {
            let id = self.identify(label);
            element.attrs.push_front(attr::id, id.clone());
            id
        })
    }
}
