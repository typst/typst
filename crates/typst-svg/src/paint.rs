use std::f64::consts::TAU;

use typst_library::foundations::Repr;
use typst_library::layout::{
    Abs, Angle, Axes, Frame, Point, Quadrant, Ratio, Size, Transform,
};
use typst_library::visualize::{Color, FillRule, Gradient, Paint, RatioOrAngle, Tiling};
use xmlwriter::XmlWriter;

use crate::path::SvgPathBuilder;
use crate::write::{SvgDisplay, SvgElem, SvgIdRef, SvgTransform, SvgUrl, SvgWrite};
use crate::{DedupId, SVGRenderer, State};

/// The number of segments in a conic gradient.
/// This is a heuristic value that seems to work well.
/// Smaller values could be interesting for optimization.
const NUM_CONIC_SEGMENTS: usize = 360;

impl SVGRenderer<'_> {
    /// Render a frame to a string.
    pub(super) fn render_tiling_frame(&mut self, state: &State, frame: &Frame) -> String {
        let mut xml = XmlWriter::new(xmlwriter::Options::default());
        let mut svg = SvgElem::new(&mut xml, "g");
        self.render_frame(&mut svg, state, frame);
        drop(svg);
        xml.end_document()
    }

    /// Write a fill attribute.
    pub(super) fn write_fill(
        &mut self,
        svg: &mut SvgElem,
        fill: &Paint,
        fill_rule: FillRule,
        aspect_ratio: Ratio,
        ts: Transform,
    ) {
        match fill {
            Paint::Solid(color) => {
                svg.attr("fill", color);
            }
            Paint::Gradient(gradient) => {
                let id = self.push_gradient(gradient, aspect_ratio, ts);
                svg.attr("fill", SvgUrl(id));
            }
            Paint::Tiling(tiling) => {
                let id = self.push_tiling(tiling, ts);
                svg.attr("fill", SvgUrl(id));
            }
        }
        match fill_rule {
            FillRule::NonZero => svg.attr("fill-rule", "nonzero"),
            FillRule::EvenOdd => svg.attr("fill-rule", "evenodd"),
        };
    }

    /// Pushes a gradient to the list of gradients to write SVG file.
    ///
    /// If the gradient is already present, returns the id of the existing
    /// gradient. Otherwise, inserts the gradient and returns the id of the
    /// inserted gradient. If the transform of the gradient is the identify
    /// matrix, the returned ID will be the ID of the "source" gradient,
    /// this is a file size optimization.
    pub(super) fn push_gradient(
        &mut self,
        gradient: &Gradient,
        aspect_ratio: Ratio,
        ts: Transform,
    ) -> DedupId {
        let gradient_id = self
            .gradients
            .insert_with((gradient, aspect_ratio), || (gradient.clone(), aspect_ratio));

        if ts.is_identity() {
            return gradient_id;
        }

        self.gradient_refs.insert_with(&(gradient_id, ts), || GradientRef {
            id: gradient_id,
            kind: gradient.into(),
            transform: ts,
        })
    }

    pub(super) fn push_tiling(&mut self, tiling: &Tiling, ts: Transform) -> DedupId {
        let tiling_size = tiling.size() + tiling.spacing();
        // Unfortunately due to a limitation of `xmlwriter`, we need to
        // render the frame twice: once to allocate all of the resources
        // that it needs and once to actually render it.
        let rendered = self.render_tiling_frame(&State::new(tiling_size), tiling.frame());

        // Use the rendered SVG as a key, since the `Tiling` itself includes
        // `Location`s which aren't stable.
        let tiling_id = self.tilings.insert_with(rendered, || tiling.clone());

        if ts.is_identity() {
            return tiling_id;
        }

        let tiling_ref = TilingRef { id: tiling_id, transform: ts };
        self.tiling_refs.insert_with(tiling_ref, || tiling_ref)
    }

    /// Write the raw gradients (without transform) to the SVG file.
    pub(super) fn write_gradients(&mut self, svg: &mut SvgElem) {
        if self.gradients.is_empty() {
            return;
        }

        let mut defs = svg.elem("defs");
        for (id, (gradient, ratio)) in self.gradients.iter() {
            let mut svg = match &gradient {
                Gradient::Linear(linear) => {
                    let mut gradient = defs.elem("linearGradient");
                    gradient.attr("id", id);
                    gradient.attr("spreadMethod", "pad");
                    gradient.attr("gradientUnits", "userSpaceOnUse");

                    let angle = Gradient::correct_aspect_ratio(linear.angle, *ratio);
                    let (sin, cos) = (angle.sin(), angle.cos());
                    let length = sin.abs() + cos.abs();
                    let (x1, y1, x2, y2) = match angle.quadrant() {
                        Quadrant::First => (0.0, 0.0, cos * length, sin * length),
                        Quadrant::Second => (1.0, 0.0, cos * length + 1.0, sin * length),
                        Quadrant::Third => {
                            (1.0, 1.0, cos * length + 1.0, sin * length + 1.0)
                        }
                        Quadrant::Fourth => (0.0, 1.0, cos * length, sin * length + 1.0),
                    };

                    gradient.attr("x1", x1);
                    gradient.attr("y1", y1);
                    gradient.attr("x2", x2);
                    gradient.attr("y2", y2);

                    gradient
                }
                Gradient::Radial(radial) => {
                    let mut gradient = defs.elem("radialGradient");
                    gradient.attr("id", id);
                    gradient.attr("spreadMethod", "pad");
                    gradient.attr("gradientUnits", "userSpaceOnUse");
                    gradient.attr("cx", radial.center.x.get());
                    gradient.attr("cy", radial.center.y.get());
                    gradient.attr("r", radial.radius.get());
                    gradient.attr("fx", radial.focal_center.x.get());
                    gradient.attr("fy", radial.focal_center.y.get());
                    gradient.attr("fr", radial.focal_radius.get());
                    gradient
                }
                Gradient::Conic(conic) => {
                    let mut pattern = defs.elem("pattern");
                    pattern.attr("id", id);
                    pattern.attr("viewBox", "0 0 1 1");
                    pattern.attr("preserveAspectRatio", "none");
                    pattern.attr("patternUnits", "userSpaceOnUse");
                    pattern.attr("width", "2");
                    pattern.attr("height", "2");
                    // TODO: Refactor this.
                    pattern.attr("x", "-0.5");
                    pattern.attr("y", "-0.5");

                    // The rotation angle, negated to match rotation in PNG.
                    let angle = -Gradient::correct_aspect_ratio(conic.angle, *ratio);
                    let center = conic.center;

                    // We build an arg segment for each segment of a circle.
                    let dtheta = Angle::rad(TAU / NUM_CONIC_SEGMENTS as f64);
                    for i in 0..NUM_CONIC_SEGMENTS {
                        let theta1 = angle + (dtheta * i as f64);
                        let theta2 = angle + (dtheta * (i + 1) as f64);

                        // Create the path for the segment.
                        let mut builder = SvgPathBuilder::empty();
                        builder.move_to(correct_tiling_pos(center.x, center.y));

                        builder.line_to(correct_tiling_pos(
                            Ratio::new(-2.0 * theta1.cos()) + center.x,
                            Ratio::new(2.0 * theta1.sin()) + center.y,
                        ));
                        builder.arc(
                            Size::splat(Abs::pt(1.0)),
                            Angle::zero(),
                            0,
                            1,
                            correct_tiling_pos(
                                Ratio::new(-2.0 * theta2.cos()) + center.x,
                                Ratio::new(2.0 * theta2.sin()) + center.y,
                            ),
                        );
                        builder.close();

                        let t1 = (i as f64) / NUM_CONIC_SEGMENTS as f64;
                        let t2 = (i + 1) as f64 / NUM_CONIC_SEGMENTS as f64;
                        let subgradient = SVGSubGradient {
                            center: conic.center,
                            t0: theta1,
                            t1: theta2,
                            c0: gradient.sample(RatioOrAngle::Ratio(Ratio::new(t1))),
                            c1: gradient.sample(RatioOrAngle::Ratio(Ratio::new(t2))),
                        };
                        let id = self
                            .conic_subgradients
                            .insert_with(subgradient.clone(), || subgradient);

                        // Add the path to the pattern.
                        pattern
                            .elem("path")
                            .attr("d", builder.finsish())
                            .attr("fill", SvgUrl(id))
                            .attr("stroke", "none")
                            .attr("shape-rendering", "optimizeSpeed");
                    }

                    // We skip the default stop generation code.
                    continue;
                }
            };

            for window in gradient.stops_ref().windows(2) {
                let (start_c, start_t) = window[0];
                let (end_c, end_t) = window[1];

                svg.elem("stop")
                    .attr("offset", start_t.repr())
                    .attr("stop-color", start_c.to_hex());

                // Generate (256 / len) stops between the two stops.
                // This is a workaround for a bug in many readers:
                // They tend to just ignore the color space of the gradient.
                // The goal is to have smooth gradients but not to balloon the file size
                // too much if there are already a lot of stops as in most presets.
                let len = if gradient.anti_alias() {
                    (256 / gradient.stops_ref().len() as u32).max(2)
                } else {
                    2
                };

                for i in 1..(len - 1) {
                    let t0 = i as f64 / (len - 1) as f64;
                    let t = start_t + (end_t - start_t) * t0;
                    let c = gradient.sample(RatioOrAngle::Ratio(t));

                    svg.elem("stop")
                        .attr("offset", t.repr())
                        .attr("stop-color", c.to_hex());
                }

                svg.elem("stop")
                    .attr("offset", end_t.repr())
                    .attr("stop-color", end_c.to_hex());
            }
        }
    }

    /// Write the sub-gradients that are used for conic gradients.
    pub(super) fn write_subgradients(&mut self, svg: &mut SvgElem) {
        if self.conic_subgradients.is_empty() {
            return;
        }

        let mut defs = svg.elem("defs");
        for (id, gradient) in self.conic_subgradients.iter() {
            let x1 = 2.0 - gradient.t0.cos() + gradient.center.x.get();
            let y1 = gradient.t0.sin() + gradient.center.y.get();
            let x2 = 2.0 - gradient.t1.cos() + gradient.center.x.get();
            let y2 = gradient.t1.sin() + gradient.center.y.get();

            defs.elem("linearGradient")
                .attr("id", id)
                .attr("gradientUnits", "objectBoundingBox")
                .attr("x1", x1)
                .attr("y1", y1)
                .attr("x2", x2)
                .attr("y2", y2)
                .with(|svg| {
                    svg.elem("stop")
                        .attr("offset", "0%")
                        .attr("stop-color", gradient.c0.to_hex());

                    svg.elem("stop")
                        .attr("offset", "100%")
                        .attr("stop-color", gradient.c1.to_hex());
                });
        }
    }

    pub(super) fn write_gradient_refs(&mut self, svg: &mut SvgElem) {
        if self.gradient_refs.is_empty() {
            return;
        }

        let mut defs = svg.elem("defs");
        for (id, gradient_ref) in self.gradient_refs.iter() {
            let (elem_name, transform_name) = match gradient_ref.kind {
                GradientKind::Linear => ("linearGradient", "gradientTransform"),
                GradientKind::Radial => ("radialGradient", "gradientTransform"),
                GradientKind::Conic => ("pattern", "patternTransform"),
            };
            defs.elem(elem_name)
                .attr(transform_name, SvgTransform(gradient_ref.transform))
                .attr("id", id)
                // Writing the href attribute to the "reference" gradient.
                .attr("href", SvgIdRef(gradient_ref.id))
                // Also writing the xlink:href attribute for compatibility.
                .attr("xlink:href", SvgIdRef(gradient_ref.id));
        }
    }

    /// Write the raw tilings (without transform) to the SVG file.
    pub(super) fn write_tilings(&mut self, svg: &mut SvgElem) {
        if self.tilings.is_empty() {
            return;
        }

        let mut defs = svg.elem("defs");
        for (id, tiling) in
            self.tilings.iter().map(|(i, p)| (i, p.clone())).collect::<Vec<_>>()
        {
            let size = tiling.size() + tiling.spacing();
            defs.elem("pattern")
                .attr("id", id)
                .attr("width", size.x.to_pt())
                .attr("height", size.y.to_pt())
                .attr("patternUnits", "userSpaceOnUse")
                .attr_with("viewBox", |attr| {
                    attr.push_nums([0.0, 0.0, size.x.to_pt(), size.y.to_pt()])
                })
                .with(|pattern| {
                    // Render the frame.
                    let state = State::new(size);
                    self.render_frame(pattern, &state, tiling.frame());
                });
        }
    }

    /// Writes the references to the deduplicated tilings for each usage site.
    pub(super) fn write_tiling_refs(&mut self, svg: &mut SvgElem) {
        if self.tiling_refs.is_empty() {
            return;
        }

        let mut defs = svg.elem("defs");
        for (id, tiling_ref) in self.tiling_refs.iter() {
            defs.elem("pattern")
                .attr("patternTransform", SvgTransform(tiling_ref.transform))
                .attr("id", id)
                // Writing the href attribute to the "reference" pattern.
                .attr("href", SvgIdRef(tiling_ref.id))
                // Also writing the xlink:href attribute for compatibility.
                .attr("xlink:href", SvgIdRef(tiling_ref.id));
        }
    }
}

/// A reference to a deduplicated tiling, with a transform matrix.
///
/// Allows tilings to be reused across multiple invocations, simply by changing
/// the transform matrix.
#[derive(Copy, Clone, Hash)]
pub struct TilingRef {
    /// The ID of the deduplicated gradient
    id: DedupId,
    /// The transform matrix to apply to the tiling.
    transform: Transform,
}

/// A reference to a deduplicated gradient, with a transform matrix.
///
/// Allows gradients to be reused across multiple invocations,
/// simply by changing the transform matrix.
#[derive(Hash)]
pub struct GradientRef {
    /// The ID of the deduplicated gradient
    id: DedupId,
    /// The gradient kind (used to determine the SVG element to use)
    /// but without needing to clone the entire gradient.
    kind: GradientKind,
    /// The transform matrix to apply to the gradient.
    transform: Transform,
}

/// A subgradient for conic gradients.
#[derive(Clone, Hash)]
pub struct SVGSubGradient {
    /// The center point of the gradient.
    center: Axes<Ratio>,
    /// The start point of the subgradient.
    t0: Angle,
    /// The end point of the subgradient.
    t1: Angle,
    /// The color at the start point of the subgradient.
    c0: Color,
    /// The color at the end point of the subgradient.
    c1: Color,
}

/// The kind of linear gradient.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum GradientKind {
    /// A linear gradient.
    Linear,
    /// A radial gradient.
    Radial,
    /// A conic gradient.
    Conic,
}

impl From<&Gradient> for GradientKind {
    fn from(value: &Gradient) -> Self {
        match value {
            Gradient::Linear { .. } => GradientKind::Linear,
            Gradient::Radial { .. } => GradientKind::Radial,
            Gradient::Conic { .. } => GradientKind::Conic,
        }
    }
}

impl SvgDisplay for Color {
    fn fmt(&self, f: &mut impl SvgWrite) {
        match *self {
            c @ Color::Rgb(_)
            | c @ Color::Luma(_)
            | c @ Color::Cmyk(_)
            | c @ Color::Hsv(_) => {
                f.push_str(&c.to_hex());
            }
            Color::LinearRgb(rgb) => {
                f.push_str("color(srgb-linear ");
                f.push_nums([rgb.red, rgb.green, rgb.blue].map(round::<5>));
                if rgb.alpha != 1.0 {
                    f.push_str(" / ");
                    f.push_num(round::<5>(rgb.alpha));
                }
                f.push_str(")");
            }
            Color::Oklab(oklab) => {
                f.push_str("oklab(");
                f.push_num(round::<3>(100.0 * oklab.l));
                f.push_str("% ");
                f.push_nums([oklab.a, oklab.b].map(round::<5>));
                if oklab.alpha != 1.0 {
                    f.push_str(" / ");
                    f.push_num(round::<5>(oklab.alpha));
                }
                f.push_str(")");
            }
            Color::Oklch(oklch) => {
                f.push_str("oklch(");
                f.push_num(round::<3>(100.0 * oklch.l));
                f.push_str("% ");
                f.push_num(round::<5>(oklch.chroma));
                f.push_str(" ");
                f.push_num(round::<5>(oklch.hue.into_degrees()));
                if oklch.alpha != 1.0 {
                    f.push_str(" / ");
                    f.push_num(round::<5>(oklch.alpha));
                }
                f.push_str(")");
            }
            Color::Hsl(hsl) => {
                if hsl.alpha != 1.0 {
                    f.push_str("hsla(");
                } else {
                    f.push_str("hsl(");
                }
                f.push(round::<3>(hsl.hue.into_degrees()));
                f.push_str("deg ");
                f.push(round::<3>(100.0 * hsl.saturation));
                f.push_str("% ");
                f.push(round::<3>(100.0 * hsl.lightness));
                if hsl.alpha != 1.0 {
                    f.push_str(" / ");
                    f.push_num(round::<5>(hsl.alpha));
                }
                f.push_str(")");
            }
        }
    }
}

fn round<const DIGITS: u32>(num: f32) -> f64 {
    let factor = 10_u32.pow(DIGITS) as f64;
    (num as f64 * factor).round() / factor
}

/// Maps a coordinate in a unit size square to a coordinate in the tiling.
pub fn correct_tiling_pos(x: Ratio, y: Ratio) -> Point {
    0.5 * Point::new(Abs::pt(x.get() + 0.5), Abs::pt(y.get() + 0.5))
}
