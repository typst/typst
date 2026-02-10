//! Conversion from Typst data types into CSS data types.

use std::fmt::{Display, Write};

use ecow::{EcoString, eco_format};
use typst_library::diag::WarningSink;
use typst_library::layout::{Abs, Angle, Em, Length, Ratio, Rel, Sides};
use typst_library::visualize::{
    Color, ColorSpace, ConicGradient, Gradient, Hsl, LinearGradient, LinearRgb, Oklab,
    Oklch, Paint, RadialGradient, Rgb,
};
use typst_utils::Numeric;

/// A list of CSS properties with values.
#[derive(Debug, Default)]
pub struct Properties(CssWriter);

impl Properties {
    /// Creates an empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new property to the list.
    pub fn push(&mut self, property: &str, value: impl ToCss) {
        let len = self.0.buf.len();
        if !self.0.is_empty() {
            self.0.write("; ");
        }
        self.0.write(property);
        self.0.write(": ");
        self.0.emit(value);
        if self.0.error {
            self.0.buf.truncate(len);
        }
        self.0.error = false;
    }

    /// Adds a new property in builder-style.
    pub fn with(mut self, property: &str, value: impl ToCss) -> Self {
        self.push(property, value);
        self
    }

    /// Turns this into a string suitable for use as an inline `style`
    /// attribute.
    pub fn into_inline_styles(self, sink: impl WarningSink) -> Option<EcoString> {
        let buf = self.0.finish(sink);
        (!buf.is_empty()).then_some(buf)
    }
}

/// Low-level writer for CSS syntax.
#[derive(Debug, Default)]
pub struct CssWriter {
    buf: EcoString,
    warnings: Vec<EcoString>,
    error: bool,
}

impl CssWriter {
    fn new() -> Self {
        Self::default()
    }

    fn finish(self, mut sink: impl WarningSink) -> EcoString {
        for warning in self.warnings {
            sink.emit(&warning);
        }
        self.buf
    }

    /// Call a CSS function.
    fn call(&mut self, name: &str, separator: Separator) -> CallWriter<'_> {
        CallWriter::start(self, name, separator)
    }

    fn emit(&mut self, value: impl ToCss) {
        value.emit(self)
    }

    fn write(&mut self, value: &str) {
        self.buf.push_str(value);
    }

    fn write_fmt(&mut self, value: impl Display) {
        write!(&mut self.buf, "{value}").unwrap();
    }

    fn ignored(&mut self, what: &str) {
        self.warnings
            .push(eco_format!("{what} was ignored during HTML export"));
    }

    fn fail(&mut self, what: &str) {
        self.ignored(what);
        self.error = true;
    }

    fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

/// Writes a CSS function call.
struct CallWriter<'a> {
    w: &'a mut CssWriter,
    count: usize,
    separator: Separator,
}

impl<'a> CallWriter<'a> {
    fn start(w: &'a mut CssWriter, name: &str, separator: Separator) -> Self {
        w.write(name);
        w.write("(");
        Self { w, count: 0, separator }
    }

    fn arg(&mut self, value: impl ToCss) -> &mut CallWriter<'a> {
        self.arg_with(value, self.separator)
    }

    fn arg_with(
        &mut self,
        value: impl ToCss,
        separator: Separator,
    ) -> &mut CallWriter<'a> {
        if self.count > 0 {
            self.w.write(match separator {
                Separator::Space => " ",
                Separator::Comma => ", ",
                Separator::Slash => " / ",
            });
        }
        self.w.emit(value);
        self.count += 1;
        self
    }
}

impl Drop for CallWriter<'_> {
    fn drop(&mut self) {
        self.w.write(")");
    }
}

/// A separator in a CSS function call argument list.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Separator {
    Space,
    Comma,
    Slash,
}

/// Serializes a value into CSS.
pub trait ToCss {
    /// Writes `self` into the writer.
    fn emit(&self, w: &mut CssWriter);

    fn to_css(&self, sink: impl WarningSink) -> impl Display {
        // TODO: Optimize.
        let mut w = CssWriter::new();
        self.emit(&mut w);
        w.finish(sink)
    }
}

impl<T: ToCss + ?Sized> ToCss for &T {
    fn emit(&self, w: &mut CssWriter) {
        (**self).emit(w);
    }
}

impl ToCss for str {
    fn emit(&self, w: &mut CssWriter) {
        w.write(self);
    }
}

/// Displays a number with four significant digits.
///
/// For a number between 0 and 1, four significant digits give us a
/// precision of 1/10_000, which is more than 12 bits (see `is_very_close`).
struct Number<T: Into<f64>>(T);

impl<T: Into<f64> + Copy> ToCss for Number<T> {
    fn emit(&self, w: &mut CssWriter) {
        w.emit(NumberWithPrecision(self.0, 4));
    }
}

/// Displays a number with N significant digits.
struct NumberWithPrecision<T: Into<f64>>(T, i16);

impl<T: Into<f64> + Copy> ToCss for NumberWithPrecision<T> {
    fn emit(&self, w: &mut CssWriter) {
        w.write_fmt(typst_utils::round_with_precision(self.0.into(), 4));
    }
}

/// Displays `A + B`.
struct Sum<A, B>(A, B);

impl<A: ToCss, B: ToCss> ToCss for Sum<A, B> {
    fn emit(&self, w: &mut CssWriter) {
        w.emit(&self.0);
        w.write(" + ");
        w.emit(&self.1);
    }
}

impl ToCss for Abs {
    fn emit(&self, w: &mut CssWriter) {
        w.emit(Number(self.to_pt()));
        w.write("pt");
    }
}

impl ToCss for Em {
    fn emit(&self, w: &mut CssWriter) {
        w.emit(Number(self.get()));
        w.write("em");
    }
}

impl ToCss for Length {
    fn emit(&self, w: &mut CssWriter) {
        match (self.abs.is_zero(), self.em.is_zero()) {
            (false, false) => {
                w.call("calc", Separator::Space).arg(Sum(self.abs, self.em));
            }
            (true, false) => w.emit(self.em),
            (_, true) => w.emit(self.abs),
        }
    }
}

impl ToCss for Angle {
    fn emit(&self, w: &mut CssWriter) {
        w.emit(Number(self.to_deg()));
        w.write("deg");
    }
}

impl ToCss for Ratio {
    fn emit(&self, w: &mut CssWriter) {
        w.emit(NumberWithPrecision(self.get() * 100.0, 2));
        w.write("%");
    }
}

impl ToCss for Rel {
    fn emit(&self, w: &mut CssWriter) {
        match (self.abs.is_zero(), self.rel.is_zero()) {
            (false, false) => {
                w.call("calc", Separator::Space).arg(Sum(self.abs, self.rel));
            }
            (true, false) => w.emit(self.rel),
            (_, true) => w.emit(self.abs),
        }
    }
}

impl ToCss for Paint {
    fn emit(&self, w: &mut CssWriter) {
        match self {
            Self::Solid(color) => w.emit(color),
            Self::Gradient(gradient) => w.emit(gradient),
            Self::Tiling(_) => w.ignored("tiling"),
        }
    }
}

impl ToCss for Color {
    fn emit(&self, w: &mut CssWriter) {
        match self {
            Color::Rgb(_) | Color::Cmyk(_) | Color::Luma(_) => w.emit(self.to_rgb()),
            Color::Oklab(v) => w.emit(v),
            Color::Oklch(v) => w.emit(v),
            Color::LinearRgb(v) => w.emit(v),
            Color::Hsl(_) | Color::Hsv(_) => w.emit(self.to_hsl()),
        }
    }
}

impl ToCss for Rgb {
    fn emit(&self, w: &mut CssWriter) {
        let low = self.into_format::<u8, u8>();
        let high = low.into_format::<f32, f32>();

        // Checks if the 8-bit representation of an f32 RGBA color is [very
        // close](is_very_close) to the original. If yes, uses a hex
        // representation, otherwise falls back to an `rgb` call.
        if is_very_close(self.red, high.red)
            && is_very_close(self.blue, high.blue)
            && is_very_close(self.green, high.green)
            && is_very_close(self.alpha, high.alpha)
        {
            let (r, g, b, a) = low.into_components();
            w.write_fmt(format_args!("#{r:02x}{g:02x}{b:02x}"));
            if a != u8::MAX {
                w.write_fmt(format_args!("{a:02x}"));
            }
        } else {
            w.call("rgb", Separator::Space)
                .arg(to_ratio(self.red))
                .arg(to_ratio(self.green))
                .arg(to_ratio(self.blue))
                .maybe_alpha_arg(self.alpha);
        }
    }
}

impl ToCss for Oklab {
    fn emit(&self, w: &mut CssWriter) {
        w.call("oklab", Separator::Space)
            .arg(to_ratio(self.l))
            .arg(Number(self.a))
            .arg(Number(self.b))
            .maybe_alpha_arg(self.alpha);
    }
}

impl ToCss for Oklch {
    fn emit(&self, w: &mut CssWriter) {
        w.call("oklch", Separator::Space)
            .arg(to_ratio(self.l))
            .arg(Number(self.chroma))
            .arg(to_angle(self.hue.into_degrees()))
            .maybe_alpha_arg(self.alpha);
    }
}

impl ToCss for LinearRgb {
    fn emit(&self, w: &mut CssWriter) {
        w.call("color", Separator::Space)
            .arg("srgb-linear")
            .arg(to_ratio(self.red))
            .arg(to_ratio(self.green))
            .arg(to_ratio(self.blue))
            .maybe_alpha_arg(self.alpha);
    }
}

impl ToCss for Hsl {
    fn emit(&self, w: &mut CssWriter) {
        w.call("hsl", Separator::Space)
            .arg(to_angle(self.hue.into_degrees()))
            .arg(to_ratio(self.saturation))
            .arg(to_ratio(self.lightness))
            .maybe_alpha_arg(self.alpha);
    }
}

/// Adds an alpha component argument to a CSS call if the alpha value is not 1.
trait MaybeAlpha {
    fn maybe_alpha_arg(&mut self, value: f32);
}

impl MaybeAlpha for CallWriter<'_> {
    fn maybe_alpha_arg(&mut self, value: f32) {
        if !is_very_close(value, 1.0) {
            self.arg_with(to_ratio(value), Separator::Slash);
        }
    }
}

/// Convert a raw degree value into an `Angle`.
fn to_angle(degrees: impl Into<f64>) -> Angle {
    Angle::deg(degrees.into())
}

/// Convert a raw value between 0 and 1 to a `Ratio`.
fn to_ratio(v: impl Into<f64>) -> Ratio {
    Ratio::new(v.into())
}

/// Whether two component values are close enough that there is no
/// difference when encoding them with 12-bit. 12 bit is the highest
/// reasonable color bit depth found in the industry.
fn is_very_close(a: impl Into<f64>, b: impl Into<f64>) -> bool {
    const MAX_BIT_DEPTH: u32 = 12;
    const EPS: f64 = 0.5 / 2_i32.pow(MAX_BIT_DEPTH) as f64;
    (a.into() - b.into()).abs() < EPS
}

impl ToCss for Gradient {
    fn emit(&self, w: &mut CssWriter) {
        match self {
            Self::Linear(v) => w.emit(v.as_ref()),
            Self::Radial(v) => w.emit(v.as_ref()),
            Self::Conic(v) => w.emit(v.as_ref()),
        }
    }
}

impl ToCss for LinearGradient {
    fn emit(&self, w: &mut CssWriter) {
        let mut call = w.call("linear-gradient", Separator::Space);
        call.arg(GradientAngle(self.angle));

        match self.space {
            ColorSpace::Oklab => {
                call.arg("in oklab");
            }
            ColorSpace::Oklch => {
                call.arg("in oklch");
            }
            ColorSpace::Srgb => {
                // This is the default in CSS.
            }
            ColorSpace::D65Gray => {
                // CSS doesn't support this, so we convert the stops instead.
                // See below.
            }
            ColorSpace::LinearRgb => {
                call.arg("in srgb-linear");
            }
            ColorSpace::Hsl => {
                call.arg("in hsl");
            }
            ColorSpace::Hsv => {
                call.w.ignored("hsv gradient color space");
            }
            ColorSpace::Cmyk => {
                call.w.ignored("cmyk gradient color space");
            }
        }

        for &(mut c, ratio) in &self.stops {
            // CSS does not directly support interpolating in D65 gray, but
            // converting the stops to it (they will be encoded as RGB) is
            // equivalent.
            // TODO: Does it make sense to do this for all spaces.
            if self.space == ColorSpace::D65Gray {
                c = c.to_space(self.space);
            }

            call.arg_with(c, Separator::Comma);
            call.arg_with(ratio, Separator::Space);
        }

        call.finish();

        if self.relative.is_custom() {
            w.ignored("relative gradient placement");
        }
    }
}

impl ToCss for RadialGradient {
    fn emit(&self, w: &mut CssWriter) {
        w.fail("radial gradient");
    }
}

impl ToCss for ConicGradient {
    fn emit(&self, w: &mut CssWriter) {
        w.fail("linear gradient");
    }
}

struct GradientAngle(Angle);

impl ToCss for GradientAngle {
    fn emit(&self, w: &mut CssWriter) {
        let v = self.0.to_deg();
        if is_very_close(v, 0.0) {
            w.write("to right");
        } else if is_very_close(v, 90.0) {
            w.write("to bottom");
        } else if is_very_close(v, 180.0) {
            w.write("to left");
        } else if is_very_close(v, 270.0) {
            w.write("to top");
        } else {
            w.emit(self.0 + Angle::deg(90.0));
        }
    }
}

impl ToCss for Sides<Rel> {
    fn emit(&self, w: &mut CssWriter) {
        if self.is_uniform() {
            w.emit(self.top);
        } else if self.top == self.bottom && self.left == self.right {
            w.emit(self.top);
            w.write(" ");
            w.emit(self.left);
        } else if self.left == self.right {
            w.emit(self.top);
            w.write(" ");
            w.emit(self.left);
            w.write(" ");
            w.emit(self.bottom);
        } else {
            w.emit(self.top);
            w.write(" ");
            w.emit(self.right);
            w.write(" ");
            w.emit(self.bottom);
            w.write(" ");
            w.emit(self.left);
        }
    }
}

// TODO: ToCss for Stroke
// if stroke.is_uniform() {
//     let s = stroke.left.unwrap();
//     let f = typst_utils::display(|f| {
//         if let Smart::Custom(length) = s.thickness {
//             write!(f, "{}", css::length(length))?;
//         }
//
//         if let Smart::Custom(Some(_)) = &s.dash {
//             write!(f, " dashed")?
//         } else {
//             write!(f, " solid")?
//         }
//
//         if let Smart::Custom(paint) = &s.paint {
//             match paint {
//                 Paint::Solid(v) => write!(f, " {}", css::color(*v))?,
//                 Paint::Gradient(_) => {
//                     cell.borrow_mut().ignored(elem.span(), "gradient")
//                 }
//                 Paint::Tiling(_) => cell.borrow_mut().ignored(elem.span(), "tiling"),
//             }
//         }
//
//         Ok(())
//     });
//
//     inline.push("border", f);
// } else {
//     todo!()
// }

trait Finish: Sized {
    fn finish(self) {}
}

impl<T> Finish for T {}
