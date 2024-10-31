use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::LazyLock;

use ecow::{eco_format, EcoString, EcoVec};
use palette::encoding::{self, Linear};
use palette::{
    Alpha, Darken, Desaturate, FromColor, Lighten, OklabHue, RgbHue, Saturate, ShiftHue,
};
use qcms::Profile;

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::foundations::{
    array, cast, func, repr, scope, ty, Args, Array, IntoValue, Module, Repr, Scope, Str,
    Value,
};
use crate::layout::{Angle, Ratio};
use crate::syntax::{Span, Spanned};

// Type aliases for `palette` internal types in f32.
pub type Oklab = palette::oklab::Oklaba<f32>;
pub type Oklch = palette::oklch::Oklcha<f32>;
pub type LinearRgb = palette::rgb::Rgba<Linear<encoding::Srgb>, f32>;
pub type Rgb = palette::rgb::Rgba<encoding::Srgb, f32>;
pub type Hsl = palette::hsl::Hsla<encoding::Srgb, f32>;
pub type Hsv = palette::hsv::Hsva<encoding::Srgb, f32>;
pub type Luma = palette::luma::Lumaa<encoding::Srgb, f32>;

/// The ICC profile used to convert from CMYK to RGB.
///
/// This is a minimal CMYK profile that only contains the necessary information
/// to convert from CMYK to RGB. It is based on the CGATS TR 001-1995
/// specification. See
/// <https://github.com/saucecontrol/Compact-ICC-Profiles#cmyk>.
static CMYK_TO_XYZ: LazyLock<Box<Profile>> = LazyLock::new(|| {
    Profile::new_from_slice(typst_assets::icc::CMYK_TO_XYZ, false).unwrap()
});

/// The target sRGB profile.
static SRGB_PROFILE: LazyLock<Box<Profile>> = LazyLock::new(|| {
    let mut out = Profile::new_sRGB();
    out.precache_output_transform();
    out
});

static TO_SRGB: LazyLock<qcms::Transform> = LazyLock::new(|| {
    qcms::Transform::new_to(
        &CMYK_TO_XYZ,
        &SRGB_PROFILE,
        qcms::DataType::CMYK,
        qcms::DataType::RGB8,
        // Our input profile only supports perceptual intent.
        qcms::Intent::Perceptual,
    )
    .unwrap()
});

/// A color in a specific color space.
///
/// Typst supports:
/// - sRGB through the [`rgb` function]($color.rgb)
/// - Device CMYK through [`cmyk` function]($color.cmyk)
/// - D65 Gray through the [`luma` function]($color.luma)
/// - Oklab through the [`oklab` function]($color.oklab)
/// - Oklch through the [`oklch` function]($color.oklch)
/// - Linear RGB through the [`color.linear-rgb` function]($color.linear-rgb)
/// - HSL through the [`color.hsl` function]($color.hsl)
/// - HSV through the [`color.hsv` function]($color.hsv)
///
///
/// # Example
///
/// ```example
/// #rect(fill: aqua)
/// ```
///
/// # Predefined colors
/// Typst defines the following built-in colors:
///
/// | Color     | Definition         |
/// |-----------|:-------------------|
/// | `black`   | `{luma(0)}`        |
/// | `gray`    | `{luma(170)}`      |
/// | `silver`  | `{luma(221)}`      |
/// | `white`   | `{luma(255)}`      |
/// | `navy`    | `{rgb("#001f3f")}` |
/// | `blue`    | `{rgb("#0074d9")}` |
/// | `aqua`    | `{rgb("#7fdbff")}` |
/// | `teal`    | `{rgb("#39cccc")}` |
/// | `eastern` | `{rgb("#239dad")}` |
/// | `purple`  | `{rgb("#b10dc9")}` |
/// | `fuchsia` | `{rgb("#f012be")}` |
/// | `maroon`  | `{rgb("#85144b")}` |
/// | `red`     | `{rgb("#ff4136")}` |
/// | `orange`  | `{rgb("#ff851b")}` |
/// | `yellow`  | `{rgb("#ffdc00")}` |
/// | `olive`   | `{rgb("#3d9970")}` |
/// | `green`   | `{rgb("#2ecc40")}` |
/// | `lime`    | `{rgb("#01ff70")}` |
///
/// The predefined colors and the most important color constructors are
/// available globally and also in the color type's scope, so you can write
/// either `color.red` or just `red`.
///
/// ```preview
/// #let colors = (
///   "black", "gray", "silver", "white",
///   "navy", "blue", "aqua", "teal",
///   "eastern", "purple", "fuchsia",
///   "maroon", "red", "orange", "yellow",
///   "olive", "green", "lime",
/// )
///
/// #set text(font: "PT Sans")
/// #set page(width: auto)
/// #grid(
///   columns: 9,
///   gutter: 10pt,
///   ..colors.map(name => {
///       let col = eval(name)
///       let luminance = luma(col).components().first()
///       set text(fill: white) if luminance < 50%
///       set square(stroke: black) if col == white
///       set align(center + horizon)
///       square(size: 50pt,  fill: col, name)
///   })
/// )
/// ```
///
/// # Predefined color maps
/// Typst also includes a number of preset color maps that can be used for
/// [gradients]($gradient.linear). These are simply arrays of colors defined in
/// the module `color.map`.
///
/// ```example
/// #circle(fill: gradient.linear(..color.map.crest))
/// ```
///
/// | Map        | Details                                                     |
/// |------------|:------------------------------------------------------------|
/// | `turbo`    | A perceptually uniform rainbow-like color map. Read [this blog post](https://ai.googleblog.com/2019/08/turbo-improved-rainbow-colormap-for.html) for more details. |
/// | `cividis`  | A blue to gray to yellow color map. See [this blog post](https://bids.github.io/colormap/) for more details. |
/// | `rainbow`  | Cycles through the full color spectrum. This color map is best used by setting the interpolation color space to [HSL]($color.hsl). The rainbow gradient is **not suitable** for data visualization because it is not perceptually uniform, so the differences between values become unclear to your readers. It should only be used for decorative purposes. |
/// | `spectral` | Red to yellow to blue color map.                            |
/// | `viridis`  | A purple to teal to yellow color map.                       |
/// | `inferno`  | A black to red to yellow color map.                         |
/// | `magma`    | A black to purple to yellow color map.                      |
/// | `plasma`   | A purple to pink to yellow color map.                       |
/// | `rocket`   | A black to red to white color map.                          |
/// | `mako`     | A black to teal to yellow color map.                        |
/// | `vlag`     | A light blue to white to red color map.                     |
/// | `icefire`  | A light teal to black to yellow color map.                  |
/// | `flare`    | A orange to purple color map that is perceptually uniform.  |
/// | `crest`    | A blue to white to red color map.                           |
///
/// Some popular presets are not included because they are not available under a
/// free licence. Others, like
/// [Jet](https://jakevdp.github.io/blog/2014/10/16/how-bad-is-your-colormap/),
/// are not included because they are not color blind friendly. Feel free to use
/// or create a package with other presets that are useful to you!
///
/// ```preview
/// #set page(width: auto, height: auto)
/// #set text(font: "PT Sans", size: 8pt)
///
/// #let maps = (
///   "turbo", "cividis", "rainbow", "spectral",
///   "viridis", "inferno", "magma", "plasma",
///   "rocket", "mako", "vlag", "icefire",
///   "flare", "crest",
/// )
///
/// #stack(dir: ltr, spacing: 3pt, ..maps.map((name) => {
///   let map = eval("color.map." + name)
///   stack(
///     dir: ttb,
///     block(
///       width: 15pt,
///       height: 100pt,
///       fill: gradient.linear(..map, angle: 90deg),
///     ),
///     block(
///       width: 15pt,
///       height: 32pt,
///       move(dy: 8pt, rotate(90deg, name)),
///     ),
///   )
/// }))
/// ```
#[ty(scope, cast)]
#[derive(Copy, Clone)]
pub enum Color {
    /// A 32-bit luma color.
    Luma(Luma),
    /// A 32-bit L\*a\*b\* color in the Oklab color space.
    Oklab(Oklab),
    /// A 32-bit LCh color in the Oklab color space.
    Oklch(Oklch),
    /// A 32-bit RGB color.
    Rgb(Rgb),
    /// A 32-bit linear RGB color.
    LinearRgb(LinearRgb),
    /// A 32-bit CMYK color.
    Cmyk(Cmyk),
    /// A 32-bit HSL color.
    Hsl(Hsl),
    /// A 32-bit HSV color.
    Hsv(Hsv),
}

#[scope]
impl Color {
    /// The module of preset color maps.
    pub const MAP: fn() -> Module = || crate::utils::singleton!(Module, map()).clone();

    pub const BLACK: Self = Self::Luma(Luma::new(0.0, 1.0));
    pub const GRAY: Self = Self::Luma(Luma::new(0.6666666, 1.0));
    pub const WHITE: Self = Self::Luma(Luma::new(1.0, 1.0));
    pub const SILVER: Self = Self::Luma(Luma::new(0.8666667, 1.0));
    pub const NAVY: Self = Self::Rgb(Rgb::new(0.0, 0.121569, 0.247059, 1.0));
    pub const BLUE: Self = Self::Rgb(Rgb::new(0.0, 0.454902, 0.85098, 1.0));
    pub const AQUA: Self = Self::Rgb(Rgb::new(0.4980392, 0.858823, 1.0, 1.0));
    pub const TEAL: Self = Self::Rgb(Rgb::new(0.223529, 0.8, 0.8, 1.0));
    pub const EASTERN: Self = Self::Rgb(Rgb::new(0.13725, 0.615686, 0.678431, 1.0));
    pub const PURPLE: Self = Self::Rgb(Rgb::new(0.694118, 0.050980, 0.788235, 1.0));
    pub const FUCHSIA: Self = Self::Rgb(Rgb::new(0.941177, 0.070588, 0.745098, 1.0));
    pub const MAROON: Self = Self::Rgb(Rgb::new(0.521569, 0.078431, 0.294118, 1.0));
    pub const RED: Self = Self::Rgb(Rgb::new(1.0, 0.254902, 0.211765, 1.0));
    pub const ORANGE: Self = Self::Rgb(Rgb::new(1.0, 0.521569, 0.105882, 1.0));
    pub const YELLOW: Self = Self::Rgb(Rgb::new(1.0, 0.8627451, 0.0, 1.0));
    pub const OLIVE: Self = Self::Rgb(Rgb::new(0.239216, 0.6, 0.4392157, 1.0));
    pub const GREEN: Self = Self::Rgb(Rgb::new(0.1803922, 0.8, 0.2509804, 1.0));
    pub const LIME: Self = Self::Rgb(Rgb::new(0.0039216, 1.0, 0.4392157, 1.0));

    /// Create a grayscale color.
    ///
    /// A grayscale color is represented internally by a single `lightness`
    /// component.
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #for x in range(250, step: 50) {
    ///   box(square(fill: luma(x)))
    /// }
    /// ```
    #[func]
    pub fn luma(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,
        /// The lightness component.
        #[external]
        lightness: Component,
        /// The alpha component.
        #[external]
        alpha: RatioComponent,
        /// Alternatively: The color to convert to grayscale.
        ///
        /// If this is given, the `lightness` should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_luma()
        } else {
            let Component(gray) =
                args.expect("gray component").unwrap_or(Component(Ratio::one()));
            let RatioComponent(alpha) =
                args.eat()?.unwrap_or(RatioComponent(Ratio::one()));
            Self::Luma(Luma::new(gray.get() as f32, alpha.get() as f32))
        })
    }

    /// Create an [Oklab](https://bottosson.github.io/posts/oklab/) color.
    ///
    /// This color space is well suited for the following use cases:
    /// - Color manipulation such as saturating while keeping perceived hue
    /// - Creating grayscale images with uniform perceived lightness
    /// - Creating smooth and uniform color transition and gradients
    ///
    /// A linear Oklab color is represented internally by an array of four
    /// components:
    /// - lightness ([`ratio`])
    /// - a ([`float`] or [`ratio`].
    ///   Ratios are relative to `{0.4}`; meaning `{50%}` is equal to `{0.2}`)
    /// - b ([`float`] or [`ratio`].
    ///   Ratios are relative to `{0.4}`; meaning `{50%}` is equal to `{0.2}`)
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(
    ///   fill: oklab(27%, 20%, -3%, 50%)
    /// )
    /// ```
    #[func]
    pub fn oklab(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,
        /// The lightness component.
        #[external]
        lightness: RatioComponent,
        /// The a ("green/red") component.
        #[external]
        a: ChromaComponent,
        /// The b ("blue/yellow") component.
        #[external]
        b: ChromaComponent,
        /// The alpha component.
        #[external]
        alpha: RatioComponent,
        /// Alternatively: The color to convert to Oklab.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_oklab()
        } else {
            let RatioComponent(l) = args.expect("lightness component")?;
            let ChromaComponent(a) = args.expect("A component")?;
            let ChromaComponent(b) = args.expect("B component")?;
            let RatioComponent(alpha) =
                args.eat()?.unwrap_or(RatioComponent(Ratio::one()));
            Self::Oklab(Oklab::new(l.get() as f32, a, b, alpha.get() as f32))
        })
    }

    /// Create an [Oklch](https://bottosson.github.io/posts/oklab/) color.
    ///
    /// This color space is well suited for the following use cases:
    /// - Color manipulation involving lightness, chroma, and hue
    /// - Creating grayscale images with uniform perceived lightness
    /// - Creating smooth and uniform color transition and gradients
    ///
    /// A linear Oklch color is represented internally by an array of four
    /// components:
    /// - lightness ([`ratio`])
    /// - chroma ([`float`] or [`ratio`].
    ///   Ratios are relative to `{0.4}`; meaning `{50%}` is equal to `{0.2}`)
    /// - hue ([`angle`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(
    ///   fill: oklch(40%, 0.2, 160deg, 50%)
    /// )
    /// ```
    #[func]
    pub fn oklch(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,
        /// The lightness component.
        #[external]
        lightness: RatioComponent,
        /// The chroma component.
        #[external]
        chroma: ChromaComponent,
        /// The hue component.
        #[external]
        hue: Angle,
        /// The alpha component.
        #[external]
        alpha: RatioComponent,
        /// Alternatively: The color to convert to Oklch.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_oklch()
        } else {
            let RatioComponent(l) = args.expect("lightness component")?;
            let ChromaComponent(c) = args.expect("chroma component")?;
            let h: Angle = args.expect("hue component")?;
            let RatioComponent(alpha) =
                args.eat()?.unwrap_or(RatioComponent(Ratio::one()));
            Self::Oklch(Oklch::new(
                l.get() as f32,
                c,
                OklabHue::from_degrees(h.to_deg() as f32),
                alpha.get() as f32,
            ))
        })
    }

    /// Create an RGB(A) color with linear luma.
    ///
    /// This color space is similar to sRGB, but with the distinction that the
    /// color component are not gamma corrected. This makes it easier to perform
    /// color operations such as blending and interpolation. Although, you
    /// should prefer to use the [`oklab` function]($color.oklab) for these.
    ///
    /// A linear RGB(A) color is represented internally by an array of four
    /// components:
    /// - red ([`ratio`])
    /// - green ([`ratio`])
    /// - blue ([`ratio`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(fill: color.linear-rgb(
    ///   30%, 50%, 10%,
    /// ))
    /// ```
    #[func(title = "Linear RGB")]
    pub fn linear_rgb(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,
        /// The red component.
        #[external]
        red: Component,
        /// The green component.
        #[external]
        green: Component,
        /// The blue component.
        #[external]
        blue: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// Alternatively: The color to convert to linear RGB(A).
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_linear_rgb()
        } else {
            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            Self::LinearRgb(LinearRgb::new(
                r.get() as f32,
                g.get() as f32,
                b.get() as f32,
                a.get() as f32,
            ))
        })
    }

    /// Create an RGB(A) color.
    ///
    /// The color is specified in the sRGB color space.
    ///
    /// An RGB(A) color is represented internally by an array of four components:
    /// - red ([`ratio`])
    /// - green ([`ratio`])
    /// - blue ([`ratio`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #square(fill: rgb("#b1f2eb"))
    /// #square(fill: rgb(87, 127, 230))
    /// #square(fill: rgb(25%, 13%, 65%))
    /// ```
    #[func(title = "RGB")]
    pub fn rgb(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,
        /// The red component.
        #[external]
        red: Component,
        /// The green component.
        #[external]
        green: Component,
        /// The blue component.
        #[external]
        blue: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// Alternatively: The color in hexadecimal notation.
        ///
        /// Accepts three, four, six or eight hexadecimal digits and optionally
        /// a leading hash.
        ///
        /// If this is given, the individual components should not be given.
        ///
        /// ```example
        /// #text(16pt, rgb("#239dad"))[
        ///   *Typst*
        /// ]
        /// ```
        #[external]
        hex: Str,
        /// Alternatively: The color to convert to RGB(a).
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(string) = args.find::<Spanned<Str>>()? {
            Self::from_str(&string.v).at(string.span)?
        } else if let Some(color) = args.find::<Color>()? {
            color.to_rgb()
        } else {
            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            Self::Rgb(Rgb::new(
                r.get() as f32,
                g.get() as f32,
                b.get() as f32,
                a.get() as f32,
            ))
        })
    }

    /// Create a CMYK color.
    ///
    /// This is useful if you want to target a specific printer. The conversion
    /// to RGB for display preview might differ from how your printer reproduces
    /// the color.
    ///
    /// A CMYK color is represented internally by an array of four components:
    /// - cyan ([`ratio`])
    /// - magenta ([`ratio`])
    /// - yellow ([`ratio`])
    /// - key ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// Note that CMYK colors are not currently supported when PDF/A output is
    /// enabled.
    ///
    /// ```example
    /// #square(
    ///   fill: cmyk(27%, 0%, 3%, 5%)
    /// )
    /// ```
    #[func(title = "CMYK")]
    pub fn cmyk(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,
        /// The cyan component.
        #[external]
        cyan: RatioComponent,
        /// The magenta component.
        #[external]
        magenta: RatioComponent,
        /// The yellow component.
        #[external]
        yellow: RatioComponent,
        /// The key component.
        #[external]
        key: RatioComponent,
        /// Alternatively: The color to convert to CMYK.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_cmyk()
        } else {
            let RatioComponent(c) = args.expect("cyan component")?;
            let RatioComponent(m) = args.expect("magenta component")?;
            let RatioComponent(y) = args.expect("yellow component")?;
            let RatioComponent(k) = args.expect("key/black component")?;
            Self::Cmyk(Cmyk::new(
                c.get() as f32,
                m.get() as f32,
                y.get() as f32,
                k.get() as f32,
            ))
        })
    }

    /// Create an HSL color.
    ///
    /// This color space is useful for specifying colors by hue, saturation and
    /// lightness. It is also useful for color manipulation, such as saturating
    /// while keeping perceived hue.
    ///
    /// An HSL color is represented internally by an array of four components:
    /// - hue ([`angle`])
    /// - saturation ([`ratio`])
    /// - lightness ([`ratio`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(
    ///   fill: color.hsl(30deg, 50%, 60%)
    /// )
    /// ```
    #[func(title = "HSL")]
    pub fn hsl(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,
        /// The hue angle.
        #[external]
        hue: Angle,
        /// The saturation component.
        #[external]
        saturation: Component,
        /// The lightness component.
        #[external]
        lightness: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// Alternatively: The color to convert to HSL.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_hsl()
        } else {
            let h: Angle = args.expect("hue component")?;
            let Component(s) = args.expect("saturation component")?;
            let Component(l) = args.expect("lightness component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            Self::Hsl(Hsl::new(
                RgbHue::from_degrees(h.to_deg() as f32),
                s.get() as f32,
                l.get() as f32,
                a.get() as f32,
            ))
        })
    }

    /// Create an HSV color.
    ///
    /// This color space is useful for specifying colors by hue, saturation and
    /// value. It is also useful for color manipulation, such as saturating
    /// while keeping perceived hue.
    ///
    /// An HSV color is represented internally by an array of four components:
    /// - hue ([`angle`])
    /// - saturation ([`ratio`])
    /// - value ([`ratio`])
    /// - alpha ([`ratio`])
    ///
    /// These components are also available using the
    /// [`components`]($color.components) method.
    ///
    /// ```example
    /// #square(
    ///   fill: color.hsv(30deg, 50%, 60%)
    /// )
    /// ```
    #[func(title = "HSV")]
    pub fn hsv(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: &mut Args,
        /// The hue angle.
        #[external]
        hue: Angle,
        /// The saturation component.
        #[external]
        saturation: Component,
        /// The value component.
        #[external]
        value: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// Alternatively: The color to convert to HSL.
        ///
        /// If this is given, the individual components should not be given.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_hsv()
        } else {
            let h: Angle = args.expect("hue component")?;
            let Component(s) = args.expect("saturation component")?;
            let Component(v) = args.expect("value component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            Self::Hsv(Hsv::new(
                RgbHue::from_degrees(h.to_deg() as f32),
                s.get() as f32,
                v.get() as f32,
                a.get() as f32,
            ))
        })
    }

    /// Extracts the components of this color.
    ///
    /// The size and values of this array depends on the color space. You can
    /// obtain the color space using [`space`]($color.space). Below is a table
    /// of the color spaces and their components:
    ///
    /// |       Color space       |     C1    |     C2     |     C3    |   C4   |
    /// |-------------------------|-----------|------------|-----------|--------|
    /// | [`luma`]($color.luma)   | Lightness |            |           |        |
    /// | [`oklab`]($color.oklab) | Lightness |    `a`     |    `b`    |  Alpha |
    /// | [`oklch`]($color.oklch) | Lightness |   Chroma   |    Hue    |  Alpha |
    /// | [`linear-rgb`]($color.linear-rgb) | Red  |   Green |    Blue |  Alpha |
    /// | [`rgb`]($color.rgb)     |    Red    |   Green    |    Blue   |  Alpha |
    /// | [`cmyk`]($color.cmyk)   |    Cyan   |   Magenta  |   Yellow  |  Key   |
    /// | [`hsl`]($color.hsl)     |     Hue   | Saturation | Lightness |  Alpha |
    /// | [`hsv`]($color.hsv)     |     Hue   | Saturation |   Value   |  Alpha |
    ///
    /// For the meaning and type of each individual value, see the documentation
    /// of the corresponding color space. The alpha component is optional and
    /// only included if the `alpha` argument is `true`. The length of the
    /// returned array depends on the number of components and whether the alpha
    /// component is included.
    ///
    /// ```example
    /// // note that the alpha component is included by default
    /// #rgb(40%, 60%, 80%).components()
    /// ```
    #[func]
    pub fn components(
        self,
        /// Whether to include the alpha component.
        #[named]
        #[default(true)]
        alpha: bool,
    ) -> Array {
        let mut components = match self {
            Self::Luma(c) => {
                array![Ratio::new(c.luma.into()), Ratio::new(c.alpha.into())]
            }
            Self::Oklab(c) => {
                array![
                    Ratio::new(c.l.into()),
                    f64::from(c.a),
                    f64::from(c.b),
                    Ratio::new(c.alpha.into())
                ]
            }
            Self::Oklch(c) => {
                array![
                    Ratio::new(c.l.into()),
                    f64::from(c.chroma),
                    hue_angle(c.hue.into_degrees()),
                    Ratio::new(c.alpha.into()),
                ]
            }
            Self::LinearRgb(c) => {
                array![
                    Ratio::new(c.red.into()),
                    Ratio::new(c.green.into()),
                    Ratio::new(c.blue.into()),
                    Ratio::new(c.alpha.into()),
                ]
            }
            Self::Rgb(c) => {
                array![
                    Ratio::new(c.red.into()),
                    Ratio::new(c.green.into()),
                    Ratio::new(c.blue.into()),
                    Ratio::new(c.alpha.into()),
                ]
            }
            Self::Cmyk(c) => {
                array![
                    Ratio::new(c.c.into()),
                    Ratio::new(c.m.into()),
                    Ratio::new(c.y.into()),
                    Ratio::new(c.k.into())
                ]
            }
            Self::Hsl(c) => {
                array![
                    hue_angle(c.hue.into_degrees()),
                    Ratio::new(c.saturation.into()),
                    Ratio::new(c.lightness.into()),
                    Ratio::new(c.alpha.into()),
                ]
            }
            Self::Hsv(c) => {
                array![
                    hue_angle(c.hue.into_degrees()),
                    Ratio::new(c.saturation.into()),
                    Ratio::new(c.value.into()),
                    Ratio::new(c.alpha.into()),
                ]
            }
        };
        // Remove the alpha component if the corresponding argument was set.
        if !alpha && !matches!(self, Self::Cmyk(_)) {
            let _ = components.pop();
        }
        components
    }

    /// Returns the constructor function for this color's space:
    /// - [`luma`]($color.luma)
    /// - [`oklab`]($color.oklab)
    /// - [`oklch`]($color.oklch)
    /// - [`linear-rgb`]($color.linear-rgb)
    /// - [`rgb`]($color.rgb)
    /// - [`cmyk`]($color.cmyk)
    /// - [`hsl`]($color.hsl)
    /// - [`hsv`]($color.hsv)
    ///
    /// ```example
    /// #let color = cmyk(1%, 2%, 3%, 4%)
    /// #(color.space() == cmyk)
    /// ```
    #[func]
    pub fn space(self) -> ColorSpace {
        match self {
            Self::Luma(_) => ColorSpace::D65Gray,
            Self::Oklab(_) => ColorSpace::Oklab,
            Self::Oklch(_) => ColorSpace::Oklch,
            Self::LinearRgb(_) => ColorSpace::LinearRgb,
            Self::Rgb(_) => ColorSpace::Srgb,
            Self::Cmyk(_) => ColorSpace::Cmyk,
            Self::Hsl(_) => ColorSpace::Hsl,
            Self::Hsv(_) => ColorSpace::Hsv,
        }
    }

    /// Returns the color's RGB(A) hex representation (such as `#ffaa32` or
    /// `#020304fe`). The alpha component (last two digits in `#020304fe`) is
    /// omitted if it is equal to `ff` (255 / 100%).
    #[func]
    pub fn to_hex(self) -> EcoString {
        let [r, g, b, a] = self.to_rgb().to_vec4_u8();
        if a != 255 {
            eco_format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            eco_format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }

    /// Lightens a color by a given factor.
    #[func]
    pub fn lighten(
        self,
        /// The factor to lighten the color by.
        factor: Ratio,
    ) -> Color {
        let factor = factor.get() as f32;
        match self {
            Self::Luma(c) => Self::Luma(c.lighten(factor)),
            Self::Oklab(c) => Self::Oklab(c.lighten(factor)),
            Self::Oklch(c) => Self::Oklch(c.lighten(factor)),
            Self::LinearRgb(c) => Self::LinearRgb(c.lighten(factor)),
            Self::Rgb(c) => Self::Rgb(c.lighten(factor)),
            Self::Cmyk(c) => Self::Cmyk(c.lighten(factor)),
            Self::Hsl(c) => Self::Hsl(c.lighten(factor)),
            Self::Hsv(c) => Self::Hsv(c.lighten(factor)),
        }
    }

    /// Darkens a color by a given factor.
    #[func]
    pub fn darken(
        self,
        /// The factor to darken the color by.
        factor: Ratio,
    ) -> Color {
        let factor = factor.get() as f32;
        match self {
            Self::Luma(c) => Self::Luma(c.darken(factor)),
            Self::Oklab(c) => Self::Oklab(c.darken(factor)),
            Self::Oklch(c) => Self::Oklch(c.darken(factor)),
            Self::LinearRgb(c) => Self::LinearRgb(c.darken(factor)),
            Self::Rgb(c) => Self::Rgb(c.darken(factor)),
            Self::Cmyk(c) => Self::Cmyk(c.darken(factor)),
            Self::Hsl(c) => Self::Hsl(c.darken(factor)),
            Self::Hsv(c) => Self::Hsv(c.darken(factor)),
        }
    }

    /// Increases the saturation of a color by a given factor.
    #[func]
    pub fn saturate(
        self,
        /// The call span
        span: Span,
        /// The factor to saturate the color by.
        factor: Ratio,
    ) -> SourceResult<Color> {
        Ok(match self {
            Self::Luma(_) => {
                bail!(
                    span, "cannot saturate grayscale color";
                    hint: "try converting your color to RGB first"
                );
            }
            Self::Oklab(_) => self.to_hsv().saturate(span, factor)?.to_oklab(),
            Self::Oklch(_) => self.to_hsv().saturate(span, factor)?.to_oklch(),
            Self::LinearRgb(_) => self.to_hsv().saturate(span, factor)?.to_linear_rgb(),
            Self::Rgb(_) => self.to_hsv().saturate(span, factor)?.to_rgb(),
            Self::Cmyk(_) => self.to_hsv().saturate(span, factor)?.to_cmyk(),
            Self::Hsl(c) => Self::Hsl(c.saturate(factor.get() as f32)),
            Self::Hsv(c) => Self::Hsv(c.saturate(factor.get() as f32)),
        })
    }

    /// Decreases the saturation of a color by a given factor.
    #[func]
    pub fn desaturate(
        self,
        /// The call span
        span: Span,
        /// The factor to desaturate the color by.
        factor: Ratio,
    ) -> SourceResult<Color> {
        Ok(match self {
            Self::Luma(_) => {
                bail!(
                    span, "cannot desaturate grayscale color";
                    hint: "try converting your color to RGB first"
                );
            }
            Self::Oklab(_) => self.to_hsv().desaturate(span, factor)?.to_oklab(),
            Self::Oklch(_) => self.to_hsv().desaturate(span, factor)?.to_oklch(),
            Self::LinearRgb(_) => self.to_hsv().desaturate(span, factor)?.to_linear_rgb(),
            Self::Rgb(_) => self.to_hsv().desaturate(span, factor)?.to_rgb(),
            Self::Cmyk(_) => self.to_hsv().desaturate(span, factor)?.to_cmyk(),
            Self::Hsl(c) => Self::Hsl(c.desaturate(factor.get() as f32)),
            Self::Hsv(c) => Self::Hsv(c.desaturate(factor.get() as f32)),
        })
    }

    /// Produces the complementary color using a provided color space.
    /// You can think of it as the opposite side on a color wheel.
    ///
    /// ```example
    /// #square(fill: yellow)
    /// #square(fill: yellow.negate())
    /// #square(fill: yellow.negate(space: rgb))
    /// ```
    #[func]
    pub fn negate(
        self,
        /// The color space used for the transformation. By default, a perceptual color space is used.
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
    ) -> Color {
        let result = match self.to_space(space) {
            Self::Luma(c) => Self::Luma(Luma::new(1.0 - c.luma, c.alpha)),
            Self::Oklab(c) => Self::Oklab(Oklab::new(1.0 - c.l, -c.a, -c.b, c.alpha)),
            Self::Oklch(c) => Self::Oklch(Oklch::new(
                1.0 - c.l,
                c.chroma,
                OklabHue::from_degrees(c.hue.into_degrees() + 180.0),
                c.alpha,
            )),
            Self::LinearRgb(c) => Self::LinearRgb(LinearRgb::new(
                1.0 - c.red,
                1.0 - c.green,
                1.0 - c.blue,
                c.alpha,
            )),
            Self::Rgb(c) => {
                Self::Rgb(Rgb::new(1.0 - c.red, 1.0 - c.green, 1.0 - c.blue, c.alpha))
            }
            Self::Cmyk(c) => Self::Cmyk(Cmyk::new(1.0 - c.c, 1.0 - c.m, 1.0 - c.y, c.k)),
            Self::Hsl(c) => Self::Hsl(Hsl::new(
                RgbHue::from_degrees(c.hue.into_degrees() + 180.0),
                c.saturation,
                c.lightness,
                c.alpha,
            )),
            Self::Hsv(c) => Self::Hsv(Hsv::new(
                RgbHue::from_degrees(c.hue.into_degrees() + 180.0),
                c.saturation,
                c.value,
                c.alpha,
            )),
        };
        result.to_space(self.space())
    }

    /// Rotates the hue of the color by a given angle.
    #[func]
    pub fn rotate(
        self,
        /// The call span
        span: Span,
        /// The angle to rotate the hue by.
        angle: Angle,
        /// The color space used to rotate. By default, this happens in a perceptual
        /// color space ([`oklch`]($color.oklch)).
        #[named]
        #[default(ColorSpace::Oklch)]
        space: ColorSpace,
    ) -> SourceResult<Color> {
        Ok(match space {
            ColorSpace::Oklch => {
                let Self::Oklch(oklch) = self.to_oklch() else {
                    unreachable!();
                };
                let rotated = oklch.shift_hue(angle.to_deg() as f32);
                Self::Oklch(rotated).to_space(self.space())
            }
            ColorSpace::Hsl => {
                let Self::Hsl(hsl) = self.to_hsl() else {
                    unreachable!();
                };
                let rotated = hsl.shift_hue(angle.to_deg() as f32);
                Self::Hsl(rotated).to_space(self.space())
            }
            ColorSpace::Hsv => {
                let Self::Hsv(hsv) = self.to_hsv() else {
                    unreachable!();
                };
                let rotated = hsv.shift_hue(angle.to_deg() as f32);
                Self::Hsv(rotated).to_space(self.space())
            }
            _ => bail!(span, "this colorspace does not support hue rotation"),
        })
    }

    /// Create a color by mixing two or more colors.
    ///
    /// In color spaces with a hue component (hsl, hsv, oklch), only two colors
    /// can be mixed at once. Mixing more than two colors in such a space will
    /// result in an error!
    ///
    /// ```example
    /// #set block(height: 20pt, width: 100%)
    /// #block(fill: red.mix(blue))
    /// #block(fill: red.mix(blue, space: rgb))
    /// #block(fill: color.mix(red, blue, white))
    /// #block(fill: color.mix((red, 70%), (blue, 30%)))
    /// ```
    #[func]
    pub fn mix(
        /// The colors, optionally with weights, specified as a pair (array of
        /// length two) of color and weight (float or ratio).
        ///
        /// The weights do not need to add to `{100%}`, they are relative to the
        /// sum of all weights.
        #[variadic]
        colors: Vec<WeightedColor>,
        /// The color space to mix in. By default, this happens in a perceptual
        /// color space ([`oklab`]($color.oklab)).
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
    ) -> StrResult<Color> {
        Self::mix_iter(colors, space)
    }

    /// Makes a color more transparent by a given factor.
    ///
    /// This method is relative to the existing alpha value.
    /// If the scale is positive, calculates `alpha - alpha * scale`.
    /// Negative scales behave like `color.opacify(-scale)`.
    ///
    /// ```example
    /// #block(fill: red)[opaque]
    /// #block(fill: red.transparentize(50%))[half red]
    /// #block(fill: red.transparentize(75%))[quarter red]
    /// ```
    #[func]
    pub fn transparentize(
        self,
        /// The factor to change the alpha value by.
        scale: Ratio,
    ) -> StrResult<Color> {
        self.scale_alpha(-scale)
    }

    /// Makes a color more opaque by a given scale.
    ///
    /// This method is relative to the existing alpha value.
    /// If the scale is positive, calculates `alpha + scale - alpha * scale`.
    /// Negative scales behave like `color.transparentize(-scale)`.
    ///
    /// ```example
    /// #let half-red = red.transparentize(50%)
    /// #block(fill: half-red.opacify(100%))[opaque]
    /// #block(fill: half-red.opacify(50%))[three quarters red]
    /// #block(fill: half-red.opacify(-50%))[one quarter red]
    /// ```
    #[func]
    pub fn opacify(
        self,
        /// The scale to change the alpha value by.
        scale: Ratio,
    ) -> StrResult<Color> {
        self.scale_alpha(scale)
    }
}

impl Color {
    /// Same as [`Color::mix`], but takes an iterator instead of a vector.
    pub fn mix_iter(
        colors: impl IntoIterator<
            Item = WeightedColor,
            IntoIter = impl ExactSizeIterator<Item = WeightedColor>,
        >,
        space: ColorSpace,
    ) -> StrResult<Color> {
        let mut colors = colors.into_iter();
        if space.hue_index().is_some() && colors.len() > 2 {
            bail!("cannot mix more than two colors in a hue-based space");
        }

        let m = if space.hue_index().is_some() && colors.len() == 2 {
            let mut m = [0.0; 4];

            let WeightedColor { color: c0, weight: w0 } = colors.next().unwrap();
            let WeightedColor { color: c1, weight: w1 } = colors.next().unwrap();

            let c0 = c0.to_space(space).to_vec4();
            let c1 = c1.to_space(space).to_vec4();
            let w0 = w0 as f32;
            let w1 = w1 as f32;

            if w0 + w1 <= 0.0 {
                bail!("sum of weights must be positive");
            }

            for i in 0..4 {
                m[i] = (w0 * c0[i] + w1 * c1[i]) / (w0 + w1);
            }

            // Ensure that the hue circle is traversed in the short direction.
            if let Some(index) = space.hue_index() {
                if (c0[index] - c1[index]).abs() > 180.0 {
                    let (h0, h1) = if c0[index] < c1[index] {
                        (c0[index] + 360.0, c1[index])
                    } else {
                        (c0[index], c1[index] + 360.0)
                    };
                    m[index] = (w0 * h0 + w1 * h1) / (w0 + w1);
                }
            }

            m
        } else {
            let mut total = 0.0;
            let mut acc = [0.0; 4];

            for WeightedColor { color, weight } in colors {
                let weight = weight as f32;
                let v = color.to_space(space).to_vec4();
                acc[0] += weight * v[0];
                acc[1] += weight * v[1];
                acc[2] += weight * v[2];
                acc[3] += weight * v[3];
                total += weight;
            }

            if total <= 0.0 {
                bail!("sum of weights must be positive");
            }

            acc.map(|v| v / total)
        };

        Ok(match space {
            ColorSpace::Oklab => Color::Oklab(Oklab::new(m[0], m[1], m[2], m[3])),
            ColorSpace::Oklch => Color::Oklch(Oklch::new(m[0], m[1], m[2], m[3])),
            ColorSpace::Srgb => Color::Rgb(Rgb::new(m[0], m[1], m[2], m[3])),
            ColorSpace::LinearRgb => {
                Color::LinearRgb(LinearRgb::new(m[0], m[1], m[2], m[3]))
            }
            ColorSpace::Hsl => {
                Color::Hsl(Hsl::new(RgbHue::from_degrees(m[0]), m[1], m[2], m[3]))
            }
            ColorSpace::Hsv => {
                Color::Hsv(Hsv::new(RgbHue::from_degrees(m[0]), m[1], m[2], m[3]))
            }
            ColorSpace::Cmyk => Color::Cmyk(Cmyk::new(m[0], m[1], m[2], m[3])),
            ColorSpace::D65Gray => Color::Luma(Luma::new(m[0], m[3])),
        })
    }

    /// Construct a new RGBA color from 8-bit values.
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::Rgb(Rgb::new(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        ))
    }

    /// Converts a 32-bit integer to an RGBA color.
    pub fn from_u32(color: u32) -> Self {
        Self::from_u8(
            ((color >> 24) & 0xFF) as u8,
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        )
    }

    /// Returns the alpha channel of the color, if it has one.
    pub fn alpha(&self) -> Option<f32> {
        match self {
            Color::Cmyk(_) => None,
            Color::Luma(c) => Some(c.alpha),
            Color::Oklab(c) => Some(c.alpha),
            Color::Oklch(c) => Some(c.alpha),
            Color::Rgb(c) => Some(c.alpha),
            Color::LinearRgb(c) => Some(c.alpha),
            Color::Hsl(c) => Some(c.alpha),
            Color::Hsv(c) => Some(c.alpha),
        }
    }

    /// Sets the alpha channel of the color, if it has one.
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        match &mut self {
            Color::Cmyk(_) => {}
            Color::Luma(c) => c.alpha = alpha,
            Color::Oklab(c) => c.alpha = alpha,
            Color::Oklch(c) => c.alpha = alpha,
            Color::Rgb(c) => c.alpha = alpha,
            Color::LinearRgb(c) => c.alpha = alpha,
            Color::Hsl(c) => c.alpha = alpha,
            Color::Hsv(c) => c.alpha = alpha,
        }

        self
    }

    /// Scales the alpha value of a color by a given amount.
    ///
    /// For positive scales, computes `alpha + scale - alpha * scale`.
    /// For non-positive scales, computes `alpha + alpha * scale`.
    fn scale_alpha(self, scale: Ratio) -> StrResult<Color> {
        #[inline]
        fn transform<C>(mut color: Alpha<C, f32>, scale: Ratio) -> Alpha<C, f32> {
            let scale = scale.get() as f32;
            let factor = if scale > 0.0 { 1.0 - color.alpha } else { color.alpha };
            color.alpha = (color.alpha + scale * factor).clamp(0.0, 1.0);
            color
        }

        Ok(match self {
            Color::Luma(c) => Color::Luma(transform(c, scale)),
            Color::Oklab(c) => Color::Oklab(transform(c, scale)),
            Color::Oklch(c) => Color::Oklch(transform(c, scale)),
            Color::Rgb(c) => Color::Rgb(transform(c, scale)),
            Color::LinearRgb(c) => Color::LinearRgb(transform(c, scale)),
            Color::Cmyk(_) => bail!("CMYK does not have an alpha component"),
            Color::Hsl(c) => Color::Hsl(transform(c, scale)),
            Color::Hsv(c) => Color::Hsv(transform(c, scale)),
        })
    }

    /// Converts the color to a vec of four floats.
    pub fn to_vec4(&self) -> [f32; 4] {
        match self {
            Color::Luma(c) => [c.luma, c.luma, c.luma, c.alpha],
            Color::Oklab(c) => [c.l, c.a, c.b, c.alpha],
            Color::Oklch(c) => {
                [c.l, c.chroma, c.hue.into_degrees().rem_euclid(360.0), c.alpha]
            }
            Color::Rgb(c) => [c.red, c.green, c.blue, c.alpha],
            Color::LinearRgb(c) => [c.red, c.green, c.blue, c.alpha],
            Color::Cmyk(c) => [c.c, c.m, c.y, c.k],
            Color::Hsl(c) => [
                c.hue.into_degrees().rem_euclid(360.0),
                c.saturation,
                c.lightness,
                c.alpha,
            ],
            Color::Hsv(c) => {
                [c.hue.into_degrees().rem_euclid(360.0), c.saturation, c.value, c.alpha]
            }
        }
    }

    /// Converts the color to a vec of four [`u8`]s.
    pub fn to_vec4_u8(&self) -> [u8; 4] {
        self.to_vec4().map(|x| (x * 255.0).round() as u8)
    }

    pub fn to_space(self, space: ColorSpace) -> Self {
        match space {
            ColorSpace::Oklab => self.to_oklab(),
            ColorSpace::Oklch => self.to_oklch(),
            ColorSpace::Srgb => self.to_rgb(),
            ColorSpace::LinearRgb => self.to_linear_rgb(),
            ColorSpace::Hsl => self.to_hsl(),
            ColorSpace::Hsv => self.to_hsv(),
            ColorSpace::Cmyk => self.to_cmyk(),
            ColorSpace::D65Gray => self.to_luma(),
        }
    }

    pub fn to_luma(self) -> Self {
        Self::Luma(match self {
            Self::Luma(c) => c,
            Self::Oklab(c) => Luma::from_color(c),
            Self::Oklch(c) => Luma::from_color(c),
            Self::Rgb(c) => Luma::from_color(c),
            Self::LinearRgb(c) => Luma::from_color(c),
            Self::Cmyk(c) => Luma::from_color(c.to_rgba()),
            Self::Hsl(c) => Luma::from_color(c),
            Self::Hsv(c) => Luma::from_color(c),
        })
    }

    pub fn to_oklab(self) -> Self {
        Self::Oklab(match self {
            Self::Luma(c) => Oklab::from_color(c),
            Self::Oklab(c) => c,
            Self::Oklch(c) => Oklab::from_color(c),
            Self::Rgb(c) => Oklab::from_color(c),
            Self::LinearRgb(c) => Oklab::from_color(c),
            Self::Cmyk(c) => Oklab::from_color(c.to_rgba()),
            Self::Hsl(c) => Oklab::from_color(c),
            Self::Hsv(c) => Oklab::from_color(c),
        })
    }

    pub fn to_oklch(self) -> Self {
        Self::Oklch(match self {
            Self::Luma(c) => Oklch::from_color(c),
            Self::Oklab(c) => Oklch::from_color(c),
            Self::Oklch(c) => c,
            Self::Rgb(c) => Oklch::from_color(c),
            Self::LinearRgb(c) => Oklch::from_color(c),
            Self::Cmyk(c) => Oklch::from_color(c.to_rgba()),
            Self::Hsl(c) => Oklch::from_color(c),
            Self::Hsv(c) => Oklch::from_color(c),
        })
    }

    pub fn to_rgb(self) -> Self {
        Self::Rgb(match self {
            Self::Luma(c) => Rgb::from_color(c),
            Self::Oklab(c) => Rgb::from_color(c),
            Self::Oklch(c) => Rgb::from_color(c),
            Self::Rgb(c) => c,
            Self::LinearRgb(c) => Rgb::from_linear(c),
            Self::Cmyk(c) => Rgb::from_color(c.to_rgba()),
            Self::Hsl(c) => Rgb::from_color(c),
            Self::Hsv(c) => Rgb::from_color(c),
        })
    }

    pub fn to_linear_rgb(self) -> Self {
        Self::LinearRgb(match self {
            Self::Luma(c) => LinearRgb::from_color(c),
            Self::Oklab(c) => LinearRgb::from_color(c),
            Self::Oklch(c) => LinearRgb::from_color(c),
            Self::Rgb(c) => LinearRgb::from_color(c),
            Self::LinearRgb(c) => c,
            Self::Cmyk(c) => LinearRgb::from_color(c.to_rgba()),
            Self::Hsl(c) => Rgb::from_color(c).into_linear(),
            Self::Hsv(c) => Rgb::from_color(c).into_linear(),
        })
    }

    pub fn to_cmyk(self) -> Self {
        Self::Cmyk(match self {
            Self::Luma(c) => Cmyk::from_luma(c),
            Self::Oklab(c) => Cmyk::from_rgba(Rgb::from_color(c)),
            Self::Oklch(c) => Cmyk::from_rgba(Rgb::from_color(c)),
            Self::Rgb(c) => Cmyk::from_rgba(c),
            Self::LinearRgb(c) => Cmyk::from_rgba(Rgb::from_linear(c)),
            Self::Cmyk(c) => c,
            Self::Hsl(c) => Cmyk::from_rgba(Rgb::from_color(c)),
            Self::Hsv(c) => Cmyk::from_rgba(Rgb::from_color(c)),
        })
    }

    pub fn to_hsl(self) -> Self {
        Self::Hsl(match self {
            Self::Luma(c) => Hsl::from_color(c),
            Self::Oklab(c) => Hsl::from_color(c),
            Self::Oklch(c) => Hsl::from_color(c),
            Self::Rgb(c) => Hsl::from_color(c),
            Self::LinearRgb(c) => Hsl::from_color(Rgb::from_linear(c)),
            Self::Cmyk(c) => Hsl::from_color(c.to_rgba()),
            Self::Hsl(c) => c,
            Self::Hsv(c) => Hsl::from_color(c),
        })
    }

    pub fn to_hsv(self) -> Self {
        Self::Hsv(match self {
            Self::Luma(c) => Hsv::from_color(c),
            Self::Oklab(c) => Hsv::from_color(c),
            Self::Oklch(c) => Hsv::from_color(c),
            Self::Rgb(c) => Hsv::from_color(c),
            Self::LinearRgb(c) => Hsv::from_color(Rgb::from_linear(c)),
            Self::Cmyk(c) => Hsv::from_color(c.to_rgba()),
            Self::Hsl(c) => Hsv::from_color(c),
            Self::Hsv(c) => c,
        })
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Luma(v) => write!(f, "Luma({}, {})", v.luma, v.alpha),
            Self::Oklab(v) => write!(f, "Oklab({}, {}, {}, {})", v.l, v.a, v.b, v.alpha),
            Self::Oklch(v) => {
                write!(
                    f,
                    "Oklch({}, {}, {:?}, {})",
                    v.l,
                    v.chroma,
                    hue_angle(v.hue.into_degrees()),
                    v.alpha
                )
            }
            Self::Rgb(v) => {
                write!(f, "Rgb({}, {}, {}, {})", v.red, v.green, v.blue, v.alpha)
            }
            Self::LinearRgb(v) => {
                write!(f, "LinearRgb({}, {}, {}, {})", v.red, v.green, v.blue, v.alpha)
            }
            Self::Cmyk(v) => write!(f, "Cmyk({}, {}, {}, {})", v.c, v.m, v.y, v.k),
            Self::Hsl(v) => write!(
                f,
                "Hsl({:?}, {}, {}, {})",
                hue_angle(v.hue.into_degrees()),
                v.saturation,
                v.lightness,
                v.alpha
            ),
            Self::Hsv(v) => write!(
                f,
                "Hsv({:?}, {}, {}, {})",
                hue_angle(v.hue.into_degrees()),
                v.saturation,
                v.value,
                v.alpha
            ),
        }
    }
}

impl Repr for Color {
    fn repr(&self) -> EcoString {
        match self {
            Self::Luma(c) => {
                if c.alpha == 1.0 {
                    eco_format!("luma({})", Ratio::new(c.luma.into()).repr())
                } else {
                    eco_format!(
                        "luma({}, {})",
                        Ratio::new(c.luma.into()).repr(),
                        Ratio::new(c.alpha.into()).repr(),
                    )
                }
            }
            Self::Rgb(_) => eco_format!("rgb({})", self.to_hex().repr()),
            Self::LinearRgb(c) => {
                if c.alpha == 1.0 {
                    eco_format!(
                        "color.linear-rgb({}, {}, {})",
                        Ratio::new(c.red.into()).repr(),
                        Ratio::new(c.green.into()).repr(),
                        Ratio::new(c.blue.into()).repr(),
                    )
                } else {
                    eco_format!(
                        "color.linear-rgb({}, {}, {}, {})",
                        Ratio::new(c.red.into()).repr(),
                        Ratio::new(c.green.into()).repr(),
                        Ratio::new(c.blue.into()).repr(),
                        Ratio::new(c.alpha.into()).repr(),
                    )
                }
            }
            Self::Cmyk(c) => {
                eco_format!(
                    "cmyk({}, {}, {}, {})",
                    Ratio::new(c.c.into()).repr(),
                    Ratio::new(c.m.into()).repr(),
                    Ratio::new(c.y.into()).repr(),
                    Ratio::new(c.k.into()).repr(),
                )
            }
            Self::Oklab(c) => {
                if c.alpha == 1.0 {
                    eco_format!(
                        "oklab({}, {}, {})",
                        Ratio::new(c.l.into()).repr(),
                        repr::format_float_component(c.a.into()),
                        repr::format_float_component(c.b.into()),
                    )
                } else {
                    eco_format!(
                        "oklab({}, {}, {}, {})",
                        Ratio::new(c.l.into()).repr(),
                        repr::format_float_component(c.a.into()),
                        repr::format_float_component(c.b.into()),
                        Ratio::new(c.alpha.into()).repr(),
                    )
                }
            }
            Self::Oklch(c) => {
                if c.alpha == 1.0 {
                    eco_format!(
                        "oklch({}, {}, {})",
                        Ratio::new(c.l.into()).repr(),
                        repr::format_float_component(c.chroma.into()),
                        hue_angle(c.hue.into_degrees()).repr(),
                    )
                } else {
                    eco_format!(
                        "oklch({}, {}, {}, {})",
                        Ratio::new(c.l.into()).repr(),
                        repr::format_float_component(c.chroma.into()),
                        hue_angle(c.hue.into_degrees()).repr(),
                        Ratio::new(c.alpha.into()).repr(),
                    )
                }
            }
            Self::Hsl(c) => {
                if c.alpha == 1.0 {
                    eco_format!(
                        "color.hsl({}, {}, {})",
                        hue_angle(c.hue.into_degrees()).repr(),
                        Ratio::new(c.saturation.into()).repr(),
                        Ratio::new(c.lightness.into()).repr(),
                    )
                } else {
                    eco_format!(
                        "color.hsl({}, {}, {}, {})",
                        hue_angle(c.hue.into_degrees()).repr(),
                        Ratio::new(c.saturation.into()).repr(),
                        Ratio::new(c.lightness.into()).repr(),
                        Ratio::new(c.alpha.into()).repr(),
                    )
                }
            }
            Self::Hsv(c) => {
                if c.alpha == 1.0 {
                    eco_format!(
                        "color.hsv({}, {}, {})",
                        hue_angle(c.hue.into_degrees()).repr(),
                        Ratio::new(c.saturation.into()).repr(),
                        Ratio::new(c.value.into()).repr(),
                    )
                } else {
                    eco_format!(
                        "color.hsv({}, {}, {}, {})",
                        hue_angle(c.hue.into_degrees()).repr(),
                        Ratio::new(c.saturation.into()).repr(),
                        Ratio::new(c.value.into()).repr(),
                        Ratio::new(c.alpha.into()).repr(),
                    )
                }
            }
        }
    }
}

fn hue_angle(degrees: f32) -> Angle {
    Angle::deg(f64::from(degrees).rem_euclid(360.0))
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // Lower precision for comparison to avoid rounding errors.
            // Keeps backward compatibility with previous versions of Typst.
            (Self::Rgb(_), Self::Rgb(_)) => self.to_vec4_u8() == other.to_vec4_u8(),
            (Self::Luma(a), Self::Luma(b)) => {
                (a.luma * 255.0).round() as u8 == (b.luma * 255.0).round() as u8
            }
            (Self::Oklab(a), Self::Oklab(b)) => a == b,
            (Self::Oklch(a), Self::Oklch(b)) => a == b,
            (Self::LinearRgb(a), Self::LinearRgb(b)) => a == b,
            (Self::Cmyk(a), Self::Cmyk(b)) => a == b,
            (Self::Hsl(a), Self::Hsl(b)) => a == b,
            (Self::Hsv(a), Self::Hsv(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Color {}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        let [x, y, z, w] = self.to_vec4();
        x.to_bits().hash(state);
        y.to_bits().hash(state);
        z.to_bits().hash(state);
        w.to_bits().hash(state);
    }
}

impl FromStr for Color {
    type Err = &'static str;

    /// Constructs a new color from hex strings like the following:
    /// - `#aef` (shorthand, with leading hash),
    /// - `7a03c2` (without alpha),
    /// - `abcdefff` (with alpha).
    ///
    /// The hash is optional and both lower and upper case are fine.
    fn from_str(hex_str: &str) -> Result<Self, Self::Err> {
        let hex_str = hex_str.strip_prefix('#').unwrap_or(hex_str);
        if hex_str.chars().any(|c| !c.is_ascii_hexdigit()) {
            return Err("color string contains non-hexadecimal letters");
        }

        let len = hex_str.len();
        let long = len == 6 || len == 8;
        let short = len == 3 || len == 4;
        let alpha = len == 4 || len == 8;
        if !long && !short {
            return Err("color string has wrong length");
        }

        let mut values: [u8; 4] = [u8::MAX; 4];
        for elem in if alpha { 0..4 } else { 0..3 } {
            let item_len = if long { 2 } else { 1 };
            let pos = elem * item_len;

            let item = &hex_str[pos..(pos + item_len)];
            values[elem] = u8::from_str_radix(item, 16).unwrap();

            if short {
                // Duplicate number for shorthand notation, i.e. `a` -> `aa`
                values[elem] += values[elem] * 16;
            }
        }

        Ok(Self::from_u8(values[0], values[1], values[2], values[3]))
    }
}

impl From<Luma> for Color {
    fn from(c: Luma) -> Self {
        Self::Luma(c)
    }
}

impl From<Oklab> for Color {
    fn from(c: Oklab) -> Self {
        Self::Oklab(c)
    }
}

impl From<Oklch> for Color {
    fn from(c: Oklch) -> Self {
        Self::Oklch(c)
    }
}

impl From<Rgb> for Color {
    fn from(c: Rgb) -> Self {
        Self::Rgb(c)
    }
}

impl From<LinearRgb> for Color {
    fn from(c: LinearRgb) -> Self {
        Self::LinearRgb(c)
    }
}

impl From<Cmyk> for Color {
    fn from(c: Cmyk) -> Self {
        Self::Cmyk(c)
    }
}

impl From<Hsl> for Color {
    fn from(c: Hsl) -> Self {
        Self::Hsl(c)
    }
}

impl From<Hsv> for Color {
    fn from(c: Hsv) -> Self {
        Self::Hsv(c)
    }
}

/// An 8-bit CMYK color.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Cmyk {
    /// The cyan component.
    pub c: f32,
    /// The magenta component.
    pub m: f32,
    /// The yellow component.
    pub y: f32,
    /// The key (black) component.
    pub k: f32,
}

impl Cmyk {
    fn new(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self { c, m, y, k }
    }

    fn from_luma(luma: Luma) -> Self {
        let l = 1.0 - luma.luma;
        Cmyk::new(l * 0.75, l * 0.68, l * 0.67, l * 0.90)
    }

    // This still uses naive conversion, because qcms does not support
    // converting to CMYK yet.
    fn from_rgba(rgba: Rgb) -> Self {
        let r = rgba.red;
        let g = rgba.green;
        let b = rgba.blue;

        let k = 1.0 - r.max(g).max(b);
        if k == 1.0 {
            return Cmyk::new(0.0, 0.0, 0.0, 1.0);
        }

        let c = (1.0 - r - k) / (1.0 - k);
        let m = (1.0 - g - k) / (1.0 - k);
        let y = (1.0 - b - k) / (1.0 - k);

        Cmyk::new(c, m, y, k)
    }

    fn to_rgba(self) -> Rgb {
        let mut dest: [u8; 3] = [0; 3];
        TO_SRGB.convert(
            &[
                (self.c * 255.0).round() as u8,
                (self.m * 255.0).round() as u8,
                (self.y * 255.0).round() as u8,
                (self.k * 255.0).round() as u8,
            ],
            &mut dest,
        );

        Rgb::new(
            f32::from(dest[0]) / 255.0,
            f32::from(dest[1]) / 255.0,
            f32::from(dest[2]) / 255.0,
            1.0,
        )
    }

    fn lighten(self, factor: f32) -> Self {
        let lighten = |u: f32| (u - u * factor).clamp(0.0, 1.0);
        Self::new(lighten(self.c), lighten(self.m), lighten(self.y), lighten(self.k))
    }

    fn darken(self, factor: f32) -> Self {
        let darken = |u: f32| (u + (1.0 - u) * factor).clamp(0.0, 1.0);
        Self::new(darken(self.c), darken(self.m), darken(self.y), darken(self.k))
    }
}

/// A color with a weight.
pub struct WeightedColor {
    color: Color,
    weight: f64,
}

impl WeightedColor {
    /// Create a new weighted color.
    pub const fn new(color: Color, weight: f64) -> Self {
        Self { color, weight }
    }
}

cast! {
    WeightedColor,
    self => array![self.color, Value::Float(self.weight)].into_value(),
    color: Color => Self { color, weight: 1.0 },
    v: Array => {
        let mut iter = v.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(c), Some(w), None) => Self {
                color: c.cast()?,
                weight: w.cast::<Weight>()?.0,
            },
            _ => bail!("expected a color or color-weight pair"),
        }
    }
}

/// A weight for color mixing.
struct Weight(f64);

cast! {
    Weight,
    v: f64 => Self(v),
    v: Ratio => Self(v.get()),
}

/// A color space for color manipulation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ColorSpace {
    /// The perceptual Oklab color space.
    Oklab,
    /// The perceptual Oklch color space.
    Oklch,
    /// The standard RGB color space.
    Srgb,
    /// The D65-gray color space.
    D65Gray,
    /// The linear RGB color space.
    LinearRgb,
    /// The HSL color space.
    Hsl,
    /// The HSV color space.
    Hsv,
    /// The CMYK color space.
    Cmyk,
}

impl ColorSpace {
    /// Returns the index of the hue component in this color space, if it has
    /// one.
    pub fn hue_index(&self) -> Option<usize> {
        match self {
            Self::Hsl | Self::Hsv => Some(0),
            Self::Oklch => Some(2),
            _ => None,
        }
    }
}

cast! {
    ColorSpace,
    self => match self {
        Self::Oklab => Color::oklab_data(),
        Self::Oklch => Color::oklch_data(),
        Self::Srgb => Color::rgb_data(),
        Self::D65Gray => Color::luma_data(),
        Self::LinearRgb => Color::linear_rgb_data(),
        Self::Hsl => Color::hsl_data(),
        Self::Hsv => Color::hsv_data(),
        Self::Cmyk => Color::cmyk_data(),
    }.into_value(),
    v: Value => {
        let expected = "expected `rgb`, `luma`, `cmyk`, `oklab`, `oklch`, `color.linear-rgb`, `color.hsl`, or `color.hsv`";
        let Value::Func(func) = v else {
            bail!("{expected}, found {}", v.ty());
        };

        // Here comparing the function pointer since it's `Eq`
        // whereas the `NativeFuncData` is not.
        if func == Color::oklab_data() {
            Self::Oklab
        } else if func == Color::oklch_data() {
            Self::Oklch
        } else if func == Color::rgb_data() {
            Self::Srgb
        } else if func == Color::luma_data() {
            Self::D65Gray
        } else if func == Color::linear_rgb_data() {
            Self::LinearRgb
        } else if func == Color::hsl_data() {
            Self::Hsl
        } else if func == Color::hsv_data() {
            Self::Hsv
        } else if func == Color::cmyk_data() {
            Self::Cmyk
        } else {
            bail!("{expected}");
        }
    },
}

/// A component that must be a ratio.
pub struct RatioComponent(Ratio);

cast! {
    RatioComponent,
    self => self.0.into_value(),
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self(v)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
}

/// A chroma color component.
///
/// Must either be:
/// - a ratio, in which case it is relative to 0.4.
/// - a float, in which case it is taken literally.
pub struct ChromaComponent(f32);

cast! {
    ChromaComponent,
    v: f64 => Self(v as f32),
    v: Ratio => Self((v.get() * 0.4) as f32),
}

/// An integer or ratio component.
pub struct Component(Ratio);

cast! {
    Component,
    self => self.0.into_value(),
    v: i64 => match v {
        0 ..= 255 => Self(Ratio::new(v as f64 / 255.0)),
        _ => bail!("number must be between 0 and 255"),
    },
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self(v)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
}

/// A module with all preset color maps.
fn map() -> Module {
    let mut scope = Scope::new();
    scope.define("turbo", turbo());
    scope.define("cividis", cividis());
    scope.define("rainbow", rainbow());
    scope.define("spectral", spectral());
    scope.define("viridis", viridis());
    scope.define("inferno", inferno());
    scope.define("magma", magma());
    scope.define("plasma", plasma());
    scope.define("rocket", rocket());
    scope.define("mako", mako());
    scope.define("vlag", vlag());
    scope.define("icefire", icefire());
    scope.define("flare", flare());
    scope.define("crest", crest());
    Module::new("map", scope)
}

/// Defines a gradient preset as a series of colors expressed as u32s.
macro_rules! preset {
    ($name:ident; $($colors:literal),* $(,)*) => {
        fn $name() -> Array {
            Array::from(
                [$(Color::from_u32($colors)),*]
                    .iter()
                    .map(|c| c.into_value())
                    .collect::<EcoVec<_>>()
            )
        }
    };
}

preset!(turbo; 0x23171bff, 0x271a28ff, 0x2b1c33ff, 0x2f1e3fff, 0x32204aff, 0x362354ff, 0x39255fff, 0x3b2768ff, 0x3e2a72ff, 0x402c7bff, 0x422f83ff, 0x44318bff, 0x453493ff, 0x46369bff, 0x4839a2ff, 0x493ca8ff, 0x493eafff, 0x4a41b5ff, 0x4a44bbff, 0x4b46c0ff, 0x4b49c5ff, 0x4b4ccaff, 0x4b4ecfff, 0x4b51d3ff, 0x4a54d7ff, 0x4a56dbff, 0x4959deff, 0x495ce2ff, 0x485fe5ff, 0x4761e7ff, 0x4664eaff, 0x4567ecff, 0x446aeeff, 0x446df0ff, 0x426ff2ff, 0x4172f3ff, 0x4075f5ff, 0x3f78f6ff, 0x3e7af7ff, 0x3d7df7ff, 0x3c80f8ff, 0x3a83f9ff, 0x3985f9ff, 0x3888f9ff, 0x378bf9ff, 0x368df9ff, 0x3590f8ff, 0x3393f8ff, 0x3295f7ff, 0x3198f7ff, 0x309bf6ff, 0x2f9df5ff, 0x2ea0f4ff, 0x2da2f3ff, 0x2ca5f1ff, 0x2ba7f0ff, 0x2aaaefff, 0x2aacedff, 0x29afecff, 0x28b1eaff, 0x28b4e8ff, 0x27b6e6ff, 0x27b8e5ff, 0x26bbe3ff, 0x26bde1ff, 0x26bfdfff, 0x25c1dcff, 0x25c3daff, 0x25c6d8ff, 0x25c8d6ff, 0x25cad3ff, 0x25ccd1ff, 0x25cecfff, 0x26d0ccff, 0x26d2caff, 0x26d4c8ff, 0x27d6c5ff, 0x27d8c3ff, 0x28d9c0ff, 0x29dbbeff, 0x29ddbbff, 0x2adfb8ff, 0x2be0b6ff, 0x2ce2b3ff, 0x2de3b1ff, 0x2ee5aeff, 0x30e6acff, 0x31e8a9ff, 0x32e9a6ff, 0x34eba4ff, 0x35eca1ff, 0x37ed9fff, 0x39ef9cff, 0x3af09aff, 0x3cf197ff, 0x3ef295ff, 0x40f392ff, 0x42f490ff, 0x44f58dff, 0x46f68bff, 0x48f788ff, 0x4af786ff, 0x4df884ff, 0x4ff981ff, 0x51fa7fff, 0x54fa7dff, 0x56fb7aff, 0x59fb78ff, 0x5cfc76ff, 0x5efc74ff, 0x61fd71ff, 0x64fd6fff, 0x66fd6dff, 0x69fd6bff, 0x6cfd69ff, 0x6ffe67ff, 0x72fe65ff, 0x75fe63ff, 0x78fe61ff, 0x7bfe5fff, 0x7efd5dff, 0x81fd5cff, 0x84fd5aff, 0x87fd58ff, 0x8afc56ff, 0x8dfc55ff, 0x90fb53ff, 0x93fb51ff, 0x96fa50ff, 0x99fa4eff, 0x9cf94dff, 0x9ff84bff, 0xa2f84aff, 0xa6f748ff, 0xa9f647ff, 0xacf546ff, 0xaff444ff, 0xb2f343ff, 0xb5f242ff, 0xb8f141ff, 0xbbf03fff, 0xbeef3eff, 0xc1ed3dff, 0xc3ec3cff, 0xc6eb3bff, 0xc9e93aff, 0xcce839ff, 0xcfe738ff, 0xd1e537ff, 0xd4e336ff, 0xd7e235ff, 0xd9e034ff, 0xdcdf33ff, 0xdedd32ff, 0xe0db32ff, 0xe3d931ff, 0xe5d730ff, 0xe7d52fff, 0xe9d42fff, 0xecd22eff, 0xeed02dff, 0xf0ce2cff, 0xf1cb2cff, 0xf3c92bff, 0xf5c72bff, 0xf7c52aff, 0xf8c329ff, 0xfac029ff, 0xfbbe28ff, 0xfdbc28ff, 0xfeb927ff, 0xffb727ff, 0xffb526ff, 0xffb226ff, 0xffb025ff, 0xffad25ff, 0xffab24ff, 0xffa824ff, 0xffa623ff, 0xffa323ff, 0xffa022ff, 0xff9e22ff, 0xff9b21ff, 0xff9921ff, 0xff9621ff, 0xff9320ff, 0xff9020ff, 0xff8e1fff, 0xff8b1fff, 0xff881eff, 0xff851eff, 0xff831dff, 0xff801dff, 0xff7d1dff, 0xff7a1cff, 0xff781cff, 0xff751bff, 0xff721bff, 0xff6f1aff, 0xfd6c1aff, 0xfc6a19ff, 0xfa6719ff, 0xf96418ff, 0xf76118ff, 0xf65f18ff, 0xf45c17ff, 0xf25916ff, 0xf05716ff, 0xee5415ff, 0xec5115ff, 0xea4f14ff, 0xe84c14ff, 0xe64913ff, 0xe44713ff, 0xe24412ff, 0xdf4212ff, 0xdd3f11ff, 0xda3d10ff, 0xd83a10ff, 0xd5380fff, 0xd3360fff, 0xd0330eff, 0xce310dff, 0xcb2f0dff, 0xc92d0cff, 0xc62a0bff, 0xc3280bff, 0xc1260aff, 0xbe2409ff, 0xbb2309ff, 0xb92108ff, 0xb61f07ff, 0xb41d07ff, 0xb11b06ff, 0xaf1a05ff, 0xac1805ff, 0xaa1704ff, 0xa81604ff, 0xa51403ff, 0xa31302ff, 0xa11202ff, 0x9f1101ff, 0x9d1000ff, 0x9b0f00ff, 0x9a0e00ff, 0x980e00ff, 0x960d00ff, 0x950c00ff, 0x940c00ff, 0x930c00ff, 0x920c00ff, 0x910b00ff, 0x910c00ff, 0x900c00ff, 0x900c00ff, 0x900c00ff);
preset!(cividis; 0x002051ff, 0x002153ff, 0x002255ff, 0x002356ff, 0x002358ff, 0x002459ff, 0x00255aff, 0x00255cff, 0x00265dff, 0x00275eff, 0x00275fff, 0x002860ff, 0x002961ff, 0x002962ff, 0x002a63ff, 0x002b64ff, 0x012b65ff, 0x022c65ff, 0x032d66ff, 0x042d67ff, 0x052e67ff, 0x052f68ff, 0x063069ff, 0x073069ff, 0x08316aff, 0x09326aff, 0x0b326aff, 0x0c336bff, 0x0d346bff, 0x0e346bff, 0x0f356cff, 0x10366cff, 0x12376cff, 0x13376dff, 0x14386dff, 0x15396dff, 0x17396dff, 0x183a6dff, 0x193b6dff, 0x1a3b6dff, 0x1c3c6eff, 0x1d3d6eff, 0x1e3e6eff, 0x203e6eff, 0x213f6eff, 0x23406eff, 0x24406eff, 0x25416eff, 0x27426eff, 0x28436eff, 0x29436eff, 0x2b446eff, 0x2c456eff, 0x2e456eff, 0x2f466eff, 0x30476eff, 0x32486eff, 0x33486eff, 0x34496eff, 0x364a6eff, 0x374a6eff, 0x394b6eff, 0x3a4c6eff, 0x3b4d6eff, 0x3d4d6eff, 0x3e4e6eff, 0x3f4f6eff, 0x414f6eff, 0x42506eff, 0x43516dff, 0x44526dff, 0x46526dff, 0x47536dff, 0x48546dff, 0x4a546dff, 0x4b556dff, 0x4c566dff, 0x4d576dff, 0x4e576eff, 0x50586eff, 0x51596eff, 0x52596eff, 0x535a6eff, 0x545b6eff, 0x565c6eff, 0x575c6eff, 0x585d6eff, 0x595e6eff, 0x5a5e6eff, 0x5b5f6eff, 0x5c606eff, 0x5d616eff, 0x5e616eff, 0x60626eff, 0x61636fff, 0x62646fff, 0x63646fff, 0x64656fff, 0x65666fff, 0x66666fff, 0x67676fff, 0x686870ff, 0x696970ff, 0x6a6970ff, 0x6b6a70ff, 0x6c6b70ff, 0x6d6c70ff, 0x6d6c71ff, 0x6e6d71ff, 0x6f6e71ff, 0x706f71ff, 0x716f71ff, 0x727071ff, 0x737172ff, 0x747172ff, 0x757272ff, 0x767372ff, 0x767472ff, 0x777473ff, 0x787573ff, 0x797673ff, 0x7a7773ff, 0x7b7774ff, 0x7b7874ff, 0x7c7974ff, 0x7d7a74ff, 0x7e7a74ff, 0x7f7b75ff, 0x807c75ff, 0x807d75ff, 0x817d75ff, 0x827e75ff, 0x837f76ff, 0x848076ff, 0x858076ff, 0x858176ff, 0x868276ff, 0x878376ff, 0x888477ff, 0x898477ff, 0x898577ff, 0x8a8677ff, 0x8b8777ff, 0x8c8777ff, 0x8d8877ff, 0x8e8978ff, 0x8e8a78ff, 0x8f8a78ff, 0x908b78ff, 0x918c78ff, 0x928d78ff, 0x938e78ff, 0x938e78ff, 0x948f78ff, 0x959078ff, 0x969178ff, 0x979278ff, 0x989278ff, 0x999378ff, 0x9a9478ff, 0x9b9578ff, 0x9b9678ff, 0x9c9678ff, 0x9d9778ff, 0x9e9878ff, 0x9f9978ff, 0xa09a78ff, 0xa19a78ff, 0xa29b78ff, 0xa39c78ff, 0xa49d78ff, 0xa59e77ff, 0xa69e77ff, 0xa79f77ff, 0xa8a077ff, 0xa9a177ff, 0xaaa276ff, 0xaba376ff, 0xaca376ff, 0xada476ff, 0xaea575ff, 0xafa675ff, 0xb0a775ff, 0xb2a874ff, 0xb3a874ff, 0xb4a974ff, 0xb5aa73ff, 0xb6ab73ff, 0xb7ac72ff, 0xb8ad72ff, 0xbaae72ff, 0xbbae71ff, 0xbcaf71ff, 0xbdb070ff, 0xbeb170ff, 0xbfb26fff, 0xc1b36fff, 0xc2b46eff, 0xc3b56dff, 0xc4b56dff, 0xc5b66cff, 0xc7b76cff, 0xc8b86bff, 0xc9b96aff, 0xcaba6aff, 0xccbb69ff, 0xcdbc68ff, 0xcebc68ff, 0xcfbd67ff, 0xd1be66ff, 0xd2bf66ff, 0xd3c065ff, 0xd4c164ff, 0xd6c263ff, 0xd7c363ff, 0xd8c462ff, 0xd9c561ff, 0xdbc660ff, 0xdcc660ff, 0xddc75fff, 0xdec85eff, 0xe0c95dff, 0xe1ca5cff, 0xe2cb5cff, 0xe3cc5bff, 0xe4cd5aff, 0xe6ce59ff, 0xe7cf58ff, 0xe8d058ff, 0xe9d157ff, 0xead256ff, 0xebd355ff, 0xecd454ff, 0xedd453ff, 0xeed553ff, 0xf0d652ff, 0xf1d751ff, 0xf1d850ff, 0xf2d950ff, 0xf3da4fff, 0xf4db4eff, 0xf5dc4dff, 0xf6dd4dff, 0xf7de4cff, 0xf8df4bff, 0xf8e04bff, 0xf9e14aff, 0xfae249ff, 0xfae349ff, 0xfbe448ff, 0xfbe548ff, 0xfce647ff, 0xfce746ff, 0xfde846ff, 0xfde946ff, 0xfdea45ff);
preset!(rainbow;  0x7c4bbbff, 0x7f4bbcff, 0x824bbdff, 0x854abeff, 0x884abeff, 0x8b4abfff, 0x8e49bfff, 0x9149c0ff, 0x9449c0ff, 0x9748c0ff, 0x9a48c1ff, 0x9e48c1ff, 0xa148c1ff, 0xa447c1ff, 0xa747c1ff, 0xaa47c0ff, 0xad47c0ff, 0xb046c0ff, 0xb446bfff, 0xb746bfff, 0xba46beff, 0xbd46beff, 0xc046bdff, 0xc346bcff, 0xc646bbff, 0xc946baff, 0xcc46b9ff, 0xcf46b8ff, 0xd246b7ff, 0xd446b5ff, 0xd747b4ff, 0xda47b3ff, 0xdd47b1ff, 0xdf47b0ff, 0xe248aeff, 0xe448acff, 0xe748abff, 0xe949a9ff, 0xec49a7ff, 0xee4aa5ff, 0xf04ba3ff, 0xf34ba1ff, 0xf54c9fff, 0xf74c9dff, 0xf94d9bff, 0xfb4e98ff, 0xfd4f96ff, 0xfe5094ff, 0xff5191ff, 0xff528fff, 0xff538dff, 0xff548aff, 0xff5588ff, 0xff5685ff, 0xff5783ff, 0xff5880ff, 0xff5a7eff, 0xff5b7bff, 0xff5c79ff, 0xff5e76ff, 0xff5f74ff, 0xff6171ff, 0xff626fff, 0xff646cff, 0xff666aff, 0xff6767ff, 0xff6965ff, 0xff6b63ff, 0xff6d60ff, 0xff6e5eff, 0xff705cff, 0xff7259ff, 0xff7457ff, 0xff7655ff, 0xff7853ff, 0xff7a51ff, 0xff7c4fff, 0xff7f4dff, 0xff814bff, 0xff8349ff, 0xff8547ff, 0xff8745ff, 0xff8a44ff, 0xff8c42ff, 0xff8e40ff, 0xff913fff, 0xff933eff, 0xff953cff, 0xff983bff, 0xfd9a3aff, 0xfb9c39ff, 0xfa9f38ff, 0xf8a137ff, 0xf6a436ff, 0xf4a636ff, 0xf2a935ff, 0xf0ab35ff, 0xeeae34ff, 0xecb034ff, 0xeab234ff, 0xe8b534ff, 0xe6b734ff, 0xe4ba34ff, 0xe1bc34ff, 0xdfbf35ff, 0xddc135ff, 0xdbc336ff, 0xd9c636ff, 0xd6c837ff, 0xd4ca38ff, 0xd2cd39ff, 0xd0cf3aff, 0xcdd13bff, 0xcbd33dff, 0xc9d63eff, 0xc7d840ff, 0xc5da41ff, 0xc3dc43ff, 0xc1de45ff, 0xbfe047ff, 0xbde249ff, 0xbbe44bff, 0xb9e64dff, 0xb7e84fff, 0xb5ea52ff, 0xb3ec54ff, 0xb2ed57ff, 0xb0ef59ff, 0xadf05aff, 0xaaf15aff, 0xa6f159ff, 0xa2f259ff, 0x9ff259ff, 0x9bf358ff, 0x97f358ff, 0x94f459ff, 0x90f459ff, 0x8df559ff, 0x89f559ff, 0x85f65aff, 0x82f65bff, 0x7ff65bff, 0x7ef75cff, 0x7cf75dff, 0x7bf75eff, 0x7af75fff, 0x79f760ff, 0x78f762ff, 0x77f763ff, 0x76f764ff, 0x75f766ff, 0x74f768ff, 0x73f769ff, 0x72f76bff, 0x71f76dff, 0x70f76fff, 0x6ff671ff, 0x6ef673ff, 0x6df675ff, 0x6df577ff, 0x6cf579ff, 0x6bf47cff, 0x6af37eff, 0x69f380ff, 0x68f283ff, 0x67f185ff, 0x66f188ff, 0x66f08aff, 0x65ef8dff, 0x64ee8fff, 0x63ed92ff, 0x62ec94ff, 0x62eb97ff, 0x61ea9aff, 0x60e89cff, 0x5fe79fff, 0x5fe6a1ff, 0x5ee4a4ff, 0x5de3a7ff, 0x5ce2a9ff, 0x5ce0acff, 0x5bdfafff, 0x5addb1ff, 0x5adbb4ff, 0x59dab6ff, 0x58d8b9ff, 0x58d6bbff, 0x57d5beff, 0x56d3c0ff, 0x56d1c2ff, 0x55cfc5ff, 0x54cdc7ff, 0x54cbc9ff, 0x53c9cbff, 0x52c7cdff, 0x52c5cfff, 0x51c3d1ff, 0x51c1d3ff, 0x50bfd5ff, 0x50bdd7ff, 0x4fbbd9ff, 0x4eb9daff, 0x4eb6dcff, 0x4db4ddff, 0x4db2dfff, 0x4cb0e0ff, 0x4caee2ff, 0x4babe3ff, 0x4ba9e4ff, 0x4aa7e5ff, 0x4aa4e6ff, 0x49a2e7ff, 0x49a0e8ff, 0x489ee8ff, 0x489be9ff, 0x4799e9ff, 0x4797eaff, 0x4694eaff, 0x4692eaff, 0x4690ebff, 0x458eebff, 0x478bebff, 0x4889ebff, 0x4a87eaff, 0x4c85eaff, 0x4e82eaff, 0x5080e9ff, 0x527ee9ff, 0x537ce8ff, 0x557ae7ff, 0x5778e7ff, 0x5975e6ff, 0x5b73e5ff, 0x5c71e4ff, 0x5e6fe3ff, 0x606de1ff, 0x626be0ff, 0x6369dfff, 0x6567ddff, 0x6765dcff, 0x6864daff, 0x6a62d9ff, 0x6b60d7ff, 0x6d5ed5ff, 0x6e5cd3ff, 0x705bd1ff, 0x7159cfff, 0x7357cdff, 0x7456cbff, 0x7554c9ff, 0x7652c7ff, 0x7751c5ff, 0x794fc2ff, 0x7a4ec0ff, 0x7b4dbeff, 0x7c4bbbff);
preset!(spectral; 0x9e0142ff, 0xd53e4fff, 0xf46d43ff, 0xfdae61ff, 0xfee08bff, 0xffffbfff, 0xe6f598ff, 0xabdda4ff, 0x66c2a5ff, 0x3288bdff, 0x5e4fa2ff);
preset!(viridis; 0x440154ff, 0x482777ff, 0x3f4a8aff, 0x31678eff, 0x26838fff, 0x1f9d8aff, 0x6cce5aff, 0xb6de2bff, 0xfee825ff);
preset!(inferno; 0x000004ff, 0x170b3aff, 0x420a68ff, 0x6b176eff, 0x932667ff, 0xbb3654ff, 0xdd513aff, 0xf3771aff, 0xfca50aff, 0xf6d644ff, 0xfcffa4ff);
preset!(magma; 0x000004ff, 0x140e37ff, 0x3b0f70ff, 0x641a80ff, 0x8c2981ff, 0xb63679ff, 0xde4968ff, 0xf66f5cff, 0xfe9f6dff, 0xfece91ff, 0xfcfdbfff);
preset!(plasma; 0x0d0887ff, 0x42039dff, 0x6a00a8ff, 0x900da3ff, 0xb12a90ff, 0xcb4678ff, 0xe16462ff, 0xf1834bff, 0xfca636ff, 0xfccd25ff, 0xf0f921ff);
preset!(rocket; 0x3051aff, 0x4051aff, 0x5061bff, 0x6071cff, 0x7071dff, 0x8081eff, 0xa091fff, 0xb0920ff, 0xd0a21ff, 0xe0b22ff, 0x100b23ff, 0x110c24ff, 0x130d25ff, 0x140e26ff, 0x160e27ff, 0x170f28ff, 0x180f29ff, 0x1a102aff, 0x1b112bff, 0x1d112cff, 0x1e122dff, 0x20122eff, 0x211330ff, 0x221331ff, 0x241432ff, 0x251433ff, 0x271534ff, 0x281535ff, 0x2a1636ff, 0x2b1637ff, 0x2d1738ff, 0x2e1739ff, 0x30173aff, 0x31183bff, 0x33183cff, 0x34193dff, 0x35193eff, 0x37193fff, 0x381a40ff, 0x3a1a41ff, 0x3c1a42ff, 0x3d1a42ff, 0x3f1b43ff, 0x401b44ff, 0x421b45ff, 0x431c46ff, 0x451c47ff, 0x461c48ff, 0x481c48ff, 0x491d49ff, 0x4b1d4aff, 0x4c1d4bff, 0x4e1d4bff, 0x501d4cff, 0x511e4dff, 0x531e4dff, 0x541e4eff, 0x561e4fff, 0x581e4fff, 0x591e50ff, 0x5b1e51ff, 0x5c1e51ff, 0x5e1f52ff, 0x601f52ff, 0x611f53ff, 0x631f53ff, 0x641f54ff, 0x661f54ff, 0x681f55ff, 0x691f55ff, 0x6b1f56ff, 0x6d1f56ff, 0x6e1f57ff, 0x701f57ff, 0x711f57ff, 0x731f58ff, 0x751f58ff, 0x761f58ff, 0x781f59ff, 0x7a1f59ff, 0x7b1f59ff, 0x7d1f5aff, 0x7f1e5aff, 0x811e5aff, 0x821e5aff, 0x841e5aff, 0x861e5bff, 0x871e5bff, 0x891e5bff, 0x8b1d5bff, 0x8c1d5bff, 0x8e1d5bff, 0x901d5bff, 0x921c5bff, 0x931c5bff, 0x951c5bff, 0x971c5bff, 0x981b5bff, 0x9a1b5bff, 0x9c1b5bff, 0x9e1a5bff, 0x9f1a5bff, 0xa11a5bff, 0xa3195bff, 0xa4195bff, 0xa6195aff, 0xa8185aff, 0xaa185aff, 0xab185aff, 0xad1759ff, 0xaf1759ff, 0xb01759ff, 0xb21758ff, 0xb41658ff, 0xb51657ff, 0xb71657ff, 0xb91657ff, 0xba1656ff, 0xbc1656ff, 0xbd1655ff, 0xbf1654ff, 0xc11754ff, 0xc21753ff, 0xc41753ff, 0xc51852ff, 0xc71951ff, 0xc81951ff, 0xca1a50ff, 0xcb1b4fff, 0xcd1c4eff, 0xce1d4eff, 0xcf1e4dff, 0xd11f4cff, 0xd2204cff, 0xd3214bff, 0xd5224aff, 0xd62449ff, 0xd72549ff, 0xd82748ff, 0xd92847ff, 0xdb2946ff, 0xdc2b46ff, 0xdd2c45ff, 0xde2e44ff, 0xdf2f44ff, 0xe03143ff, 0xe13342ff, 0xe23442ff, 0xe33641ff, 0xe43841ff, 0xe53940ff, 0xe63b40ff, 0xe73d3fff, 0xe83f3fff, 0xe8403eff, 0xe9423eff, 0xea443eff, 0xeb463eff, 0xeb483eff, 0xec4a3eff, 0xec4c3eff, 0xed4e3eff, 0xed503eff, 0xee523fff, 0xee543fff, 0xef5640ff, 0xef5840ff, 0xef5a41ff, 0xf05c42ff, 0xf05e42ff, 0xf06043ff, 0xf16244ff, 0xf16445ff, 0xf16646ff, 0xf26747ff, 0xf26948ff, 0xf26b49ff, 0xf26d4bff, 0xf26f4cff, 0xf3714dff, 0xf3734eff, 0xf37450ff, 0xf37651ff, 0xf37852ff, 0xf47a54ff, 0xf47c55ff, 0xf47d57ff, 0xf47f58ff, 0xf4815aff, 0xf4835bff, 0xf4845dff, 0xf4865eff, 0xf58860ff, 0xf58a61ff, 0xf58b63ff, 0xf58d64ff, 0xf58f66ff, 0xf59067ff, 0xf59269ff, 0xf5946bff, 0xf5966cff, 0xf5976eff, 0xf59970ff, 0xf69b71ff, 0xf69c73ff, 0xf69e75ff, 0xf6a077ff, 0xf6a178ff, 0xf6a37aff, 0xf6a47cff, 0xf6a67eff, 0xf6a880ff, 0xf6a981ff, 0xf6ab83ff, 0xf6ad85ff, 0xf6ae87ff, 0xf6b089ff, 0xf6b18bff, 0xf6b38dff, 0xf6b48fff, 0xf6b691ff, 0xf6b893ff, 0xf6b995ff, 0xf6bb97ff, 0xf6bc99ff, 0xf6be9bff, 0xf6bf9dff, 0xf6c19fff, 0xf7c2a2ff, 0xf7c4a4ff, 0xf7c6a6ff, 0xf7c7a8ff, 0xf7c9aaff, 0xf7caacff, 0xf7ccafff, 0xf7cdb1ff, 0xf7cfb3ff, 0xf7d0b5ff, 0xf8d1b8ff, 0xf8d3baff, 0xf8d4bcff, 0xf8d6beff, 0xf8d7c0ff, 0xf8d9c3ff, 0xf8dac5ff, 0xf8dcc7ff, 0xf9ddc9ff, 0xf9dfcbff, 0xf9e0cdff, 0xf9e2d0ff, 0xf9e3d2ff, 0xf9e5d4ff, 0xfae6d6ff, 0xfae8d8ff, 0xfae9daff, 0xfaebddff);
preset!(mako; 0xb0405ff, 0xd0406ff, 0xe0508ff, 0xf0609ff, 0x10060aff, 0x11070cff, 0x12080dff, 0x13090fff, 0x140910ff, 0x150a12ff, 0x160b13ff, 0x170c15ff, 0x180d16ff, 0x190e18ff, 0x1a0e19ff, 0x1b0f1aff, 0x1c101cff, 0x1d111dff, 0x1e111fff, 0x1f1220ff, 0x201322ff, 0x211423ff, 0x221425ff, 0x231526ff, 0x241628ff, 0x251729ff, 0x26172bff, 0x27182dff, 0x28192eff, 0x291930ff, 0x291a31ff, 0x2a1b33ff, 0x2b1c35ff, 0x2c1c36ff, 0x2d1d38ff, 0x2e1e39ff, 0x2e1e3bff, 0x2f1f3dff, 0x30203eff, 0x312140ff, 0x312142ff, 0x322243ff, 0x332345ff, 0x342447ff, 0x342548ff, 0x35254aff, 0x35264cff, 0x36274dff, 0x37284fff, 0x372851ff, 0x382953ff, 0x382a54ff, 0x392b56ff, 0x3a2c58ff, 0x3a2c59ff, 0x3b2d5bff, 0x3b2e5dff, 0x3b2f5fff, 0x3c3060ff, 0x3c3162ff, 0x3d3164ff, 0x3d3266ff, 0x3e3367ff, 0x3e3469ff, 0x3e356bff, 0x3f366dff, 0x3f366fff, 0x3f3770ff, 0x403872ff, 0x403974ff, 0x403a76ff, 0x403b78ff, 0x403c79ff, 0x413d7bff, 0x413e7dff, 0x413e7fff, 0x413f80ff, 0x414082ff, 0x414184ff, 0x414285ff, 0x414387ff, 0x414488ff, 0x40468aff, 0x40478bff, 0x40488dff, 0x40498eff, 0x3f4a8fff, 0x3f4b90ff, 0x3f4c92ff, 0x3e4d93ff, 0x3e4f94ff, 0x3e5095ff, 0x3d5195ff, 0x3d5296ff, 0x3c5397ff, 0x3c5598ff, 0x3b5698ff, 0x3b5799ff, 0x3b589aff, 0x3a599aff, 0x3a5b9bff, 0x3a5c9bff, 0x395d9cff, 0x395e9cff, 0x385f9cff, 0x38619dff, 0x38629dff, 0x38639dff, 0x37649eff, 0x37659eff, 0x37669eff, 0x37689fff, 0x36699fff, 0x366a9fff, 0x366b9fff, 0x366ca0ff, 0x366da0ff, 0x366fa0ff, 0x3670a0ff, 0x3671a0ff, 0x3572a1ff, 0x3573a1ff, 0x3574a1ff, 0x3575a1ff, 0x3576a2ff, 0x3578a2ff, 0x3579a2ff, 0x357aa2ff, 0x357ba3ff, 0x357ca3ff, 0x357da3ff, 0x357ea4ff, 0x347fa4ff, 0x3480a4ff, 0x3482a4ff, 0x3483a5ff, 0x3484a5ff, 0x3485a5ff, 0x3486a5ff, 0x3487a6ff, 0x3488a6ff, 0x3489a6ff, 0x348ba6ff, 0x348ca7ff, 0x348da7ff, 0x348ea7ff, 0x348fa7ff, 0x3490a8ff, 0x3491a8ff, 0x3492a8ff, 0x3493a8ff, 0x3495a9ff, 0x3496a9ff, 0x3497a9ff, 0x3498a9ff, 0x3499aaff, 0x349aaaff, 0x359baaff, 0x359caaff, 0x359eaaff, 0x359fabff, 0x35a0abff, 0x35a1abff, 0x36a2abff, 0x36a3abff, 0x36a4abff, 0x37a5acff, 0x37a6acff, 0x37a8acff, 0x38a9acff, 0x38aaacff, 0x39abacff, 0x39acacff, 0x3aadacff, 0x3aaeadff, 0x3bafadff, 0x3cb1adff, 0x3cb2adff, 0x3db3adff, 0x3eb4adff, 0x3fb5adff, 0x3fb6adff, 0x40b7adff, 0x41b8adff, 0x42b9adff, 0x43baadff, 0x44bcadff, 0x45bdadff, 0x46beadff, 0x47bfadff, 0x48c0adff, 0x49c1adff, 0x4bc2adff, 0x4cc3adff, 0x4dc4adff, 0x4fc5adff, 0x50c6adff, 0x52c7adff, 0x53c9adff, 0x55caadff, 0x57cbadff, 0x59ccadff, 0x5bcdadff, 0x5ecdadff, 0x60ceacff, 0x62cfacff, 0x65d0adff, 0x68d1adff, 0x6ad2adff, 0x6dd3adff, 0x70d4adff, 0x73d4adff, 0x76d5aeff, 0x79d6aeff, 0x7cd6afff, 0x7fd7afff, 0x82d8b0ff, 0x85d9b1ff, 0x88d9b1ff, 0x8bdab2ff, 0x8edbb3ff, 0x91dbb4ff, 0x94dcb5ff, 0x96ddb5ff, 0x99ddb6ff, 0x9cdeb7ff, 0x9edfb8ff, 0xa1dfb9ff, 0xa4e0bbff, 0xa6e1bcff, 0xa9e1bdff, 0xabe2beff, 0xaee3c0ff, 0xb0e4c1ff, 0xb2e4c2ff, 0xb5e5c4ff, 0xb7e6c5ff, 0xb9e6c7ff, 0xbbe7c8ff, 0xbee8caff, 0xc0e9ccff, 0xc2e9cdff, 0xc4eacfff, 0xc6ebd1ff, 0xc8ecd2ff, 0xcaedd4ff, 0xccedd6ff, 0xceeed7ff, 0xd0efd9ff, 0xd2f0dbff, 0xd4f1dcff, 0xd6f1deff, 0xd8f2e0ff, 0xdaf3e1ff, 0xdcf4e3ff, 0xdef5e5ff);
preset!(vlag; 0x2369bdff, 0x266abdff, 0x296cbcff, 0x2c6dbcff, 0x2f6ebcff, 0x316fbcff, 0x3470bcff, 0x3671bcff, 0x3972bcff, 0x3b73bcff, 0x3d74bcff, 0x3f75bcff, 0x4276bcff, 0x4477bcff, 0x4678bcff, 0x4879bcff, 0x4a7bbcff, 0x4c7cbcff, 0x4e7dbcff, 0x507ebcff, 0x517fbcff, 0x5380bcff, 0x5581bcff, 0x5782bcff, 0x5983bdff, 0x5b84bdff, 0x5c85bdff, 0x5e86bdff, 0x6087bdff, 0x6288bdff, 0x6489beff, 0x658abeff, 0x678bbeff, 0x698cbeff, 0x6a8dbfff, 0x6c8ebfff, 0x6e90bfff, 0x6f91bfff, 0x7192c0ff, 0x7393c0ff, 0x7594c0ff, 0x7695c1ff, 0x7896c1ff, 0x7997c1ff, 0x7b98c2ff, 0x7d99c2ff, 0x7e9ac2ff, 0x809bc3ff, 0x829cc3ff, 0x839dc4ff, 0x859ec4ff, 0x87a0c4ff, 0x88a1c5ff, 0x8aa2c5ff, 0x8ba3c6ff, 0x8da4c6ff, 0x8fa5c7ff, 0x90a6c7ff, 0x92a7c8ff, 0x93a8c8ff, 0x95a9c8ff, 0x97abc9ff, 0x98acc9ff, 0x9aadcaff, 0x9baecbff, 0x9dafcbff, 0x9fb0ccff, 0xa0b1ccff, 0xa2b2cdff, 0xa3b4cdff, 0xa5b5ceff, 0xa7b6ceff, 0xa8b7cfff, 0xaab8d0ff, 0xabb9d0ff, 0xadbbd1ff, 0xafbcd1ff, 0xb0bdd2ff, 0xb2bed3ff, 0xb3bfd3ff, 0xb5c0d4ff, 0xb7c2d5ff, 0xb8c3d5ff, 0xbac4d6ff, 0xbbc5d7ff, 0xbdc6d7ff, 0xbfc8d8ff, 0xc0c9d9ff, 0xc2cadaff, 0xc3cbdaff, 0xc5cddbff, 0xc7cedcff, 0xc8cfddff, 0xcad0ddff, 0xcbd1deff, 0xcdd3dfff, 0xcfd4e0ff, 0xd0d5e0ff, 0xd2d7e1ff, 0xd4d8e2ff, 0xd5d9e3ff, 0xd7dae4ff, 0xd9dce5ff, 0xdadde5ff, 0xdcdee6ff, 0xdde0e7ff, 0xdfe1e8ff, 0xe1e2e9ff, 0xe2e3eaff, 0xe4e5ebff, 0xe6e6ecff, 0xe7e7ecff, 0xe9e9edff, 0xebeaeeff, 0xecebefff, 0xeeedf0ff, 0xefeef1ff, 0xf1eff2ff, 0xf2f0f2ff, 0xf3f1f3ff, 0xf5f2f4ff, 0xf6f3f4ff, 0xf7f4f4ff, 0xf8f4f5ff, 0xf9f5f5ff, 0xf9f5f5ff, 0xfaf5f5ff, 0xfaf5f5ff, 0xfaf5f4ff, 0xfaf5f4ff, 0xfaf4f3ff, 0xfaf3f3ff, 0xfaf3f2ff, 0xfaf2f1ff, 0xfaf0efff, 0xf9efeeff, 0xf9eeedff, 0xf8edebff, 0xf7ebeaff, 0xf7eae8ff, 0xf6e8e7ff, 0xf5e7e5ff, 0xf5e5e4ff, 0xf4e3e2ff, 0xf3e2e0ff, 0xf2e0dfff, 0xf2dfddff, 0xf1dddbff, 0xf0dbdaff, 0xefdad8ff, 0xefd8d6ff, 0xeed7d5ff, 0xedd5d3ff, 0xecd3d2ff, 0xecd2d0ff, 0xebd0ceff, 0xeacfcdff, 0xeacdcbff, 0xe9cbc9ff, 0xe8cac8ff, 0xe7c8c6ff, 0xe7c7c5ff, 0xe6c5c3ff, 0xe5c3c1ff, 0xe5c2c0ff, 0xe4c0beff, 0xe3bfbdff, 0xe3bdbbff, 0xe2bcb9ff, 0xe1bab8ff, 0xe1b9b6ff, 0xe0b7b5ff, 0xdfb5b3ff, 0xdfb4b2ff, 0xdeb2b0ff, 0xdeb1aeff, 0xddafadff, 0xdcaeabff, 0xdcacaaff, 0xdbaba8ff, 0xdaa9a7ff, 0xdaa8a5ff, 0xd9a6a4ff, 0xd9a5a2ff, 0xd8a3a0ff, 0xd7a29fff, 0xd7a09dff, 0xd69f9cff, 0xd59d9aff, 0xd59c99ff, 0xd49a97ff, 0xd49896ff, 0xd39794ff, 0xd29593ff, 0xd29491ff, 0xd19290ff, 0xd1918eff, 0xd08f8dff, 0xcf8e8bff, 0xcf8c8aff, 0xce8b88ff, 0xcd8987ff, 0xcd8885ff, 0xcc8784ff, 0xcc8582ff, 0xcb8481ff, 0xca827fff, 0xca817eff, 0xc97f7dff, 0xc87e7bff, 0xc87c7aff, 0xc77b78ff, 0xc77977ff, 0xc67875ff, 0xc57674ff, 0xc57572ff, 0xc47371ff, 0xc3726fff, 0xc3706eff, 0xc26f6dff, 0xc16d6bff, 0xc16c6aff, 0xc06a68ff, 0xc06967ff, 0xbf6765ff, 0xbe6664ff, 0xbe6463ff, 0xbd6361ff, 0xbc6160ff, 0xbc605eff, 0xbb5e5dff, 0xba5d5cff, 0xb95b5aff, 0xb95a59ff, 0xb85857ff, 0xb75756ff, 0xb75555ff, 0xb65453ff, 0xb55252ff, 0xb55151ff, 0xb44f4fff, 0xb34d4eff, 0xb24c4cff, 0xb24a4bff, 0xb1494aff, 0xb04748ff, 0xaf4647ff, 0xaf4446ff, 0xae4244ff, 0xad4143ff, 0xac3f42ff, 0xac3e40ff, 0xab3c3fff, 0xaa3a3eff, 0xa9393cff, 0xa9373bff);
preset!(icefire; 0xbde7dbff, 0xbae5daff, 0xb7e3d9ff, 0xb4e1d9ff, 0xb2dfd8ff, 0xafddd7ff, 0xacdbd7ff, 0xa9d9d6ff, 0xa7d7d5ff, 0xa4d5d5ff, 0xa1d3d4ff, 0x9ed1d3ff, 0x9bcfd3ff, 0x98cdd2ff, 0x95cbd2ff, 0x93cad1ff, 0x90c8d1ff, 0x8dc6d0ff, 0x8ac4d0ff, 0x87c2cfff, 0x84c1cfff, 0x81bfcfff, 0x7ebdceff, 0x7bbbceff, 0x78b9ceff, 0x75b8ceff, 0x72b6ceff, 0x6eb4cdff, 0x6bb2cdff, 0x68b0cdff, 0x65afcdff, 0x63adcdff, 0x60abcdff, 0x5da9cdff, 0x5aa7cdff, 0x58a5cdff, 0x55a3cdff, 0x53a2cdff, 0x50a0cdff, 0x4e9ecdff, 0x4c9ccdff, 0x499aceff, 0x4798ceff, 0x4596ceff, 0x4394ceff, 0x4192ceff, 0x3f90ceff, 0x3e8ecfff, 0x3c8ccfff, 0x3a89cfff, 0x3987cfff, 0x3885d0ff, 0x3783d0ff, 0x3781d0ff, 0x377fd0ff, 0x377cd0ff, 0x377ad0ff, 0x3878cfff, 0x3975cfff, 0x3a73ceff, 0x3b71cdff, 0x3d6eccff, 0x3e6ccbff, 0x3f69c9ff, 0x4167c7ff, 0x4265c5ff, 0x4363c3ff, 0x4560c1ff, 0x465ebeff, 0x475cbcff, 0x475ab9ff, 0x4858b6ff, 0x4956b3ff, 0x4954b0ff, 0x4952adff, 0x4a50a9ff, 0x4a4fa5ff, 0x494da1ff, 0x494c9eff, 0x494a9aff, 0x484996ff, 0x474792ff, 0x47468eff, 0x46458aff, 0x454386ff, 0x444282ff, 0x43417fff, 0x42407bff, 0x413e77ff, 0x3f3d74ff, 0x3e3c70ff, 0x3d3b6dff, 0x3c3a69ff, 0x3b3866ff, 0x393763ff, 0x38365fff, 0x37355cff, 0x363459ff, 0x343356ff, 0x333153ff, 0x323050ff, 0x312f4dff, 0x302e4aff, 0x2e2d48ff, 0x2d2c45ff, 0x2c2b42ff, 0x2b2a40ff, 0x2a293dff, 0x29283bff, 0x282739ff, 0x272636ff, 0x262534ff, 0x252532ff, 0x242430ff, 0x24232eff, 0x23222dff, 0x22222bff, 0x222129ff, 0x212028ff, 0x212026ff, 0x202025ff, 0x201f24ff, 0x1f1f23ff, 0x1f1f21ff, 0x1f1e21ff, 0x1f1e20ff, 0x1f1e1fff, 0x1f1e1eff, 0x1f1e1eff, 0x201e1eff, 0x211e1eff, 0x221e1eff, 0x231e1eff, 0x251e1fff, 0x261e1fff, 0x271e1fff, 0x291e20ff, 0x2a1e20ff, 0x2c1e21ff, 0x2d1f21ff, 0x2f1f22ff, 0x311f23ff, 0x332023ff, 0x352024ff, 0x372025ff, 0x392126ff, 0x3b2127ff, 0x3d2228ff, 0x3f2228ff, 0x412329ff, 0x43232aff, 0x46242bff, 0x48242cff, 0x4a252eff, 0x4d252fff, 0x4f2630ff, 0x522731ff, 0x542732ff, 0x572833ff, 0x5a2834ff, 0x5c2935ff, 0x5f2936ff, 0x622937ff, 0x642a38ff, 0x672a39ff, 0x6a2b3aff, 0x6d2b3bff, 0x702b3cff, 0x722c3dff, 0x752c3eff, 0x782c3fff, 0x7b2d40ff, 0x7e2d40ff, 0x812d41ff, 0x842d42ff, 0x872d42ff, 0x8a2e43ff, 0x8d2e43ff, 0x902e44ff, 0x932e44ff, 0x962e44ff, 0x992e44ff, 0x9c2f45ff, 0x9f2f44ff, 0xa22f44ff, 0xa52f44ff, 0xa83044ff, 0xab3043ff, 0xae3143ff, 0xb13242ff, 0xb33341ff, 0xb63441ff, 0xb93540ff, 0xbb363fff, 0xbe373eff, 0xc0393dff, 0xc33a3cff, 0xc53c3cff, 0xc73d3bff, 0xc93f3aff, 0xcc4139ff, 0xce4338ff, 0xd04537ff, 0xd24737ff, 0xd34936ff, 0xd54b35ff, 0xd74e35ff, 0xd95034ff, 0xda5334ff, 0xdc5534ff, 0xde5733ff, 0xdf5a33ff, 0xe15c33ff, 0xe25f33ff, 0xe36233ff, 0xe56433ff, 0xe66734ff, 0xe76a34ff, 0xe86d35ff, 0xe96f36ff, 0xea7238ff, 0xeb753aff, 0xec783bff, 0xed7b3eff, 0xed7e40ff, 0xee8142ff, 0xef8445ff, 0xef8748ff, 0xf0894bff, 0xf18c4eff, 0xf18f51ff, 0xf29255ff, 0xf29558ff, 0xf3985bff, 0xf39a5fff, 0xf49d63ff, 0xf5a066ff, 0xf5a36aff, 0xf6a56dff, 0xf6a871ff, 0xf7ab75ff, 0xf7ae79ff, 0xf8b07cff, 0xf8b380ff, 0xf9b684ff, 0xfab887ff, 0xfabb8bff, 0xfbbe8fff, 0xfbc192ff, 0xfcc396ff, 0xfcc69aff, 0xfdc99eff, 0xfdcca1ff, 0xfecea5ff, 0xfed1a9ff, 0xffd4acff);
preset!(flare; 0xedb081ff, 0xedaf80ff, 0xedae7fff, 0xedad7fff, 0xedac7eff, 0xedab7eff, 0xecaa7dff, 0xeca97cff, 0xeca87cff, 0xeca77bff, 0xeca67bff, 0xeca57aff, 0xeca479ff, 0xeca379ff, 0xeca278ff, 0xeca178ff, 0xeca077ff, 0xec9f76ff, 0xeb9e76ff, 0xeb9d75ff, 0xeb9c75ff, 0xeb9b74ff, 0xeb9a73ff, 0xeb9973ff, 0xeb9972ff, 0xeb9872ff, 0xeb9771ff, 0xea9671ff, 0xea9570ff, 0xea946fff, 0xea936fff, 0xea926eff, 0xea916eff, 0xea906dff, 0xea8f6cff, 0xea8e6cff, 0xe98d6bff, 0xe98c6bff, 0xe98b6aff, 0xe98a6aff, 0xe98969ff, 0xe98868ff, 0xe98768ff, 0xe98667ff, 0xe88567ff, 0xe88466ff, 0xe88366ff, 0xe88265ff, 0xe88165ff, 0xe88064ff, 0xe87f64ff, 0xe77e63ff, 0xe77d63ff, 0xe77c63ff, 0xe77b62ff, 0xe77a62ff, 0xe67961ff, 0xe67861ff, 0xe67760ff, 0xe67660ff, 0xe67560ff, 0xe5745fff, 0xe5735fff, 0xe5725fff, 0xe5715eff, 0xe5705eff, 0xe46f5eff, 0xe46e5eff, 0xe46d5dff, 0xe46c5dff, 0xe36b5dff, 0xe36a5dff, 0xe3695dff, 0xe3685cff, 0xe2675cff, 0xe2665cff, 0xe2655cff, 0xe1645cff, 0xe1635cff, 0xe1625cff, 0xe0615cff, 0xe0605cff, 0xe05f5cff, 0xdf5f5cff, 0xdf5e5cff, 0xde5d5cff, 0xde5c5cff, 0xde5b5cff, 0xdd5a5cff, 0xdd595cff, 0xdc585cff, 0xdc575cff, 0xdb565dff, 0xdb565dff, 0xda555dff, 0xda545dff, 0xd9535dff, 0xd9525eff, 0xd8525eff, 0xd7515eff, 0xd7505eff, 0xd64f5fff, 0xd64f5fff, 0xd54e5fff, 0xd44d60ff, 0xd44c60ff, 0xd34c60ff, 0xd24b60ff, 0xd24a61ff, 0xd14a61ff, 0xd04962ff, 0xd04962ff, 0xcf4862ff, 0xce4763ff, 0xcd4763ff, 0xcc4663ff, 0xcc4664ff, 0xcb4564ff, 0xca4564ff, 0xc94465ff, 0xc84465ff, 0xc84365ff, 0xc74366ff, 0xc64366ff, 0xc54266ff, 0xc44267ff, 0xc34167ff, 0xc24167ff, 0xc14168ff, 0xc14068ff, 0xc04068ff, 0xbf4069ff, 0xbe3f69ff, 0xbd3f69ff, 0xbc3f69ff, 0xbb3f6aff, 0xba3e6aff, 0xb93e6aff, 0xb83e6bff, 0xb73d6bff, 0xb63d6bff, 0xb53d6bff, 0xb43d6bff, 0xb33c6cff, 0xb23c6cff, 0xb13c6cff, 0xb13c6cff, 0xb03b6dff, 0xaf3b6dff, 0xae3b6dff, 0xad3b6dff, 0xac3a6dff, 0xab3a6dff, 0xaa3a6eff, 0xa93a6eff, 0xa8396eff, 0xa7396eff, 0xa6396eff, 0xa5396eff, 0xa4386fff, 0xa3386fff, 0xa2386fff, 0xa1386fff, 0xa1376fff, 0xa0376fff, 0x9f376fff, 0x9e3770ff, 0x9d3670ff, 0x9c3670ff, 0x9b3670ff, 0x9a3670ff, 0x993570ff, 0x983570ff, 0x973570ff, 0x963570ff, 0x953470ff, 0x943470ff, 0x943471ff, 0x933471ff, 0x923371ff, 0x913371ff, 0x903371ff, 0x8f3371ff, 0x8e3271ff, 0x8d3271ff, 0x8c3271ff, 0x8b3271ff, 0x8a3171ff, 0x893171ff, 0x883171ff, 0x873171ff, 0x873171ff, 0x863071ff, 0x853071ff, 0x843071ff, 0x833070ff, 0x822f70ff, 0x812f70ff, 0x802f70ff, 0x7f2f70ff, 0x7e2f70ff, 0x7d2e70ff, 0x7c2e70ff, 0x7b2e70ff, 0x7a2e70ff, 0x792e6fff, 0x782e6fff, 0x772d6fff, 0x762d6fff, 0x752d6fff, 0x752d6fff, 0x742d6eff, 0x732c6eff, 0x722c6eff, 0x712c6eff, 0x702c6eff, 0x6f2c6dff, 0x6e2c6dff, 0x6d2b6dff, 0x6c2b6dff, 0x6b2b6cff, 0x6a2b6cff, 0x692b6cff, 0x682a6cff, 0x672a6bff, 0x662a6bff, 0x652a6bff, 0x642a6aff, 0x642a6aff, 0x63296aff, 0x62296aff, 0x612969ff, 0x602969ff, 0x5f2969ff, 0x5e2868ff, 0x5d2868ff, 0x5c2868ff, 0x5b2867ff, 0x5a2767ff, 0x592767ff, 0x582766ff, 0x582766ff, 0x572766ff, 0x562666ff, 0x552665ff, 0x542665ff, 0x532665ff, 0x522564ff, 0x512564ff, 0x502564ff, 0x4f2463ff, 0x4f2463ff, 0x4e2463ff, 0x4d2463ff, 0x4c2362ff, 0x4b2362ff);
preset!(crest; 0xa5cd90ff, 0xa4cc90ff, 0xa3cc91ff, 0xa2cb91ff, 0xa0cb91ff, 0x9fca91ff, 0x9eca91ff, 0x9dc991ff, 0x9cc891ff, 0x9bc891ff, 0x9ac791ff, 0x99c791ff, 0x98c691ff, 0x96c691ff, 0x95c591ff, 0x94c591ff, 0x93c491ff, 0x92c491ff, 0x91c391ff, 0x90c391ff, 0x8fc291ff, 0x8ec291ff, 0x8dc191ff, 0x8bc191ff, 0x8ac091ff, 0x89bf91ff, 0x88bf91ff, 0x87be91ff, 0x86be91ff, 0x85bd91ff, 0x84bd91ff, 0x82bc91ff, 0x81bc91ff, 0x80bb91ff, 0x7fbb91ff, 0x7eba91ff, 0x7dba91ff, 0x7cb991ff, 0x7bb991ff, 0x79b891ff, 0x78b891ff, 0x77b791ff, 0x76b791ff, 0x75b690ff, 0x74b690ff, 0x73b590ff, 0x72b490ff, 0x71b490ff, 0x70b390ff, 0x6fb390ff, 0x6eb290ff, 0x6db290ff, 0x6cb190ff, 0x6bb190ff, 0x6ab090ff, 0x69b090ff, 0x68af90ff, 0x67ae90ff, 0x66ae90ff, 0x65ad90ff, 0x64ad90ff, 0x63ac90ff, 0x62ac90ff, 0x62ab90ff, 0x61aa90ff, 0x60aa90ff, 0x5fa990ff, 0x5ea990ff, 0x5da890ff, 0x5ca890ff, 0x5ba790ff, 0x5ba690ff, 0x5aa690ff, 0x59a590ff, 0x58a590ff, 0x57a490ff, 0x57a490ff, 0x56a390ff, 0x55a290ff, 0x54a290ff, 0x53a190ff, 0x53a190ff, 0x52a090ff, 0x519f90ff, 0x509f90ff, 0x509e90ff, 0x4f9e90ff, 0x4e9d90ff, 0x4e9d90ff, 0x4d9c90ff, 0x4c9b90ff, 0x4b9b90ff, 0x4b9a8fff, 0x4a9a8fff, 0x49998fff, 0x49988fff, 0x48988fff, 0x47978fff, 0x47978fff, 0x46968fff, 0x45958fff, 0x45958fff, 0x44948fff, 0x43948fff, 0x43938fff, 0x42928fff, 0x41928fff, 0x41918fff, 0x40918fff, 0x40908eff, 0x3f8f8eff, 0x3e8f8eff, 0x3e8e8eff, 0x3d8e8eff, 0x3c8d8eff, 0x3c8c8eff, 0x3b8c8eff, 0x3a8b8eff, 0x3a8b8eff, 0x398a8eff, 0x388a8eff, 0x38898eff, 0x37888eff, 0x37888dff, 0x36878dff, 0x35878dff, 0x35868dff, 0x34858dff, 0x33858dff, 0x33848dff, 0x32848dff, 0x31838dff, 0x31828dff, 0x30828dff, 0x2f818dff, 0x2f818dff, 0x2e808dff, 0x2d808cff, 0x2d7f8cff, 0x2c7e8cff, 0x2c7e8cff, 0x2b7d8cff, 0x2a7d8cff, 0x2a7c8cff, 0x297b8cff, 0x287b8cff, 0x287a8cff, 0x277a8cff, 0x27798cff, 0x26788cff, 0x25788cff, 0x25778cff, 0x24778bff, 0x24768bff, 0x23758bff, 0x23758bff, 0x22748bff, 0x22748bff, 0x21738bff, 0x21728bff, 0x20728bff, 0x20718bff, 0x20718bff, 0x1f708bff, 0x1f6f8aff, 0x1e6f8aff, 0x1e6e8aff, 0x1e6d8aff, 0x1e6d8aff, 0x1d6c8aff, 0x1d6c8aff, 0x1d6b8aff, 0x1d6a8aff, 0x1d6a8aff, 0x1c6989ff, 0x1c6889ff, 0x1c6889ff, 0x1c6789ff, 0x1c6689ff, 0x1c6689ff, 0x1c6589ff, 0x1c6488ff, 0x1c6488ff, 0x1c6388ff, 0x1d6388ff, 0x1d6288ff, 0x1d6188ff, 0x1d6187ff, 0x1d6087ff, 0x1d5f87ff, 0x1d5f87ff, 0x1e5e87ff, 0x1e5d86ff, 0x1e5d86ff, 0x1e5c86ff, 0x1e5b86ff, 0x1f5b86ff, 0x1f5a85ff, 0x1f5985ff, 0x1f5985ff, 0x205885ff, 0x205784ff, 0x205784ff, 0x205684ff, 0x215584ff, 0x215583ff, 0x215483ff, 0x225383ff, 0x225283ff, 0x225282ff, 0x225182ff, 0x235082ff, 0x235081ff, 0x234f81ff, 0x244e81ff, 0x244e80ff, 0x244d80ff, 0x254c80ff, 0x254c7fff, 0x254b7fff, 0x254a7fff, 0x26497eff, 0x26497eff, 0x26487eff, 0x27477dff, 0x27477dff, 0x27467cff, 0x27457cff, 0x28457cff, 0x28447bff, 0x28437bff, 0x28427aff, 0x29427aff, 0x29417aff, 0x294079ff, 0x294079ff, 0x2a3f78ff, 0x2a3e78ff, 0x2a3d78ff, 0x2a3d77ff, 0x2a3c77ff, 0x2a3b76ff, 0x2b3b76ff, 0x2b3a76ff, 0x2b3975ff, 0x2b3875ff, 0x2b3875ff, 0x2b3774ff, 0x2b3674ff, 0x2c3574ff, 0x2c3573ff, 0x2c3473ff, 0x2c3373ff, 0x2c3272ff, 0x2c3172ff, 0x2c3172ff);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_strings() {
        #[track_caller]
        fn test(hex: &str, r: u8, g: u8, b: u8, a: u8) {
            assert_eq!(Color::from_str(hex), Ok(Color::from_u8(r, g, b, a)));
        }

        test("f61243ff", 0xf6, 0x12, 0x43, 255);
        test("b3d8b3", 0xb3, 0xd8, 0xb3, 255);
        test("fCd2a9AD", 0xfc, 0xd2, 0xa9, 0xad);
        test("233", 0x22, 0x33, 0x33, 255);
        test("111b", 0x11, 0x11, 0x11, 0xbb);
    }

    #[test]
    fn test_parse_invalid_colors() {
        #[track_caller]
        fn test(hex: &str, message: &str) {
            assert_eq!(Color::from_str(hex), Err(message));
        }

        test("a5", "color string has wrong length");
        test("12345", "color string has wrong length");
        test("f075ff011", "color string has wrong length");
        test("hmmm", "color string contains non-hexadecimal letters");
        test("14B2AH", "color string contains non-hexadecimal letters");
    }
}
