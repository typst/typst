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
use crate::tags::{self, Placeholder, TagNode};
use crate::util::{AbsExt, PointExt};

pub(crate) struct LinkAnnotation {
    pub(crate) id: tags::LinkId,
    pub(crate) placeholder: Placeholder,
    pub(crate) alt: Option<String>,
    pub(crate) quad_points: Vec<kg::Quadrilateral>,
    pub(crate) target: Target,
    pub(crate) span: Span,
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

    let Some((link_id, link, link_nodes)) = gc.tags.stack.find_parent_link() else {
        unreachable!("expected a link parent")
    };
    let alt = link.alt.as_ref().map(EcoString::to_string);

    let quad = to_quadrilateral(fc, size);

    // Unfortunately quadpoints still aren't well supported by most PDF readers,
    // even by acrobat. Which is understandable since they were only introduced
    // in PDF 1.6 (2005) /s
    let should_use_quadpoints = gc.options.standards.config.validator() == Validator::UA1;
    match fc.get_link_annotation(link_id) {
        Some(annotation) if should_use_quadpoints => annotation.quad_points.push(quad),
        _ => {
            let placeholder = gc.tags.placeholders.reserve();
            link_nodes.push(TagNode::Placeholder(placeholder));
            fc.push_link_annotation(LinkAnnotation {
                id: link_id,
                placeholder,
                quad_points: vec![quad],
                alt,
                target,
                span: link.span,
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
