//! Conversion from Typst data types into CSS data types.

use std::fmt::{Display, Write};
use std::ops::Deref;

use ecow::{EcoString, EcoVec, eco_format};
use typst_library::diag::WarningSink;
use typst_library::layout::{Abs, Angle, Em, Length, Ratio, Rel};
use typst_library::visualize::{Color, Hsl, LinearRgb, Oklab, Oklch, Paint, Rgb};
use typst_utils::Numeric;

/// A list of CSS properties with values.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Properties(EcoVec<Property>);

impl Properties {
    /// Creates an empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty list.
    pub fn build() -> PropertiesBuilder {
        PropertiesBuilder::default()
    }

    /// Adds a new property to the list.
    pub fn push(&mut self, property: &'static str, value: impl Into<EcoString>) {
        let property = Property::new(property, value.into());
        let res = self.0.binary_search_by_key(&property.name, |p| p.name);
        match res {
            Ok(idx) => self.0.make_mut()[idx] = property,
            Err(idx) => self.0.insert(idx, property),
        }
    }

    /// Adds a new property in builder-style.
    pub fn with(mut self, property: &'static str, value: impl Into<EcoString>) -> Self {
        self.push(property, value);
        self
    }
}

impl Deref for Properties {
    type Target = [Property];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default)]
pub struct PropertiesBuilder {
    warnings: Vec<EcoString>,
    props: Properties,
}

impl PropertiesBuilder {
    /// Adds a new property to the list.
    pub fn push(&mut self, property: &'static str, value: impl ToCss) {
        let mut writer = CssWriter::new(&mut self.warnings);
        writer.emit(value);

        if !writer.error {
            self.props.push(property, writer.buf);
        }
    }

    /// Adds a new property in builder-style.
    pub fn with(mut self, property: &'static str, value: impl ToCss) -> Self {
        self.push(property, value);
        self
    }

    /// Finish building the properties and propagate warnings.
    pub fn finish(self, mut sink: impl WarningSink) -> Properties {
        for warning in self.warnings {
            sink.emit(&warning);
        }
        self.props
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Property {
    // TODO: Use something similar to `HtmlAttr`.
    pub name: &'static str,
    pub value: EcoString,
}

impl Property {
    pub fn new(name: &'static str, value: EcoString) -> Self {
        Self { name, value }
    }
}

/// Low-level writer for CSS syntax.
#[derive(Debug)]
pub struct CssWriter<'a> {
    warnings: &'a mut Vec<EcoString>,
    buf: EcoString,
    error: bool,
}

impl<'a> CssWriter<'a> {
    fn new(warnings: &'a mut Vec<EcoString>) -> Self {
        Self { warnings, buf: EcoString::new(), error: false }
    }

    /// Call a CSS function.
    fn call<'b>(&'b mut self, name: &str, separator: Separator) -> CallWriter<'a, 'b> {
        CallWriter::start(self, name, separator)
    }

    /// Start a `calc` call expression.
    fn calc<'b>(&'b mut self) -> CalcWriter<'a, 'b> {
        CalcWriter::start(self)
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
}

/// Writes a CSS function call.
struct CallWriter<'a, 'b> {
    w: &'b mut CssWriter<'a>,
    count: usize,
    separator: Separator,
}

impl<'a, 'b> CallWriter<'a, 'b> {
    fn start(w: &'b mut CssWriter<'a>, name: &str, separator: Separator) -> Self {
        w.write(name);
        w.write("(");
        Self { w, count: 0, separator }
    }

    fn arg(&mut self, value: impl ToCss) -> &mut Self {
        self.arg_with(value, self.separator)
    }

    fn arg_with(&mut self, value: impl ToCss, separator: Separator) -> &mut Self {
        if self.count > 0 {
            self.w.write(match separator {
                Separator::Space => " ",
                Separator::Slash => " / ",
            });
        }
        self.w.emit(value);
        self.count += 1;
        self
    }
}

impl Drop for CallWriter<'_, '_> {
    fn drop(&mut self) {
        self.w.write(")");
    }
}

/// A separator in a CSS function call argument list.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Separator {
    Space,
    Slash,
}

/// Writes a lazy CSS `calc(<calc-sum>) expression`.
/// If only a single operand is written, `calc(<calc-sum>)` is omitted.
struct CalcWriter<'a, 'b> {
    w: &'b mut CssWriter<'a>,
    /// The byte-index in the [`CssWriter::buf`].
    start_idx: usize,
    count: usize,
}

impl<'a, 'b> CalcWriter<'a, 'b> {
    fn start(w: &'b mut CssWriter<'a>) -> Self {
        let start_idx = w.buf.len();
        Self { w, start_idx, count: 0 }
    }

    /// Add a value.
    ///
    /// - If it is zero, it will be omitted.
    /// - If it is negative, it will be negated and subtracted. This makes the
    ///   assumption that the formatted string `"+ {val}"` is equivalent to
    ///   `"- {val.neg()}"`.
    ///   This would for example not be the case if a value somehow implements
    ///   [`Ord`] and would be formatted as a non-parenthesized binary operator
    ///   with the same or lower precedence, e.g.
    ///   `"+ -1pt + 2em"` != `"- 1pt - 2em"`.
    fn sum<T>(&mut self, value: T) -> &mut Self
    where
        T: ToCss + Numeric + Ord,
    {
        if value == T::zero() {
            return self;
        }

        if self.count == 0 {
            self.w.emit(value);
        } else {
            // Negate the value and subtract it, in case it is negative.
            if value < T::zero() {
                self.w.write(" - ");
                self.w.emit(value.neg());
            } else {
                self.w.write(" + ");
                self.w.emit(value);
            }
        }
        self.count += 1;
        self
    }
}

impl Drop for CalcWriter<'_, '_> {
    fn drop(&mut self) {
        match self.count {
            0 => {
                // An empty `calc()` function call is invalid, so write a `0`,
                // which is also valid for lengths.
                self.w.write("0");
            }
            1 => (),
            2.. => {
                // NOTE: This assumes the `ToCss` implementation of all values
                // should only modify text that itself has written into the
                // buffer, which seems reasonable.
                self.w.buf.insert_str(self.start_idx, "calc(");
                self.w.write(")");
            }
        }
    }
}

/// Serializes a value into CSS.
pub trait ToCss {
    /// Writes `self` into the writer.
    fn emit(&self, w: &mut CssWriter);

    /// Convert to a string.
    fn to_css(&self, mut sink: impl WarningSink) -> EcoString {
        let mut warnings = Vec::new();
        let mut w = CssWriter::new(&mut warnings);
        self.emit(&mut w);
        let value = w.buf;
        for w in warnings {
            sink.emit(w);
        }
        value
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
        w.write_fmt(typst_utils::round_with_precision(self.0.into(), self.1));
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
        w.calc().sum(self.em).sum(self.abs);
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
        w.calc().sum(self.rel).sum(self.abs.em).sum(self.abs.abs);
    }
}

impl ToCss for Paint {
    fn emit(&self, w: &mut CssWriter) {
        match self {
            Self::Solid(color) => w.emit(color),
            Self::Gradient(_) => w.fail("gradient"),
            Self::Tiling(_) => w.fail("tiling"),
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

impl MaybeAlpha for CallWriter<'_, '_> {
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
