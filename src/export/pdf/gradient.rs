use std::any::Any;

use pdf_writer::{Ref, PdfWriter, types::ShadingType};

use crate::geom::{GradientStop, Color, RgbaColor, Gradient, Ratio};

use super::{PdfContext, RefExt};

pub fn write_gradients(ctx: &mut PdfContext) {
    for pdf_gradient in ctx.gradient_map.items() {
        let shading_ref = ctx.alloc.bump();
        ctx.gradient_refs.push(shading_ref);

        match &pdf_gradient.gradient {
            Gradient::Linear(lg) => {
                let function_ref = ctx.alloc.bump();

                gradient_stops_function(&mut ctx.alloc, &mut ctx.writer, function_ref, &lg.stops);

                let (start, end) = lg.axial_coords(pdf_gradient.size);
                let coords: Vec<f32> = vec![start.x, start.y, end.x, end.y]
                    .iter().map(|v| v.to_raw() as f32).collect();

                // TODO: set the coords according to the shape
                ctx.writer.shading(shading_ref)
                    .shading_type(ShadingType::Axial)
                    .coords(coords)
                    .extend([true, true])
                    .function(function_ref)
                    .color_space().device_rgb();
            },
        }
    }
}

/// Create a stitched function (7.10.3) for multiple gradient stops.
/// The function is a `1`-in `n`-out function mapping the domain `0.0-1.0`
/// to the range of colors in the given colorspace, where `n` is the number
/// of components of the colorspace.
/// 
/// Right now, only DeviceRGB colors are supported.
fn gradient_stops_function(alloc: &mut Ref, writer: &mut PdfWriter, reference: Ref, stops: &[GradientStop]) -> Option<()> {

    let mut mk_exp = |id: Ref, from: Color, to: Color| {
        let rgb_array = |color: Color| {
            let RgbaColor {r, g, b, a: _} = color.to_rgba();
            [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
        };

        writer.exponential_function(id)
            .domain([0.0, 1.0])
            .n(1.0)
            .c0(rgb_array(from))
            .c1(rgb_array(to));
    };

    if stops.is_empty() {
        return None;
    } else if stops.len() == 1 {
        // if we only have 1 stop, we can just return a
        // linear function.
        let color = stops[0].color;
        mk_exp(reference, color, color);

        return Some(());
    }

    let mut bounds = vec![];
    let mut encode = vec![];
    let mut functions = vec![];

    for window in stops.windows(2) {
        let (start, end) = (&window[0], &window[1]);

        let fn_ref = alloc.bump();

        functions.push(fn_ref);
        bounds.push(end.position.get() as f32);
        encode.extend([0.0, 1.0]);

        mk_exp(fn_ref, start.color, end.color);
    }
    bounds.pop();

    println!("stitching function: {reference:?}");
    println!("  domain: [0.0, 1.0]");
    println!("  bounds: {bounds:?}");
    println!("  encode: {encode:?}");
    println!("  functions: {functions:?}");

    writer.stitching_function(reference)
        .domain([0.0, 1.0])
        .bounds(bounds)
        .encode(encode)
        .functions(functions);

    Some(())
}
