use krilla::action::{Action, LinkAction};
use krilla::annotation::{LinkAnnotation, Target};
use krilla::destination::XyzDestination;
use krilla::geom::Rect;
use typst_library::layout::{Abs, Point, Position, Size};
use typst_library::model::Destination;

use crate::convert::{FrameContext, GlobalContext, PageIndexConverter};
use crate::util::{AbsExt, PointExt};

pub(crate) fn handle_link(
    fc: &mut FrameContext,
    gc: &mut GlobalContext,
    dest: &Destination,
    size: Size,
) {
    let mut min_x = Abs::inf();
    let mut min_y = Abs::inf();
    let mut max_x = -Abs::inf();
    let mut max_y = -Abs::inf();

    let pos = Point::zero();

    // Compute the bounding box of the transformed link.
    for point in [
        pos,
        pos + Point::with_x(size.x),
        pos + Point::with_y(size.y),
        pos + size.to_point(),
    ] {
        let t = point.transform(fc.state().transform());
        min_x.set_min(t.x);
        min_y.set_min(t.y);
        max_x.set_max(t.x);
        max_y.set_max(t.y);
    }

    let x1 = min_x.to_f32();
    let x2 = max_x.to_f32();
    let y1 = min_y.to_f32();
    let y2 = max_y.to_f32();

    let rect = Rect::from_ltrb(x1, y1, x2, y2).unwrap();

    // TODO: Support quad points.

    let pos = match dest {
        Destination::Url(u) => {
            fc.push_annotation(
                LinkAnnotation::new(
                    rect,
                    Target::Action(Action::Link(LinkAction::new(u.to_string()))),
                )
                .into(),
            );
            return;
        }
        Destination::Position(p) => *p,
        Destination::Location(loc) => {
            if let Some(nd) = gc.loc_to_names.get(loc) {
                // If a named destination has been registered, it's already guaranteed to
                // not point to an excluded page.
                fc.push_annotation(
                    LinkAnnotation::new(
                        rect,
                        Target::Destination(krilla::destination::Destination::Named(
                            nd.clone(),
                        )),
                    )
                    .into(),
                );
                return;
            } else {
                gc.document.introspector.position(*loc)
            }
        }
    };

    if let Some(xyz) = pos_to_xyz(&gc.page_index_converter, pos) {
        fc.push_annotation(
            LinkAnnotation::new(
                rect,
                Target::Destination(krilla::destination::Destination::Xyz(xyz)),
            )
            .into(),
        );
    }
}

/// Turns a position link into a PDF XYZ destination.
///
/// - Takes into account page index conversion (if only part of the document is
///   exported)
/// - Consistently shifts the link by 10pt because the position of e.g.
///   backlinks to footnotes is always at the baseline and if you link directly
///   to it, the text will not be visible since it is right above.
pub(crate) fn pos_to_xyz(
    pic: &PageIndexConverter,
    pos: Position,
) -> Option<XyzDestination> {
    let page_index = pic.pdf_page_index(pos.page.get() - 1)?;
    let adjusted =
        Point::new(pos.point.x, (pos.point.y - Abs::pt(10.0)).max(Abs::zero()));
    Some(XyzDestination::new(page_index, adjusted.to_krilla()))
}
