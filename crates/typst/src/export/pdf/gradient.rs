use ecow::{eco_format, EcoString};
use pdf_writer::types::FunctionShadingType;
use pdf_writer::{types::ColorSpaceOperand, Name};
use pdf_writer::{Finish, Ref};

use super::color::{ColorSpaceExt, PaintEncode};
use super::page::{PageContext, Transforms};
use super::{AbsExt, PdfContext};
use crate::geom::{
    Abs, Color, ColorSpace, Gradient, Numeric, Quadrant, Ratio, Relative, Transform,
};

/// A unique-transform-aspect-ratio combination that will be encoded into the
/// PDF.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PdfGradient {
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

                let angle = Gradient::correct_aspect_ratio(linear.angle, aspect_ratio);
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

        shading_pattern.matrix(transform_to_array(transform));
    }
}

/// Writes an expotential or stitched function that expresses the gradient.
fn shading_function(ctx: &mut PdfContext, gradient: &Gradient) -> Ref {
    let function = ctx.alloc.bump();
    let mut functions = vec![];
    let mut bounds = vec![];
    let mut encode = vec![];

    // Create the individual gradient functions for each pair of stops.
    for window in gradient.stops_ref().windows(2) {
        let (first, second) = (window[0], window[1]);

        // Skip stops with the same position.
        if first.1.get() == second.1.get() {
            continue;
        }

        // If the color space is HSL or HSV, and we cross the 0°/360° boundary,
        // we need to create two separate stops.
        if gradient.space() == ColorSpace::Hsl || gradient.space() == ColorSpace::Hsv {
            let t1 = first.1.get() as f32;
            let t2 = second.1.get() as f32;
            let [h1, s1, x1, _] = first.0.to_space(gradient.space()).to_vec4();
            let [h2, s2, x2, _] = second.0.to_space(gradient.space()).to_vec4();

            // Compute the intermediary stop at 360°.
            if (h1 - h2).abs() > 180.0 {
                let h1 = if h1 < h2 { h1 + 360.0 } else { h1 };
                let h2 = if h2 < h1 { h2 + 360.0 } else { h2 };

                // We compute where the crossing happens between zero and one
                let t = (360.0 - h1) / (h2 - h1);
                // We then map it back to the original range.
                let t_prime = t * (t2 - t1) + t1;

                // If the crossing happens between the two stops,
                // we need to create an extra stop.
                if t_prime <= t2 && t_prime >= t1 {
                    bounds.push(t_prime);
                    bounds.push(t_prime);
                    bounds.push(t2);
                    encode.extend([0.0, 1.0]);
                    encode.extend([0.0, 1.0]);
                    encode.extend([0.0, 1.0]);

                    // These need to be individual function to encode 360.0 correctly.
                    let func1 = ctx.alloc.bump();
                    ctx.writer
                        .exponential_function(func1)
                        .range(gradient.space().range())
                        .c0(gradient.space().convert(first.0))
                        .c1([1.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t])
                        .domain([0.0, 1.0])
                        .n(1.0);

                    let func2 = ctx.alloc.bump();
                    ctx.writer
                        .exponential_function(func2)
                        .range(gradient.space().range())
                        .c0([1.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t])
                        .c1([0.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t])
                        .domain([0.0, 1.0])
                        .n(1.0);

                    let func3 = ctx.alloc.bump();
                    ctx.writer
                        .exponential_function(func3)
                        .range(gradient.space().range())
                        .c0([0.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t])
                        .c1(gradient.space().convert(second.0))
                        .domain([0.0, 1.0])
                        .n(1.0);

                    functions.push(func1);
                    functions.push(func2);
                    functions.push(func3);

                    continue;
                }
            }
        }

        bounds.push(second.1.get() as f32);
        functions.push(single_gradient(ctx, first.0, second.0, gradient.space()));
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

/// Writes an expontential function that expresses a single segment (between two
/// stops) of a gradient.
fn single_gradient(
    ctx: &mut PdfContext,
    first_color: Color,
    second_color: Color,
    color_space: ColorSpace,
) -> Ref {
    let reference = ctx.alloc.bump();

    ctx.writer
        .exponential_function(reference)
        .range(color_space.range())
        .c0(color_space.convert(first_color))
        .c1(color_space.convert(second_color))
        .domain([0.0, 1.0])
        .n(1.0);

    reference
}

impl PaintEncode for Gradient {
    fn set_as_fill(&self, ctx: &mut PageContext, transforms: Transforms) {
        ctx.reset_fill_color_space();

        let id = register_gradient(ctx, self, transforms);
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
    }

    fn set_as_stroke(&self, ctx: &mut PageContext, transforms: Transforms) {
        ctx.reset_stroke_color_space();

        let id = register_gradient(ctx, self, transforms);
        let name = Name(id.as_bytes());

        ctx.content.set_stroke_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_stroke_pattern(None, name);
    }
}

/// Deduplicates a gradient to a named PDF resource.
fn register_gradient(
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
        Relative::Self_ => transforms.size,
        Relative::Parent => transforms.container_size,
    };

    let (offset_x, offset_y) = match gradient.angle().quadrant() {
        Quadrant::First => (Abs::zero(), Abs::zero()),
        Quadrant::Second => (size.x, Abs::zero()),
        Quadrant::Third => (size.x, size.y),
        Quadrant::Fourth => (Abs::zero(), size.y),
    };

    let transform = match gradient.unwrap_relative(false) {
        Relative::Self_ => transforms.transform,
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
            .pre_concat(Transform::rotate(Gradient::correct_aspect_ratio(
                gradient.angle(),
                size.aspect_ratio(),
            ))),
        gradient: gradient.clone(),
    };

    let index = ctx.parent.gradient_map.insert(pdf_gradient);
    eco_format!("Gr{}", index)
}

/// Convert to an array of floats.
fn transform_to_array(ts: Transform) -> [f32; 6] {
    [
        ts.sx.get() as f32,
        ts.ky.get() as f32,
        ts.kx.get() as f32,
        ts.sy.get() as f32,
        ts.tx.to_f32(),
        ts.ty.to_f32(),
    ]
}
