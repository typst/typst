//! Convert paint types from typst to krilla.

use krilla::color::{self, cmyk, luma, rgb};
use krilla::num::NormalizedF32;
use krilla::paint::{
    Fill, LinearGradient, Pattern, RadialGradient, SpreadMethod, Stop, Stroke,
    StrokeDash, SweepGradient,
};
use krilla::surface::Surface;
use typst_library::diag::SourceResult;
use typst_library::layout::{Abs, Angle, Quadrant, Ratio, Size, Transform};
use typst_library::visualize::{
    Color, ColorSpace, DashPattern, FillRule, FixedStroke, Gradient, Paint, RatioOrAngle,
    RelativeTo, Tiling, WeightedColor,
};
use typst_utils::Numeric;

use crate::convert::{handle_frame, FrameContext, GlobalContext, State};
use crate::util::{AbsExt, FillRuleExt, LineCapExt, LineJoinExt, TransformExt};

pub(crate) fn convert_fill(
    gc: &mut GlobalContext,
    paint_: &Paint,
    fill_rule_: FillRule,
    on_text: bool,
    surface: &mut Surface,
    state: &State,
    size: Size,
) -> SourceResult<Fill> {
    let (paint, opacity) = convert_paint(gc, paint_, on_text, surface, state, size)?;

    Ok(Fill {
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
    state: &State,
    size: Size,
) -> SourceResult<Stroke> {
    let (paint, opacity) =
        convert_paint(fc, &stroke.paint, on_text, surface, state, size)?;

    Ok(Stroke {
        paint,
        width: stroke.thickness.to_f32(),
        miter_limit: stroke.miter_limit.get() as f32,
        line_join: stroke.join.to_krilla(),
        line_cap: stroke.cap.to_krilla(),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
        dash: stroke.dash.as_ref().map(convert_dash),
    })
}

fn convert_paint(
    gc: &mut GlobalContext,
    paint: &Paint,
    on_text: bool,
    surface: &mut Surface,
    state: &State,
    mut size: Size,
) -> SourceResult<(krilla::paint::Paint, u8)> {
    // Edge cases for strokes.
    if size.x.is_zero() {
        size.x = Abs::pt(1.0);
    }

    if size.y.is_zero() {
        size.y = Abs::pt(1.0);
    }

    match paint {
        Paint::Solid(c) => {
            let (c, a) = convert_solid(c);
            Ok((c.into(), a))
        }
        Paint::Gradient(g) => Ok(convert_gradient(g, on_text, state, size)),
        Paint::Tiling(p) => convert_pattern(gc, p, on_text, surface, state),
    }
}

fn convert_solid(color: &Color) -> (color::Color, u8) {
    match color.space() {
        ColorSpace::D65Gray => {
            let (c, a) = convert_luma(color);
            (c.into(), a)
        }
        ColorSpace::Cmyk => (convert_cmyk(color).into(), 255),
        // Convert all other colors in different colors spaces into RGB.
        _ => {
            let (c, a) = convert_rgb(color);
            (c.into(), a)
        }
    }
}

fn convert_cmyk(color: &Color) -> cmyk::Color {
    let components = color.to_space(ColorSpace::Cmyk).to_vec4_u8();

    cmyk::Color::new(components[0], components[1], components[2], components[3])
}

fn convert_rgb(color: &Color) -> (rgb::Color, u8) {
    let components = color.to_space(ColorSpace::Srgb).to_vec4_u8();
    (rgb::Color::new(components[0], components[1], components[2]), components[3])
}

fn convert_luma(color: &Color) -> (luma::Color, u8) {
    let components = color.to_space(ColorSpace::D65Gray).to_vec4_u8();
    (luma::Color::new(components[0]), components[3])
}

fn convert_pattern(
    gc: &mut GlobalContext,
    pattern: &Tiling,
    on_text: bool,
    surface: &mut Surface,
    state: &State,
) -> SourceResult<(krilla::paint::Paint, u8)> {
    let transform = correct_transform(state, pattern.unwrap_relative(on_text));

    let mut stream_builder = surface.stream_builder();
    let mut surface = stream_builder.surface();
    let mut fc = FrameContext::new(pattern.frame().size());
    handle_frame(&mut fc, pattern.frame(), None, &mut surface, gc)?;
    surface.finish();
    let stream = stream_builder.finish();
    let pattern = Pattern {
        stream,
        transform: transform.to_krilla(),
        width: (pattern.size().x + pattern.spacing().x).to_pt() as _,
        height: (pattern.size().y + pattern.spacing().y).to_pt() as _,
    };

    Ok((pattern.into(), 255))
}

fn convert_gradient(
    gradient: &Gradient,
    on_text: bool,
    state: &State,
    size: Size,
) -> (krilla::paint::Paint, u8) {
    let size = match gradient.unwrap_relative(on_text) {
        RelativeTo::Self_ => size,
        RelativeTo::Parent => state.container_size(),
    };

    let angle = gradient.angle().unwrap_or_else(Angle::zero);
    let base_transform = correct_transform(state, gradient.unwrap_relative(on_text));
    let stops = convert_gradient_stops(gradient);
    match &gradient {
        Gradient::Linear(_) => {
            let (x1, y1, x2, y2) = {
                let (mut sin, mut cos) = (angle.sin(), angle.cos());

                // Scale to edges of unit square.
                let factor = cos.abs() + sin.abs();
                sin *= factor;
                cos *= factor;

                match angle.quadrant() {
                    Quadrant::First => (0.0, 0.0, cos as f32, sin as f32),
                    Quadrant::Second => (1.0, 0.0, cos as f32 + 1.0, sin as f32),
                    Quadrant::Third => (1.0, 1.0, cos as f32 + 1.0, sin as f32 + 1.0),
                    Quadrant::Fourth => (0.0, 1.0, cos as f32, sin as f32 + 1.0),
                }
            };

            let linear = LinearGradient {
                x1,
                y1,
                x2,
                y2,
                // x and y coordinates are normalized, so need to scale by the size.
                transform: base_transform
                    .pre_concat(Transform::scale(
                        Ratio::new(size.x.to_f32() as f64),
                        Ratio::new(size.y.to_f32() as f64),
                    ))
                    .to_krilla(),
                spread_method: SpreadMethod::Pad,
                stops: stops.into(),
                anti_alias: gradient.anti_alias(),
            };

            (linear.into(), 255)
        }
        Gradient::Radial(radial) => {
            let radial = RadialGradient {
                fx: radial.focal_center.x.get() as f32,
                fy: radial.focal_center.y.get() as f32,
                fr: radial.focal_radius.get() as f32,
                cx: radial.center.x.get() as f32,
                cy: radial.center.y.get() as f32,
                cr: radial.radius.get() as f32,
                transform: base_transform
                    .pre_concat(Transform::scale(
                        Ratio::new(size.x.to_f32() as f64),
                        Ratio::new(size.y.to_f32() as f64),
                    ))
                    .to_krilla(),
                spread_method: SpreadMethod::Pad,
                stops: stops.into(),
                anti_alias: gradient.anti_alias(),
            };

            (radial.into(), 255)
        }
        Gradient::Conic(conic) => {
            // Correct the gradient's angle.
            let cx = size.x.to_f32() * conic.center.x.get() as f32;
            let cy = size.y.to_f32() * conic.center.y.get() as f32;
            let actual_transform = base_transform
                // Adjust for the angle.
                .pre_concat(Transform::rotate_at(
                    angle,
                    Abs::pt(cx as f64),
                    Abs::pt(cy as f64),
                ))
                // Default start point in krilla and typst are at the opposite side, so we need
                // to flip it horizontally.
                .pre_concat(Transform::scale_at(
                    -Ratio::one(),
                    Ratio::one(),
                    Abs::pt(cx as f64),
                    Abs::pt(cy as f64),
                ));

            let sweep = SweepGradient {
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

fn convert_gradient_stops(gradient: &Gradient) -> Vec<Stop> {
    let mut stops = vec![];

    let use_cmyk = gradient.stops().iter().all(|s| s.color.space() == ColorSpace::Cmyk);

    let mut add_single = |color: &Color, offset: Ratio| {
        let (color, opacity) = if use_cmyk {
            (convert_cmyk(color).into(), 255)
        } else {
            let (c, a) = convert_rgb(color);
            (c.into(), a)
        };

        let opacity = NormalizedF32::new((opacity as f32) / 255.0).unwrap();
        let offset = NormalizedF32::new(offset.get() as f32).unwrap();
        let stop = Stop { offset, color, opacity };
        stops.push(stop);
    };

    // Convert stops.
    match &gradient {
        Gradient::Linear(_) | Gradient::Radial(_) => {
            if let Some(s) = gradient.stops().first() {
                add_single(&s.color, s.offset.unwrap());
            }

            // Create the individual gradient functions for each pair of stops.
            for window in gradient.stops().windows(2) {
                let (first, second) = (window[0], window[1]);

                // If we have a hue index or are using Oklab, we will create several
                // stops in-between to make the gradient smoother without interpolation
                // issues with native color spaces.
                if gradient.space().hue_index().is_some() {
                    for i in 0..=32 {
                        let t = i as f64 / 32.0;
                        let real_t = Ratio::new(
                            first.offset.unwrap().get() * (1.0 - t)
                                + second.offset.unwrap().get() * t,
                        );

                        let c = gradient.sample(RatioOrAngle::Ratio(real_t));
                        add_single(&c, real_t);
                    }
                }

                add_single(&second.color, second.offset.unwrap());
            }
        }
        Gradient::Conic(conic) => {
            if let Some((c, t)) = conic.stops.first() {
                add_single(c, *t);
            }

            for window in conic.stops.windows(2) {
                let ((c0, t0), (c1, t1)) = (window[0], window[1]);

                // Precision:
                // - On an even color, insert a stop every 90deg.
                // - For a hue-based color space, insert 200 stops minimum.
                // - On any other, insert 20 stops minimum.
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
        }
    }

    stops
}

fn convert_dash(dash: &DashPattern<Abs, Abs>) -> StrokeDash {
    StrokeDash {
        array: dash.array.iter().map(|e| e.to_f32()).collect(),
        offset: dash.phase.to_f32(),
    }
}

fn correct_transform(state: &State, relative: RelativeTo) -> Transform {
    // In krilla, if we have a shape with a transform and a complex paint,
    // then the paint will inherit the transform of the shape.
    match relative {
        // Because of the above, we don't need to apply an additional transform here.
        RelativeTo::Self_ => Transform::identity(),
        // Because of the above, we need to first reverse the transform that will be
        // applied from the shape, and then re-apply the transform that is used for
        // the next parent container.
        RelativeTo::Parent => state
            .transform()
            .invert()
            .unwrap()
            .pre_concat(state.container_transform()),
    }
}
