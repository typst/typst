use ecow::eco_format;
use pdf_writer::types::{ColorSpaceOperand, PaintType, TilingType};
use pdf_writer::{Filter, Finish, Name, Rect, Ref};
use typst::layout::{Abs, Ratio, Transform};
use typst::util::Numeric;
use typst::visualize::{Pattern, RelativeTo};

use crate::color::PaintEncode;
use crate::content::{self, Resource, ResourceKind};
use crate::{transform_to_array, ConstructContext, PdfChunk, PdfResource};

pub struct Patterns;

impl PdfResource for Patterns {
    type Output = Vec<Ref>;

    /// Writes the actual patterns (tiling patterns) to the PDF.
    /// This is performed once after writing all pages.
    fn write(&self, context: &ConstructContext, chunk: &mut PdfChunk) -> Self::Output {
        let pattern_map = &context.remapped_patterns;
        let mut patterns = Vec::new();

        for PdfPattern { transform, pattern, content, resources } in pattern_map.items() {
            let tiling = chunk.alloc();
            patterns.push(tiling);

            let mut tiling_pattern = chunk.tiling_pattern(tiling, content);
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
                    .map(|(res, ref_)| (res.name(), ref_)),
            );

            resources_map.fonts().pairs(
                resources
                    .iter()
                    .filter(|(res, _)| res.is_font())
                    .map(|(res, ref_)| (res.name(), ref_)),
            );

            context
                .colors
                .write_color_spaces(resources_map.color_spaces(), &context.globals);

            resources_map
                .patterns()
                .pairs(
                    resources
                        .iter()
                        .filter(|(res, _)| res.is_pattern())
                        .map(|(res, ref_)| (res.name(), ref_)),
                )
                .pairs(
                    resources
                        .iter()
                        .filter(|(res, _)| res.is_gradient())
                        .map(|(res, ref_)| (res.name(), ref_)),
                );

            resources_map.ext_g_states().pairs(
                resources
                    .iter()
                    .filter(|(res, _)| res.is_ext_g_state())
                    .map(|(res, ref_)| (res.name(), ref_)),
            );

            resources_map.finish();
            tiling_pattern
                .matrix(transform_to_array(
                    transform
                        .pre_concat(Transform::scale(Ratio::one(), -Ratio::one()))
                        .post_concat(Transform::translate(
                            Abs::zero(),
                            pattern.spacing().y,
                        )),
                ))
                .filter(Filter::FlateDecode);
        }

        patterns
    }

    fn save(context: &mut crate::WriteContext, output: Self::Output) {
        context.patterns = output;
    }
}

/// A pattern and its transform.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PdfPattern<R> {
    /// The transform to apply to the pattern.
    pub transform: Transform,
    /// The pattern to paint.
    pub pattern: Pattern,
    /// The rendered pattern.
    pub content: Vec<u8>,
    /// The resources used by the pattern.
    pub resources: Vec<(Resource, R)>,
}

/// Registers a pattern with the PDF.
fn register_pattern(
    ctx: &mut content::Builder,
    pattern: &Pattern,
    on_text: bool,
    mut transforms: content::Transforms,
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
    let content = content::build(ctx.parent, pattern.frame());

    let mut pdf_pattern = PdfPattern {
        transform,
        pattern: pattern.clone(),
        content: content.content.wait().clone(),
        resources: content.resources.into_iter().collect(),
    };

    pdf_pattern.resources.sort();

    ctx.parent.patterns.insert(pdf_pattern)
}

impl PaintEncode for Pattern {
    fn set_as_fill(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) {
        ctx.reset_fill_color_space();

        let index = register_pattern(ctx, self, on_text, transforms);
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
        ctx.resources.insert(Resource::new(ResourceKind::Pattern, id), index);
    }

    fn set_as_stroke(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) {
        ctx.reset_stroke_color_space();

        let index = register_pattern(ctx, self, on_text, transforms);
        let id = eco_format!("P{index}");
        let name = Name(id.as_bytes());

        ctx.content.set_stroke_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_stroke_pattern(None, name);
        ctx.resources.insert(Resource::new(ResourceKind::Pattern, id), index);
    }
}
