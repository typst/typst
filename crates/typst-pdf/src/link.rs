use crate::krilla::{FrameContext, GlobalContext};
use crate::util::{AbsExt, PointExt};
use krilla::action::{Action, LinkAction};
use krilla::annotation::{LinkAnnotation, Target};
use krilla::destination::XyzDestination;
use krilla::geom::Rect;
use typst_library::layout::{Abs, Point, Size};
use typst_library::model::Destination;

/// Save a link for later writing in the annotations dictionary.
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
        let t = point.transform(fc.state().transform);
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

    let pos = match dest {
        Destination::Url(u) => {
            fc.annotations.push(
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
            if let Some(named_dest) = gc.loc_to_named.get(loc) {
                fc.annotations.push(
                    LinkAnnotation::new(
                        rect,
                        Target::Destination(krilla::destination::Destination::Named(
                            named_dest.clone(),
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

    let page_index = pos.page.get() - 1;
    if !gc.page_excluded(page_index) {
        fc.annotations.push(
            LinkAnnotation::new(
                rect,
                Target::Destination(krilla::destination::Destination::Xyz(
                    XyzDestination::new(page_index, pos.point.to_krilla()),
                )),
            )
            .into(),
        );
    }
}
