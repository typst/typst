use std::collections::VecDeque;

use comemo::Track;
use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use rustc_hash::{FxHashMap, FxHashSet};
use typst_library::foundations::{Label, NativeElement};
use typst_library::introspection::{Introspector, Location, Tag};
use typst_library::layout::{Frame, FrameItem, Point};
use typst_library::model::{Destination, LinkElem};
use typst_utils::PicoStr;

use crate::{HtmlElement, HtmlNode, attr, tag};

/// Searches for links within a frame.
///
/// If all links are created via `LinkElem` in the future, this can be removed
/// in favor of the query in `identify_link_targets`. For the time being, some
/// links are created without existence of a `LinkElem`, so this is
/// unfortunately necessary.
pub fn introspect_frame_links(frame: &Frame, targets: &mut FxHashSet<Location>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Link(Destination::Location(loc), _) => {
                targets.insert(*loc);
            }
            FrameItem::Group(group) => introspect_frame_links(&group.frame, targets),
            _ => {}
        }
    }
}

/// Attaches IDs to nodes produced by link targets to make them linkable.
///
/// May produce `<span>`s for link targets that turned into text nodes or no
/// nodes at all. See the [`LinkElem`] documentation for more details.
pub fn identify_link_targets(
    root: &mut HtmlElement,
    introspector: &mut Introspector,
    mut targets: FxHashSet<Location>,
) {
    // Query for all links with an intra-doc (i.e. `Location`) destination to
    // know what needs IDs.
    targets.extend(
        introspector
            .query(&LinkElem::ELEM.select())
            .iter()
            .map(|elem| elem.to_packed::<LinkElem>().unwrap())
            .filter_map(|elem| match elem.dest.resolve(introspector.track()) {
                Ok(Destination::Location(loc)) => Some(loc),
                _ => None,
            }),
    );

    if targets.is_empty() {
        // Nothing to do.
        return;
    }

    // Assign IDs to all link targets.
    let mut work = Work::new();
    traverse(
        &mut work,
        &targets,
        &mut Identificator::new(introspector),
        &mut root.children,
    );

    // Add the mapping from locations to IDs to the introspector to make it
    // available to links in the next iteration.
    introspector.set_html_ids(work.ids);
}

/// Traverses a list of nodes.
fn traverse(
    work: &mut Work,
    targets: &FxHashSet<Location>,
    identificator: &mut Identificator<'_>,
    nodes: &mut EcoVec<HtmlNode>,
) {
    let mut i = 0;
    while i < nodes.len() {
        let node = &mut nodes.make_mut()[i];
        match node {
            // When visiting a start tag, we check whether the element needs an
            // ID and if so, add it to the queue, so that its first child node
            // receives an ID.
            HtmlNode::Tag(Tag::Start(elem)) => {
                let loc = elem.location().unwrap();
                if targets.contains(&loc) {
                    work.enqueue(loc, elem.label());
                }
            }

            // When we reach an end tag, we check whether it closes an element
            // that is still in our queue. If so, that means the element
            // produced no nodes and we need to insert an empty span.
            HtmlNode::Tag(Tag::End(loc, _)) => {
                work.remove(*loc, |label| {
                    let mut element = HtmlElement::new(tag::span);
                    let id = identificator.assign(&mut element, label);
                    nodes.insert(i + 1, HtmlNode::Element(element));
                    id
                });
            }

            // When visiting an element and the queue is non-empty, we assign an
            // ID. Then, we traverse its children.
            HtmlNode::Element(element) => {
                work.drain(|label| identificator.assign(element, label));
                traverse(work, targets, identificator, &mut element.children);
            }

            // When visiting text and the queue is non-empty, we generate a span
            // and assign an ID.
            HtmlNode::Text(..) => {
                work.drain(|label| {
                    let mut element =
                        HtmlElement::new(tag::span).with_children(eco_vec![node.clone()]);
                    let id = identificator.assign(&mut element, label);
                    *node = HtmlNode::Element(element);
                    id
                });
            }

            // When visiting a frame and the queue is non-empty, we assign an
            // ID to it (will be added to the resulting SVG element).
            HtmlNode::Frame(frame) => {
                work.drain(|label| {
                    frame.id.get_or_insert_with(|| identificator.identify(label)).clone()
                });
                traverse_frame(
                    work,
                    targets,
                    identificator,
                    &frame.inner,
                    &mut frame.link_points,
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
    identificator: &mut Identificator<'_>,
    frame: &Frame,
    link_points: &mut EcoVec<(Point, EcoString)>,
) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Tag(Tag::Start(elem)) => {
                let loc = elem.location().unwrap();
                if targets.contains(&loc) {
                    let pos = identificator.introspector.position(loc).point;
                    let id = identificator.identify(elem.label());
                    work.ids.insert(loc, id.clone());
                    link_points.push((pos, id));
                }
            }
            FrameItem::Group(group) => {
                traverse_frame(work, targets, identificator, &group.frame, link_points);
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

/// Creates unique IDs for elements.
struct Identificator<'a> {
    introspector: &'a Introspector,
    loc_counter: usize,
    label_counter: FxHashMap<Label, usize>,
}

impl<'a> Identificator<'a> {
    /// Creates a new identificator.
    fn new(introspector: &'a Introspector) -> Self {
        Self {
            introspector,
            loc_counter: 0,
            label_counter: FxHashMap::default(),
        }
    }

    /// Assigns an ID to an element or reuses an existing ID.
    fn assign(&mut self, element: &mut HtmlElement, label: Option<Label>) -> EcoString {
        element.attrs.get(attr::id).cloned().unwrap_or_else(|| {
            let id = self.identify(label);
            element.attrs.push_front(attr::id, id.clone());
            id
        })
    }

    /// Generates an ID, potentially based on a label.
    fn identify(&mut self, label: Option<Label>) -> EcoString {
        if let Some(label) = label {
            let resolved = label.resolve();
            let text = resolved.as_str();
            if can_use_label_as_id(text) {
                if self.introspector.label_count(label) == 1 {
                    return text.into();
                }

                let counter = self.label_counter.entry(label).or_insert(0);
                *counter += 1;
                return disambiguate(self.introspector, text, counter);
            }
        }

        self.loc_counter += 1;
        disambiguate(self.introspector, "loc", &mut self.loc_counter)
    }
}

/// Whether the label is both a valid CSS identifier and a valid URL fragment
/// for linking.
///
/// This is slightly more restrictive than HTML and CSS, but easier to
/// understand and explain.
fn can_use_label_as_id(label: &str) -> bool {
    !label.is_empty()
        && label.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
        && !label.starts_with(|c: char| c.is_numeric() || c == '-')
}

/// Disambiguates `text` with the suffix `-{counter}`, while ensuring that this
/// does not result in a collision with an existing label.
fn disambiguate(
    introspector: &Introspector,
    text: &str,
    counter: &mut usize,
) -> EcoString {
    loop {
        let disambiguated = eco_format!("{text}-{counter}");
        if PicoStr::get(&disambiguated)
            .and_then(Label::new)
            .is_some_and(|label| introspector.label_count(label) > 0)
        {
            *counter += 1;
        } else {
            break disambiguated;
        }
    }
}

