use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use pdf_writer::types::FunctionShadingType;
use pdf_writer::writers::StreamShadingType;
use pdf_writer::{types::ColorSpaceOperand, Name};
use pdf_writer::{Filter, Finish, Ref};

use super::color::{ColorSpaceExt, PaintEncode, QuantizedColor};
use super::page::{PageContext, Transforms};
use super::{AbsExt, PdfContext};
use crate::export::pdf::deflate;
use crate::geom::{
    Abs, Angle, Color, ColorSpace, ConicGradient, Gradient, Numeric, Point, Quadrant,
    Ratio, Relative, Transform, WeightedColor,
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
    /// Whether the gradient is applied to text.
    pub on_text: bool,
}

/// Writes the actual gradients (shading patterns) to the PDF.
/// This is performed once after writing all pages.
pub fn write_gradients(ctx: &mut PdfContext) {
    for PdfGradient { transform, aspect_ratio, gradient, on_text } in
        ctx.gradient_map.items().cloned().collect::<Vec<_>>()
    {
        let shading = ctx.alloc.bump();
        ctx.gradient_refs.push(shading);

        let mut shading_pattern = match &gradient {
            Gradient::Linear(linear) => {
                let shading_function = shading_function(ctx, &gradient);
                let mut shading_pattern = ctx.pdf.shading_pattern(shading);
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
            Gradient::Radial(radial) => {
                let shading_function = shading_function(ctx, &gradient);
                let mut shading_pattern = ctx.pdf.shading_pattern(shading);
                let mut shading = shading_pattern.function_shading();
                shading.shading_type(FunctionShadingType::Radial);

                ctx.colors
                    .write(gradient.space(), shading.color_space(), &mut ctx.alloc);

                shading
                    .anti_alias(gradient.anti_alias())
                    .function(shading_function)
                    .coords([
                        radial.focal_center.x.get() as f32,
                        radial.focal_center.y.get() as f32,
                        radial.focal_radius.get() as f32,
                        radial.center.x.get() as f32,
                        radial.center.y.get() as f32,
                        radial.radius.get() as f32,
                    ])
                    .extend([true; 2]);

                shading.finish();

                shading_pattern
            }
            Gradient::Conic(conic) => {
                let vertices = compute_vertex_stream(conic, aspect_ratio, on_text);

                let stream_shading_id = ctx.alloc.bump();
                let mut stream_shading =
                    ctx.pdf.stream_shading(stream_shading_id, &vertices);

                ctx.colors.write(
                    conic.space,
                    stream_shading.color_space(),
                    &mut ctx.alloc,
                );

                let range = conic.space.range();
                stream_shading
                    .bits_per_coordinate(16)
                    .bits_per_component(16)
                    .bits_per_flag(8)
                    .shading_type(StreamShadingType::CoonsPatch)
                    .decode([
                        0.0, 1.0, 0.0, 1.0, range[0], range[1], range[2], range[3],
                        range[4], range[5],
                    ])
                    .anti_alias(gradient.anti_alias())
                    .filter(Filter::FlateDecode);

                stream_shading.finish();

                let mut shading_pattern = ctx.pdf.shading_pattern(shading);
                shading_pattern.shading_ref(stream_shading_id);
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
                    ctx.pdf
                        .exponential_function(func1)
                        .range(gradient.space().range())
                        .c0(gradient.space().convert(first.0))
                        .c1([1.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t])
                        .domain([0.0, 1.0])
                        .n(1.0);

                    let func2 = ctx.alloc.bump();
                    ctx.pdf
                        .exponential_function(func2)
                        .range(gradient.space().range())
                        .c0([1.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t])
                        .c1([0.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t])
                        .domain([0.0, 1.0])
                        .n(1.0);

                    let func3 = ctx.alloc.bump();
                    ctx.pdf
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
    ctx.pdf
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

    ctx.pdf
        .exponential_function(reference)
        .range(color_space.range())
        .c0(color_space.convert(first_color))
        .c1(color_space.convert(second_color))
        .domain([0.0, 1.0])
        .n(1.0);

    reference
}

impl PaintEncode for Gradient {
    fn set_as_fill(&self, ctx: &mut PageContext, on_text: bool, transforms: Transforms) {
        ctx.reset_fill_color_space();

        let id = register_gradient(ctx, self, on_text, transforms);
        let name = Name(id.as_bytes());

        ctx.content.set_fill_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_fill_pattern(None, name);
    }

    fn set_as_stroke(
        &self,
        ctx: &mut PageContext,
        on_text: bool,
        transforms: Transforms,
    ) {
        ctx.reset_stroke_color_space();

        let id = register_gradient(ctx, self, on_text, transforms);
        let name = Name(id.as_bytes());

        ctx.content.set_stroke_color_space(ColorSpaceOperand::Pattern);
        ctx.content.set_stroke_pattern(None, name);
    }
}

/// Deduplicates a gradient to a named PDF resource.
fn register_gradient(
    ctx: &mut PageContext,
    gradient: &Gradient,
    on_text: bool,
    mut transforms: Transforms,
) -> EcoString {
    // Edge cases for strokes.
    if transforms.size.x.is_zero() {
        transforms.size.x = Abs::pt(1.0);
    }

    if transforms.size.y.is_zero() {
        transforms.size.y = Abs::pt(1.0);
    }

    let size = match gradient.unwrap_relative(on_text) {
        Relative::Self_ => transforms.size,
        Relative::Parent => transforms.container_size,
    };

    // Correction for y-axis flipping on text.
    let angle = gradient.angle().unwrap_or_else(Angle::zero);
    let angle = if on_text { Angle::rad(TAU as f64) - angle } else { angle };

    let (offset_x, offset_y) = match gradient {
        Gradient::Conic(conic) => (
            -size.x * (1.0 - conic.center.x.get() / 2.0) / 2.0,
            -size.y * (1.0 - conic.center.y.get() / 2.0) / 2.0,
        ),
        _ => match angle.quadrant() {
            Quadrant::First => (Abs::zero(), Abs::zero()),
            Quadrant::Second => (size.x, Abs::zero()),
            Quadrant::Third => (size.x, size.y),
            Quadrant::Fourth => (Abs::zero(), size.y),
        },
    };

    let rotation = match gradient {
        Gradient::Conic(_) => Angle::zero(),
        _ => angle,
    };

    let transform = match gradient.unwrap_relative(on_text) {
        Relative::Self_ => transforms.transform,
        Relative::Parent => transforms.container_transform,
    };

    let scale_offset = match gradient {
        Gradient::Conic(_) => 4.0_f64,
        _ => 1.0,
    };

    let pdf_gradient = PdfGradient {
        aspect_ratio: size.aspect_ratio(),
        transform: transform
            .pre_concat(Transform::translate(
                offset_x * scale_offset,
                offset_y * scale_offset,
            ))
            .pre_concat(Transform::scale(
                Ratio::new(size.x.to_pt() * scale_offset),
                Ratio::new(size.y.to_pt() * scale_offset),
            ))
            .pre_concat(Transform::rotate(Gradient::correct_aspect_ratio(
                rotation,
                size.aspect_ratio(),
            ))),
        gradient: gradient.clone(),
        on_text,
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

/// Writes a single Coons Patch as defined in the PDF specification
/// to a binary vec.
///
/// Structure:
///  - flag: `u8`
///  - points: `[u16; 24]`
///  - colors: `[u16; 12]`
fn write_patch(
    target: &mut Vec<u8>,
    t: f32,
    t1: f32,
    c0: [u16; 3],
    c1: [u16; 3],
    angle: Angle,
    on_text: bool,
) {
    let mut theta = -TAU * t + angle.to_rad() as f32 + PI;
    let mut theta1 = -TAU * t1 + angle.to_rad() as f32 + PI;

    // Correction for y-axis flipping on text.
    if on_text {
        theta = (TAU - theta).rem_euclid(TAU);
        theta1 = (TAU - theta1).rem_euclid(TAU);
    }

    let (cp1, cp2) =
        control_point(Point::new(Abs::pt(0.5), Abs::pt(0.5)), 0.5, theta, theta1);

    // Push the flag
    target.push(0);

    let p1 =
        [u16::quantize(0.5, [0.0, 1.0]).to_be(), u16::quantize(0.5, [0.0, 1.0]).to_be()];

    let p2 = [
        u16::quantize(theta.cos(), [-1.0, 1.0]).to_be(),
        u16::quantize(theta.sin(), [-1.0, 1.0]).to_be(),
    ];

    let p3 = [
        u16::quantize(theta1.cos(), [-1.0, 1.0]).to_be(),
        u16::quantize(theta1.sin(), [-1.0, 1.0]).to_be(),
    ];

    let cp1 = [
        u16::quantize(cp1.x.to_f32(), [0.0, 1.0]).to_be(),
        u16::quantize(cp1.y.to_f32(), [0.0, 1.0]).to_be(),
    ];

    let cp2 = [
        u16::quantize(cp2.x.to_f32(), [0.0, 1.0]).to_be(),
        u16::quantize(cp2.y.to_f32(), [0.0, 1.0]).to_be(),
    ];

    // Push the points
    target.extend_from_slice(bytemuck::cast_slice(&[
        p1, p1, p2, p2, cp1, cp2, p3, p3, p1, p1, p1, p1,
    ]));

    let colors =
        [c0.map(u16::to_be), c0.map(u16::to_be), c1.map(u16::to_be), c1.map(u16::to_be)];

    // Push the colors.
    target.extend_from_slice(bytemuck::cast_slice(&colors));
}

fn control_point(c: Point, r: f32, angle_start: f32, angle_end: f32) -> (Point, Point) {
    let n = (TAU / (angle_end - angle_start)).abs();
    let f = ((angle_end - angle_start) / n).tan() * 4.0 / 3.0;

    let p1 = c + Point::new(
        Abs::pt((r * angle_start.cos() - f * r * angle_start.sin()) as f64),
        Abs::pt((r * angle_start.sin() + f * r * angle_start.cos()) as f64),
    );

    let p2 = c + Point::new(
        Abs::pt((r * angle_end.cos() + f * r * angle_end.sin()) as f64),
        Abs::pt((r * angle_end.sin() - f * r * angle_end.cos()) as f64),
    );

    (p1, p2)
}

#[comemo::memoize]
fn compute_vertex_stream(
    conic: &ConicGradient,
    aspect_ratio: Ratio,
    on_text: bool,
) -> Arc<Vec<u8>> {
    // Generated vertices for the Coons patches
    let mut vertices = Vec::new();

    // Correct the gradient's angle
    let angle = Gradient::correct_aspect_ratio(conic.angle, aspect_ratio);

    // We want to generate a vertex based on some conditions, either:
    // - At the boundary of a stop
    // - At the boundary of a quadrant
    // - When we cross the boundary of a hue turn (for HSV and HSL only)
    for window in conic.stops.windows(2) {
        let ((c0, t0), (c1, t1)) = (window[0], window[1]);

        // Skip stops with the same position
        if t0 == t1 {
            continue;
        }

        // If the angle between the two stops is greater than 90 degrees, we need to
        // generate a vertex at the boundary of the quadrant.
        // However, we add more stops in-between to make the gradient smoother, so we
        // need to generate a vertex at least every 5 degrees.
        // If the colors are the same, we do it every quadrant only.
        let slope = 1.0 / (t1.get() - t0.get());
        let mut t_x = t0.get();
        let dt = (t1.get() - t0.get()).min(0.25);
        while t_x < t1.get() {
            let t_next = (t_x + dt).min(t1.get());

            let t1 = slope * (t_x - t0.get());
            let t2 = slope * (t_next - t0.get());

            // We don't use `Gradient::sample` to avoid issues with sharp gradients.
            let c = Color::mix_iter(
                [WeightedColor::new(c0, 1.0 - t1), WeightedColor::new(c1, t1)],
                conic.space,
            )
            .unwrap();

            let c_next = Color::mix_iter(
                [WeightedColor::new(c0, 1.0 - t2), WeightedColor::new(c1, t2)],
                conic.space,
            )
            .unwrap();

            // If the color space is HSL or HSV, and we cross the 0°/360° boundary,
            // we need to create two separate stops.
            if conic.space == ColorSpace::Hsl || conic.space == ColorSpace::Hsv {
                let [h1, s1, x1, _] = c.to_space(conic.space).to_vec4();
                let [h2, s2, x2, _] = c_next.to_space(conic.space).to_vec4();

                // Compute the intermediary stop at 360°.
                if (h1 - h2).abs() > 180.0 {
                    let h1 = if h1 < h2 { h1 + 360.0 } else { h1 };
                    let h2 = if h2 < h1 { h2 + 360.0 } else { h2 };

                    // We compute where the crossing happens between zero and one
                    let t = (360.0 - h1) / (h2 - h1);
                    // We then map it back to the original range.
                    let t_prime = t * (t_next as f32 - t_x as f32) + t_x as f32;

                    // If the crossing happens between the two stops,
                    // we need to create an extra stop.
                    if t_prime <= t_next as f32 && t_prime >= t_x as f32 {
                        let c0 = [1.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t];
                        let c1 = [0.0, s1 * (1.0 - t) + s2 * t, x1 * (1.0 - t) + x2 * t];
                        let c0 = c0.map(|c| u16::quantize(c, [0.0, 1.0]));
                        let c1 = c1.map(|c| u16::quantize(c, [0.0, 1.0]));

                        write_patch(
                            &mut vertices,
                            t_x as f32,
                            t_prime,
                            conic.space.convert(c),
                            c0,
                            angle,
                            on_text,
                        );

                        write_patch(
                            &mut vertices,
                            t_prime,
                            t_prime,
                            c0,
                            c1,
                            angle,
                            on_text,
                        );

                        write_patch(
                            &mut vertices,
                            t_prime,
                            t_next as f32,
                            c1,
                            conic.space.convert(c_next),
                            angle,
                            on_text,
                        );

                        t_x = t_next;
                        continue;
                    }
                }
            }

            write_patch(
                &mut vertices,
                t_x as f32,
                t_next as f32,
                conic.space.convert(c),
                conic.space.convert(c_next),
                angle,
                on_text,
            );

            t_x = t_next;
        }
    }

    Arc::new(deflate(&vertices))
}
