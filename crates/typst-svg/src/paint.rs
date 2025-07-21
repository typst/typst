use std::f32::consts::TAU;

use ecow::{EcoString, eco_format};
use ttf_parser::OutlineBuilder;
use typst_library::foundations::Repr;
use typst_library::layout::{Angle, Axes, Frame, Quadrant, Ratio, Size, Transform};
use typst_library::visualize::{Color, FillRule, Gradient, Paint, RatioOrAngle, Tiling};
use typst_utils::hash128;
use xmlwriter::XmlWriter;

use crate::{Id, SVGRenderer, State, SvgMatrix, SvgPathBuilder};

/// The number of segments in a conic gradient.
/// This is a heuristic value that seems to work well.
/// Smaller values could be interesting for optimization.
const CONIC_SEGMENT: usize = 360;

impl SVGRenderer<'_> {
    /// Render a frame to a string.
    pub(super) fn render_tiling_frame(
        &mut self,
        state: State,
        ts: Transform,
        frame: &Frame,
    ) -> String {
        let mut xml = XmlWriter::new(xmlwriter::Options::default());
        std::mem::swap(&mut self.xml, &mut xml);
        self.render_frame(state, ts, frame);
        std::mem::swap(&mut self.xml, &mut xml);
        xml.end_document()
    }

    /// Write a fill attribute.
    pub(super) fn write_fill(
        &mut self,
        fill: &Paint,
        fill_rule: FillRule,
        size: Size,
        ts: Transform,
    ) {
        match fill {
            Paint::Solid(color) => self.xml.write_attribute("fill", &color.encode()),
            Paint::Gradient(gradient) => {
                let id = self.push_gradient(gradient, size, ts);
                self.xml.write_attribute_fmt("fill", format_args!("url(#{id})"));
            }
            Paint::Tiling(tiling) => {
                let id = self.push_tiling(tiling, size, ts);
                self.xml.write_attribute_fmt("fill", format_args!("url(#{id})"));
            }
        }
        match fill_rule {
            FillRule::NonZero => self.xml.write_attribute("fill-rule", "nonzero"),
            FillRule::EvenOdd => self.xml.write_attribute("fill-rule", "evenodd"),
        }
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
        size: Size,
        ts: Transform,
    ) -> Id {
        let gradient_id = self
            .gradients
            .insert_with(hash128(&(gradient, size.aspect_ratio())), || {
                (gradient.clone(), size.aspect_ratio())
            });

        if ts.is_identity() {
            return gradient_id;
        }

        self.gradient_refs
            .insert_with(hash128(&(gradient_id, ts)), || GradientRef {
                id: gradient_id,
                kind: gradient.into(),
                transform: ts,
            })
    }

    pub(super) fn push_tiling(
        &mut self,
        tiling: &Tiling,
        size: Size,
        ts: Transform,
    ) -> Id {
        let tiling_size = tiling.size() + tiling.spacing();
        // Unfortunately due to a limitation of `xmlwriter`, we need to
        // render the frame twice: once to allocate all of the resources
        // that it needs and once to actually render it.
        self.render_tiling_frame(
            State::new(tiling_size, Transform::identity()),
            Transform::identity(),
            tiling.frame(),
        );

        let tiling_id = self.tilings.insert_with(hash128(tiling), || tiling.clone());
        self.tiling_refs.insert_with(hash128(&(tiling_id, ts)), || TilingRef {
            id: tiling_id,
            transform: ts,
            ratio: Axes::new(
                Ratio::new(tiling_size.x.to_pt() / size.x.to_pt()),
                Ratio::new(tiling_size.y.to_pt() / size.y.to_pt()),
            ),
        })
    }

    /// Write the raw gradients (without transform) to the SVG file.
    pub(super) fn write_gradients(&mut self) {
        if self.gradients.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "gradients");

        for (id, (gradient, ratio)) in self.gradients.iter() {
            match &gradient {
                Gradient::Linear(linear) => {
                    self.xml.start_element("linearGradient");
                    self.xml.write_attribute("id", &id);
                    self.xml.write_attribute("spreadMethod", "pad");
                    self.xml.write_attribute("gradientUnits", "userSpaceOnUse");

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

                    self.xml.write_attribute("x1", &x1);
                    self.xml.write_attribute("y1", &y1);
                    self.xml.write_attribute("x2", &x2);
                    self.xml.write_attribute("y2", &y2);
                }
                Gradient::Radial(radial) => {
                    self.xml.start_element("radialGradient");
                    self.xml.write_attribute("id", &id);
                    self.xml.write_attribute("spreadMethod", "pad");
                    self.xml.write_attribute("gradientUnits", "userSpaceOnUse");
                    self.xml.write_attribute("cx", &radial.center.x.get());
                    self.xml.write_attribute("cy", &radial.center.y.get());
                    self.xml.write_attribute("r", &radial.radius.get());
                    self.xml.write_attribute("fx", &radial.focal_center.x.get());
                    self.xml.write_attribute("fy", &radial.focal_center.y.get());
                    self.xml.write_attribute("fr", &radial.focal_radius.get());
                }
                Gradient::Conic(conic) => {
                    self.xml.start_element("pattern");
                    self.xml.write_attribute("id", &id);
                    self.xml.write_attribute("viewBox", "0 0 1 1");
                    self.xml.write_attribute("preserveAspectRatio", "none");
                    self.xml.write_attribute("patternUnits", "userSpaceOnUse");
                    self.xml.write_attribute("width", "2");
                    self.xml.write_attribute("height", "2");
                    self.xml.write_attribute("x", "-0.5");
                    self.xml.write_attribute("y", "-0.5");

                    // The rotation angle, negated to match rotation in PNG.
                    let angle: f32 =
                        -(Gradient::correct_aspect_ratio(conic.angle, *ratio).to_rad()
                            as f32)
                            .rem_euclid(TAU);
                    let center: (f32, f32) =
                        (conic.center.x.get() as f32, conic.center.y.get() as f32);

                    // We build an arg segment for each segment of a circle.
                    let dtheta = TAU / CONIC_SEGMENT as f32;
                    for i in 0..CONIC_SEGMENT {
                        let theta1 = dtheta * i as f32;
                        let theta2 = dtheta * (i + 1) as f32;

                        // Create the path for the segment.
                        let mut builder = SvgPathBuilder::default();
                        builder.move_to(
                            correct_tiling_pos(center.0),
                            correct_tiling_pos(center.1),
                        );
                        builder.line_to(
                            correct_tiling_pos(-2.0 * (theta1 + angle).cos() + center.0),
                            correct_tiling_pos(2.0 * (theta1 + angle).sin() + center.1),
                        );
                        builder.arc(
                            (2.0, 2.0),
                            0.0,
                            0,
                            1,
                            (
                                correct_tiling_pos(
                                    -2.0 * (theta2 + angle).cos() + center.0,
                                ),
                                correct_tiling_pos(
                                    2.0 * (theta2 + angle).sin() + center.1,
                                ),
                            ),
                        );
                        builder.close();

                        let t1 = (i as f32) / CONIC_SEGMENT as f32;
                        let t2 = (i + 1) as f32 / CONIC_SEGMENT as f32;
                        let subgradient = SVGSubGradient {
                            center: conic.center,
                            t0: Angle::rad((theta1 + angle) as f64),
                            t1: Angle::rad((theta2 + angle) as f64),
                            c0: gradient
                                .sample(RatioOrAngle::Ratio(Ratio::new(t1 as f64))),
                            c1: gradient
                                .sample(RatioOrAngle::Ratio(Ratio::new(t2 as f64))),
                        };
                        let id = self
                            .conic_subgradients
                            .insert_with(hash128(&subgradient), || subgradient);

                        // Add the path to the pattern.
                        self.xml.start_element("path");
                        self.xml.write_attribute("d", &builder.0);
                        self.xml.write_attribute_fmt("fill", format_args!("url(#{id})"));
                        self.xml
                            .write_attribute_fmt("stroke", format_args!("url(#{id})"));
                        self.xml.write_attribute("stroke-width", "0");
                        self.xml.write_attribute("shape-rendering", "optimizeSpeed");
                        self.xml.end_element();
                    }

                    // We skip the default stop generation code.
                    self.xml.end_element();
                    continue;
                }
            }

            for window in gradient.stops_ref().windows(2) {
                let (start_c, start_t) = window[0];
                let (end_c, end_t) = window[1];

                self.xml.start_element("stop");
                self.xml.write_attribute("offset", &start_t.repr());
                self.xml.write_attribute("stop-color", &start_c.to_hex());
                self.xml.end_element();

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

                    self.xml.start_element("stop");
                    self.xml.write_attribute("offset", &t.repr());
                    self.xml.write_attribute("stop-color", &c.to_hex());
                    self.xml.end_element();
                }

                self.xml.start_element("stop");
                self.xml.write_attribute("offset", &end_t.repr());
                self.xml.write_attribute("stop-color", &end_c.to_hex());
                self.xml.end_element()
            }

            self.xml.end_element();
        }

        self.xml.end_element()
    }

    /// Write the sub-gradients that are used for conic gradients.
    pub(super) fn write_subgradients(&mut self) {
        if self.conic_subgradients.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "subgradients");
        for (id, gradient) in self.conic_subgradients.iter() {
            let x1 = 2.0 - gradient.t0.cos() as f32 + gradient.center.x.get() as f32;
            let y1 = gradient.t0.sin() as f32 + gradient.center.y.get() as f32;
            let x2 = 2.0 - gradient.t1.cos() as f32 + gradient.center.x.get() as f32;
            let y2 = gradient.t1.sin() as f32 + gradient.center.y.get() as f32;

            self.xml.start_element("linearGradient");
            self.xml.write_attribute("id", &id);
            self.xml.write_attribute("gradientUnits", "objectBoundingBox");
            self.xml.write_attribute("x1", &x1);
            self.xml.write_attribute("y1", &y1);
            self.xml.write_attribute("x2", &x2);
            self.xml.write_attribute("y2", &y2);

            self.xml.start_element("stop");
            self.xml.write_attribute("offset", "0%");
            self.xml.write_attribute("stop-color", &gradient.c0.to_hex());
            self.xml.end_element();

            self.xml.start_element("stop");
            self.xml.write_attribute("offset", "100%");
            self.xml.write_attribute("stop-color", &gradient.c1.to_hex());
            self.xml.end_element();

            self.xml.end_element();
        }
        self.xml.end_element();
    }

    pub(super) fn write_gradient_refs(&mut self) {
        if self.gradient_refs.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "gradient-refs");
        for (id, gradient_ref) in self.gradient_refs.iter() {
            match gradient_ref.kind {
                GradientKind::Linear => {
                    self.xml.start_element("linearGradient");
                    self.xml.write_attribute(
                        "gradientTransform",
                        &SvgMatrix(gradient_ref.transform),
                    );
                }
                GradientKind::Radial => {
                    self.xml.start_element("radialGradient");
                    self.xml.write_attribute(
                        "gradientTransform",
                        &SvgMatrix(gradient_ref.transform),
                    );
                }
                GradientKind::Conic => {
                    self.xml.start_element("pattern");
                    self.xml.write_attribute(
                        "patternTransform",
                        &SvgMatrix(gradient_ref.transform),
                    );
                }
            }

            self.xml.write_attribute("id", &id);

            // Writing the href attribute to the "reference" gradient.
            self.xml
                .write_attribute_fmt("href", format_args!("#{}", gradient_ref.id));

            // Also writing the xlink:href attribute for compatibility.
            self.xml
                .write_attribute_fmt("xlink:href", format_args!("#{}", gradient_ref.id));
            self.xml.end_element();
        }

        self.xml.end_element();
    }

    /// Write the raw tilings (without transform) to the SVG file.
    pub(super) fn write_tilings(&mut self) {
        if self.tilings.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "tilings");

        for (id, tiling) in
            self.tilings.iter().map(|(i, p)| (i, p.clone())).collect::<Vec<_>>()
        {
            let size = tiling.size() + tiling.spacing();
            self.xml.start_element("pattern");
            self.xml.write_attribute("id", &id);
            self.xml.write_attribute("width", &size.x.to_pt());
            self.xml.write_attribute("height", &size.y.to_pt());
            self.xml.write_attribute("patternUnits", "userSpaceOnUse");
            self.xml.write_attribute_fmt(
                "viewBox",
                format_args!("0 0 {:.3} {:.3}", size.x.to_pt(), size.y.to_pt()),
            );

            // Render the frame.
            let state = State::new(size, Transform::identity());
            let ts = Transform::identity();
            self.render_frame(state, ts, tiling.frame());

            self.xml.end_element();
        }

        self.xml.end_element()
    }

    /// Writes the references to the deduplicated tilings for each usage site.
    pub(super) fn write_tiling_refs(&mut self) {
        if self.tiling_refs.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "tilings-refs");
        for (id, tiling_ref) in self.tiling_refs.iter() {
            self.xml.start_element("pattern");
            self.xml
                .write_attribute("patternTransform", &SvgMatrix(tiling_ref.transform));

            self.xml.write_attribute("id", &id);

            // Writing the href attribute to the "reference" pattern.
            self.xml
                .write_attribute_fmt("href", format_args!("#{}", tiling_ref.id));

            // Also writing the xlink:href attribute for compatibility.
            self.xml
                .write_attribute_fmt("xlink:href", format_args!("#{}", tiling_ref.id));
            self.xml.end_element();
        }

        self.xml.end_element();
    }
}

/// A reference to a deduplicated tiling, with a transform matrix.
///
/// Allows tilings to be reused across multiple invocations, simply by changing
/// the transform matrix.
#[derive(Hash)]
pub struct TilingRef {
    /// The ID of the deduplicated gradient
    id: Id,
    /// The transform matrix to apply to the tiling.
    transform: Transform,
    /// The ratio of the size of the cell to the size of the filled area.
    ratio: Axes<Ratio>,
}

/// A reference to a deduplicated gradient, with a transform matrix.
///
/// Allows gradients to be reused across multiple invocations,
/// simply by changing the transform matrix.
#[derive(Hash)]
pub struct GradientRef {
    /// The ID of the deduplicated gradient
    id: Id,
    /// The gradient kind (used to determine the SVG element to use)
    /// but without needing to clone the entire gradient.
    kind: GradientKind,
    /// The transform matrix to apply to the gradient.
    transform: Transform,
}

/// A subgradient for conic gradients.
#[derive(Hash)]
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
#[derive(Hash, Clone, Copy, PartialEq, Eq)]
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

/// Encode the color as an SVG color.
pub trait ColorEncode {
    /// Encode the color.
    fn encode(&self) -> EcoString;
}

impl ColorEncode for Color {
    fn encode(&self) -> EcoString {
        match *self {
            c @ Color::Rgb(_)
            | c @ Color::Luma(_)
            | c @ Color::Cmyk(_)
            | c @ Color::Hsv(_) => c.to_hex(),
            Color::LinearRgb(rgb) => {
                if rgb.alpha != 1.0 {
                    eco_format!(
                        "color(srgb-linear {:.5} {:.5} {:.5} / {:.5})",
                        rgb.red,
                        rgb.green,
                        rgb.blue,
                        rgb.alpha
                    )
                } else {
                    eco_format!(
                        "color(srgb-linear {:.5} {:.5} {:.5})",
                        rgb.red,
                        rgb.green,
                        rgb.blue,
                    )
                }
            }
            Color::Oklab(oklab) => {
                if oklab.alpha != 1.0 {
                    eco_format!(
                        "oklab({:.3}% {:.5} {:.5} / {:.5})",
                        oklab.l * 100.0,
                        oklab.a,
                        oklab.b,
                        oklab.alpha
                    )
                } else {
                    eco_format!(
                        "oklab({:.3}% {:.5} {:.5})",
                        oklab.l * 100.0,
                        oklab.a,
                        oklab.b,
                    )
                }
            }
            Color::Oklch(oklch) => {
                if oklch.alpha != 1.0 {
                    eco_format!(
                        "oklch({:.3}% {:.5} {:.3}deg / {:.3})",
                        oklch.l * 100.0,
                        oklch.chroma,
                        oklch.hue.into_degrees(),
                        oklch.alpha
                    )
                } else {
                    eco_format!(
                        "oklch({:.3}% {:.5} {:.3}deg)",
                        oklch.l * 100.0,
                        oklch.chroma,
                        oklch.hue.into_degrees(),
                    )
                }
            }
            Color::Hsl(hsl) => {
                if hsl.alpha != 1.0 {
                    eco_format!(
                        "hsla({:.3}deg {:.3}% {:.3}% / {:.5})",
                        hsl.hue.into_degrees(),
                        hsl.saturation * 100.0,
                        hsl.lightness * 100.0,
                        hsl.alpha,
                    )
                } else {
                    eco_format!(
                        "hsl({:.3}deg {:.3}% {:.3}%)",
                        hsl.hue.into_degrees(),
                        hsl.saturation * 100.0,
                        hsl.lightness * 100.0,
                    )
                }
            }
        }
    }
}

/// Maps a coordinate in a unit size square to a coordinate in the tiling.
pub fn correct_tiling_pos(x: f32) -> f32 {
    (x + 0.5) / 2.0
}
