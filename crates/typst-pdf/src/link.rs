use ecow::EcoString;
use krilla::action::{Action, LinkAction};
use krilla::annotation::Target;
use krilla::destination::XyzDestination;
use krilla::geom as kg;
use typst_library::diag::{SourceResult, bail};
use typst_library::layout::{Abs, Point, Position, Size};
use typst_library::model::Destination;
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext, PageIndexConverter};
use crate::tags::{self, AnnotationId, GroupId};
use crate::util::{AbsExt, PointExt};

pub(crate) struct LinkAnnotation {
    pub kind: LinkAnnotationKind,
    pub alt: Option<String>,
    pub span: Span,
    pub quad_points: Vec<kg::Quadrilateral>,
    pub target: Target,
}

pub(crate) enum LinkAnnotationKind {
    /// A link annotation that is tagged within a `Link` structure element.
    Tagged(AnnotationId),
    /// A link annotation within an artifact.
    Artifact,
}

pub(crate) fn handle_link(
    fc: &mut FrameContext,
    gc: &mut GlobalContext,
    dest: &Destination,
    size: Size,
) -> SourceResult<()> {
    let target = match dest {
        Destination::Url(u) => {
            Target::Action(Action::Link(LinkAction::new(u.to_string())))
        }
        Destination::Position(p) => {
            let Some(dest) = pos_to_xyz(&gc.page_index_converter, *p) else {
                return Ok(());
            };
            Target::Destination(krilla::destination::Destination::Xyz(dest))
        }
        Destination::Location(loc) => {
            if let Some(nd) = gc.loc_to_names.get(loc) {
                // If a named destination has been registered, it's already guaranteed to
                // not point to an excluded page.
                Target::Destination(krilla::destination::Destination::Named(nd.clone()))
            } else {
                let pos = gc.document.introspector.position(*loc);
                let Some(dest) = pos_to_xyz(&gc.page_index_converter, pos) else {
                    return Ok(());
                };
                Target::Destination(krilla::destination::Destination::Xyz(dest))
            }
        }
    };

    let quad = to_quadrilateral(fc, size);

    if tags::disabled(gc) {
        if gc.tags.in_tiling && gc.options.is_pdf_ua() {
            let validator = gc.options.standards.config.validator().as_str();
            bail!(
                Span::detached(),
                "{validator} error: PDF artifacts may not contain links";
                hint: "a link was used within a tiling";
                hint: "references, citations, and footnotes \
                      are also considered links in PDF"
            );
        }

        fc.push_link_annotation(
            GroupId::INVALID,
            LinkAnnotation {
                kind: LinkAnnotationKind::Artifact,
                alt: None,
                span: Span::detached(),
                quad_points: vec![quad],
                target,
            },
        );
        return Ok(());
    }

    let (group_id, link) = gc.tags.tree.parent_link().expect("link parent");
    let alt = link.alt.as_ref().map(EcoString::to_string);

    if gc.tags.tree.parent_artifact().is_some() {
        if gc.options.is_pdf_ua() {
            let validator = gc.options.standards.config.validator().as_str();
            bail!(
                link.span(),
                "{validator} error: PDF artifacts may not contain links";
                hint: "references, citations, and footnotes \
                      are also considered links in PDF"
            );
        }

        fc.push_link_annotation(
            group_id,
            LinkAnnotation {
                kind: LinkAnnotationKind::Artifact,
                alt,
                span: link.span(),
                quad_points: vec![quad],
                target,
            },
        );
        return Ok(());
    }

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
    let join_annotations = gc.options.is_pdf_ua();
    match fc.get_link_annotation(group_id) {
        Some(annotation) if join_annotations => annotation.quad_points.push(quad),
        _ => {
            let annot_id = gc.tags.annotations.reserve();
            fc.push_link_annotation(
                group_id,
                LinkAnnotation {
                    kind: LinkAnnotationKind::Tagged(annot_id),
                    alt,
                    span: link.span(),
                    quad_points: vec![quad],
                    target,
                },
            );
            let group = gc.tags.tree.groups.get_mut(group_id);
            group.push_annotation(annot_id);
        }
    }

    Ok(())
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
