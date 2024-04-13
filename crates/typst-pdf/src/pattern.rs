use ecow::eco_format;
use pdf_writer::types::{ColorSpaceOperand, PaintType, TilingType};
use pdf_writer::{Filter, Finish, Name, Rect};
use typst::layout::{Abs, Ratio, Transform};
use typst::util::Numeric;
use typst::visualize::{Pattern, RelativeTo};

use crate::color::PaintEncode;
use crate::page::{construct_page, PageContext, PageResource, ResourceKind, Transforms};
use crate::{transform_to_array, PdfContext};

/// Writes the actual patterns (tiling patterns) to the PDF.
/// This is performed once after writing all pages.
pub(crate) fn write_patterns(ctx: &mut PdfContext) {
    for PdfPattern { transform, pattern, content, resources } in ctx.pattern_map.items() {
        let tiling = ctx.alloc.bump();
        ctx.pattern_refs.push(tiling);

        let mut tiling_pattern = ctx.pdf.tiling_pattern(tiling, content);
        tiling_pattern
            .tiling_type(TilingType::ConstantSpacing)
            .paint_type(PaintType::Colored)
            .bbox(Rect::new(
                0.0,
                0.0,
                pattern.size().x.to_pt() as _,
                pattern.size().y.to_pt() as _,
            ))
            .x_step((pattern.size().x + pattern.spacing().x).to_pt() as _)
            .y_step((pattern.size().y + pattern.spacing().y).to_pt() as _);

        let mut resources_map = tiling_pattern.resources();

        resources_map.x_objects().pairs(
            resources
                .iter()
                .filter(|(res, _)| res.is_x_object())
                .map(|(res, ref_)| (res.name(), ctx.image_refs[*ref_])),
        );

        resources_map.fonts().pairs(
            resources
                .iter()
                .filter(|(res, _)| res.is_font())
                .map(|(res, ref_)| (res.name(), ctx.font_refs[*ref_])),
        );

        ctx.colors
            .write_color_spaces(resources_map.color_spaces(), &mut ctx.alloc);

        resources_map
            .patterns()
            .pairs(
                resources
                    .iter()
                    .filter(|(res, _)| res.is_pattern())
                    .map(|(res, ref_)| (res.name(), ctx.pattern_refs[*ref_])),
            )
            .pairs(
                resources
                    .iter()
                    .filter(|(res, _)| res.is_gradient())
                    .map(|(res, ref_)| (res.name(), ctx.gradient_refs[*ref_])),
            );

        resources_map.ext_g_states().pairs(
            resources
                .iter()
                .filter(|(res, _)| res.is_ext_g_state())
                .map(|(res, ref_)| (res.name(), ctx.ext_gs_refs[*ref_])),
        );

        resources_map.finish();
        tiling_pattern
            .matrix(transform_to_array(
                transform
                    .pre_concat(Transform::scale(Ratio::one(), -Ratio::one()))
                    .post_concat(Transform::translate(Abs::zero(), pattern.spacing().y)),
            ))
            .filter(Filter::FlateDecode);
    }
}

/// A pattern and its transform.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PdfPattern {
    /// The transform to apply to the pattern.
    pub transform: Transform,
    /// The pattern to paint.
    pub pattern: Pattern,
    /// The rendered pattern.
    pub content: Vec<u8>,
    /// The resources used by the pattern.
    pub resources: Vec<(PageResource, usize)>,
}

/// Registers a pattern with the PDF.
fn register_pattern(
    ctx: &mut PageContext,
    pattern: &Pattern,
    on_text: bool,
    mut transforms: Transforms,
) -> usize {
    // Edge cases for strokes.
    if transforms.size.x.is_zero() {
        transforms.size.x = Abs::pt(1.0);
    }

    if transforms.size.y.is_zero() {
        transforms.size.y = Abs::pt(1.0);
    }

    let transform = match pattern.unwrap_relative(on_text) {
        RelativeTo::Self_ => transforms.transform,
        RelativeTo::Parent => transforms.container_transform,
    };

    // Render the body.
    let (_, content) = construct_page(ctx.parent, pattern.frame());

    let mut pdf_pattern = PdfPattern {
        transform,
        pattern: pattern.clone(),
        content: content.content.wait().clone(),
        resources: content.resources.into_iter().collect(),
    };

    pdf_pattern.resources.sort();

    ctx.parent.pattern_map.insert(pdf_pattern)
}

impl PaintEncode for Pattern {
    fn set_as_fill(&self, ctx: &mut PageContext, on_text: bool, transforms: Transforms) {
        ctx.reset_fill_color_space();

        let index = register_pattern(ctx, self, on_text, transforms);
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
        ctx.resources
            .insert(PageResource::new(ResourceKind::Pattern, id), index);
    }

    fn set_as_stroke(
        &self,
        ctx: &mut PageContext,
        on_text: bool,
        transforms: Transforms,
    ) {
        ctx.reset_stroke_color_space();

        let index = register_pattern(ctx, self, on_text, transforms);
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_stroke_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_stroke_pattern(None, name);
        ctx.resources
            .insert(PageResource::new(ResourceKind::Pattern, id), index);
    }
}
