use heck::{ToKebabCase, ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_quote, Ident, Result, Token};

use crate::util::{
    determine_name_and_title, documentation, foundations, has_attr, kw, parse_attr,
    parse_flag, parse_string, parse_string_array, validate_attrs, BlockWithReturn,
};

/// Expand the `#[elem]` macro.
pub fn elem(stream: TokenStream, body: syn::ItemStruct) -> Result<TokenStream> {
    let element = parse(stream, &body)?;
    create(&element)
}

/// Details about an element.
struct Elem {
    /// The element's name as exposed to Typst.
    name: String,
    /// The element's title case name.
    title: String,
    /// Whether this element has an associated scope defined by the `#[scope]` macro.
    scope: bool,
    /// A list of alternate search terms for this element.
    keywords: Vec<String>,
    /// The documentation for this element as a string.
    docs: String,
    /// The element's visibility.
    vis: syn::Visibility,
    /// The struct name for this element given in Rust.
    ident: Ident,
    /// The list of capabilities for this element.
    capabilities: Vec<Ident>,
    /// The fields of this element.
    fields: Vec<Field>,
}

impl Elem {
    /// Calls the closure to produce a token stream if the
    /// element has the given capability.
    fn can(&self, name: &str) -> bool {
        self.capabilities.iter().any(|capability| capability == name)
    }

    /// Calls the closure to produce a token stream if the
    /// element does not have the given capability.
    fn cannot(&self, name: &str) -> bool {
        !self.can(name)
    }
}

impl Elem {
    /// All fields that are not just external.
    fn real_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.fields.iter().filter(|field| !field.external)
    }

    /// Fields that are present in the generated struct.
    fn struct_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields().filter(|field| !field.ghost)
    }

    /// Fields that are relevant for equality.
    ///
    /// Synthesized fields are excluded to ensure equality before and after
    /// synthesis.
    fn eq_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.struct_fields().filter(|field| !field.synthesized)
    }

    /// Fields that show up in the documentation.
    fn doc_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.fields
            .iter()
            .filter(|field| !field.internal && !field.synthesized)
    }

    /// Fields that are relevant for `Construct` impl.
    ///
    /// The reason why fields that are `parse` and internal are allowed is
    /// because it's a pattern used a lot for parsing data from the input and
    /// then storing it in a field.
    fn construct_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields().filter(|field| {
            field.parse.is_some() || (!field.synthesized && !field.internal)
        })
    }

    /// Fields that can be configured with set rules.
    fn set_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.construct_fields().filter(|field| !field.required)
    }

    /// Fields that can be accessed from the style chain.
    fn style_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields()
            .filter(|field| !field.required && !field.synthesized)
    }

    /// Fields that are visible to the user.
    fn visible_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields().filter(|field| !field.internal)
    }
}

/// A field of an [element definition][`Elem`].
struct Field {
    /// The name of this field.
    ident: Ident,
    /// The identifier `{ident}_in`.
    ident_in: Ident,
    /// The identifier `with_{ident}`.
    with_ident: Ident,
    /// The identifier `push_{ident}`.
    push_ident: Ident,
    /// The identifier `set_{ident}`.
    set_ident: Ident,
    /// The upper camel-case version of `ident`, used for the enum variant name.
    enum_ident: Ident,
    /// The all-caps snake-case version of `ident`, used for the constant name.
    const_ident: Ident,
    /// The visibility of this field.
    vis: syn::Visibility,
    /// The type of this field.
    ty: syn::Type,
    /// The type returned by accessor methods for this field.
    ///
    /// Usually, this is the same as `ty`, but this might be different
    /// if this field has a `#[resolve]` attribute.
    output: syn::Type,
    /// The field's identifier as exposed to Typst.
    name: String,
    /// The documentation for this field as a string.
    docs: String,
    /// Whether this field is positional (as opposed to named).
    positional: bool,
    /// Whether this field is required.
    required: bool,
    /// Whether this field is variadic; that is, has its values
    /// taken from a variable number of arguments.
    variadic: bool,
    /// Whether this field has a `#[resolve]` attribute.
    resolve: bool,
    /// Whether this field has a `#[fold]` attribute.
    fold: bool,
    /// Whether this field is excluded from documentation.
    internal: bool,
    /// Whether this field exists only in documentation.
    external: bool,
    /// Whether this field has a `#[borrowed]` attribute.
    borrowed: bool,
    /// Whether this field has a `#[ghost]` attribute.
    ghost: bool,
    /// Whether this field has a `#[synthesized]` attribute.
    synthesized: bool,
    /// The contents of the `#[parse({..})]` attribute, if any.
    parse: Option<BlockWithReturn>,
    /// The contents of the `#[default(..)]` attribute, if any.
    default: Option<syn::Expr>,
}

/// The `..` in `#[elem(..)]`.
struct Meta {
    scope: bool,
    name: Option<String>,
    title: Option<String>,
    keywords: Vec<String>,
    capabilities: Vec<Ident>,
}

impl Parse for Meta {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            scope: parse_flag::<kw::scope>(input)?,
            name: parse_string::<kw::name>(input)?,
            title: parse_string::<kw::title>(input)?,
            keywords: parse_string_array::<kw::keywords>(input)?,
            capabilities: Punctuated::<Ident, Token![,]>::parse_terminated(input)?
                .into_iter()
                .collect(),
        })
    }
}

/// Parse details about the element from its struct definition.
fn parse(stream: TokenStream, body: &syn::ItemStruct) -> Result<Elem> {
    let meta: Meta = syn::parse2(stream)?;
    let (name, title) = determine_name_and_title(
        meta.name,
        meta.title,
        &body.ident,
        Some(|base| base.trim_end_matches("Elem")),
    )?;

    let docs = documentation(&body.attrs);

    let syn::Fields::Named(named) = &body.fields else {
        bail!(body, "expected named fields");
    };

    let fields = named.named.iter().map(parse_field).collect::<Result<Vec<_>>>()?;
    if fields.iter().any(|field| field.ghost && !field.internal)
        && meta.capabilities.iter().all(|capability| capability != "Construct")
    {
        bail!(
            body.ident,
            "cannot have public ghost fields and an auto-generated constructor"
        );
    }

    Ok(Elem {
        name,
        title,
        scope: meta.scope,
        keywords: meta.keywords,
        docs,
        vis: body.vis.clone(),
        ident: body.ident.clone(),
        capabilities: meta.capabilities,
        fields,
    })
}

fn parse_field(field: &syn::Field) -> Result<Field> {
    let Some(ident) = field.ident.clone() else {
        bail!(field, "expected named field");
    };

    if ident == "label" {
        bail!(ident, "invalid field name `label`");
    }

    let mut attrs = field.attrs.clone();
    let variadic = has_attr(&mut attrs, "variadic");
    let required = has_attr(&mut attrs, "required") || variadic;
    let positional = has_attr(&mut attrs, "positional") || required;

    let mut field = Field {
        ident: ident.clone(),
        ident_in: format_ident!("{ident}_in"),
        with_ident: format_ident!("with_{ident}"),
        push_ident: format_ident!("push_{ident}"),
        set_ident: format_ident!("set_{ident}"),
        enum_ident: Ident::new(&ident.to_string().to_upper_camel_case(), ident.span()),
        const_ident: Ident::new(&ident.to_string().to_shouty_snake_case(), ident.span()),
        vis: field.vis.clone(),
        ty: field.ty.clone(),
        output: field.ty.clone(),
        name: ident.to_string().to_kebab_case(),
        docs: documentation(&attrs),
        positional,
        required,
        variadic,
        resolve: has_attr(&mut attrs, "resolve"),
        fold: has_attr(&mut attrs, "fold"),
        internal: has_attr(&mut attrs, "internal"),
        external: has_attr(&mut attrs, "external"),
        borrowed: has_attr(&mut attrs, "borrowed"),
        ghost: has_attr(&mut attrs, "ghost"),
        synthesized: has_attr(&mut attrs, "synthesized"),
        parse: parse_attr(&mut attrs, "parse")?.flatten(),
        default: parse_attr::<syn::Expr>(&mut attrs, "default")?.flatten(),
    };

    if field.required && field.synthesized {
        bail!(ident, "required fields cannot be synthesized");
    }

    if (field.required || field.synthesized)
        && (field.default.is_some() || field.fold || field.resolve || field.ghost)
    {
        bail!(
            ident,
            "required and synthesized fields cannot be default, fold, resolve, or ghost"
        );
    }

    if field.resolve {
        let ty = &field.ty;
        field.output = parse_quote! { <#ty as #foundations::Resolve>::Output };
    }

    validate_attrs(&attrs)?;

    Ok(field)
}

/// Produce the element's definition.
fn create(element: &Elem) -> Result<TokenStream> {
    // The struct itself.
    let struct_ = create_struct(element);
    let inherent_impl = create_inherent_impl(element);

    // The enum with the struct's fields.
    let fields_enum = create_fields_enum(element);

    // The statics with borrowed fields' default values.
    let default_statics = element
        .style_fields()
        .filter(|field| field.borrowed)
        .map(create_default_static);

    // Trait implementations.
    let native_element_impl = create_native_elem_impl(element);
    let partial_eq_impl =
        element.cannot("PartialEq").then(|| create_partial_eq_impl(element));
    let construct_impl =
        element.cannot("Construct").then(|| create_construct_impl(element));
    let set_impl = element.cannot("Set").then(|| create_set_impl(element));
    let capable_impl = create_capable_impl(element);
    let fields_impl = create_fields_impl(element);
    let repr_impl = element.cannot("Repr").then(|| create_repr_impl(element));
    let locatable_impl = element.can("Locatable").then(|| create_locatable_impl(element));
    let mathy_impl = element.can("Mathy").then(|| create_mathy_impl(element));
    let into_value_impl = create_into_value_impl(element);

    // We use a const block to create an anonymous scope, as to not leak any
    // local definitions.
    Ok(quote! {
        #struct_

        const _: () = {
            #fields_enum
            #(#default_statics)*
            #inherent_impl
            #native_element_impl
            #fields_impl
            #capable_impl
            #construct_impl
            #set_impl
            #partial_eq_impl
            #repr_impl
            #locatable_impl
            #mathy_impl
            #into_value_impl
        };
    })
}

/// Create the struct definition itself.
fn create_struct(element: &Elem) -> TokenStream {
    let Elem { vis, ident, docs, .. } = element;

    let debug = element.cannot("Debug").then(|| quote! { Debug, });
    let fields = element.struct_fields().map(create_field);

    quote! {
        #[doc = #docs]
        #[derive(#debug Clone, Hash)]
        #[allow(clippy::derived_hash_with_manual_eq)]
        #[allow(rustdoc::broken_intra_doc_links)]
        #vis struct #ident {
            #(#fields,)*
        }
    }
}

/// Create a field declaration for the struct.
fn create_field(field: &Field) -> TokenStream {
    let Field { vis, ident, ty, .. } = field;
    if field.required {
        quote! { #vis #ident: #ty }
    } else {
        quote! { #ident: ::std::option::Option<#ty> }
    }
}

/// Creates the element's enum for field identifiers.
fn create_fields_enum(element: &Elem) -> TokenStream {
    let variants: Vec<_> = element.real_fields().map(|field| &field.enum_ident).collect();
    let names: Vec<_> = element.real_fields().map(|field| &field.name).collect();
    let consts: Vec<_> = element.real_fields().map(|field| &field.const_ident).collect();
    let repr = (!variants.is_empty()).then(|| quote! { #[repr(u8)] });

    quote! {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        #repr
        pub enum Fields {
            #(#variants,)*
        }

        impl Fields {
            /// Converts the field identifier to the field name.
            pub fn to_str(self) -> &'static str {
                match self {
                    #(Self::#variants => #names,)*
                }
            }
        }

        impl ::std::convert::TryFrom<u8> for Fields {
            type Error = #foundations::FieldAccessError;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                #(const #consts: u8 = Fields::#variants as u8;)*
                match value {
                    #(#consts => Ok(Self::#variants),)*
                    _ => Err(#foundations::FieldAccessError::Internal),
                }
            }
        }

        impl ::std::str::FromStr for Fields {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(#names => Ok(Self::#variants),)*
                    _ => Err(()),
                }
            }
        }

        impl ::std::fmt::Display for Fields {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.pad(self.to_str())
            }
        }
    }
}

/// Creates a static with a borrowed field's default value.
fn create_default_static(field: &Field) -> TokenStream {
    let Field { const_ident, default, ty, .. } = field;

    let init = match default {
        Some(default) => quote! { || #default },
        None => quote! { ::std::default::Default::default },
    };

    quote! {
        static #const_ident: ::std::sync::LazyLock<#ty> =
        ::std::sync::LazyLock::new(#init);
    }
}

/// Create the inherent implementation of the struct.
fn create_inherent_impl(element: &Elem) -> TokenStream {
    let Elem { ident, .. } = element;

    let new_func = create_new_func(element);
    let with_field_methods = element.struct_fields().map(create_with_field_method);
    let push_field_methods = element.struct_fields().map(create_push_field_method);
    let field_methods = element.struct_fields().map(create_field_method);
    let field_in_methods = element.style_fields().map(create_field_in_method);
    let set_field_methods = element.style_fields().map(create_set_field_method);

    quote! {
        impl #ident {
            #new_func
            #(#with_field_methods)*
            #(#push_field_methods)*
            #(#field_methods)*
            #(#field_in_methods)*
            #(#set_field_methods)*
        }
    }
}

/// Create the `new` function for the element.
fn create_new_func(element: &Elem) -> TokenStream {
    let params = element
        .struct_fields()
        .filter(|field| field.required)
        .map(|Field { ident, ty, .. }| quote! { #ident: #ty });

    let fields = element.struct_fields().map(|field| {
        let ident = &field.ident;
        if field.required {
            quote! { #ident }
        } else {
            quote! { #ident: None }
        }
    });

    quote! {
        /// Create a new instance of the element.
        pub fn new(#(#params),*) -> Self {
            Self { #(#fields,)* }
        }
    }
}

/// Create a builder-style setter method for a field.
fn create_with_field_method(field: &Field) -> TokenStream {
    let Field { vis, ident, with_ident, push_ident, name, ty, .. } = field;
    let doc = format!("Builder-style setter for the [`{name}`](Self::{ident}) field.");
    quote! {
        #[doc = #doc]
        #vis fn #with_ident(mut self, #ident: #ty) -> Self {
            self.#push_ident(#ident);
            self
        }
    }
}

/// Create a setter method for a field.
fn create_push_field_method(field: &Field) -> TokenStream {
    let Field { vis, ident, push_ident, name, ty, .. } = field;
    let doc = format!("Setter for the [`{name}`](Self::{ident}) field.");

    let expr = if field.required {
        quote! { #ident }
    } else {
        quote! { Some(#ident) }
    };

    quote! {
        #[doc = #doc]
        #vis fn #push_ident(&mut self, #ident: #ty) {
            self.#ident = #expr;
        }
    }
}

/// Create an accessor method for a field.
fn create_field_method(field: &Field) -> TokenStream {
    let Field { vis, docs, ident, output, .. } = field;

    if field.required {
        quote! {
            #[doc = #docs]
            #vis fn #ident(&self) -> &#output {
                &self.#ident
            }
        }
    } else if field.synthesized {
        quote! {
            #[doc = #docs]
            #[track_caller]
            #vis fn #ident(&self) -> ::std::option::Option<&#output> {
                self.#ident.as_ref()
            }
        }
    } else {
        let sig = if field.borrowed {
            quote! { <'a>(&'a self, styles: #foundations::StyleChain<'a>) -> &'a #output }
        } else {
            quote! { (&self, styles: #foundations::StyleChain) -> #output }
        };

        let mut value = create_style_chain_access(
            field,
            field.borrowed,
            quote! { self.#ident.as_ref() },
        );
        if field.resolve {
            value = quote! { #foundations::Resolve::resolve(#value, styles) };
        }

        quote! {
            #[doc = #docs]
            #vis fn #ident #sig {
                #value
            }
        }
    }
}

/// Create a style accessor method for a field.
fn create_field_in_method(field: &Field) -> TokenStream {
    let Field { vis, ident_in, name, output, .. } = field;
    let doc = format!("Access the `{name}` field in the given style chain.");

    let ref_ = field.borrowed.then(|| quote! { & });

    let mut value = create_style_chain_access(field, field.borrowed, quote! { None });
    if field.resolve {
        value = quote! { #foundations::Resolve::resolve(#value, styles) };
    }

    quote! {
        #[doc = #doc]
        #vis fn #ident_in(styles: #foundations::StyleChain) -> #ref_ #output {
            #value
        }
    }
}

/// Create a style setter method for a field.
fn create_set_field_method(field: &Field) -> TokenStream {
    let Field { vis, ident, set_ident, enum_ident, ty, name, .. } = field;
    let doc = format!("Create a style property for the `{name}` field.");

    quote! {
        #[doc = #doc]
        #vis fn #set_ident(#ident: #ty) -> #foundations::Property {
            #foundations::Property::new::<Self, _>(
                Fields::#enum_ident as u8,
                #ident,
            )
        }
    }
}

/// Create a style chain access method for a field.
fn create_style_chain_access(
    field: &Field,
    borrowed: bool,
    inherent: TokenStream,
) -> TokenStream {
    let Field { ty, default, enum_ident, const_ident, .. } = field;

    let getter = match (field.fold, borrowed) {
        (false, false) => quote! { get },
        (false, true) => quote! { get_ref },
        (true, _) => quote! { get_folded },
    };

    let default = if borrowed {
        quote! { || &#const_ident }
    } else {
        match default {
            Some(default) => quote! { || #default },
            None => quote! { ::std::default::Default::default },
        }
    };

    quote! {
        styles.#getter::<#ty>(
            <Self as #foundations::NativeElement>::elem(),
            Fields::#enum_ident as u8,
            #inherent,
            #default,
        )
    }
}

/// Creates the element's `NativeElement` implementation.
fn create_native_elem_impl(element: &Elem) -> TokenStream {
    let Elem { name, ident, title, scope, keywords, docs, .. } = element;

    let local_name = if element.can("LocalName") {
        quote! { Some(<#foundations::Packed<#ident> as ::typst_library::text::LocalName>::local_name) }
    } else {
        quote! { None }
    };

    let scope = if *scope {
        quote! { <#ident as #foundations::NativeScope>::scope() }
    } else {
        quote! { #foundations::Scope::new() }
    };

    let params = element.doc_fields().map(create_param_info);

    let data = quote! {
        #foundations::NativeElementData {
            name: #name,
            title: #title,
            docs: #docs,
            keywords: &[#(#keywords),*],
            construct: <#ident as #foundations::Construct>::construct,
            set: <#ident as #foundations::Set>::set,
            vtable:  <#ident as #foundations::Capable>::vtable,
            field_id: |name| name.parse().ok().map(|id: Fields| id as u8),
            field_name: |id| id.try_into().ok().map(Fields::to_str),
            field_from_styles: <#ident as #foundations::Fields>::field_from_styles,
            local_name: #local_name,
            scope: #foundations::LazyLock::new(|| #scope),
            params: #foundations::LazyLock::new(|| ::std::vec![#(#params),*])
        }
    };

    quote! {
        impl #foundations::NativeElement for #ident {
            fn data() -> &'static #foundations::NativeElementData {
                static DATA: #foundations::NativeElementData = #data;
                &DATA
            }
        }
    }
}

/// Creates a parameter info for a field.
fn create_param_info(field: &Field) -> TokenStream {
    let Field {
        name,
        docs,
        positional,
        variadic,
        required,
        default,
        ty,
        ..
    } = field;

    let named = !positional;
    let settable = !field.required;

    let default = if settable {
        let default = default
            .clone()
            .unwrap_or_else(|| parse_quote! { ::std::default::Default::default() });
        quote! {
            Some(|| <#ty as #foundations::IntoValue>::into_value(#default))
        }
    } else {
        quote! { None }
    };

    let ty = if *variadic {
        quote! { <#ty as #foundations::Container>::Inner }
    } else {
        quote! { #ty }
    };

    quote! {
        #foundations::ParamInfo {
            name: #name,
            docs: #docs,
            input: <#ty as #foundations::Reflect>::input(),
            default: #default,
            positional: #positional,
            named: #named,
            variadic: #variadic,
            required: #required,
            settable: #settable,
        }
    }
}

/// Creates the element's `PartialEq` implementation.
fn create_partial_eq_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let empty = element.eq_fields().next().is_none().then(|| quote! { true });
    let fields = element.eq_fields().map(|field| &field.ident);

    quote! {
        impl PartialEq for #ident {
            fn eq(&self, other: &Self) -> bool {
                #empty
                #(self.#fields == other.#fields)&&*
            }
        }
    }
}

/// Creates the element's `Construct` implementation.
fn create_construct_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let setup = element.construct_fields().map(|field| {
        let (prefix, value) = create_field_parser(field);
        let ident = &field.ident;
        quote! {
            #prefix
            let #ident = #value;
        }
    });

    let fields = element.struct_fields().map(|field| {
        let ident = &field.ident;
        if field.synthesized {
            quote! { #ident: None }
        } else {
            quote! { #ident }
        }
    });

    quote! {
        impl #foundations::Construct for #ident {
            fn construct(
                engine: &mut ::typst_library::engine::Engine,
                args: &mut #foundations::Args,
            ) -> ::typst_library::diag::SourceResult<#foundations::Content> {
                #(#setup)*
                Ok(#foundations::Content::new(Self { #(#fields),* }))
            }
        }
    }
}

/// Creates the element's `Set` implementation.
fn create_set_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let handlers = element.set_fields().map(|field| {
        let set_ident = &field.set_ident;
        let (prefix, value) = create_field_parser(field);
        quote! {
            #prefix
            if let Some(value) = #value {
                styles.set(Self::#set_ident(value));
            }
        }
    });

    quote! {
        impl #foundations::Set for #ident {
            fn set(
                engine: &mut ::typst_library::engine::Engine,
                args: &mut #foundations::Args,
            ) -> ::typst_library::diag::SourceResult<#foundations::Styles> {
                let mut styles = #foundations::Styles::new();
                #(#handlers)*
                Ok(styles)
            }
        }
    }
}

/// Create argument parsing code for a field.
fn create_field_parser(field: &Field) -> (TokenStream, TokenStream) {
    if let Some(BlockWithReturn { prefix, expr }) = &field.parse {
        return (quote! { #(#prefix);* }, quote! { #expr });
    }

    let name = &field.name;
    let value = if field.variadic {
        quote! { args.all()? }
    } else if field.required {
        quote! { args.expect(#name)? }
    } else if field.positional {
        quote! { args.find()? }
    } else {
        quote! { args.named(#name)? }
    };

    (quote! {}, value)
}

/// Creates the element's casting vtable.
fn create_capable_impl(element: &Elem) -> TokenStream {
    // Forbidden capabilities (i.e capabilities that are not object safe).
    const FORBIDDEN: &[&str] =
        &["Debug", "PartialEq", "Hash", "Construct", "Set", "Repr", "LocalName"];

    let ident = &element.ident;
    let relevant = element
        .capabilities
        .iter()
        .filter(|&ident| !FORBIDDEN.contains(&(&ident.to_string() as &str)));

    let checks = relevant.map(|capability| {
        quote! {
            if capability == ::std::any::TypeId::of::<dyn #capability>() {
                // Safety: The vtable function doesn't require initialized
                // data, so it's fine to use a dangling pointer.
                return Some(unsafe {
                    ::typst_utils::fat::vtable(dangling as *const dyn #capability)
                });
            }
        }
    });

    quote! {
        unsafe impl #foundations::Capable for #ident {
            fn vtable(capability: ::std::any::TypeId) -> ::std::option::Option<::std::ptr::NonNull<()>> {
                let dangling = ::std::ptr::NonNull::<#foundations::Packed<#ident>>::dangling().as_ptr();
                #(#checks)*
                None
            }
        }
    }
}

/// Creates the element's `Fields` implementation.
fn create_fields_impl(element: &Elem) -> TokenStream {
    let into_value = quote! { #foundations::IntoValue::into_value };
    let visible_non_ghost = || element.visible_fields().filter(|field| !field.ghost);

    // Fields that can be checked using the `has` method.
    let has_arms = visible_non_ghost().map(|field| {
        let Field { enum_ident, ident, .. } = field;

        let expr = if field.required {
            quote! { true }
        } else {
            quote! { self.#ident.is_some() }
        };

        quote! { Fields::#enum_ident => #expr }
    });

    // Fields that can be accessed using the `field` method.
    let field_arms = visible_non_ghost().map(|field| {
        let Field { enum_ident, ident, .. } = field;

        let expr = if field.required {
            quote! { Ok(#into_value(self.#ident.clone())) }
        } else {
            quote! { self.#ident.clone().map(#into_value).ok_or(#foundations::FieldAccessError::Unset) }
        };

        quote! { Fields::#enum_ident => #expr }
    });

    // Fields that can be accessed using the `field_with_styles` method.
    let field_with_styles_arms = element.visible_fields().map(|field| {
        let Field { enum_ident, ident, .. } = field;

        let expr = if field.required {
            quote! { Ok(#into_value(self.#ident.clone())) }
        } else if field.synthesized {
            quote! { self.#ident.clone().map(#into_value).ok_or(#foundations::FieldAccessError::Unset) }
        } else {
            let value = create_style_chain_access(
                field,
                false,
                if field.ghost { quote!(None) } else { quote!(self.#ident.as_ref()) },
            );

            quote! { Ok(#into_value(#value)) }
        };

        quote! { Fields::#enum_ident => #expr }
    });

    // Fields that can be accessed using the `field_from_styles` method.
    let field_from_styles_arms = element.visible_fields().map(|field| {
        let Field { enum_ident, .. } = field;

        let expr = if field.required || field.synthesized {
            quote! { Err(#foundations::FieldAccessError::Unknown) }
        } else {
            let value = create_style_chain_access(field, false, quote!(None));
            quote! { Ok(#into_value(#value)) }
        };

        quote! { Fields::#enum_ident => #expr }
    });

    // Sets fields from the style chain.
    let materializes = visible_non_ghost()
        .filter(|field| !field.required && !field.synthesized)
        .map(|field| {
            let Field { ident, .. } = field;
            let value = create_style_chain_access(
                field,
                false,
                if field.ghost { quote!(None) } else { quote!(self.#ident.as_ref()) },
            );

            if field.fold {
                quote! { self.#ident = Some(#value); }
            } else {
                quote! {
                    if self.#ident.is_none() {
                        self.#ident = Some(#value);
                    }
                }
            }
        });

    // Creation of the `fields` dictionary for inherent fields.
    let field_inserts = visible_non_ghost().map(|field| {
        let Field { ident, name, .. } = field;
        let string = quote! { #name.into() };

        if field.required {
            quote! {
                fields.insert(#string, #into_value(self.#ident.clone()));
            }
        } else {
            quote! {
                if let Some(value) = &self.#ident {
                    fields.insert(#string, #into_value(value.clone()));
                }
            }
        }
    });

    let Elem { ident, .. } = element;

    let result = quote! {
        Result<#foundations::Value, #foundations::FieldAccessError>
    };

    quote! {
        impl #foundations::Fields for #ident {
            type Enum = Fields;

            fn has(&self, id: u8) -> bool {
                let Ok(id) = Fields::try_from(id) else {
                    return false;
                };

                match id {
                    #(#has_arms,)*
                    _ => false,
                }
            }

            fn field(&self, id: u8) -> #result {
                let id = Fields::try_from(id)?;
                match id {
                    #(#field_arms,)*
                    // This arm might be reached if someone tries to access an
                    // internal field.
                    _ => Err(#foundations::FieldAccessError::Unknown),
                }
            }

            fn field_with_styles(&self, id: u8, styles: #foundations::StyleChain) -> #result {
                let id = Fields::try_from(id)?;
                match id {
                    #(#field_with_styles_arms,)*
                    // This arm might be reached if someone tries to access an
                    // internal field.
                    _ => Err(#foundations::FieldAccessError::Unknown),
                }
            }

            fn field_from_styles(id: u8, styles: #foundations::StyleChain) -> #result {
                let id = Fields::try_from(id)?;
                match id {
                    #(#field_from_styles_arms,)*
                    // This arm might be reached if someone tries to access an
                    // internal field.
                    _ => Err(#foundations::FieldAccessError::Unknown),
                }
            }

            fn materialize(&mut self, styles: #foundations::StyleChain) {
               #(#materializes)*
            }

            fn fields(&self) -> #foundations::Dict {
                let mut fields = #foundations::Dict::new();
                #(#field_inserts)*
                fields
            }
        }
    }
}

/// Creates the element's `Repr` implementation.
fn create_repr_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let repr_format = format!("{}{{}}", element.name);
    quote! {
        impl #foundations::Repr for #ident {
            fn repr(&self) -> ::ecow::EcoString {
                let fields = #foundations::Fields::fields(self)
                    .into_iter()
                    .map(|(name, value)| ::ecow::eco_format!("{}: {}", name, value.repr()))
                    .collect::<Vec<_>>();
                ::ecow::eco_format!(
                    #repr_format,
                    #foundations::repr::pretty_array_like(&fields, false),
                )
            }
        }
    }
}

/// Creates the element's `Locatable` implementation.
fn create_locatable_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    quote! { impl ::typst_library::introspection::Locatable for #foundations::Packed<#ident> {} }
}

/// Creates the element's `Mathy` implementation.
fn create_mathy_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    quote! { impl ::typst_library::math::Mathy for #foundations::Packed<#ident> {} }
}

/// Creates the element's `IntoValue` implementation.
fn create_into_value_impl(element: &Elem) -> TokenStream {
    let Elem { ident, .. } = element;
    quote! {
        impl #foundations::IntoValue for #ident {
            fn into_value(self) -> #foundations::Value {
                #foundations::Value::Content(#foundations::Content::new(self))
            }
        }
    }
}
