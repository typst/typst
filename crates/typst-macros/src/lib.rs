//! Procedural macros for Typst.

extern crate proc_macro;

#[macro_use]
mod util;
mod cast;
mod elem;
mod func;
mod scope;
mod symbols;
mod ty;

use proc_macro::TokenStream as BoundaryStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{parse_quote, DeriveInput, Ident, Result, Token};

use self::util::*;

/// Makes a native Rust function usable as a Typst function.
///
/// This implements `NativeFunction` for a freshly generated type with the same
/// name as a function. (In Rust, functions and types live in separate
/// namespace, so both can coexist.)
///
/// If the function is in an impl block annotated with `#[scope]`, things work a
/// bit differently because the no type can be generated within the impl block.
/// In that case, a function named `{name}_data` that returns `&'static
/// NativeFuncData` is generated. You typically don't need to interact with this
/// function though because the `#[scope]` macro hooks everything up for you.
///
/// ```ignore
/// /// Doubles an integer.
/// #[func]
/// fn double(x: i64) -> i64 {
///     2 * x
/// }
/// ```
///
/// # Properties
/// You can customize some properties of the resulting function:
/// - `scope`: Indicates that the function has an associated scope defined by
///   the `#[scope]` macro.
/// - `name`: The functions's normal name (e.g. `min`). Defaults to the Rust
///   name in kebab-case.
/// - `title`: The functions's title case name (e.g. `Minimum`). Defaults to the
///   normal name in title case.
///
/// # Arguments
/// By default, function arguments are positional and required. You can use
/// various attributes to configure their parsing behaviour:
///
/// - `#[named]`: Makes the argument named and optional. The argument type must
///   either be `Option<_>` _or_ the `#[default]` attribute must be used. (If
///   it's both `Option<_>` and `#[default]`, then the argument can be specified
///   as `none` in Typst).
/// - `#[default]`: Specifies the default value of the argument as
///   `Default::default()`.
/// - `#[default(..)]`: Specifies the default value of the argument as `..`.
/// - `#[variadic]`: Parses a variable number of arguments. The argument type
///   must be `Vec<_>`.
/// - `#[external]`: The argument appears in documentation, but is otherwise
///   ignored. Can be useful if you want to do something manually for more
///   flexibility.
///
/// Defaults can be specified for positional and named arguments. This is in
/// contrast to user-defined functions which currently cannot have optional
/// positional arguments (except through argument sinks).
///
/// In the example below, we define a `min` function that could be called as
/// `min(1, 2, 3, default: 0)` in Typst.
///
/// ```ignore
/// /// Determines the minimum of a sequence of values.
/// #[func(title = "Minimum")]
/// fn min(
///     /// The values to extract the minimum from.
///     #[variadic]
///     values: Vec<i64>,
///     /// A default value to return if there are no values.
///     #[named]
///     #[default(0)]
///     default: i64,
/// ) -> i64 {
///     self.values.iter().min().unwrap_or(default)
/// }
/// ```
///
/// As you can see, arguments can also have doc-comments, which will be rendered
/// in the documentation. The first line of documentation should be concise and
/// self-contained as it is the designated short description, which is used in
/// overviews in the documentation (and for autocompletion).
#[proc_macro_attribute]
pub fn func(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemFn);
    func::func(stream.into(), &item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Makes a native Rust type usable as a Typst type.
///
/// This implements `NativeType` for the given type.
///
/// ```ignore
/// /// A sequence of codepoints.
/// #[ty(scope, title = "String")]
/// struct Str(EcoString);
///
/// #[scope]
/// impl Str {
///     ...
/// }
/// ```
///
/// # Properties
/// You can customize some properties of the resulting type:
/// - `scope`: Indicates that the type has an associated scope defined by the
///   `#[scope]` macro
/// - `name`: The type's normal name (e.g. `str`). Defaults to the Rust name in
///   kebab-case.
/// - `title`: The type's title case name (e.g. `String`). Defaults to the
///   normal name in title case.
#[proc_macro_attribute]
pub fn ty(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::Item);
    ty::ty(stream.into(), item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Makes a native Rust type usable as a Typst element.
///
/// This implements `NativeElement` for the given type.
///
/// ```
/// /// A section heading.
/// #[elem(Show, Count)]
/// struct HeadingElem {
///     /// The logical nesting depth of the heading, starting from one.
///     #[default(NonZeroUsize::ONE)]
///     level: NonZeroUsize,
///
///     /// The heading's title.
///     #[required]
///     body: Content,
/// }
/// ```
///
/// # Properties
/// You can customize some properties of the resulting type:
/// - `scope`: Indicates that the type has an associated scope defined by the
///   `#[scope]` macro
/// - `name`: The element's normal name (e.g. `str`). Defaults to the Rust name
///   in kebab-case.
/// - `title`: The type's title case name (e.g. `String`). Defaults to the long
///   name in title case.
/// - The remaining entries in the `elem` macros list are traits the element
///   is capable of. These can be dynamically accessed.
///
/// # Fields
/// By default, element fields are named and optional (and thus settable). You
/// can use various attributes to configure their parsing behaviour:
///
/// - `#[positional]`: Makes the argument positional (but still optional).
/// - `#[required]`: Makes the argument positional and required.
/// - `#[default(..)]`: Specifies the default value of the argument as `..`.
/// - `#[variadic]`: Parses a variable number of arguments. The field type must
///   be `Vec<_>`. The field will be exposed as an array.
/// - `#[parse({ .. })]`: A block of code that parses the field manually.
///
/// In addition that there are a number of attributes that configure other
/// aspects of the field than the parsing behaviour.
/// - `#[resolve]`: When accessing the field, it will be automatically
///   resolved through the `Resolve` trait. This, for instance, turns `Length`
///   into `Abs`. It's just convenient.
/// - `#[fold]`: When there are multiple set rules for the field, all values
///   are folded together into one. E.g. `set rect(stroke: 2pt)` and
///   `set rect(stroke: red)` are combined into the equivalent of
///   `set rect(stroke: 2pt + red)` instead of having `red` override `2pt`.
/// - `#[internal]`: The field does not appear in the documentation.
/// - `#[external]`: The field appears in the documentation, but is otherwise
///   ignored. Can be useful if you want to do something manually for more
///   flexibility.
/// - `#[synthesized]`: The field cannot be specified in a constructor or set
///   rule. Instead, it is added to an element before its show rule runs
///   through the `Synthesize` trait.
#[proc_macro_attribute]
pub fn elem(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemStruct);
    elem::elem(stream.into(), item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Provides an associated scope to a native function, type, or element.
///
/// This implements `NativeScope` for the function's shadow type, the type, or
/// the element.
///
/// The implementation block can contain four kinds of items:
/// - constants, which will be defined through `scope.define`
/// - functions, which will be defined through `scope.define_func`
/// - types, which will be defined through `scope.define_type`
/// - elements, which will be defined through `scope.define_elem`
///
/// ```ignore
/// #[func(scope)]
/// fn name() { .. }
///
/// #[scope]
/// impl name {
///     /// A simple constant.
///     const VAL: u32 = 0;
///
///     /// A function.
///     #[func]
///     fn foo() -> EcoString {
///         "foo!".into()
///     }
///
///     /// A type.
///     type Brr;
///
///     /// An element.
///     #[elem]
///     type NiceElem;
/// }
///
/// #[ty]
/// struct Brr;
///
/// #[elem]
/// struct NiceElem {}
/// ```
#[proc_macro_attribute]
pub fn scope(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::Item);
    scope::scope(stream.into(), item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implements `Reflect`, `FromValue`, and `IntoValue` for a type.
///
/// - `Reflect` makes Typst's runtime aware of the type's characteristics.
///   It's important for autocompletion, error messages, etc.
/// - `FromValue` defines how to cast from a value into this type.
/// - `IntoValue` defines how to cast fromthis type into a value.
///
/// ```ignore
/// /// An integer between 0 and 13.
/// struct CoolInt(u8);
///
/// cast! {
///     CoolInt,
///
///     // Defines how to turn a `CoolInt` into a value.
///     self => self.0.into_value(),
///
///     // Defines "match arms" of types that can be cast into a `CoolInt`.
///     // These types needn't be value primitives, they can themselves use
///     // `cast!`.
///     v: bool => Self(v as u8),
///     v: i64 => if matches!(v, 0..=13) {
///         Self(v as u8)
///     } else {
///         bail!("integer is not nice :/")
///     },
/// }
/// ```
#[proc_macro]
pub fn cast(stream: BoundaryStream) -> BoundaryStream {
    cast::cast(stream.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implements `Reflect`, `FromValue`, and `IntoValue` for an enum.
///
/// The enum will become castable from kebab-case strings. The doc-comments will
/// become user-facing documentation for each variant. The `#[string]` attribute
/// can be used to override the string corresponding to a variant.
///
/// ```ignore
/// /// A stringy enum of options.
/// #[derive(Cast)]
/// enum Niceness {
///     /// Clearly nice (parses from `"nice"`).
///     Nice,
///     /// Not so nice (parses from `"not-nice"`).
///     NotNice,
///     /// Very much not nice (parses from `"❌"`).
///     #[string("❌")]
///     Unnice,
/// }
/// ```
#[proc_macro_derive(Cast, attributes(string))]
pub fn derive_cast(item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as DeriveInput);
    cast::derive_cast(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Defines a list of `Symbol`s.
///
/// ```ignore
/// const EMOJI: &[(&str, Symbol)] = symbols! {
///    // A plain symbol without modifiers.
///    abacus: '🧮',
///
///    // A symbol with a modifierless default and one modifier.
///    alien: ['👽', monster: '👾'],
///
///    // A symbol where each variant has a modifier. The first one will be
///    // the default.
///    clock: [one: '🕐', two: '🕑', ...],
/// }
/// ```
///
/// _Note:_ While this could use `macro_rules!` instead of a proc-macro, it was
/// horribly slow in rust-analyzer. The underlying cause might be
/// [this issue](https://github.com/rust-lang/rust-analyzer/issues/11108).
#[proc_macro]
pub fn symbols(stream: BoundaryStream) -> BoundaryStream {
    symbols::symbols(stream.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
