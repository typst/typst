use heck::{ToKebabCase, ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_quote, Ident, Result, Token};

use crate::util::{
    determine_name_and_title, documentation, foundations, has_attr, kw, parse_attr,
    parse_flag, parse_string, parse_string_array, quote_option, validate_attrs,
    BlockWithReturn,
};

/// Expand the `#[elem]` macro.
pub fn elem(stream: TokenStream, body: syn::ItemStruct) -> Result<TokenStream> {
    let element = parse(stream, &body)?;
    create(&element)
}

/// Details about an element.
struct Elem {
    name: String,
    title: String,
    scope: bool,
    keywords: Vec<String>,
    docs: String,
    vis: syn::Visibility,
    ident: Ident,
    enum_ident: Ident,
    capabilities: Vec<Ident>,
    fields: Vec<Field>,
}

impl Elem {
    /// Calls the closure to produce a token stream if the
    /// element does not have the given capability.
    fn unless_capability(
        &self,
        name: &str,
        closure: impl FnOnce() -> TokenStream,
    ) -> Option<TokenStream> {
        self.capabilities
            .iter()
            .all(|capability| capability != name)
            .then(closure)
    }

    /// Calls the closure to produce a token stream if the
    /// element has the given capability.
    fn if_capability(
        &self,
        name: &str,
        closure: impl FnOnce() -> TokenStream,
    ) -> Option<TokenStream> {
        self.capabilities
            .iter()
            .any(|capability| capability == name)
            .then(closure)
    }

    /// All fields.
    ///
    /// This includes:
    /// - Fields that are not external and therefore present in the struct.
    /// - Fields that are ghost fields.
    fn real_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.fields.iter().filter(|field| !field.external)
    }

    /// Fields that are present in the struct.
    ///
    /// This includes:
    /// - Fields that are not external and therefore present in the struct.
    /// - Fields that are not ghost fields.
    fn present_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields().filter(|field| !field.ghost)
    }

    /// Fields that are inherent to the element.
    fn inherent_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields().filter(|field| field.inherent())
    }

    /// Fields that can be set with style rules.
    ///
    /// The reason why fields that are `parse` and internal are allowed
    /// is because it's a pattern used a lot for parsing data from the
    /// input and then storing it in a field.
    ///
    /// This includes:
    /// - Fields that are not synthesized.
    /// - Fields that are not inherent and therefore present at all times.
    /// - Fields that are not internal.
    fn settable_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields().filter(|field| {
            !field.synthesized
                && field.settable()
                && (!field.internal || field.parse.is_some())
        })
    }

    /// Fields that are visible to the user.
    ///
    /// This includes:
    /// - Fields that are not internal.
    fn visible_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields().filter(|field| !field.internal)
    }

    /// Fields that are relevant for equality.
    ///
    /// This includes:
    /// - Fields that are not synthesized (guarantees equality before and after synthesis).
    fn eq_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.present_fields().filter(|field| !field.synthesized)
    }

    /// Fields that are relevant for `Construct` impl.
    ///
    /// This includes:
    /// - Fields that are not synthesized.
    fn construct_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.real_fields().filter(|field| !field.synthesized)
    }
}

struct Field {
    ident: Ident,
    ident_in: Ident,
    with_ident: Ident,
    push_ident: Ident,
    set_ident: Ident,
    enum_ident: Ident,
    const_ident: Ident,
    vis: syn::Visibility,
    ty: syn::Type,
    output: syn::Type,
    name: String,
    docs: String,
    positional: bool,
    required: bool,
    variadic: bool,
    resolve: bool,
    fold: bool,
    internal: bool,
    external: bool,
    synthesized: bool,
    borrowed: bool,
    ghost: bool,
    parse: Option<BlockWithReturn>,
    default: Option<syn::Expr>,
}

impl Field {
    /// Whether the field is present on every instance of the element.
    fn inherent(&self) -> bool {
        (self.required || self.variadic) && !self.ghost
    }

    /// Whether the field can be used with set rules.
    fn settable(&self) -> bool {
        !self.inherent()
    }
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
    let fields = named.named.iter().map(parse_field).collect::<Result<_>>()?;

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
        enum_ident: Ident::new(&format!("{}Fields", body.ident), body.ident.span()),
    })
}

fn parse_field(field: &syn::Field) -> Result<Field> {
    let Some(ident) = field.ident.clone() else {
        bail!(field, "expected named field");
    };

    if ident == "label" {
        bail!(ident, "invalid field name");
    }

    let mut attrs = field.attrs.clone();
    let variadic = has_attr(&mut attrs, "variadic");
    let required = has_attr(&mut attrs, "required") || variadic;
    let positional = has_attr(&mut attrs, "positional") || required;

    let mut field = Field {
        name: ident.to_string().to_kebab_case(),
        docs: documentation(&attrs),
        internal: has_attr(&mut attrs, "internal"),
        external: has_attr(&mut attrs, "external"),
        positional,
        required,
        variadic,
        borrowed: has_attr(&mut attrs, "borrowed"),
        synthesized: has_attr(&mut attrs, "synthesized"),
        fold: has_attr(&mut attrs, "fold"),
        resolve: has_attr(&mut attrs, "resolve"),
        ghost: has_attr(&mut attrs, "ghost"),
        parse: parse_attr(&mut attrs, "parse")?.flatten(),
        default: parse_attr::<syn::Expr>(&mut attrs, "default")?.flatten(),
        vis: field.vis.clone(),
        ident: ident.clone(),
        ident_in: Ident::new(&format!("{ident}_in"), ident.span()),
        with_ident: Ident::new(&format!("with_{ident}"), ident.span()),
        push_ident: Ident::new(&format!("push_{ident}"), ident.span()),
        set_ident: Ident::new(&format!("set_{ident}"), ident.span()),
        enum_ident: Ident::new(&ident.to_string().to_upper_camel_case(), ident.span()),
        const_ident: Ident::new(&ident.to_string().to_shouty_snake_case(), ident.span()),
        ty: field.ty.clone(),
        output: field.ty.clone(),
    };

    if field.required && (field.fold || field.resolve) {
        bail!(ident, "required fields cannot be folded or resolved");
    }

    if field.required && !field.positional {
        bail!(ident, "only positional fields can be required");
    }

    if field.resolve {
        let output = &field.output;
        field.output = parse_quote! { <#output as #foundations::Resolve>::Output };
    }

    if field.fold {
        let output = &field.output;
        field.output = parse_quote! { <#output as #foundations::Fold>::Output };
    }

    validate_attrs(&attrs)?;

    Ok(field)
}

/// Produce the element's definition.
fn create(element: &Elem) -> Result<TokenStream> {
    let Elem { vis, ident, docs, .. } = element;

    if element.fields.iter().any(|field| field.ghost)
        && element
            .capabilities
            .iter()
            .all(|capability| capability != "Construct")
    {
        bail!(ident, "cannot have ghost fields and have `Construct` auto generated");
    }

    let all = element.real_fields();
    let present = element.present_fields();
    let settable = all.clone().filter(|field| !field.synthesized && field.settable());

    let fields = present.clone().map(create_field);
    let fields_enum = create_fields_enum(element);

    let new = create_new_func(element);
    let field_methods = all.clone().map(|field| create_field_method(element, field));
    let field_in_methods =
        settable.clone().map(|field| create_field_in_method(element, field));
    let with_field_methods = present.clone().map(create_with_field_method);
    let push_field_methods = present.clone().map(create_push_field_method);
    let field_style_methods =
        settable.clone().map(|field| create_set_field_method(element, field));

    // Trait implementations.
    let native_element_impl = create_native_elem_impl(element);
    let construct_impl =
        element.unless_capability("Construct", || create_construct_impl(element));
    let set_impl = element.unless_capability("Set", || create_set_impl(element));
    let locatable_impl =
        element.if_capability("Locatable", || create_locatable_impl(element));
    let partial_eq_impl =
        element.unless_capability("PartialEq", || create_partial_eq_impl(element));
    let repr_impl = element.unless_capability("Repr", || create_repr_impl(element));

    let label_and_location = element.unless_capability("Unlabellable", || {
        quote! {
            location: Option<::typst::introspection::Location>,
            label: Option<#foundations::Label>,
            prepared: bool,
        }
    });

    Ok(quote! {
        #[doc = #docs]
        #[derive(Debug, Clone, Hash)]
        #[allow(clippy::derived_hash_with_manual_eq)]
        #vis struct #ident {
            span: ::typst::syntax::Span,
            #label_and_location
            guards: ::std::vec::Vec<#foundations::Guard>,

            #(#fields,)*
        }

        #fields_enum

        impl #ident {
            #new
            #(#field_methods)*
            #(#field_in_methods)*
            #(#with_field_methods)*
            #(#push_field_methods)*
            #(#field_style_methods)*
        }

        #native_element_impl
        #construct_impl
        #set_impl
        #locatable_impl
        #partial_eq_impl
        #repr_impl

        impl #foundations::IntoValue for #ident {
            fn into_value(self) -> #foundations::Value {
                #foundations::Value::Content(#foundations::Content::new(self))
            }
        }
    })
}

/// Create a field declaration.
fn create_field(field: &Field) -> TokenStream {
    let Field {
        ident, ty, docs, required, synthesized, default, ..
    } = field;

    let ty = required.then(|| quote! { #ty }).unwrap_or_else(|| {
        if *synthesized && default.is_some() {
            quote! { #ty }
        } else {
            quote! { ::std::option::Option<#ty> }
        }
    });
    quote! {
        #[doc = #docs]
        #ident: #ty
    }
}

/// Creates the element's enum for field identifiers.
fn create_fields_enum(element: &Elem) -> TokenStream {
    let Elem { ident, enum_ident, .. } = element;

    let fields = element.real_fields().collect::<Vec<_>>();
    let field_names = fields.iter().map(|Field { name, .. }| name).collect::<Vec<_>>();
    let field_consts = fields
        .iter()
        .map(|Field { const_ident, .. }| const_ident)
        .collect::<Vec<_>>();

    let field_variants = fields
        .iter()
        .map(|Field { enum_ident, .. }| enum_ident)
        .collect::<Vec<_>>();

    let definitions = fields.iter().map(|Field { enum_ident, .. }| {
        quote! { #enum_ident }
    });

    quote! {
        // To hide the private type
        const _: () = {
            impl #foundations::ElementFields for #ident {
                type Fields = #enum_ident;
            }

            #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
            #[repr(u8)]
            pub enum #enum_ident {
                #(#definitions,)*
                Label = 255,
            }

            impl #enum_ident {
                /// Converts this field identifier to the field name.
                pub fn to_str(self) -> &'static str {
                    match self {
                        #(Self::#field_variants => #field_names,)*
                        Self::Label => "label",
                    }
                }
            }

            impl ::std::convert::TryFrom<u8> for #enum_ident {
                type Error = ();

                fn try_from(value: u8) -> Result<Self, Self::Error> {
                    #(const #field_consts: u8 = #enum_ident::#field_variants as u8;)*
                    match value {
                        #(#field_consts => Ok(Self::#field_variants),)*
                        255 => Ok(Self::Label),
                        _ => Err(()),
                    }
                }
            }

            impl ::std::fmt::Display for #enum_ident {
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    f.pad(self.to_str())
                }
            }

            impl ::std::str::FromStr for #enum_ident {
                type Err = ();

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        #(#field_names => Ok(Self::#field_variants),)*
                        "label" => Ok(Self::Label),
                        _ => Err(()),
                    }
                }
            }
        };
    }
}

/// Create the `new` function for the element.
fn create_new_func(element: &Elem) -> TokenStream {
    let relevant = element
        .fields
        .iter()
        .filter(|field| !field.external && !field.synthesized && field.inherent());
    let params = relevant.clone().map(|Field { ident, ty, .. }| {
        quote! { #ident: #ty }
    });
    let required = relevant.map(|Field { ident, .. }| {
        quote! { #ident }
    });
    let defaults = element
        .fields
        .iter()
        .filter(|field| {
            !field.external && !field.inherent() && !field.synthesized && !field.ghost
        })
        .map(|Field { ident, .. }| quote! { #ident: None });
    let default_synthesized = element
        .fields
        .iter()
        .filter(|field| !field.external && field.synthesized && !field.ghost)
        .map(|Field { ident, default, .. }| {
            if let Some(expr) = default {
                quote! { #ident: #expr }
            } else {
                quote! { #ident: None }
            }
        });

    let label_and_location = element.unless_capability("Unlabellable", || {
        quote! {
            location: None,
            label: None,
            prepared: false,
        }
    });

    quote! {
        /// Create a new element.
        pub fn new(#(#params),*) -> Self {
            Self {
                span: ::typst::syntax::Span::detached(),
                #label_and_location
                guards: ::std::vec::Vec::with_capacity(0),
                #(#required,)*
                #(#defaults,)*
                #(#default_synthesized,)*
            }
        }
    }
}

/// Create a builder pattern method for a field.
fn create_with_field_method(field: &Field) -> TokenStream {
    let Field {
        vis,
        ident,
        with_ident,
        name,
        ty,
        synthesized,
        default,
        ..
    } = field;
    let doc = format!("Set the [`{name}`](Self::{ident}) field.");

    let set = if field.inherent() || (*synthesized && default.is_some()) {
        quote! { self.#ident = #ident; }
    } else {
        quote! { self.#ident = Some(#ident); }
    };
    quote! {
        #[doc = #doc]
        #vis fn #with_ident(mut self, #ident: #ty) -> Self {
            #set
            self
        }
    }
}

/// Create a set-style method for a field.
fn create_push_field_method(field: &Field) -> TokenStream {
    let Field {
        vis,
        ident,
        push_ident,
        name,
        ty,
        synthesized,
        default,
        ..
    } = field;
    let doc = format!("Push the [`{name}`](Self::{ident}) field.");
    let set = if (field.inherent() && !synthesized) || (*synthesized && default.is_some())
    {
        quote! { self.#ident = #ident; }
    } else {
        quote! { self.#ident = Some(#ident); }
    };
    quote! {
        #[doc = #doc]
        #vis fn #push_ident(&mut self, #ident: #ty) {
            #set
        }
    }
}

/// Create a setter method for a field.
fn create_set_field_method(element: &Elem, field: &Field) -> TokenStream {
    let elem = &element.ident;
    let Field { vis, ident, set_ident, enum_ident, ty, name, .. } = field;
    let doc = format!("Create a style property for the `{name}` field.");
    quote! {
        #[doc = #doc]
        #vis fn #set_ident(#ident: #ty) -> #foundations::Style {
            #foundations::Style::Property(#foundations::Property::new(
                <Self as #foundations::NativeElement>::elem(),
                <#elem as #foundations::ElementFields>::Fields::#enum_ident as u8,
                #ident,
            ))
        }
    }
}

/// Create a style chain access method for a field.
fn create_field_in_method(element: &Elem, field: &Field) -> TokenStream {
    let Field { vis, ident_in, name, output, .. } = field;
    let doc = format!("Access the `{name}` field in the given style chain.");
    let access = create_style_chain_access(element, field, quote! { None });

    let output = if field.borrowed {
        quote! { &#output }
    } else {
        quote! { #output }
    };

    quote! {
        #[doc = #doc]
        #vis fn #ident_in(styles: #foundations::StyleChain) -> #output {
            #access
        }
    }
}

/// Create an accessor methods for a field.
fn create_field_method(element: &Elem, field: &Field) -> TokenStream {
    let Field { vis, docs, ident, output, ghost, .. } = field;

    let inherent = if *ghost {
        quote! { None }
    } else {
        quote! { self.#ident.as_ref() }
    };

    if (field.inherent() && !field.synthesized)
        || (field.synthesized && field.default.is_some())
    {
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
            #vis fn #ident(&self) -> &#output {
                self.#ident.as_ref().unwrap()
            }
        }
    } else if field.borrowed {
        let access = create_style_chain_access(element, field, inherent);

        quote! {
            #[doc = #docs]
            #vis fn #ident<'a>(&'a self, styles: #foundations::StyleChain<'a>) -> &'a #output {
                #access
            }
        }
    } else {
        let access = create_style_chain_access(element, field, inherent);

        quote! {
            #[doc = #docs]
            #vis fn #ident(&self, styles: #foundations::StyleChain) -> #output {
                #access
            }
        }
    }
}

/// Create a style chain access method for a field.
fn create_style_chain_access(
    element: &Elem,
    field: &Field,
    inherent: TokenStream,
) -> TokenStream {
    let elem = &element.ident;

    let Field { ty, default, enum_ident, .. } = field;
    let getter = match (field.fold, field.resolve, field.borrowed) {
        (false, false, false) => quote! { get },
        (false, false, true) => quote! { get_borrowed },
        (false, true, _) => quote! { get_resolve },
        (true, false, _) => quote! { get_fold },
        (true, true, _) => quote! { get_resolve_fold },
    };

    let default = default
        .clone()
        .unwrap_or_else(|| parse_quote! { ::std::default::Default::default() });
    let (init, default) = field.fold.then(|| (None, quote! { || #default })).unwrap_or_else(|| (
        Some(quote! {
            static DEFAULT: ::once_cell::sync::Lazy<#ty> = ::once_cell::sync::Lazy::new(|| #default);
        }),
        quote! { &DEFAULT },
    ));

    quote! {
        #init
        styles.#getter::<#ty>(
            <Self as #foundations::NativeElement>::elem(),
            <#elem as #foundations::ElementFields>::Fields::#enum_ident as u8,
            #inherent,
            #default,
        )
    }
}

/// Creates the element's `Pack` implementation.
fn create_native_elem_impl(element: &Elem) -> TokenStream {
    let Elem { name, ident, title, scope, keywords, docs, .. } = element;

    let vtable_func = create_vtable_func(element);
    let params = element
        .fields
        .iter()
        .filter(|field| !field.internal && !field.synthesized)
        .map(create_param_info);

    let scope = if *scope {
        quote! { <#ident as #foundations::NativeScope>::scope() }
    } else {
        quote! { #foundations::Scope::new() }
    };

    // Fields that can be accessed using the `field` method.
    let field_matches = element.visible_fields().map(|field| {
        let elem = &element.ident;
        let name = &field.enum_ident;
        let field_ident = &field.ident;

        if field.ghost {
            quote! {
                <#elem as #foundations::ElementFields>::Fields::#name => None,
            }
        } else if field.inherent() || (field.synthesized && field.default.is_some()) {
            quote! {
                <#elem as #foundations::ElementFields>::Fields::#name => Some(
                    #foundations::IntoValue::into_value(self.#field_ident.clone())
                ),
            }
        } else {
            quote! {
                <#elem as #foundations::ElementFields>::Fields::#name => {
                    self.#field_ident.clone().map(#foundations::IntoValue::into_value)
                }
            }
        }
    });

    // Fields that can be checked using the `has` method.
    let field_has_matches = element.visible_fields().map(|field| {
        let elem = &element.ident;
        let name = &field.enum_ident;
        let field_ident = &field.ident;

        if field.ghost {
            quote! {
                <#elem as #foundations::ElementFields>::Fields::#name => false,
            }
        } else if field.inherent() || (field.synthesized && field.default.is_some()) {
            quote! {
                <#elem as #foundations::ElementFields>::Fields::#name => true,
            }
        } else {
            quote! {
                <#elem as #foundations::ElementFields>::Fields::#name => self.#field_ident.is_some(),
            }
        }
    });

    // Fields that can be set using the `set_field` method.
    let field_set_matches = element
        .visible_fields()
        .filter(|field| field.settable() && !field.synthesized && !field.ghost)
        .map(|field| {
            let elem = &element.ident;
            let name = &field.enum_ident;
            let field_ident = &field.ident;

            quote! {
                <#elem as #foundations::ElementFields>::Fields::#name => {
                    self.#field_ident = Some(#foundations::FromValue::from_value(value)?);
                    return Ok(());
                }
            }
        });

    // Fields that are inherent.
    let field_inherent_matches = element
        .visible_fields()
        .filter(|field| field.inherent() && !field.ghost)
        .map(|field| {
            let elem = &element.ident;
            let name = &field.enum_ident;
            let field_ident = &field.ident;

            quote! {
                <#elem as #foundations::ElementFields>::Fields::#name => {
                    self.#field_ident = #foundations::FromValue::from_value(value)?;
                    return Ok(());
                }
            }
        });

    // Fields that cannot be set or are internal create an error.
    let field_not_set_matches = element
        .real_fields()
        .filter(|field| field.internal || field.synthesized || field.ghost)
        .map(|field| {
            let elem = &element.ident;
            let ident = &field.enum_ident;
            let field_name = &field.name;
            if field.internal {
                // Internal fields create an error that they are unknown.
                let unknown_field = format!("unknown field `{field_name}` on `{name}`");
                quote! {
                    <#elem as #foundations::ElementFields>::Fields::#ident => ::typst::diag::bail!(#unknown_field),
                }
            } else {
                // Fields that cannot be set create an error that they are not settable.
                let not_settable = format!("cannot set `{field_name}` on `{name}`");
                quote! {
                    <#elem as #foundations::ElementFields>::Fields::#ident => ::typst::diag::bail!(#not_settable),
                }
            }
        });

    // Statistically compute whether we need preparation or not.
    let needs_preparation = element
        .unless_capability("Unlabellable", || {
            element
                .capabilities
                .iter()
                .any(|capability| capability == "Locatable" || capability == "Synthesize")
                .then(|| quote! { !self.prepared })
                .unwrap_or_else(|| quote! { self.label().is_some() && !self.prepared })
        })
        .unwrap_or_else(|| {
            assert!(element.capabilities.iter().all(|capability| capability
                != "Locatable"
                && capability != "Synthesize"));
            quote! { false }
        });

    // Creation of the fields dictionary for inherent fields.
    let field_dict = element.inherent_fields().clone().map(|field| {
        let name = &field.name;
        let field_ident = &field.ident;

        let field_call = if name.len() > 15 {
            quote! { ::ecow::EcoString::from(#name).into() }
        } else {
            quote! { ::ecow::EcoString::inline(#name).into() }
        };

        quote! {
            fields.insert(
                #field_call,
                #foundations::IntoValue::into_value(self.#field_ident.clone())
            );
        }
    });

    // Creation of the fields dictionary for optional fields.
    let field_opt_dict = element
        .visible_fields()
        .filter(|field| !field.inherent() && !field.ghost)
        .clone()
        .map(|field| {
            let name = &field.name;
            let field_ident = &field.ident;

            let field_call = if name.len() > 15 {
                quote! { ::ecow::EcoString::from(#name).into() }
            } else {
                quote! { ::ecow::EcoString::inline(#name).into() }
            };

            if field.synthesized && field.default.is_some() {
                quote! {
                    fields.insert(
                        #field_call,
                        #foundations::IntoValue::into_value(self.#field_ident.clone())
                    );
                }
            } else {
                quote! {
                    if let Some(value) = &self.#field_ident {
                        fields.insert(
                            #field_call,
                            #foundations::IntoValue::into_value(value.clone())
                        );
                    }
                }
            }
        });

    let location = element
        .unless_capability("Unlabellable", || quote! { self.location })
        .unwrap_or_else(|| quote! { None });

    let set_location = element
        .unless_capability("Unlabellable", || {
            quote! {
                self.location = Some(location);
            }
        })
        .unwrap_or_else(|| quote! { drop(location) });

    let label = element
        .unless_capability("Unlabellable", || quote! { self.label })
        .unwrap_or_else(|| quote! { None });

    let set_label = element
        .unless_capability("Unlabellable", || {
            quote! {
                self.label = Some(label);
            }
        })
        .unwrap_or_else(|| quote! { drop(label) });

    let label_field = element
        .unless_capability("Unlabellable", || {
            quote! {
                self.label().map(#foundations::Value::Label)
            }
        })
        .unwrap_or_else(|| quote! { None });

    let label_has_field = element
        .unless_capability("Unlabellable", || quote! { self.label().is_some() })
        .unwrap_or_else(|| quote! { false });

    let label_field_dict = element.unless_capability("Unlabellable", || {
        quote! {
            if let Some(label) = self.label() {
                fields.insert(
                    ::ecow::EcoString::inline("label").into(),
                    #foundations::IntoValue::into_value(label)
                );
            }
        }
    });

    let mark_prepared = element
        .unless_capability("Unlabellable", || quote! { self.prepared = true; })
        .unwrap_or_else(|| quote! {});

    let prepared = element
        .unless_capability("Unlabellable", || quote! { self.prepared })
        .unwrap_or_else(|| quote! { true });

    let local_name = element
        .if_capability(
            "LocalName",
            || quote! { Some(<#ident as ::typst::text::LocalName>::local_name) },
        )
        .unwrap_or_else(|| quote! { None });

    let unknown_field = format!("unknown field {{}} on {name}");
    let label_error = format!("cannot set label on {name}");
    let data = quote! {
        #foundations::NativeElementData {
            name: #name,
            title: #title,
            docs: #docs,
            keywords: &[#(#keywords),*],
            construct: <#ident as #foundations::Construct>::construct,
            set: <#ident as #foundations::Set>::set,
            vtable: #vtable_func,
            field_id: |name|
                <
                    <#ident as #foundations::ElementFields>::Fields as ::std::str::FromStr
                >::from_str(name).ok().map(|id| id as u8),
            field_name: |id|
                <
                    <#ident as #foundations::ElementFields>::Fields as ::std::convert::TryFrom<u8>
                >::try_from(id).ok().map(<#ident as #foundations::ElementFields>::Fields::to_str),
            local_name: #local_name,
            scope: #foundations::Lazy::new(|| #scope),
            params: #foundations::Lazy::new(|| ::std::vec![#(#params),*])
        }
    };

    quote! {
        impl #foundations::NativeElement for #ident {
            fn data() -> &'static #foundations::NativeElementData {
                static DATA: #foundations::NativeElementData = #data;
                &DATA
            }

            fn dyn_elem(&self) -> #foundations::Element {
                #foundations::Element::of::<Self>()
            }

            fn dyn_hash(&self, mut state: &mut dyn ::std::hash::Hasher) {
                // Also hash the TypeId since values with different types but
                // equal data should be different.
                ::std::hash::Hash::hash(&::std::any::TypeId::of::<Self>(), &mut state);
                ::std::hash::Hash::hash(self, &mut state);
            }

            fn dyn_eq(&self, other: &#foundations::Content) -> bool {
                if let Some(other) = other.to::<Self>() {
                    <Self as ::std::cmp::PartialEq>::eq(self, other)
                } else {
                    false
                }
            }

            fn dyn_clone(&self) -> ::std::sync::Arc<dyn #foundations::NativeElement> {
                ::std::sync::Arc::new(Clone::clone(self))
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
                self
            }

            fn into_any(self: ::std::sync::Arc<Self>) -> ::std::sync::Arc<dyn ::std::any::Any + Send + Sync> {
                self
            }

            fn span(&self) -> ::typst::syntax::Span {
                self.span
            }

            fn set_span(&mut self, span: ::typst::syntax::Span) {
                if self.span().is_detached() {
                    self.span = span;
                }
            }

            fn label(&self) -> Option<#foundations::Label> {
                #label
            }

            fn set_label(&mut self, label: #foundations::Label) {
                #set_label
            }

            fn location(&self) -> Option<::typst::introspection::Location> {
                #location
            }

            fn set_location(&mut self, location: ::typst::introspection::Location) {
                #set_location
            }

            fn push_guard(&mut self, guard: #foundations::Guard) {
                self.guards.push(guard);
            }

            fn is_guarded(&self, guard: #foundations::Guard) -> bool {
                self.guards.contains(&guard)
            }

            fn is_pristine(&self) -> bool {
                self.guards.is_empty()
            }

            fn mark_prepared(&mut self) {
                #mark_prepared
            }

            fn needs_preparation(&self) -> bool {
                #needs_preparation
            }

            fn is_prepared(&self) -> bool {
                #prepared
            }

            fn field(&self, id: u8) -> Option<#foundations::Value> {
                let id = <#ident as #foundations::ElementFields>::Fields::try_from(id).ok()?;
                match id {
                    <#ident as #foundations::ElementFields>::Fields::Label => #label_field,
                    #(#field_matches)*
                    _ => None,
                }
            }

            fn has(&self, id: u8) -> bool {
                let Ok(id) = <#ident as #foundations::ElementFields>::Fields::try_from(id) else {
                    return false;
                };

                match id {
                    <#ident as #foundations::ElementFields>::Fields::Label => #label_has_field,
                    #(#field_has_matches)*
                    _ => false,
                }
            }

            fn fields(&self) -> #foundations::Dict {
                let mut fields = #foundations::Dict::new();
                #label_field_dict
                #(#field_dict)*
                #(#field_opt_dict)*
                fields
            }

            fn set_field(&mut self, id: u8, value: #foundations::Value) -> ::typst::diag::StrResult<()> {
                let id = <#ident as #foundations::ElementFields>::Fields::try_from(id)
                    .map_err(|_| ::ecow::eco_format!(#unknown_field, id))?;
                match id {
                    #(#field_set_matches)*
                    #(#field_inherent_matches)*
                    #(#field_not_set_matches)*
                    <#ident as #foundations::ElementFields>::Fields::Label => {
                        ::typst::diag::bail!(#label_error);
                    }
                }
            }
        }
    }
}

/// Creates the element's `Construct` implementation.
fn create_construct_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let pre = element.construct_fields().map(|field| {
        let (prefix, value) = create_field_parser(field);
        let ident = &field.ident;
        quote! {
            #prefix
            let #ident = #value;
        }
    });

    let handlers =
        element
            .construct_fields()
            .filter(|field| field.settable())
            .map(|field| {
                let push_ident = &field.push_ident;
                let ident = &field.ident;
                quote! {
                    if let Some(value) = #ident {
                        element.#push_ident(value);
                    }
                }
            });

    let defaults = element
        .construct_fields()
        .filter(|field| !field.settable())
        .map(|field| &field.ident);

    quote! {
        impl #foundations::Construct for #ident {
            fn construct(
                engine: &mut ::typst::engine::Engine,
                args: &mut #foundations::Args,
            ) -> ::typst::diag::SourceResult<#foundations::Content> {
                #(#pre)*

                let mut element = Self::new(#(#defaults),*);

                #(#handlers)*

                Ok(#foundations::Content::new(element))
            }
        }
    }
}

/// Creates the element's `Set` implementation.
fn create_set_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let handlers = element.settable_fields().map(|field| {
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
                engine: &mut ::typst::engine::Engine,
                args: &mut #foundations::Args,
            ) -> ::typst::diag::SourceResult<#foundations::Styles> {
                let mut styles = #foundations::Styles::new();
                #(#handlers)*
                Ok(styles)
            }
        }
    }
}

/// Creates the element's `Locatable` implementation.
fn create_locatable_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    quote! { impl ::typst::introspection::Locatable for #ident {} }
}

/// Creates the element's `PartialEq` implementation.
fn create_partial_eq_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let all = element.eq_fields().map(|field| &field.ident).collect::<Vec<_>>();

    let empty = all.is_empty().then(|| quote! { true });
    quote! {
        impl PartialEq for #ident {
            fn eq(&self, other: &Self) -> bool {
                #empty
                #(self.#all == other.#all)&&*
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
                let fields = #foundations::NativeElement::fields(self).into_iter()
                    .map(|(name, value)| ::ecow::eco_format!("{}: {}", name, value.repr()))
                    .collect::<Vec<_>>();
                ::ecow::eco_format!(#repr_format, #foundations::repr::pretty_array_like(&fields, false))
            }
        }
    }
}

/// Creates the element's casting vtable.
fn create_vtable_func(element: &Elem) -> TokenStream {
    // Forbidden capabilities (i.e capabilities that are not object safe).
    const FORBIDDEN: &[&str] = &["Construct", "PartialEq", "Hash", "LocalName"];

    let ident = &element.ident;
    let relevant = element
        .capabilities
        .iter()
        .filter(|&ident| !FORBIDDEN.contains(&(&ident.to_string() as &str)));
    let checks = relevant.map(|capability| {
        quote! {
            if id == ::std::any::TypeId::of::<dyn #capability>() {
                let vtable = unsafe {
                    let dangling = ::std::ptr::NonNull::<#ident>::dangling().as_ptr() as *const dyn #capability;
                    ::typst::util::fat::vtable(dangling)
                };
                return Some(vtable);
            }
        }
    });

    quote! {
        |id| {
            #(#checks)*
            None
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
        fold,
        ty,
        output,
        ..
    } = field;
    let named = !positional;
    let settable = field.settable();
    let default_ty = if *fold { &output } else { &ty };
    let default = quote_option(&settable.then(|| {
        let default = default
            .clone()
            .unwrap_or_else(|| parse_quote! { ::std::default::Default::default() });
        quote! {
            || {
                let typed: #default_ty = #default;
                #foundations::IntoValue::into_value(typed)
            }
        }
    }));
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
