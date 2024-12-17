//! Convert paint types from typst to krilla.

use crate::krilla::{handle_frame, FrameContext, GlobalContext, Transforms};
use crate::util::{AbsExt, ColorExt, FillRuleExt, LineCapExt, LineJoinExt, TransformExt};
use krilla::geom::NormalizedF32;
use krilla::paint::SpreadMethod;
use krilla::surface::Surface;
use typst_library::diag::SourceResult;
use typst_library::layout::{Abs, Angle, Quadrant, Ratio, Transform};
use typst_library::visualize::{
    Color, ColorSpace, DashPattern, FillRule, FixedStroke, Gradient, Paint, RatioOrAngle,
    RelativeTo, Tiling, WeightedColor,
};
use typst_utils::Numeric;

pub(crate) fn convert_fill(
    gc: &mut GlobalContext,
    paint_: &Paint,
    fill_rule_: FillRule,
    on_text: bool,
    surface: &mut Surface,
    transforms: Transforms,
) -> SourceResult<krilla::path::Fill> {
    let (paint, opacity) = convert_paint(gc, paint_, on_text, surface, transforms)?;

    Ok(krilla::path::Fill {
        paint,
        rule: fill_rule_.to_krilla(),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
    })
}

pub(crate) fn convert_stroke(
    fc: &mut GlobalContext,
    stroke: &FixedStroke,
    on_text: bool,
    surface: &mut Surface,
    transforms: Transforms,
) -> SourceResult<krilla::path::Stroke> {
    let (paint, opacity) = convert_paint(fc, &stroke.paint, on_text, surface, transforms)?;

    Ok(krilla::path::Stroke {
        paint,
        width: stroke.thickness.to_f32(),
        miter_limit: stroke.miter_limit.get() as f32,
        line_join: stroke.join.to_krilla(),
        line_cap: stroke.cap.to_krilla(),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
        dash: stroke.dash.as_ref().map(|d| convert_dash(d)),
    })
}

fn convert_dash(dash: &DashPattern<Abs, Abs>) -> krilla::path::StrokeDash {
    krilla::path::StrokeDash {
        array: dash.array.iter().map(|e| e.to_f32()).collect(),
        offset: dash.phase.to_f32(),
    }
}

fn convert_paint(
    gc: &mut GlobalContext,
    paint: &Paint,
    on_text: bool,
    surface: &mut Surface,
    transforms: Transforms,
) -> SourceResult<(krilla::paint::Paint, u8)> {
    match paint {
        Paint::Solid(c) => Ok(convert_solid(c)),
        Paint::Gradient(g) => Ok(convert_gradient(g, on_text, transforms)),
        Paint::Tiling(p) => convert_pattern(gc, p, on_text, surface, transforms),
    }
}

fn convert_solid(color: &Color) -> (krilla::paint::Paint, u8) {
    match color.space() {
        ColorSpace::D65Gray => {
            let components = color.to_vec4_u8();
            (krilla::color::luma::Color::new(components[0]).into(), components[3])
        }
        ColorSpace::Cmyk => {
            let components = color.to_vec4_u8();
            (
                krilla::color::cmyk::Color::new(
                    components[0],
                    components[1],
                    components[2],
                    components[3],
                )
                    .into(),
                // Typst doesn't support alpha on CMYK colors.
                255,
            )
        }
        // Convert all remaining colors into RGB
        _ => {
            let (c, a) = color.to_krilla_rgb();
            (c.into(), a)
        }
    }
}

fn convert_pattern(
    gc: &mut GlobalContext,
    pattern: &Tiling,
    on_text: bool,
    surface: &mut Surface,
    mut transforms: Transforms,
) -> SourceResult<(krilla::paint::Paint, u8)> {
    // Edge cases for strokes.
    if transforms.size.x.is_zero() {
        transforms.size.x = Abs::pt(1.0);
    }

    if transforms.size.y.is_zero() {
        transforms.size.y = Abs::pt(1.0);
    }

    let transform = match pattern.unwrap_relative(on_text) {
        RelativeTo::Self_ => Transform::identity(),
        RelativeTo::Parent => transforms
            .transform_chain_
            .invert()
            .unwrap()
            .pre_concat(transforms.container_transform_chain),
    }
    .to_krilla();

    let mut stream_builder = surface.stream_builder();
    let mut surface = stream_builder.surface();
    let mut fc = FrameContext::new(pattern.frame().size());
    handle_frame(&mut fc, pattern.frame(), None, &mut surface, gc)?;
    surface.finish();
    let stream = stream_builder.finish();
    let pattern = krilla::paint::Pattern {
        stream,
        transform,
        width: (pattern.size().x + pattern.spacing().x).to_pt() as _,
        height: (pattern.size().y + pattern.spacing().y).to_pt() as _,
    };

    Ok((pattern.into(), 255))
}

fn convert_gradient(
    gradient: &Gradient,
    on_text: bool,
    mut transforms: Transforms,
) -> (krilla::paint::Paint, u8) {
    // Edge cases for strokes.
    if transforms.size.x.is_zero() {
        transforms.size.x = Abs::pt(1.0);
    }

    if transforms.size.y.is_zero() {
        transforms.size.y = Abs::pt(1.0);
    }
    let size = match gradient.unwrap_relative(on_text) {
        RelativeTo::Self_ => transforms.size,
        RelativeTo::Parent => transforms.container_size,
    };
    let rotation = gradient.angle().unwrap_or_else(Angle::zero);

    let transform = match gradient.unwrap_relative(on_text) {
        RelativeTo::Self_ => transforms.transform_chain_,
        RelativeTo::Parent => transforms.container_transform_chain,
    };

    let angle = rotation;

    let mut stops: Vec<krilla::paint::Stop<krilla::color::rgb::Color>> = vec![];

    let mut add_single = |color: &Color, offset: Ratio| {
        let (color, opacity) = color.to_krilla_rgb();
        let opacity = NormalizedF32::new((opacity as f32) / 255.0).unwrap();
        let offset = NormalizedF32::new(offset.get() as f32).unwrap();
        let stop = krilla::paint::Stop { offset, color, opacity };
        stops.push(stop);
    };

    match &gradient {
        Gradient::Linear(linear) => {
            let actual_transform =
                transforms.transform_chain_.invert().unwrap().pre_concat(transform);

            if let Some((c, t)) = linear.stops.first() {
                add_single(c, *t);
            }

            // Create the individual gradient functions for each pair of stops.
            for window in linear.stops.windows(2) {
                let (first, second) = (window[0], window[1]);

                // If we have a hue index or are using Oklab, we will create several
                // stops in-between to make the gradient smoother without interpolation
                // issues with native color spaces.
                if gradient.space().hue_index().is_some() {
                    for i in 0..=32 {
                        let t = i as f64 / 32.0;
                        let real_t =
                            Ratio::new(first.1.get() * (1.0 - t) + second.1.get() * t);

                        let c = gradient.sample(RatioOrAngle::Ratio(real_t));
                        add_single(&c, real_t);
                    }
                }

                add_single(&second.0, second.1);
            }

            let (mut sin, mut cos) = (angle.sin(), angle.cos());

            // Scale to edges of unit square.
            let factor = cos.abs() + sin.abs();
            sin *= factor;
            cos *= factor;

            let (x1, y1, x2, y2): (f32, f32, f32, f32) = match angle.quadrant() {
                Quadrant::First => (0.0, 0.0, cos as f32, sin as f32),
                Quadrant::Second => (1.0, 0.0, cos as f32 + 1.0, sin as f32),
                Quadrant::Third => (1.0, 1.0, cos as f32 + 1.0, sin as f32 + 1.0),
                Quadrant::Fourth => (0.0, 1.0, cos as f32, sin as f32 + 1.0),
            };

            let linear = krilla::paint::LinearGradient {
                x1,
                y1,
                x2,
                y2,
                transform: actual_transform.to_krilla().pre_concat(
                    krilla::geom::Transform::from_scale(size.x.to_f32(), size.y.to_f32()),
                ),
                spread_method: SpreadMethod::Pad,
                stops: stops.into(),
                anti_alias: gradient.anti_alias(),
            };

            (linear.into(), 255)
        }
        Gradient::Radial(radial) => {
            let actual_transform =
                transforms.transform_chain_.invert().unwrap().pre_concat(transform);

            if let Some((c, t)) = radial.stops.first() {
                add_single(c, *t);
            }

            // Create the individual gradient functions for each pair of stops.
            for window in radial.stops.windows(2) {
                let (first, second) = (window[0], window[1]);

                // If we have a hue index or are using Oklab, we will create several
                // stops in-between to make the gradient smoother without interpolation
                // issues with native color spaces.
                if gradient.space().hue_index().is_some() {
                    for i in 0..=32 {
                        let t = i as f64 / 32.0;
                        let real_t =
                            Ratio::new(first.1.get() * (1.0 - t) + second.1.get() * t);

                        let c = gradient.sample(RatioOrAngle::Ratio(real_t));
                        add_single(&c, real_t);
                    }
                }

                add_single(&second.0, second.1);
            }

            let radial = krilla::paint::RadialGradient {
                fx: radial.focal_center.x.get() as f32,
                fy: radial.focal_center.y.get() as f32,
                fr: radial.focal_radius.get() as f32,
                cx: radial.center.x.get() as f32,
                cy: radial.center.y.get() as f32,
                cr: radial.radius.get() as f32,
                transform: actual_transform.to_krilla().pre_concat(
                    krilla::geom::Transform::from_scale(size.x.to_f32(), size.y.to_f32()),
                ),
                spread_method: SpreadMethod::Pad,
                stops: stops.into(),
                anti_alias: gradient.anti_alias(),
            };

            (radial.into(), 255)
        }
        Gradient::Conic(conic) => {
            // Correct the gradient's angle
            let cx = size.x.to_f32() * conic.center.x.get() as f32;
            let cy = size.y.to_f32() * conic.center.y.get() as f32;
            let actual_transform = transforms
                .transform_chain_
                .invert()
                .unwrap()
                .pre_concat(transform)
                .pre_concat(Transform::rotate_at(
                    angle,
                    Abs::pt(cx as f64),
                    Abs::pt(cy as f64),
                ))
                .pre_concat(Transform::scale_at(
                    -Ratio::one(),
                    Ratio::one(),
                    Abs::pt(cx as f64),
                    Abs::pt(cy as f64),
                ));

            if let Some((c, t)) = conic.stops.first() {
                add_single(c, *t);
            }

            for window in conic.stops.windows(2) {
                let ((c0, t0), (c1, t1)) = (window[0], window[1]);

                // Precision:
                // - On an even color, insert a stop every 90deg
                // - For a hue-based color space, insert 200 stops minimum
                // - On any other, insert 20 stops minimum
                let max_dt = if c0 == c1 {
                    0.25
                } else if conic.space.hue_index().is_some() {
                    0.005
                } else {
                    0.05
                };

                let mut t_x = t0.get();
                let dt = (t1.get() - t0.get()).min(max_dt);

                // Special casing for sharp gradients.
                if t0 == t1 {
                    add_single(&c1, t1);
                    continue;
                }

                while t_x < t1.get() {
                    let t_next = (t_x + dt).min(t1.get());

                    // The current progress in the current window.
                    let t = |t| (t - t0.get()) / (t1.get() - t0.get());

                    let c_next = Color::mix_iter(
                        [
                            WeightedColor::new(c0, 1.0 - t(t_next)),
                            WeightedColor::new(c1, t(t_next)),
                        ],
                        conic.space,
                    )
                    .unwrap();

                    add_single(&c_next, Ratio::new(t_next));
                    t_x = t_next;
                }

                add_single(&c1, t1);
            }

            let sweep = krilla::paint::SweepGradient {
                cx,
                cy,
                start_angle: 0.0,
                end_angle: 360.0,
                transform: actual_transform.to_krilla(),
                spread_method: SpreadMethod::Pad,
                stops: stops.into(),
                anti_alias: gradient.anti_alias(),
            };

            (sweep.into(), 255)
        }
    }
}
