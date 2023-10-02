use ecow::{eco_format, EcoString};
use pdf_writer::types::FunctionShadingType;
use pdf_writer::{types::ColorSpaceOperand, Name};
use pdf_writer::{Finish, Ref};

use super::color::{ColorSpaceExt, PaintEncode};
use super::page::{PageContext, Transforms};
use super::{PdfContext, RefExt};
use crate::geom::{
    Abs, Color, ColorSpace, Gradient, Numeric, Quadrant, Ratio, Relative, Transform,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(super) struct PdfGradient {
    /// The transform to apply to the gradient.
    pub transform: Transform,

    /// The aspect ratio of the gradient.
    /// Required for aspect ratio correction.
    pub aspect_ratio: Ratio,

    /// The gradient.
    pub gradient: Gradient,
}

/// Writes the actual gradients (shading patterns) to the PDF.
/// This is performed once after writing all pages.
pub fn write_gradients(ctx: &mut PdfContext) {
    for PdfGradient { transform, aspect_ratio, gradient } in
        ctx.gradient_map.items().cloned().collect::<Vec<_>>()
    {
        let shading = ctx.alloc.bump();
        ctx.gradient_refs.push(shading);

        let mut shading_pattern = match &gradient {
            Gradient::Linear(linear) => {
                let shading_function = shading_function(ctx, &gradient);
                let mut shading_pattern = ctx.writer.shading_pattern(shading);
                let mut shading = shading_pattern.function_shading();
                shading.shading_type(FunctionShadingType::Axial);

                ctx.colors
                    .write(gradient.space(), shading.color_space(), &mut ctx.alloc);

                let angle = linear.angle.correct_aspect_ratio(aspect_ratio);
                let (sin, cos) = (angle.sin(), angle.cos());
                let length = sin.abs() + cos.abs();

                shading
                    .anti_alias(gradient.anti_alias())
                    .function(shading_function)
                    .coords([0.0, 0.0, length as f32, 0.0])
                    .extend([true; 2]);

                shading.finish();

                shading_pattern
            }
        };

        shading_pattern.matrix(transform.as_array());
    }
}

fn shading_function(ctx: &mut PdfContext, gradient: &Gradient) -> Ref {
    let function = ctx.alloc.bump();
    let mut functions = vec![];
    let mut bounds = vec![];
    let mut encode = vec![];

    // Create the individual gradient functions for each pair of stops.
    for window in gradient.stops().windows(2) {
        let (first, second) = (window[0], window[1]);

        // Skip stops with the same position.
        if first.offset.unwrap().get() == second.offset.unwrap().get() {
            continue;
        }

        bounds.push(second.offset.unwrap().get() as f32);
        functions.push(single_gradient(ctx, first.color, second.color, gradient.space()));
        encode.extend([0.0, 1.0]);
    }

    // Special case for gradients with only two stops.
    if functions.len() == 1 {
        return functions[0];
    }

    // Remove the last bound, since it's not needed for the stitching function.
    bounds.pop();

    // Create the stitching function.
    ctx.writer
        .stitching_function(function)
        .domain([0.0, 1.0])
        .range(gradient.space().range())
        .functions(functions)
        .bounds(bounds)
        .encode(encode);

    function
}

fn single_gradient(
    ctx: &mut PdfContext,
    first_color: Color,
    second_color: Color,
    color_space: ColorSpace,
) -> Ref {
    let reference = ctx.alloc.bump();
    let mut exp = ctx.writer.exponential_function(reference);

    exp.range(color_space.range());
    exp.c0(color_space.convert(first_color));
    exp.c1(color_space.convert(second_color));
    exp.domain([0.0, 1.0]);
    exp.n(1.0);
    exp.finish();

    reference
}

impl PaintEncode for Gradient {
    fn set_as_fill(&self, ctx: &mut PageContext, transforms: Transforms) {
        ctx.reset_fill_color_space();

        let id = use_gradient(ctx, self, transforms);
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
    }

    fn set_as_stroke(&self, ctx: &mut PageContext, transforms: Transforms) {
        ctx.reset_stroke_color_space();

        let id = use_gradient(ctx, self, transforms);
        let name = Name(id.as_bytes());

        ctx.content.set_stroke_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_stroke_pattern(None, name);
    }
}

fn use_gradient(
    ctx: &mut PageContext,
    gradient: &Gradient,
    mut transforms: Transforms,
) -> EcoString {
    // Edge cases for strokes.
    if transforms.size.x.is_zero() {
        transforms.size.x = Abs::pt(1.0);
    }

    if transforms.size.y.is_zero() {
        transforms.size.y = Abs::pt(1.0);
    }

    let size = match gradient.unwrap_relative(false) {
        Relative::This => transforms.size,
        Relative::Parent => transforms.container_size,
    };

    let (offset_x, offset_y) = match gradient.dir().quadrant() {
        Quadrant::First => (Abs::zero(), Abs::zero()),
        Quadrant::Second => (size.x, Abs::zero()),
        Quadrant::Third => (size.x, size.y),
        Quadrant::Fourth => (Abs::zero(), size.y),
    };

    let transform = match gradient.unwrap_relative(false) {
        Relative::This => transforms.transform,
        Relative::Parent => transforms.container_transform,
    };

    let pdf_gradient = PdfGradient {
        aspect_ratio: size.aspect_ratio(),
        transform: transform
            .pre_concat(Transform::translate(offset_x, offset_y))
            .pre_concat(Transform::scale(
                Ratio::new(size.x.to_pt()),
                Ratio::new(size.y.to_pt()),
            ))
            .pre_concat(Transform::rotate(
                gradient.dir().correct_aspect_ratio(size.aspect_ratio()),
            )),
        gradient: gradient.clone(),
    };

    let index = ctx.parent.gradient_map.insert(pdf_gradient);
    eco_format!("Gr{}", index)
}

impl Transform {
    /// Convert to an array of floats.
    pub fn as_array(self) -> [f32; 6] {
        [
            self.sx.get() as f32,
            self.ky.get() as f32,
            self.kx.get() as f32,
            self.sy.get() as f32,
            self.tx.to_pt() as f32,
            self.ty.to_pt() as f32,
        ]
    }
}
