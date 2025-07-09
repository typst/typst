//! Conversion from Typst data types into CSS data types.

use std::fmt::{self, Display, Write};

use ecow::EcoString;
use typst_library::html::{attr, HtmlElem};
use typst_library::layout::{Length, Rel};
use typst_library::visualize::{Color, Hsl, LinearRgb, Oklab, Oklch, Rgb};
use typst_utils::Numeric;

/// Additional methods for [`HtmlElem`].
pub trait HtmlElemExt {
    /// Adds the styles to an element if the property list is non-empty.
    fn with_styles(self, properties: Properties) -> Self;
}

impl HtmlElemExt for HtmlElem {
    /// Adds CSS styles to an element.
    fn with_styles(self, properties: Properties) -> Self {
        if let Some(value) = properties.into_inline_styles() {
            self.with_attr(attr::style, value)
        } else {
            self
        }
    }
}

/// A list of CSS properties with values.
#[derive(Debug, Default)]
pub struct Properties(EcoString);

impl Properties {
    /// Creates an empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new property to the list.
    pub fn push(&mut self, property: &str, value: impl Display) {
        if !self.0.is_empty() {
            self.0.push_str("; ");
        }
        write!(&mut self.0, "{property}: {value}").unwrap();
    }

    /// Adds a new property in builder-style.
    #[expect(unused)]
    pub fn with(mut self, property: &str, value: impl Display) -> Self {
        self.push(property, value);
        self
    }

    /// Turns this into a string suitable for use as an inline `style`
    /// attribute.
    pub fn into_inline_styles(self) -> Option<EcoString> {
        (!self.0.is_empty()).then_some(self.0)
    }
}

pub fn rel(rel: Rel) -> impl Display {
    typst_utils::display(move |f| match (rel.abs.is_zero(), rel.rel.is_zero()) {
        (false, false) => {
            write!(f, "calc({}% + {})", rel.rel.get(), length(rel.abs))
        }
        (true, false) => write!(f, "{}%", rel.rel.get()),
        (_, true) => write!(f, "{}", length(rel.abs)),
    })
}

pub fn length(length: Length) -> impl Display {
    typst_utils::display(move |f| match (length.abs.is_zero(), length.em.is_zero()) {
        (false, false) => {
            write!(f, "calc({}pt + {}em)", length.abs.to_pt(), length.em.get())
        }
        (true, false) => write!(f, "{}em", length.em.get()),
        (_, true) => write!(f, "{}pt", length.abs.to_pt()),
    })
}

pub fn color(color: Color) -> impl Display {
    typst_utils::display(move |f| match color {
        Color::Rgb(_) | Color::Cmyk(_) | Color::Luma(_) => rgb(f, color.to_rgb()),
        Color::Oklab(v) => oklab(f, v),
        Color::Oklch(v) => oklch(f, v),
        Color::LinearRgb(v) => linear_rgb(f, v),
        Color::Hsl(_) | Color::Hsv(_) => hsl(f, color.to_hsl()),
    })
}

fn oklab(f: &mut fmt::Formatter<'_>, v: Oklab) -> fmt::Result {
    write!(f, "oklab({} {} {}{})", percent(v.l), number(v.a), number(v.b), alpha(v.alpha))
}

fn oklch(f: &mut fmt::Formatter<'_>, v: Oklch) -> fmt::Result {
    write!(
        f,
        "oklch({} {} {}deg{})",
        percent(v.l),
        number(v.chroma),
        number(v.hue.into_degrees()),
        alpha(v.alpha)
    )
}

fn rgb(f: &mut fmt::Formatter<'_>, v: Rgb) -> fmt::Result {
    if let Some(v) = rgb_to_8_bit_lossless(v) {
        let (r, g, b, a) = v.into_components();
        write!(f, "#{r:02x}{g:02x}{b:02x}")?;
        if a != u8::MAX {
            write!(f, "{a:02x}")?;
        }
        Ok(())
    } else {
        write!(
            f,
            "rgb({} {} {}{})",
            percent(v.red),
            percent(v.green),
            percent(v.blue),
            alpha(v.alpha)
        )
    }
}

/// Converts an f32 RGBA color to its 8-bit representation if the result is
/// [very close](is_very_close) to the original.
fn rgb_to_8_bit_lossless(
    v: Rgb,
) -> Option<palette::rgb::Rgba<palette::encoding::Srgb, u8>> {
    let l = v.into_format::<u8, u8>();
    let h = l.into_format::<f32, f32>();
    (is_very_close(v.red, h.red)
        && is_very_close(v.blue, h.blue)
        && is_very_close(v.green, h.green)
        && is_very_close(v.alpha, h.alpha))
    .then_some(l)
}

fn linear_rgb(f: &mut fmt::Formatter<'_>, v: LinearRgb) -> fmt::Result {
    write!(
        f,
        "color(srgb-linear {} {} {}{})",
        percent(v.red),
        percent(v.green),
        percent(v.blue),
        alpha(v.alpha),
    )
}

fn hsl(f: &mut fmt::Formatter<'_>, v: Hsl) -> fmt::Result {
    write!(
        f,
        "hsl({}deg {} {}{})",
        number(v.hue.into_degrees()),
        percent(v.saturation),
        percent(v.lightness),
        alpha(v.alpha),
    )
}

/// Displays an alpha component if it not 1.
fn alpha(value: f32) -> impl Display {
    typst_utils::display(move |f| {
        if !is_very_close(value, 1.0) {
            write!(f, " / {}", percent(value))?;
        }
        Ok(())
    })
}

/// Displays a rounded percentage.
///
/// For a percentage, two significant digits after the comma gives us a
/// precision of 1/10_000, which is more than 12 bits (see `is_very_close`).
fn percent(ratio: f32) -> impl Display {
    typst_utils::display(move |f| {
        write!(f, "{}%", typst_utils::round_with_precision(ratio as f64 * 100.0, 2))
    })
}

/// Rounds a number for display.
///
/// For a number between 0 and 1, four significant digits give us a
/// precision of 1/10_000, which is more than 12 bits (see `is_very_close`).
fn number(value: f32) -> impl Display {
    typst_utils::round_with_precision(value as f64, 4)
}

/// Whether two component values are close enough that there is no
/// difference when encoding them with 12-bit. 12 bit is the highest
/// reasonable color bit depth found in the industry.
fn is_very_close(a: f32, b: f32) -> bool {
    const MAX_BIT_DEPTH: u32 = 12;
    const EPS: f32 = 0.5 / 2_i32.pow(MAX_BIT_DEPTH) as f32;
    (a - b).abs() < EPS
}
