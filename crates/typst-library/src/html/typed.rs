//! The typed HTML element API (e.g. `html.div`).
//!
//! The typed API is backed by generated data derived from the HTML
//! specification. See [generated] and `tools/codegen`.

use std::fmt::Write;
use std::marker::PhantomData;
use std::num::{NonZeroI64, NonZeroU64};
use std::sync::LazyLock;

use bumpalo::Bump;
use comemo::Tracked;
use ecow::{eco_format, eco_vec, EcoString};
use typst_macros::cast;

use crate::diag::{bail, At, Hint, HintedStrResult, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    Args, Array, CastInfo, Content, Context, Datetime, Dict, Duration, FromValue,
    IntoValue, NativeFuncData, NativeFuncPtr, ParamInfo, PositiveF64, Reflect, Scope,
    Smart, Str, Type, Value,
};
use crate::html::{generated, tag};
use crate::html::{HtmlAttr, HtmlAttrs, HtmlElem, HtmlTag};
use crate::layout::{Axes, Axis, Dir, Length};
use crate::visualize::Color;

/// Hook up all typed HTML definitions.
pub(super) fn define(html: &mut Scope) {
    for data in FUNCS.iter() {
        html.define_func_with_data(data);
    }
}

/// Lazily created functions for all typed HTML constructors.
static FUNCS: LazyLock<&'static [NativeFuncData]> = LazyLock::new(|| {
    // Leaking is okay here. It's not meaningfully different from having
    // memory-managed values as `FUNCS` is a static.
    let bump = Box::leak(Box::new(Bump::new()));
    Vec::leak(
        generated::ELEMENTS
            .iter()
            .map(|info| create_func_data(info, bump))
            .collect(),
    )
});

/// Creates metadata for a native HTML element constructor function.
fn create_func_data(
    element: &'static ElementInfo,
    bump: &'static Bump,
) -> NativeFuncData {
    NativeFuncData {
        function: NativeFuncPtr(bump.alloc(
            move |_: &mut Engine, _: Tracked<Context>, args: &mut Args| {
                construct(element, args)
            },
        )),
        name: element.name,
        title: {
            let title = bump.alloc_str(element.name);
            title[0..1].make_ascii_uppercase();
            title
        },
        docs: element.docs,
        keywords: &[],
        contextual: false,
        scope: LazyLock::new(&|| Scope::new()),
        params: LazyLock::new(bump.alloc(move || create_param_info(element))),
        returns: LazyLock::new(&|| CastInfo::Type(Type::of::<Content>())),
    }
}

/// Creates parameter signature metadata for an element.
fn create_param_info(element: &'static ElementInfo) -> Vec<ParamInfo> {
    let mut params = vec![];
    for attr in element.attributes() {
        params.push(ParamInfo {
            name: attr.name,
            docs: attr.docs,
            input: (attr.ty.input)(),
            default: None,
            positional: false,
            named: true,
            variadic: false,
            required: false,
            settable: false,
        });
    }
    if !tag::is_void(element.tag) {
        params.push(ParamInfo {
            name: "body",
            docs: "The contents of the HTML element.",
            input: CastInfo::Type(Type::of::<Content>()),
            default: None,
            positional: true,
            named: false,
            variadic: false,
            required: false,
            settable: false,
        });
    }
    params
}

/// The native constructor function shared by all HTML elements.
fn construct(element: &'static ElementInfo, args: &mut Args) -> SourceResult<Value> {
    let mut attrs = HtmlAttrs::default();
    let mut errors = eco_vec![];

    args.items.retain(|item| {
        let Some(name) = &item.name else { return true };
        let Some(attr) = element.get_attr(name) else { return true };

        let span = item.value.span;
        let value = std::mem::take(&mut item.value.v);
        match (attr.ty.cast)(value).at(span) {
            Ok(Some(string)) => attrs.push(attr.attr, string),
            Ok(None) => {}
            Err(diags) => errors.extend(diags),
        }

        false
    });

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut elem = HtmlElem::new(element.tag);
    if !attrs.0.is_empty() {
        elem.push_attrs(attrs);
    }

    if !tag::is_void(element.tag) {
        let body = args.eat::<Content>()?;
        elem.push_body(body);
    }

    Ok(elem.into_value())
}

/// Details about an HTML element.
pub struct ElementInfo {
    /// The element's tag.
    tag: HtmlTag,
    /// The element's name, same as `tag`, but different representation.
    name: &'static str,
    /// A description for the element.
    docs: &'static str,
    /// Indices of the element's attributes in `ATTRS`.
    attrs: &'static [u8],
}

impl ElementInfo {
    /// Creates element information from its parts.
    ///
    /// The `attrs` slice consists of indices pointing into `generated::ATTRS`.
    /// It must be sorted by index (and, by extension, also by name of the
    /// pointed-to attributes because the attributes themselves are sorted).
    pub const fn new(
        name: &'static str,
        docs: &'static str,
        attrs: &'static [u8],
    ) -> ElementInfo {
        ElementInfo { tag: HtmlTag::constant(name), name, docs, attrs }
    }

    /// Iterates over all attributes an element of this type can have
    /// (both specific and global).
    fn attributes(&self) -> impl Iterator<Item = &'static AttrInfo> {
        self.attrs
            .iter()
            .map(|&i| &generated::ATTRS[usize::from(i)])
            .chain(&generated::ATTRS[..generated::ATTRS_GLOBAL])
    }

    /// Provides metadata for an attribute with the given name if it exists for
    /// this element. The attribute may be specific or global.
    fn get_attr(&self, name: &str) -> Option<&'static AttrInfo> {
        self.get_specific_attr(name)
            .or_else(|| self.get_global_attr(name))
            .map(|i| &generated::ATTRS[i])
    }

    /// Tries to locate the index of a specific attribute in `ATTRS`.
    fn get_specific_attr(&self, name: &str) -> Option<usize> {
        self.attrs
            .binary_search_by_key(&name, |&i| generated::ATTRS[usize::from(i)].name)
            .map(|k| usize::from(self.attrs[k]))
            .ok()
    }

    /// Tries to locate the index of a global attribute in `ATTRS`.
    fn get_global_attr(&self, name: &str) -> Option<usize> {
        generated::ATTRS[..generated::ATTRS_GLOBAL]
            .binary_search_by_key(&name, |attr| attr.name)
            .ok()
    }
}

/// Details about an HTML attribute.
pub struct AttrInfo {
    /// The attribute itself.
    attr: HtmlAttr,
    /// The attribute's name, same as `attr`, but different representation.
    name: &'static str,
    /// A description for the attribute.
    docs: &'static str,
    /// Type information for the attribute.
    ty: AttrType,
}

impl AttrInfo {
    /// Creates attribute information from its parts.
    pub const fn new<T: IntoOptionalAttr>(
        name: &'static str,
        docs: &'static str,
    ) -> AttrInfo {
        AttrInfo {
            attr: HtmlAttr::constant(name),
            name,
            docs,
            ty: AttrType::of::<T>(),
        }
    }
}

/// A dynamic representation of an attribute's type.
struct AttrType {
    /// Describes the attribute's schema.
    input: fn() -> CastInfo,
    /// Tries to cast a value into this attribute's value given the attribute
    /// name. If `None`, this is a boolean presence-based attribute.
    cast: fn(value: Value) -> HintedStrResult<Option<EcoString>>,
}

impl AttrType {
    const fn of<T: IntoOptionalAttr>() -> Self {
        Self {
            cast: |value| {
                let this = value.cast::<T>()?;
                Ok(this.into_optional_attr())
            },
            input: T::input,
        }
    }
}

/// Casts a type into an optional HTML attribute.
///
/// If `into_optional_attr`, no attribute is written.
pub trait IntoOptionalAttr: FromValue {
    /// Turn the value into an attribute string or indicate the absence of an
    /// attribute via `None`.
    fn into_optional_attr(self) -> Option<EcoString>;
}

impl<T: IntoAttr> IntoOptionalAttr for T {
    fn into_optional_attr(self) -> Option<EcoString> {
        Some(self.into_attr())
    }
}

/// A boolean that is encoded by presence of the attribute:
/// - `false` is encoded by an absent attribute.
/// - `true` is encoded by the empty string:
///   `<input checked="">` which collapses into `<input checked>`
pub struct NamedBool(pub bool);

cast! {
    NamedBool,
    v: bool => Self(v),
}

impl IntoOptionalAttr for NamedBool {
    fn into_optional_attr(self) -> Option<EcoString> {
        self.0.then(EcoString::new)
    }
}

/// Casts a type into an HTML attribute.
pub trait IntoAttr: FromValue {
    /// Turn the value into an attribute string.
    fn into_attr(self) -> EcoString;
}

impl IntoAttr for Str {
    fn into_attr(self) -> EcoString {
        self.into()
    }
}

/// A boolean that is encoded as a string:
/// - `false` is encoded as `"false"`
/// - `true` is encoded as `"true"`
pub struct StrBool(pub bool);

cast! {
    StrBool,
    v: bool => Self(v),
}

impl IntoAttr for StrBool {
    fn into_attr(self) -> EcoString {
        if self.0 { "true" } else { "false" }.into()
    }
}

/// A boolean that is encoded as a string:
/// - `false` is encoded as `"no"`
/// - `true` is encoded as `"yes"`
pub struct YesNoBool(pub bool);

cast! {
    YesNoBool,
    v: bool => Self(v),
}

impl IntoAttr for YesNoBool {
    fn into_attr(self) -> EcoString {
        if self.0 { "yes" } else { "no" }.into()
    }
}

/// A boolean that is encoded as a string:
/// - `false` is encoded as `"off"`
/// - `true` is encoded as `"on"`
pub struct OnOffBool(pub bool);

cast! {
    OnOffBool,
    v: bool => Self(v),
}

impl IntoAttr for OnOffBool {
    fn into_attr(self) -> EcoString {
        if self.0 { "on" } else { "off" }.into()
    }
}

impl IntoAttr for char {
    fn into_attr(self) -> EcoString {
        eco_format!("{self}")
    }
}

impl IntoAttr for i64 {
    fn into_attr(self) -> EcoString {
        eco_format!("{self}")
    }
}

impl IntoAttr for u64 {
    fn into_attr(self) -> EcoString {
        eco_format!("{self}")
    }
}

impl IntoAttr for NonZeroI64 {
    fn into_attr(self) -> EcoString {
        eco_format!("{self}")
    }
}

impl IntoAttr for NonZeroU64 {
    fn into_attr(self) -> EcoString {
        eco_format!("{self}")
    }
}

impl IntoAttr for f64 {
    fn into_attr(self) -> EcoString {
        // HTML float literal allows all the things that Rust's float `Display`
        // impl produces.
        eco_format!("{self}")
    }
}

impl IntoAttr for PositiveF64 {
    fn into_attr(self) -> EcoString {
        self.get().into_attr()
    }
}

impl IntoAttr for Color {
    fn into_attr(self) -> EcoString {
        eco_format!("{}", css::color(self))
    }
}

impl IntoAttr for Duration {
    fn into_attr(self) -> EcoString {
        // https://html.spec.whatwg.org/#valid-duration-string
        let mut out = EcoString::new();
        macro_rules! part {
            ($s:literal) => {
                if !out.is_empty() {
                    out.push(' ');
                }
                write!(out, $s).unwrap();
            };
        }

        let [weeks, days, hours, minutes, seconds] = self.decompose();
        if weeks > 0 {
            part!("{weeks}w");
        }
        if days > 0 {
            part!("{days}d");
        }
        if hours > 0 {
            part!("{hours}h");
        }
        if minutes > 0 {
            part!("{minutes}m");
        }
        if seconds > 0 || out.is_empty() {
            part!("{seconds}s");
        }

        out
    }
}

impl IntoAttr for Datetime {
    fn into_attr(self) -> EcoString {
        let fmt = typst_utils::display(|f| match self {
            Self::Date(date) => datetime::date(f, date),
            Self::Time(time) => datetime::time(f, time),
            Self::Datetime(datetime) => datetime::datetime(f, datetime),
        });
        eco_format!("{fmt}")
    }
}

mod datetime {
    use std::fmt::{self, Formatter, Write};

    pub fn datetime(f: &mut Formatter, datetime: time::PrimitiveDateTime) -> fmt::Result {
        // https://html.spec.whatwg.org/#valid-global-date-and-time-string
        date(f, datetime.date())?;
        f.write_char('T')?;
        time(f, datetime.time())
    }

    pub fn date(f: &mut Formatter, date: time::Date) -> fmt::Result {
        // https://html.spec.whatwg.org/#valid-date-string
        write!(f, "{:04}-{:02}-{:02}", date.year(), date.month() as u8, date.day())
    }

    pub fn time(f: &mut Formatter, time: time::Time) -> fmt::Result {
        // https://html.spec.whatwg.org/#valid-time-string
        write!(f, "{:02}:{:02}", time.hour(), time.minute())?;
        if time.second() > 0 {
            write!(f, ":{:02}", time.second())?;
        }
        Ok(())
    }
}

/// A direction on the X axis: `ltr` or `rtl`.
pub struct HorizontalDir(Dir);

cast! {
    HorizontalDir,
    v: Dir => {
        if v.axis() == Axis::Y {
            bail!("direction must be horizontal");
        }
        Self(v)
    },
}

impl IntoAttr for HorizontalDir {
    fn into_attr(self) -> EcoString {
        self.0.into_attr()
    }
}

impl IntoAttr for Dir {
    fn into_attr(self) -> EcoString {
        match self {
            Self::LTR => "ltr".into(),
            Self::RTL => "rtl".into(),
            Self::TTB => "ttb".into(),
            Self::BTT => "btt".into(),
        }
    }
}

/// An optional value that represents `None` with one of three strings.
pub struct StrOption<T, const INDEX: usize>(Option<T>);

pub type StrOptionEmpty<T> = StrOption<T, 0>;
pub type StrOptionNone<T> = StrOption<T, 1>;
pub type StrOptionUndefined<T> = StrOption<T, 2>;

const NONE_STRS: &[&str] = &["", "none", "undefined"];

impl<T: Reflect, const INDEX: usize> Reflect for StrOption<T, INDEX> {
    fn input() -> CastInfo {
        Option::<T>::input()
    }

    fn output() -> CastInfo {
        Option::<T>::output()
    }

    fn castable(value: &Value) -> bool {
        Option::<T>::castable(value)
    }
}

impl<T: FromValue, const INDEX: usize> FromValue for StrOption<T, INDEX> {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        value.cast().map(Self)
    }
}

impl<T: IntoOptionalAttr, const INDEX: usize> IntoOptionalAttr for StrOption<T, INDEX> {
    fn into_optional_attr(self) -> Option<EcoString> {
        match self.0 {
            None => Some(NONE_STRS[INDEX].into()),
            Some(v) => v.into_optional_attr(),
        }
    }
}

impl<T: IntoOptionalAttr> IntoOptionalAttr for Smart<T> {
    fn into_optional_attr(self) -> Option<EcoString> {
        match self {
            Smart::Auto => Some("auto".into()),
            Smart::Custom(v) => v.into_optional_attr(),
        }
    }
}

/// A list of items separated by a specific separator char.
///
/// - https://html.spec.whatwg.org/#space-separated-tokens>
/// - https://html.spec.whatwg.org/#comma-separated-tokens>
pub struct TokenList<T, const SEP: char, const SHORTHAND: bool = true>(
    EcoString,
    PhantomData<T>,
);

impl<T: Reflect, const SEP: char, const SHORTHAND: bool> Reflect
    for TokenList<T, SEP, SHORTHAND>
{
    fn input() -> CastInfo {
        if SHORTHAND {
            Array::input() + T::input()
        } else {
            Array::input()
        }
    }

    fn output() -> CastInfo {
        if SHORTHAND {
            Array::output() + T::input()
        } else {
            Array::output()
        }
    }

    fn castable(value: &Value) -> bool {
        Array::castable(value) || (SHORTHAND && T::castable(value))
    }
}

impl<T: FromValue + IntoAttr, const SEP: char, const SHORTHAND: bool> FromValue
    for TokenList<T, SEP, SHORTHAND>
{
    fn from_value(value: Value) -> HintedStrResult<Self> {
        if Array::castable(&value) {
            let array = value.cast::<Array>()?;
            let mut out = EcoString::new();
            for (i, item) in array.into_iter().enumerate() {
                let item = item.cast::<T>()?.into_attr();
                if item.as_str().contains(SEP) {
                    let buf;
                    let name = match SEP {
                        ' ' => "space",
                        ',' => "comma",
                        _ => {
                            buf = eco_format!("'{SEP}'");
                            buf.as_str()
                        }
                    };
                    bail!(
                        "array item may not contain a {name}";
                        hint: "the array attribute will be encoded as a \
                               {name}-separated string"
                    );
                }
                if i > 0 {
                    out.push(SEP);
                    if SEP == ',' {
                        out.push(' ');
                    }
                }
                out.push_str(&item);
            }
            Ok(Self(out, PhantomData))
        } else if SHORTHAND && T::castable(&value) {
            let item = value.cast::<T>()?.into_attr();
            Ok(Self(item, PhantomData))
        } else {
            Err(<Self as Reflect>::error(&value))
        }
    }
}

impl<T: FromValue + IntoAttr, const SEP: char, const SHORTHAND: bool> IntoAttr
    for TokenList<T, SEP, SHORTHAND>
{
    fn into_attr(self) -> EcoString {
        self.0
    }
}

/// An enumeration with generated string variants.
///
/// `START` and `END` are used to index into `generated::ATTR_STRINGS`.
pub struct StrEnum<const START: usize, const END: usize>(Str);

impl<const START: usize, const END: usize> Reflect for StrEnum<START, END> {
    fn input() -> CastInfo {
        CastInfo::Union(
            generated::ATTR_STRINGS[START..END]
                .iter()
                .map(|&(string, docs)| CastInfo::Value(string.into_value(), docs))
                .collect(),
        )
    }

    fn output() -> CastInfo {
        Self::input()
    }

    fn castable(value: &Value) -> bool {
        match value {
            Value::Str(s) => generated::ATTR_STRINGS[START..END]
                .iter()
                .any(|&(v, _)| v == s.as_str()),
            _ => false,
        }
    }
}

impl<const START: usize, const END: usize> FromValue for StrEnum<START, END> {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        if Self::castable(&value) {
            Ok(Self(value.cast()?))
        } else {
            Err(<Self as Reflect>::error(&value))
        }
    }
}

impl<const START: usize, const END: usize> IntoAttr for StrEnum<START, END> {
    fn into_attr(self) -> EcoString {
        self.0.into()
    }
}

/// One attribute type or another.
pub enum Or<A, B> {
    A(A),
    B(B),
}

impl<A: Reflect, B: Reflect> Reflect for Or<A, B> {
    fn input() -> CastInfo {
        A::input() + B::input()
    }

    fn output() -> CastInfo {
        A::output() + B::output()
    }

    fn castable(value: &Value) -> bool {
        A::castable(value) || B::castable(value)
    }
}

impl<A: FromValue, B: FromValue> FromValue for Or<A, B> {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        if A::castable(&value) {
            A::from_value(value).map(Self::A)
        } else if B::castable(&value) {
            B::from_value(value).map(Self::B)
        } else {
            Err(<Self as Reflect>::error(&value))
        }
    }
}

impl<A: IntoOptionalAttr, B: IntoOptionalAttr> IntoOptionalAttr for Or<A, B> {
    fn into_optional_attr(self) -> Option<EcoString> {
        match self {
            Self::A(v) => v.into_optional_attr(),
            Self::B(v) => v.into_optional_attr(),
        }
    }
}

/// A value of an `<input>` element.
pub struct InputValue(EcoString);

cast! {
    InputValue,
    v: Str => Self(v.into_attr()),
    v: f64 => Self(v.into_attr()),
    v: Datetime => Self(v.into_attr()),
    v: Color => Self(v.into_attr()),
    v: TokenList<Str, ','> => Self(v.into_attr()),
}

impl IntoAttr for InputValue {
    fn into_attr(self) -> EcoString {
        self.0
    }
}

/// A min/max bound of an `<input>` element.
pub struct InputBound(EcoString);

cast! {
    InputBound,
    v: Str => Self(v.into_attr()),
    v: f64 => Self(v.into_attr()),
    v: Datetime => Self(v.into_attr()),
}

impl IntoAttr for InputBound {
    fn into_attr(self) -> EcoString {
        self.0
    }
}

/// <https://html.spec.whatwg.org/#image-candidate-string>
pub struct ImageCandidate(EcoString);

cast! {
    ImageCandidate,
    mut v: Dict => {
        let src = v.take("src")?.cast::<EcoString>()?;
        let width: Option<NonZeroU64> =
            v.take("width").ok().map(Value::cast).transpose()?;
        let density: Option<PositiveF64> =
            v.take("density").ok().map(Value::cast).transpose()?;
        v.finish(&["src", "width", "density"])?;

        if src.is_empty() {
            bail!("`src` must not be empty");
        } else if src.starts_with(',') || src.ends_with(',') {
            bail!("`src` must not start or end with a comma");
        }

        let mut out = src;
        match (width, density) {
            (None, None) => {}
            (Some(width), None) => write!(out, " {width}w").unwrap(),
            (None, Some(density)) => write!(out, " {}d", density.get()).unwrap(),
            (Some(_), Some(_)) => bail!("cannot specify both `width` and `density`"),
        }

        Self(out)
    },
}

impl IntoAttr for ImageCandidate {
    fn into_attr(self) -> EcoString {
        self.0
    }
}

/// <https://html.spec.whatwg.org/multipage/images.html#valid-source-size-list>
pub struct SourceSize(EcoString);

cast! {
    SourceSize,
    mut v: Dict => {
        let condition = v.take("condition")?.cast::<EcoString>()?;
        let size = v
            .take("size")?
            .cast::<Length>()
            .hint("CSS lengths that are not expressible as Typst lengths are not yet supported")
            .hint("you can use `html.elem` to create a raw attribute")?;
        Self(eco_format!("({condition}) {}", css::length(size)))
    },
}

impl IntoAttr for SourceSize {
    fn into_attr(self) -> EcoString {
        self.0
    }
}

/// A width/height pair for `<link rel="icon" sizes="..." />`.
pub struct IconSize(Axes<u64>);

cast! {
    IconSize,
    v: Axes<u64> => Self(v),
}

impl IntoAttr for IconSize {
    fn into_attr(self) -> EcoString {
        eco_format!("{}x{}", self.0.x, self.0.y)
    }
}

/// Conversion from Typst data types into CSS data types.
///
/// This can be moved elsewhere once we start supporting more CSS stuff.
mod css {
    use std::fmt::{self, Display};

    use typst_utils::Numeric;

    use crate::layout::Length;
    use crate::visualize::{Color, Hsl, LinearRgb, Oklab, Oklch, Rgb};

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
        write!(
            f,
            "oklab({} {} {}{})",
            percent(v.l),
            number(v.a),
            number(v.b),
            alpha(v.alpha)
        )
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
}
