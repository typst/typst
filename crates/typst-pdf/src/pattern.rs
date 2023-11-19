use ecow::{eco_format, EcoString};
use pdf_writer::{
    types::{ColorSpaceOperand, PaintType, TilingType},
    Filter, Finish, Name, Rect, Ref,
};
use typst::geom::{Abs, Numeric, Pattern, Ratio, Relative, Transform};

use crate::{
    color::PaintEncode,
    deflate,
    page::{construct_page, PageContext, PageResource, Transforms},
    transform_to_array, PdfContext,
};

/// Writes the actual patterns (tiling patterns) to the PDF.
/// This is performed once after writing all pages.
pub(crate) fn write_patterns(ctx: &mut PdfContext) {
    for (tiling, PdfPattern { transform, pattern, content, resources }) in
        ctx.pattern_map.items()
    {
        ctx.pattern_refs.push(tiling);

        let content = deflate(&content);
        let mut tiling_pattern = ctx.pdf.tiling_pattern(tiling, &content);
        tiling_pattern
            .paint_type(PaintType::Colored)
            .tiling_type(TilingType::FastConstantSpacing)
            .bbox(Rect::new(
                0.0,
                0.0,
                pattern.bbox.x.to_pt() as _,
                pattern.bbox.y.to_pt() as _,
            ))
            .x_step(pattern.bbox.x.to_pt() as _)
            .y_step(pattern.bbox.y.to_pt() as _);

        let mut resources_map = tiling_pattern.resources();

        resources_map.x_objects().pairs(
            resources
                .iter()
                .filter(|(res, _)| res.is_x_object())
                .map(|(res, ref_)| (res.name(), *ref_)),
        );

        resources_map.fonts().pairs(
            resources
                .iter()
                .filter(|(res, _)| res.is_font())
                .map(|(res, ref_)| (res.name(), *ref_)),
        );

        ctx.colors.write_color_spaces(resources_map.color_spaces(), &mut ctx.alloc);

        resources_map.patterns().pairs(
            resources
                .iter()
                .filter(|(res, _)| res.is_pattern())
                .map(|(res, ref_)| (res.name(), *ref_)),
        );

        resources_map.ext_g_states().pairs(
            resources
                .iter()
                .filter(|(res, _)| res.is_ext_g_state())
                .map(|(res, ref_)| (res.name(), *ref_)),
        );

        resources_map.finish();
        tiling_pattern
            .matrix(transform_to_array(*transform))
            .filter(Filter::FlateDecode);
    }
}

/// A pattern and its transform.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PdfPattern {
    /// The transform to apply to the gradient.
    pub transform: Transform,
    /// The pattern to paint.
    pub pattern: Pattern,
    /// The rendered pattern.
    pub content: Vec<u8>,
    /// The resources used by the pattern.
    pub resources: Vec<(PageResource, Ref)>,
}

fn register_pattern(
    ctx: &mut PageContext,
    pattern: &Pattern,
    on_text: bool,
    mut transforms: Transforms,
) -> (Ref, EcoString) {
    // Edge cases for strokes.
    if transforms.size.x.is_zero() {
        transforms.size.x = Abs::pt(1.0);
    }

    if transforms.size.y.is_zero() {
        transforms.size.y = Abs::pt(1.0);
    }

    let transform = match pattern.unwrap_relative(on_text) {
        Relative::Self_ => Transform::identity(),
        Relative::Parent => transforms
            .container_transform
            .post_concat(transforms.transform.invert().unwrap())
            .pre_concat(Transform::scale(
                Ratio::new(transforms.size.x / transforms.container_size.x),
                Ratio::new(transforms.size.y / transforms.container_size.y),
            )),
    };

    /*let transform = relative_transform
    .pre_concat(Transform::scale(sx, sy));*/

    // Render the body.
    let (_, content) = construct_page(ctx.parent, &pattern.body);

    let pdf_pattern = PdfPattern {
        transform,
        pattern: pattern.clone(),
        content: content.content,
        resources: content.resources.into_iter().collect(),
    };

    let (ref_, index) = ctx.parent.pattern_map.insert(&mut ctx.parent.alloc, pdf_pattern);
    (ref_, eco_format!("P{}", index))
}

impl PaintEncode for Pattern {
    fn set_as_fill(&self, ctx: &mut PageContext, on_text: bool, transforms: Transforms) {
        ctx.reset_fill_color_space();

        let (ref_, id) = register_pattern(ctx, self, on_text, transforms);
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
        ctx.resources.insert(PageResource::Pattern(id), ref_);
    }

    fn set_as_stroke(&self, ctx: &mut PageContext, transforms: Transforms) {
        todo!()
    }
}
