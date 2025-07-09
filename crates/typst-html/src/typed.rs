//! The typed HTML element API (e.g. `html.div`).
//!
//! The typed API is backed by generated data derived from the HTML
//! specification. See [generated] and `tools/codegen`.

use std::fmt::Write;
use std::num::{NonZeroI64, NonZeroU64};
use std::sync::LazyLock;

use bumpalo::Bump;
use comemo::Tracked;
use ecow::{eco_format, eco_vec, EcoString};
use typst_assets::html as data;
use typst_library::diag::{bail, At, Hint, HintedStrResult, SourceResult};
use typst_library::engine::Engine;
use typst_library::foundations::{
    Args, Array, AutoValue, CastInfo, Content, Context, Datetime, Dict, Duration,
    FromValue, IntoValue, NativeFuncData, NativeFuncPtr, NoneValue, ParamInfo,
    PositiveF64, Reflect, Scope, Str, Type, Value,
};
use typst_library::html::tag;
use typst_library::html::{HtmlAttr, HtmlAttrs, HtmlElem, HtmlTag};
use typst_library::layout::{Axes, Axis, Dir, Length};
use typst_library::visualize::Color;
use typst_macros::cast;

use crate::css;

/// Hook up all typed HTML definitions.
pub(super) fn define(html: &mut Scope) {
    for data in FUNCS.iter() {
        html.define_func_with_data(data);
    }
}

/// Lazily created functions for all typed HTML constructors.
static FUNCS: LazyLock<Vec<NativeFuncData>> = LazyLock::new(|| {
    // Leaking is okay here. It's not meaningfully different from having
    // memory-managed values as `FUNCS` is a static.
    let bump = Box::leak(Box::new(Bump::new()));
    data::ELEMS.iter().map(|info| create_func_data(info, bump)).collect()
});

/// Creates metadata for a native HTML element constructor function.
fn create_func_data(
    element: &'static data::ElemInfo,
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
fn create_param_info(element: &'static data::ElemInfo) -> Vec<ParamInfo> {
    let mut params = vec![];
    for attr in element.attributes() {
        params.push(ParamInfo {
            name: attr.name,
            docs: attr.docs,
            input: AttrType::convert(attr.ty).input(),
            default: None,
            positional: false,
            named: true,
            variadic: false,
            required: false,
            settable: false,
        });
    }
    let tag = HtmlTag::constant(element.name);
    if !tag::is_void(tag) {
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
fn construct(element: &'static data::ElemInfo, args: &mut Args) -> SourceResult<Value> {
    let mut attrs = HtmlAttrs::default();
    let mut errors = eco_vec![];

    args.items.retain(|item| {
        let Some(name) = &item.name else { return true };
        let Some(attr) = element.get_attr(name) else { return true };

        let span = item.value.span;
        let value = std::mem::take(&mut item.value.v);
        let ty = AttrType::convert(attr.ty);
        match ty.cast(value).at(span) {
            Ok(Some(string)) => attrs.push(HtmlAttr::constant(attr.name), string),
            Ok(None) => {}
            Err(diags) => errors.extend(diags),
        }

        false
    });

    if !errors.is_empty() {
        return Err(errors);
    }

    let tag = HtmlTag::constant(element.name);
    let mut elem = HtmlElem::new(tag);
    if !attrs.0.is_empty() {
        elem.attrs.set(attrs);
    }

    if !tag::is_void(tag) {
        let body = args.eat::<Content>()?;
        elem.body.set(body);
    }

    Ok(elem.into_value())
}

/// A dynamic representation of an attribute's type.
///
/// See the documentation of [`data::Type`] for more details on variants.
enum AttrType {
    Presence,
    Native(NativeType),
    Strings(StringsType),
    Union(UnionType),
    List(ListType),
}

impl AttrType {
    /// Converts the type definition into a representation suitable for casting
    /// and reflection.
    const fn convert(ty: data::Type) -> AttrType {
        use data::Type;
        match ty {
            Type::Presence => Self::Presence,
            Type::None => Self::of::<NoneValue>(),
            Type::NoneEmpty => Self::of::<NoneEmpty>(),
            Type::NoneUndefined => Self::of::<NoneUndefined>(),
            Type::Auto => Self::of::<AutoValue>(),
            Type::TrueFalse => Self::of::<TrueFalseBool>(),
            Type::YesNo => Self::of::<YesNoBool>(),
            Type::OnOff => Self::of::<OnOffBool>(),
            Type::Int => Self::of::<i64>(),
            Type::NonNegativeInt => Self::of::<u64>(),
            Type::PositiveInt => Self::of::<NonZeroU64>(),
            Type::Float => Self::of::<f64>(),
            Type::PositiveFloat => Self::of::<PositiveF64>(),
            Type::Str => Self::of::<Str>(),
            Type::Char => Self::of::<char>(),
            Type::Datetime => Self::of::<Datetime>(),
            Type::Duration => Self::of::<Duration>(),
            Type::Color => Self::of::<Color>(),
            Type::HorizontalDir => Self::of::<HorizontalDir>(),
            Type::IconSize => Self::of::<IconSize>(),
            Type::ImageCandidate => Self::of::<ImageCandidate>(),
            Type::SourceSize => Self::of::<SourceSize>(),
            Type::Strings(start, end) => Self::Strings(StringsType { start, end }),
            Type::Union(variants) => Self::Union(UnionType(variants)),
            Type::List(inner, separator, shorthand) => {
                Self::List(ListType { inner, separator, shorthand })
            }
        }
    }

    /// Produces the dynamic representation of an attribute type backed by a
    /// native Rust type.
    const fn of<T: IntoAttr>() -> Self {
        Self::Native(NativeType::of::<T>())
    }

    /// See [`Reflect::input`].
    fn input(&self) -> CastInfo {
        match self {
            Self::Presence => bool::input(),
            Self::Native(ty) => (ty.input)(),
            Self::Union(ty) => ty.input(),
            Self::Strings(ty) => ty.input(),
            Self::List(ty) => ty.input(),
        }
    }

    /// See [`Reflect::castable`].
    fn castable(&self, value: &Value) -> bool {
        match self {
            Self::Presence => bool::castable(value),
            Self::Native(ty) => (ty.castable)(value),
            Self::Union(ty) => ty.castable(value),
            Self::Strings(ty) => ty.castable(value),
            Self::List(ty) => ty.castable(value),
        }
    }

    /// Tries to cast the value into this attribute's type and serialize it into
    /// an HTML attribute string.
    fn cast(&self, value: Value) -> HintedStrResult<Option<EcoString>> {
        match self {
            Self::Presence => value.cast::<bool>().map(|b| b.then(EcoString::new)),
            Self::Native(ty) => (ty.cast)(value),
            Self::Union(ty) => ty.cast(value),
            Self::Strings(ty) => ty.cast(value),
            Self::List(ty) => ty.cast(value),
        }
    }
}

/// An enumeration with generated string variants.
///
/// `start` and `end` are used to index into `data::ATTR_STRINGS`.
struct StringsType {
    start: usize,
    end: usize,
}

impl StringsType {
    fn input(&self) -> CastInfo {
        CastInfo::Union(
            self.strings()
                .iter()
                .map(|(val, desc)| CastInfo::Value(val.into_value(), desc))
                .collect(),
        )
    }

    fn castable(&self, value: &Value) -> bool {
        match value {
            Value::Str(s) => self.strings().iter().any(|&(v, _)| v == s.as_str()),
            _ => false,
        }
    }

    fn cast(&self, value: Value) -> HintedStrResult<Option<EcoString>> {
        if self.castable(&value) {
            value.cast().map(Some)
        } else {
            Err(self.input().error(&value))
        }
    }

    fn strings(&self) -> &'static [(&'static str, &'static str)] {
        &data::ATTR_STRINGS[self.start..self.end]
    }
}

/// A type that accepts any of the contained types.
struct UnionType(&'static [data::Type]);

impl UnionType {
    fn input(&self) -> CastInfo {
        CastInfo::Union(self.iter().map(|ty| ty.input()).collect())
    }

    fn castable(&self, value: &Value) -> bool {
        self.iter().any(|ty| ty.castable(value))
    }

    fn cast(&self, value: Value) -> HintedStrResult<Option<EcoString>> {
        for item in self.iter() {
            if item.castable(&value) {
                return item.cast(value);
            }
        }
        Err(self.input().error(&value))
    }

    fn iter(&self) -> impl Iterator<Item = AttrType> {
        self.0.iter().map(|&ty| AttrType::convert(ty))
    }
}

/// A list of items separated by a specific separator char.
///
/// - <https://html.spec.whatwg.org/#space-separated-tokens>
/// - <https://html.spec.whatwg.org/#comma-separated-tokens>
struct ListType {
    inner: &'static data::Type,
    separator: char,
    shorthand: bool,
}

impl ListType {
    fn input(&self) -> CastInfo {
        if self.shorthand {
            Array::input() + self.inner().input()
        } else {
            Array::input()
        }
    }

    fn castable(&self, value: &Value) -> bool {
        Array::castable(value) || (self.shorthand && self.inner().castable(value))
    }

    fn cast(&self, value: Value) -> HintedStrResult<Option<EcoString>> {
        let ty = self.inner();
        if Array::castable(&value) {
            let array = value.cast::<Array>()?;
            let mut out = EcoString::new();
            for (i, item) in array.into_iter().enumerate() {
                let item = ty.cast(item)?.unwrap();
                if item.as_str().contains(self.separator) {
                    let buf;
                    let name = match self.separator {
                        ' ' => "space",
                        ',' => "comma",
                        _ => {
                            buf = eco_format!("'{}'", self.separator);
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
                    out.push(self.separator);
                    if self.separator == ',' {
                        out.push(' ');
                    }
                }
                out.push_str(&item);
            }
            Ok(Some(out))
        } else if self.shorthand && ty.castable(&value) {
            let item = ty.cast(value)?.unwrap();
            Ok(Some(item))
        } else {
            Err(self.input().error(&value))
        }
    }

    fn inner(&self) -> AttrType {
        AttrType::convert(*self.inner)
    }
}

/// A dynamic representation of attribute backed by a native type implementing
/// - the standard `Reflect` and `FromValue` traits for casting from a value,
/// - the special `IntoAttr` trait for conversion into an attribute string.
#[derive(Copy, Clone)]
struct NativeType {
    input: fn() -> CastInfo,
    cast: fn(Value) -> HintedStrResult<Option<EcoString>>,
    castable: fn(&Value) -> bool,
}

impl NativeType {
    /// Creates a dynamic native type from a native Rust type.
    const fn of<T: IntoAttr>() -> Self {
        Self {
            cast: |value| {
                let this = value.cast::<T>()?;
                Ok(Some(this.into_attr()))
            },
            input: T::input,
            castable: T::castable,
        }
    }
}

/// Casts a native type into an HTML attribute.
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
pub struct TrueFalseBool(pub bool);

cast! {
    TrueFalseBool,
    v: bool => Self(v),
}

impl IntoAttr for TrueFalseBool {
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

impl IntoAttr for AutoValue {
    fn into_attr(self) -> EcoString {
        "auto".into()
    }
}

impl IntoAttr for NoneValue {
    fn into_attr(self) -> EcoString {
        "none".into()
    }
}

/// A `none` value that turns into an empty string attribute.
struct NoneEmpty;

cast! {
    NoneEmpty,
    _: NoneValue => NoneEmpty,
}

impl IntoAttr for NoneEmpty {
    fn into_attr(self) -> EcoString {
        "".into()
    }
}

/// A `none` value that turns into the string `"undefined"`.
struct NoneUndefined;

cast! {
    NoneUndefined,
    _: NoneValue => NoneUndefined,
}

impl IntoAttr for NoneUndefined {
    fn into_attr(self) -> EcoString {
        "undefined".into()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags_and_attr_const_internible() {
        for elem in data::ELEMS {
            let _ = HtmlTag::constant(elem.name);
        }
        for attr in data::ATTRS {
            let _ = HtmlAttr::constant(attr.name);
        }
    }
}
