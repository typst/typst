use ecow::EcoString;
use krilla::action::{Action, LinkAction};
use krilla::annotation::Target;
use krilla::configure::Validator;
use krilla::destination::XyzDestination;
use krilla::geom as kg;
use typst_library::layout::{Point, Position, Size};
use typst_library::model::Destination;
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::tags::{LinkId, Placeholder, TagNode};
use crate::util::{AbsExt, PointExt};

pub(crate) struct LinkAnnotation {
    pub(crate) kind: LinkAnnotationKind,
    pub(crate) quad_points: Vec<kg::Quadrilateral>,
    pub(crate) target: Target,
}

impl LinkAnnotation {
    pub(crate) fn id(&self) -> Option<LinkId> {
        match self.kind {
            LinkAnnotationKind::Tagged { id, .. } => Some(id),
            LinkAnnotationKind::Artifact => None,
        }
    }
}

pub(crate) enum LinkAnnotationKind {
    /// A link annotation that is tagged within a `Link` structure element.
    Tagged { id: LinkId, placeholder: Placeholder, alt: Option<String>, span: Span },
    /// A link annotation within an artifact.
    Artifact,
}

pub(crate) fn handle_link(
    fc: &mut FrameContext,
    gc: &mut GlobalContext,
    dest: &Destination,
    size: Size,
) {
    let target = match dest {
        Destination::Url(u) => {
            Target::Action(Action::Link(LinkAction::new(u.to_string())))
        }
        Destination::Position(p) => match pos_to_target(gc, *p) {
            Some(target) => target,
            None => return,
        },
        Destination::Location(loc) => {
            if let Some(nd) = gc.loc_to_names.get(loc) {
                // If a named destination has been registered, it's already guaranteed to
                // not point to an excluded page.
                Target::Destination(krilla::destination::Destination::Named(nd.clone()))
            } else {
                let pos = gc.document.introspector.position(*loc);
                match pos_to_target(gc, pos) {
                    Some(target) => target,
                    None => return,
                }
            }
        }
    };

    let quad = to_quadrilateral(fc, size);

    if gc.tags.disable.is_some() {
        fc.push_link_annotation(LinkAnnotation {
            kind: LinkAnnotationKind::Artifact,
            quad_points: vec![quad],
            target,
        });
        return;
    }

    let (link_id, tagging_ctx) = match gc.tags.stack.find_parent_link() {
        Some((link_id, link, nodes)) => (link_id, Some((link, nodes))),
        None if gc.options.disable_tags => {
            let link_id = gc.tags.next_link_id();
            (link_id, None)
        }
        None => unreachable!("expected a link parent"),
    };

    // Unfortunately quadpoints still aren't well supported by most PDF readers.
    // So only add multiple quadpoint entries to one annotation when targeting
    // PDF/UA. Otherwise generate multiple annotations, to avoid pdf readers
    // falling back to the bounding box rectangle, which can span parts unrelated
    // to the link. For example if there is a linebreak:
    // ```
    // Imagine this is a paragraph containing a link. It starts here https://github.com/
    // typst/typst and then ends on another line.
    // ```
    // The bounding box would span the entire paragraph, which is undesirable.
    let join_annotations = gc.options.standards.config.validator() == Validator::UA1;
    match fc.get_link_annotation(link_id) {
        Some(annotation) if join_annotations => annotation.quad_points.push(quad),
        _ => {
            let placeholder = gc.tags.placeholders.reserve();
            let (alt, span) = if let Some((link, id)) = tagging_ctx {
                let nodes = &mut gc.tags.groups.get_mut(id).nodes;
                nodes.push(TagNode::Placeholder(placeholder));
                let alt = link.alt.as_ref().map(EcoString::to_string);
                (alt, link.span())
            } else {
                (None, Span::detached())
            };
            fc.push_link_annotation(LinkAnnotation {
                kind: LinkAnnotationKind::Tagged { id: link_id, placeholder, alt, span },
                quad_points: vec![quad],
                target,
            });
        }
    }
}

/// Compute the quadrilateral representing the transformed rectangle of this frame.
fn to_quadrilateral(fc: &FrameContext, size: Size) -> kg::Quadrilateral {
    let pos = Point::zero();
    let points = [
        pos + Point::with_y(size.y),
        pos + size.to_point(),
        pos + Point::with_x(size.x),
        pos,
    ];

    kg::Quadrilateral(points.map(|point| {
        let p = point.transform(fc.state().transform());
        kg::Point::from_xy(p.x.to_f32(), p.y.to_f32())
    }))
}

fn pos_to_target(gc: &mut GlobalContext, pos: Position) -> Option<Target> {
    let page_index = pos.page.get() - 1;
    let index = gc.page_index_converter.pdf_page_index(page_index)?;

    let dest = XyzDestination::new(index, pos.point.to_krilla());
    Some(Target::Destination(krilla::destination::Destination::Xyz(dest)))
}
