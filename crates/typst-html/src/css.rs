//! Conversion from Typst data types into CSS data types.

use std::fmt::{Display, Write};

use ecow::{EcoString, eco_format};
use typst_library::diag::WarningSink;
use typst_library::foundations::Smart;
use typst_library::layout::{
    Abs, Angle, Axes, Corners, Em, Length, Ratio, Rel, Sides, Sizing,
};
use typst_library::visualize::{
    Color, ColorSpace, ConicGradient, DashPattern, Gradient, Hsl, LinearGradient,
    LinearRgb, Oklab, Oklch, Paint, RadialGradient, Rgb, Stroke, Tiling,
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

    /// Warn that something was ignored during HTMl export.
    pub fn ignored(&mut self, what: &str) {
        self.0.ignored(what);
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

    /// Start a `calc` call expression.
    fn calc(&mut self) -> CalcWriter<'_> {
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
        w.write_fmt(typst_utils::round_with_precision(self.0.into(), self.1));
    }
}

/// Writes a lazy CSS `calc(<calc-sum>) expression`.
/// If only a single operand is written, `calc(<calc-sum>)` is omitted.
struct CalcWriter<'a> {
    w: &'a mut CssWriter,
    /// The byte-index in the [`CssWriter::buf`].
    start_idx: usize,
    count: usize,
}

impl<'a> CalcWriter<'a> {
    fn start(w: &'a mut CssWriter) -> Self {
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
    fn sum<T>(&mut self, value: T) -> &mut CalcWriter<'a>
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

impl Drop for CalcWriter<'_> {
    fn drop(&mut self) {
        match self.count {
            0 => {
                // An empty `calc()` function call is invalid, so write a `0`, which is
                // also valid for lengths.
                self.w.write("0");
            }
            1 => (),
            2.. => {
                self.w.buf.insert_str(self.start_idx, "calc(");
                self.w.write(")");
            }
        }
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
            Self::Gradient(gradient) => w.emit(gradient),
            Self::Tiling(tiling) => w.emit(tiling),
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
        call.arg(LinearGradientAngle(self.angle.normalized()));
        gradient_color_interpolation_method(&mut call, self.space);

        gradient_color_stops(&mut call, self.space, self.stops.iter().copied());

        call.finish();

        if self.relative.is_custom() {
            w.ignored("relative gradient placement");
        }
    }
}

struct LinearGradientAngle(Angle);

impl ToCss for LinearGradientAngle {
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

impl ToCss for RadialGradient {
    fn emit(&self, w: &mut CssWriter) {
        let mut call = w.call("radial-gradient", Separator::Space);

        // CSS default is `farthest-corner`, so also write the Typst default of `50%`.
        //
        // https://drafts.csswg.org/css-images/#valdef-radial-gradient-radial-size
        call.arg(RadialGradientSize(self.radius));

        if !is_center(self.center) {
            call.arg(GradientPosition(self.center));
        }
        if !(is_very_close(self.focal_center.x.get(), self.center.x.get())
            && is_very_close(self.focal_center.y.get(), self.center.y.get()))
        {
            call.w.ignored("radial gradient focal-center");
        }

        gradient_color_interpolation_method(&mut call, self.space);

        if !is_very_close(self.focal_radius.get(), 0.0) {
            // Since CSS gradients don't support an explicit focal radius, remap
            // the color stop percentages to mimic one.
            let remap = |(color, ratio)| {
                let min = self.focal_radius;
                let max = self.radius;
                let prev_range = max;
                let new_range = max - min;
                let scale = new_range / prev_range;
                let offset = Ratio::new(min / prev_range);
                let remapped_ratio = (scale * ratio) + offset;
                (color, remapped_ratio)
            };
            gradient_color_stops(
                &mut call,
                self.space,
                self.stops.iter().copied().map(remap),
            );
        } else {
            gradient_color_stops(&mut call, self.space, self.stops.iter().copied());
        }

        call.finish();

        if self.relative.is_custom() {
            w.ignored("relative gradient placement");
        }
    }
}

/// The gradient's radius, write two `length-percentage`s for an elliptic
/// radial shape.
///
/// https://drafts.csswg.org/css-images/#typedef-radial-size
struct RadialGradientSize(Ratio);

impl ToCss for RadialGradientSize {
    fn emit(&self, w: &mut CssWriter) {
        w.emit(self.0);
        w.write(" ");
        w.emit(self.0);
    }
}

/// Whether this position is at the center (50%, 50%).
fn is_center(pos: Axes<Ratio>) -> bool {
    is_very_close(pos.x.get(), 0.5) && is_very_close(pos.y.get(), 0.5)
}

impl ToCss for ConicGradient {
    fn emit(&self, w: &mut CssWriter) {
        // NOTE: Conic gradients in Typst should scale to the aspect ratio of
        // the parent container, but in CSS they keep the same proportions.
        let mut call = w.call("conic-gradient", Separator::Space);

        call.arg(ConicGradientAngle(self.angle.normalized()));

        if !is_center(self.center) {
            call.arg(GradientPosition(self.center));
        }

        gradient_color_interpolation_method(&mut call, self.space);

        gradient_color_stops(&mut call, self.space, self.stops.iter().copied());

        call.finish();

        if self.relative.is_custom() {
            w.ignored("relative gradient placement");
        }
    }
}

struct ConicGradientAngle(Angle);

impl ToCss for ConicGradientAngle {
    fn emit(&self, w: &mut CssWriter) {
        w.write("from ");
        w.emit((self.0 - Angle::deg(90.0)).normalized());
    }
}

/// https://drafts.csswg.org/css-values-5/#
#[derive(Copy, Clone)]
struct GradientPosition(Axes<Ratio>);

impl ToCss for GradientPosition {
    fn emit(&self, w: &mut CssWriter) {
        w.write("at ");

        let x = self.0.x.get();
        let y = self.0.y.get();

        match (is_very_close(x, 0.5), is_very_close(y, 0.5)) {
            (true, true) => w.write("center"),
            (true, false) => {
                if !is_very_close(x, 0.0) || is_very_close(x, 1.0) {
                    w.write("center ");
                }
                self.y_position(w)
            }
            (false, true) => self.x_position(w),
            (false, false) => {
                self.x_position(w);
                w.write(" ");
                self.y_position(w);
            }
        }
    }
}

impl GradientPosition {
    fn x_position(self, w: &mut CssWriter) {
        let x = self.0.x.get();
        if is_very_close(x, 0.0) {
            w.write("left");
        } else if is_very_close(x, 0.5) {
            w.write("center");
        } else if is_very_close(x, 1.0) {
            w.write("right");
        } else {
            w.emit(self.0.x);
        }
    }

    fn y_position(self, w: &mut CssWriter) {
        let y = self.0.y.get();
        if is_very_close(y, 0.0) {
            w.write("top");
        } else if is_very_close(y, 0.5) {
            w.write("center");
        } else if is_very_close(y, 1.0) {
            w.write("bottom");
        } else {
            w.emit(self.0.y);
        }
    }
}

/// The `<color-interpolation-method>` is the same between `linear-`, `radial-`
/// and `connic-gradient`.
///
/// https://www.w3.org/TR/css-color-4/#color-interpolation-method
fn gradient_color_interpolation_method(call: &mut CallWriter, space: ColorSpace) {
    match space {
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
}

/// Since Typst only supports ratios for gradient stops, this code can be shared
/// between all CSS gradient types.
fn gradient_color_stops(
    call: &mut CallWriter,
    space: ColorSpace,
    stops: impl IntoIterator<Item = (Color, Ratio)>,
) {
    for (mut c, ratio) in stops {
        // CSS does not directly support interpolating in D65 gray, but
        // converting the stops to it (they will be encoded as RGB) is
        // equivalent.
        // TODO: Does it make sense to do this for all spaces?
        if space == ColorSpace::D65Gray {
            c = c.to_space(space);
        }

        call.arg_with(c, Separator::Comma);
        call.arg_with(ratio, Separator::Space);
    }
}

impl ToCss for Tiling {
    fn emit(&self, w: &mut CssWriter) {
        w.fail("tiling");
    }
}

impl ToCss for Sides<Length> {
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

impl Properties {
    pub fn push_border(&mut self, border: &Sides<Option<Border>>) {
        if border.is_uniform() {
            let border = border.as_ref().left;
            if let Some(border) = border {
                self.push("border", border);
            }
        } else {
            // TODO: More concise definitions, by setting `border` and overriding
            // deviating sides or writing multiple values using `border-width`, etc.
            let names = ["border-left", "border-top", "border-right", "border-bottom"];
            for (name, value) in names.iter().zip(border.iter()) {
                if let Some(value) = value {
                    self.push(name, value);
                }
            }
        }
    }
}

/// This is a helper type to aid the conversion from the Typst container model
/// to the HTML container/box model [^1].
///
/// Here are the main differences between the two:
/// 1. The `stroke` in Typst doesn't affect layout, while `border` in HTML does.
///    The Typst stroke is drawn centered on top of the edge of the container.
/// 2. Typst's `outset` is most closely modelled by a negative HTML `margin`,
///    but the outset only affects the container itself, not its nested body.
///    Meaning the size from which the inset is applied doesn't change with the
///    outset.
///    In HTML, a negative margin will increase the size of the whole container,
///    and will thus also affect the size from which the padding is applied and
///    the size of the body.
/// 3. If a concrete size, such as `50pt` is specified for a Typst container,
///    specifying any other property won't affect the surrounding layout; and
///    only the inset will affect the layout of its body.
///    HTML uses `box-sizing: content-box` by default. The `content-box` doesn't
///    include the padding or border, and thus changing any of those properties
///    will affect the surrounding layout. Inversely changing the padding
///    doesn't change the layout of the body.
///    When using `box-sizing: border-box` instead, changing the padding and
///    border won't affect the outer layout. But in comparison to the Typst
///    container changing the border will still affect the layout of the body.
///
/// __Typst container model__
///
/// ```txt
/// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// ┆ ┏━━━━━━━━━━━━━━━━━━━ ---
/// ┆ ┃ ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄  |
/// ┆ ┃ ┆                   | outset
/// ┆ ┃ ┆    ┏━━━━━━━━━━━━ ---
/// ┆ ┃ ┆    ┃              | inset
/// ┆ ┃ ┆    ┃    ┏━━━━━━━ ---
/// ┆ ┃ ┆    ┃    ┃         | body
/// ┆ ┃ ┆    ┃    ┗━━━━━━━ ---
/// ┆ ┃ ┆    ┃
/// ┆ ┃ ┆    ┗━━━━━━━━━━━━
/// ┆ ┃ ┆    |
/// ┆ ┃ ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ ---
/// ┆ ┗━━━━━━┿━━━━━━━━━━━━  | stroke
/// ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ ---
///          |
///          |_ size
/// ```
///
/// __HTML container/box model__
///
/// ```txt
///  ┏━━━━━━━━━━━━━━━━━━━━━ ---
///  ┃                       | margin
///  ┃   ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ ---
///  ┃   ┆                   | border
///  ┃   ┆   ╭┄┄┄┄┄┄┄┄┄┄┄┄┄ ---
///  ┃   ┆   ┆               | padding
///  ┃   ┆   ┆    ┏━━━━━━━━ ---
///  ┃   ┆   ┆    ┃          | body
///  ┃   ┆   ┆    ┗━━━━━━━━ ---
///  ┃   ┆   ┆    |
///  ┃   ┆   ╰┄┄┄┄┄┄┄┄┄┄┄┄┄
///  ┃   ┆        |
///  ┃   ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
///  ┃   |        |
///  ┗━━━━━━━━━━━━━━━━━━━━━
///      |        |
///      |        |_ content-box
///      |
///      |_ border-box
///
/// ```
///
/// [^1]: <https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Box_model/Introduction>
pub struct BoxModel {
    pub width: Sizing,
    pub height: Sizing,
    pub margin: Option<Sides<Length>>,
    pub padding: Option<Sides<Length>>,
    pub box_sizing: Option<BoxSizing>,
    /// Margins in CSS are computed on the width of the containing block, which
    /// is quite different from Typst.
    pub ignored_relative_outset: bool,
    /// Padding in CSS is computed on the width of the containing block, which
    /// is quite different from Typst.
    pub ignored_relative_inset: bool,
}

impl BoxModel {
    /// Computes parameters for an element using a `content-box` box-model.
    pub fn resolve(
        width: Sizing,
        height: Sizing,
        outset: Sides<Option<Rel>>,
        inset: Sides<Option<Rel>>,
        border: &Sides<Option<Border>>,
        has_body: bool,
    ) -> BoxModel {
        let outset = outset.unwrap_or_default();
        let inset = inset.unwrap_or_default();

        // Ignore relative outset and inset.
        let ignored_relative_outset = outset.iter().any(|rel| !rel.rel.is_zero());
        let ignored_relative_inset =
            has_body && inset.iter().any(|rel| !rel.rel.is_zero());
        let outset = outset.map(|rel| rel.abs);
        let inset = inset.map(|rel| rel.abs);

        // Strokes in Typst don't affect layout, but they do in the HTML/CSS
        // layout model. Try to replicate the Typst behavior by using negative
        // margins and adjust the padding of the container.
        let border_widths = border
            .as_ref()
            .map(|s| s.as_ref().map(|s| s.width_or_default()))
            .unwrap_or_default();

        // If the container isn't auto-sized, add the outset to the size.
        let width = match width {
            Sizing::Auto => Sizing::Auto,
            Sizing::Rel(rel) => Sizing::Rel(
                rel + outset.sum_by_axis().x + 0.5 * border_widths.sum_by_axis().x,
            ),
            // TODO: Once fractions are supported, handle the outset.
            Sizing::Fr(fr) => Sizing::Fr(fr),
        };
        let height = match height {
            Sizing::Auto => Sizing::Auto,
            Sizing::Rel(rel) => Sizing::Rel(
                rel + outset.sum_by_axis().y + 0.5 * border_widths.sum_by_axis().y,
            ),
            // TODO: Once fractions are supported, handle the outset.
            Sizing::Fr(fr) => Sizing::Fr(fr),
        };

        // Use negative margin to represent outset.
        let margin = outset
            .zip(border_widths)
            .map(|(outset, stroke)| -(outset + 0.5 * stroke));

        // This might compute a negative padding, which is invalid in CSS, that
        // case is handled
        let padding = outset
            .zip(inset)
            .zip(border_widths)
            .map(|((outset, inset), stroke)| outset + inset - 0.5 * stroke);

        let has_margin = margin.iter().any(|margin| !margin.is_zero());
        let has_border = border_widths.iter().any(|l| !l.is_zero());
        let has_padding = has_body && padding.iter().any(|l| !l.is_zero());

        let box_sizing = if has_border || has_padding {
            BoxSizing::BorderBox
        } else {
            BoxSizing::ContentBox
        };

        let margin = has_margin.then_some(margin);
        let padding = has_padding.then_some(padding);
        let box_sizing = box_sizing.is_border_box().then_some(box_sizing);

        BoxModel {
            width,
            height,
            margin,
            padding,
            box_sizing,
            ignored_relative_outset,
            ignored_relative_inset,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BoxSizing {
    ContentBox,
    BorderBox,
}

impl BoxSizing {
    /// Returns `true` if the box sizing is `border-box`.
    pub fn is_border_box(&self) -> bool {
        matches!(self, Self::BorderBox)
    }
}

impl ToCss for BoxSizing {
    fn emit(&self, w: &mut CssWriter) {
        w.write(match self {
            BoxSizing::ContentBox => "content-box",
            BoxSizing::BorderBox => "border-box",
        });
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Border<'a> {
    width: Smart<Length>,
    color: Smart<&'a Paint>,
    dash: Option<&'a DashPattern>,
}

impl<'a> Border<'a> {
    pub fn resolve(stroke: &'a Option<Stroke>) -> Option<Self> {
        let stroke = stroke.as_ref()?;

        let width = stroke.thickness;
        let color = stroke.paint.as_ref();
        let dash = stroke.dash.as_ref().custom().and_then(|d| d.as_ref());

        Some(Self { width, color, dash })
    }

    /// Always write the border width, since the default in HTML is 3px compared
    /// to 1pt for Typst.
    fn width_or_default(&self) -> Length {
        self.width.unwrap_or(Abs::pt(1.0).into())
    }

    fn style(&self) -> &'static str {
        if self.dash.is_some() { "dashed" } else { "solid" }
    }
}

impl ToCss for Border<'_> {
    fn emit(&self, w: &mut CssWriter) {
        w.emit(self.width_or_default());
        w.write(" ");

        // Always write the style.
        w.write(self.style());

        if let Smart::Custom(paint) = &self.color {
            w.write(" ");
            match paint {
                Paint::Solid(color) => w.emit(color),
                // TODO: `border-image` doesn't really work here, consider using
                // a wrapping div and setting a clipping to represent the border
                // in the presentational profile.
                Paint::Gradient(_) => w.ignored("stroke gradient"),
                Paint::Tiling(_) => w.ignored("stroke tiling"),
            }
        }
    }
}

impl ToCss for Corners<Rel> {
    fn emit(&self, w: &mut CssWriter) {
        if self.is_uniform() {
            w.emit(self.top_left);
        } else if self.is_diagonal() {
            w.emit(self.top_left);
            w.write(" ");
            w.emit(self.top_right);
        } else {
            w.emit(self.top_left);
            w.write(" ");
            w.emit(self.top_right);
            w.write(" ");
            w.emit(self.bottom_right);
            w.write(" ");
            w.emit(self.bottom_left);
        }
    }
}

trait Finish: Sized {
    fn finish(self) {}
}

impl<T> Finish for T {}
