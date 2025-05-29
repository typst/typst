//! Procedural macros for Typst.

extern crate proc_macro;

#[macro_use]
mod util;
mod cast;
mod elem;
mod func;
mod scope;
mod time;
mod ty;

use proc_macro::TokenStream as BoundaryStream;
use syn::DeriveInput;

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
/// - `contextual`: Indicates that the function makes use of context. This has
///   no effect on the behaviour itself, but is used for the docs.
/// - `name`: The functions's normal name (e.g. `min`), as exposed to Typst.
///   Defaults to the Rust name in kebab-case.
/// - `title`: The functions's title case name (e.g. `Minimum`). Defaults to the
///   normal name in title case.
/// - `keywords = [..]`: A list of alternate search terms for this function.
/// - `constructor`: Indicates that the function is a constructor.
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
///
/// Additionally, some arguments are treated specially by this macro:
///
/// - `engine`: The compilation context (`Engine`).
/// - `context`: The introspection context (`Tracked<Context>`).
/// - `args`: The rest of the arguments passed into this function (`&mut Args`).
/// - `span`: The span of the function call (`Span`).
///
/// These should always come after `self`, in the order specified.
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
/// - `cast`: Indicates that the type has a custom `cast!` implementation.
///   The macro will then not autogenerate one.
/// - `name`: The type's normal name (e.g. `str`), as exposed to Typst.
///   Defaults to the Rust name in kebab-case.
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
/// ```ignore
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
///   `#[scope]` macro.
/// - `name = "<name>"`: The element's normal name (e.g. `align`), as exposed to Typst.
///   Defaults to the Rust name in kebab-case.
/// - `title = "<title>"`: The type's title case name (e.g. `Align`). Defaults to the long
///   name in title case.
/// - `keywords = [..]`: A list of alternate search terms for this element.
///   Defaults to the empty list.
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
/// - `#[borrowed]`: For fields that are accessed through the style chain,
///   indicates that accessor methods to this field should return references
///   to the value instead of cloning.
/// - `#[internal]`: The field does not appear in the documentation.
/// - `#[external]`: The field appears in the documentation, but is otherwise
///   ignored. Can be useful if you want to do something manually for more
///   flexibility.
/// - `#[synthesized]`: The field cannot be specified in a constructor or set
///   rule. Instead, it is added to an element before its show rule runs
///   through the `Synthesize` trait. This implies `#[internal]`. If a
///   synthesized field needs to be exposed to the user, that should be done via
///   a getter method.
/// - `#[ghost]`: Allows creating fields that are only present in the style chain,
///   this means that they *cannot* be accessed by the user, they cannot be set
///   on an individual instantiated element, and must be set via the style chain.
///   This is useful for fields that are only used internally by the style chain,
///   such as the fields from `ParElem` and `TextElem`. If your element contains
///   any ghost fields, then you cannot auto-generate `Construct` for it, and
///   you must implement `Construct` manually.
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
/// - `IntoValue` defines how to cast from this type into a value.
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

/// Times function invocations.
///
/// When tracing is enabled in the typst-cli, this macro will record the
/// invocations of the function and store them in a global map. The map can be
/// accessed through the `typst_trace::RECORDER` static.
///
/// You can also specify the span of the function invocation:
/// - `#[time(span = ..)]` to record the span, which will be used for the
///   `EventKey`.
///
/// By default, all tracing is omitted using the `wasm32` target flag.
/// This is done to avoid bloating the web app, which doesn't need tracing.
///
/// ```ignore
/// #[time]
/// fn fibonacci(n: u64) -> u64 {
///     if n <= 1 {
///         1
///     } else {
///         fibonacci(n - 1) + fibonacci(n - 2)
///     }
/// }
///
/// #[time(span = span)]
/// fn fibonacci_spanned(n: u64, span: Span) -> u64 {
///     if n <= 1 {
///         1
///     } else {
///         fibonacci(n - 1) + fibonacci(n - 2)
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn time(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemFn);
    time::time(stream.into(), item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
