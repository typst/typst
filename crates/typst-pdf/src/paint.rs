//! Convert paint types from typst to krilla.

use crate::krilla::{process_frame, FrameContext, GlobalContext, Transforms};
use crate::primitive::{AbsExt, FillRuleExt, LineCapExt, LineJoinExt, TransformExt};
use krilla::geom::NormalizedF32;
use krilla::page::{NumberingStyle, PageLabel};
use krilla::paint::SpreadMethod;
use krilla::surface::Surface;
use std::num::NonZeroUsize;
use typst_library::diag::SourceResult;
use typst_library::layout::{Abs, Angle, Quadrant, Ratio, Transform};
use typst_library::model::Numbering;
use typst_library::visualize::{
    Color, ColorSpace, DashPattern, FillRule, FixedStroke, Gradient, Paint, Pattern,
    RatioOrAngle, RelativeTo, WeightedColor,
};
use typst_utils::Numeric;

pub(crate) fn fill(
    gc: &mut GlobalContext,
    paint_: &Paint,
    fill_rule_: FillRule,
    on_text: bool,
    surface: &mut Surface,
    transforms: Transforms,
) -> SourceResult<krilla::path::Fill> {
    let (paint, opacity) = paint(gc, paint_, on_text, surface, transforms)?;

    Ok(krilla::path::Fill {
        paint,
        rule: fill_rule_.as_krilla(),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
    })
}

pub(crate) fn stroke(
    fc: &mut GlobalContext,
    stroke: &FixedStroke,
    on_text: bool,
    surface: &mut Surface,
    transforms: Transforms,
) -> SourceResult<krilla::path::Stroke> {
    let (paint, opacity) = paint(fc, &stroke.paint, on_text, surface, transforms)?;

    Ok(krilla::path::Stroke {
        paint,
        width: stroke.thickness.to_f32(),
        miter_limit: stroke.miter_limit.get() as f32,
        line_join: stroke.join.as_krilla(),
        line_cap: stroke.cap.as_krilla(),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
        dash: stroke.dash.as_ref().map(|d| dash(d)),
    })
}

fn dash(dash: &DashPattern<Abs, Abs>) -> krilla::path::StrokeDash {
    krilla::path::StrokeDash {
        array: dash.array.iter().map(|e| e.to_f32()).collect(),
        offset: dash.phase.to_f32(),
    }
}

fn convert_color(color: &Color) -> (krilla::color::rgb::Color, u8) {
    let components = color.to_space(ColorSpace::Srgb).to_vec4_u8();
    (
        krilla::color::rgb::Color::new(components[0], components[1], components[2])
            .into(),
        components[3],
    )
}

fn paint(
    gc: &mut GlobalContext,
    paint: &Paint,
    on_text: bool,
    surface: &mut Surface,
    transforms: Transforms,
) -> SourceResult<(krilla::paint::Paint, u8)> {
    match paint {
        Paint::Solid(c) => {
            let (c, alpha) = convert_color(c);
            Ok((c.into(), alpha))
        }
        Paint::Gradient(g) => Ok(convert_gradient(g, on_text, transforms)),
        Paint::Pattern(p) => convert_pattern(gc, p, on_text, surface, transforms),
    }
}

pub(crate) trait PageLabelExt {
    fn generate(numbering: &Numbering, number: usize) -> Option<PageLabel>;
    fn arabic(number: usize) -> PageLabel;
}

impl PageLabelExt for PageLabel {
    /// Create a new `PageLabel` from a `Numbering` applied to a page
    /// number.
    fn generate(numbering: &Numbering, number: usize) -> Option<PageLabel> {
        {
            let Numbering::Pattern(pat) = numbering else {
                return None;
            };

            let (prefix, kind) = pat.pieces.first()?;

            // If there is a suffix, we cannot use the common style optimisation,
            // since PDF does not provide a suffix field.
            let style = if pat.suffix.is_empty() {
                use krilla::page::NumberingStyle as Style;
                use typst_library::model::NumberingKind as Kind;
                match kind {
                    Kind::Arabic => Some(Style::Arabic),
                    Kind::LowerRoman => Some(Style::LowerRoman),
                    Kind::UpperRoman => Some(Style::UpperRoman),
                    Kind::LowerLatin if number <= 26 => Some(Style::LowerAlpha),
                    Kind::LowerLatin if number <= 26 => Some(Style::UpperAlpha),
                    _ => None,
                }
            } else {
                None
            };

            // Prefix and offset depend on the style: If it is supported by the PDF
            // spec, we use the given prefix and an offset. Otherwise, everything
            // goes into prefix.
            let prefix = if style.is_none() {
                Some(pat.apply(&[number]))
            } else {
                (!prefix.is_empty()).then(|| prefix.clone())
            };

            let offset = style.and(NonZeroUsize::new(number));
            Some(PageLabel::new(style, prefix.map(|s| s.to_string()), offset))
        }
    }

    /// Creates an arabic page label with the specified page number.
    /// For example, this will display page label `11` when given the page
    /// number 11.
    fn arabic(number: usize) -> PageLabel {
        PageLabel::new(Some(NumberingStyle::Arabic), None, NonZeroUsize::new(number))
    }
}

pub(crate) fn convert_pattern(
    gc: &mut GlobalContext,
    pattern: &Pattern,
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
    .as_krilla();

    let mut stream_builder = surface.stream_builder();
    let mut surface = stream_builder.surface();
    let mut fc = FrameContext::new(pattern.frame().size());
    process_frame(&mut fc, pattern.frame(), None, &mut surface, gc)?;
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
        let (color, opacity) = convert_color(color);
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
                transform: actual_transform.as_krilla().pre_concat(
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
                let mut last_c = first.0;
                if gradient.space().hue_index().is_some() {
                    for i in 0..=32 {
                        let t = i as f64 / 32.0;
                        let real_t =
                            Ratio::new(first.1.get() * (1.0 - t) + second.1.get() * t);

                        let c = gradient.sample(RatioOrAngle::Ratio(real_t));
                        add_single(&c, real_t);
                        last_c = c;
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
                transform: actual_transform.as_krilla().pre_concat(
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
                transform: actual_transform.as_krilla(),
                spread_method: SpreadMethod::Pad,
                stops: stops.into(),
                anti_alias: gradient.anti_alias(),
            };

            (sweep.into(), 255)
        }
    }
}
