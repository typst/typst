use heck::ToKebabCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Result, Token};

use crate::util::{
    BlockWithReturn, determine_name_and_title, documentation, foundations, has_attr, kw,
    parse_attr, parse_flag, parse_string, parse_string_array, validate_attrs,
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
    /// Whether the element has the given trait listed as a capability.
    fn can(&self, name: &str) -> bool {
        self.capabilities.iter().any(|capability| capability == name)
    }

    /// Whether the element does not have the given trait listed as a
    /// capability.
    fn cannot(&self, name: &str) -> bool {
        !self.can(name)
    }

    /// Whether the element has the given trait listed as a capability.
    fn with(&self, name: &str) -> Option<&Ident> {
        self.capabilities.iter().find(|capability| *capability == name)
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

    /// Fields that get accessor, with, and push methods.
    fn accessor_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.struct_fields().filter(|field| !field.required)
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
}

/// A field of an [element definition][`Elem`].
struct Field {
    /// The index of the field among all.
    i: u8,
    /// The name of this field.
    ident: Ident,
    /// The identifier `with_{ident}`.
    with_ident: Ident,
    /// The visibility of this field.
    vis: syn::Visibility,
    /// The type of this field.
    ty: syn::Type,
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
    /// Whether this field has a `#[fold]` attribute.
    fold: bool,
    /// Whether this field is excluded from documentation.
    internal: bool,
    /// Whether this field exists only in documentation.
    external: bool,
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

    let mut fields = named.named.iter().map(parse_field).collect::<Result<Vec<_>>>()?;
    fields.sort_by_key(|field| field.internal);
    for (i, field) in fields.iter_mut().enumerate() {
        field.i = i as u8;
    }

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

    let field = Field {
        i: 0,
        ident: ident.clone(),
        with_ident: format_ident!("with_{ident}"),
        vis: field.vis.clone(),
        ty: field.ty.clone(),
        name: ident.to_string().to_kebab_case(),
        docs: documentation(&attrs),
        positional,
        required,
        variadic,
        fold: has_attr(&mut attrs, "fold"),
        internal: has_attr(&mut attrs, "internal"),
        external: has_attr(&mut attrs, "external"),
        ghost: has_attr(&mut attrs, "ghost"),
        synthesized: has_attr(&mut attrs, "synthesized"),
        parse: parse_attr(&mut attrs, "parse")?.flatten(),
        default: parse_attr::<syn::Expr>(&mut attrs, "default")?.flatten(),
    };

    if field.required && field.synthesized {
        bail!(ident, "required fields cannot be synthesized");
    }

    if (field.required || field.synthesized)
        && (field.default.is_some() || field.fold || field.ghost)
    {
        bail!(ident, "required and synthesized fields cannot be default, fold, or ghost");
    }

    validate_attrs(&attrs)?;

    Ok(field)
}

/// Produce the element's definition.
fn create(element: &Elem) -> Result<TokenStream> {
    // The struct itself.
    let struct_ = create_struct(element);

    // Implementations.
    let inherent_impl = create_inherent_impl(element);
    let native_element_impl = create_native_elem_impl(element);
    let field_impls =
        element.fields.iter().map(|field| create_field_impl(element, field));
    let construct_impl =
        element.cannot("Construct").then(|| create_construct_impl(element));
    let set_impl = element.cannot("Set").then(|| create_set_impl(element));
    let unqueriable_impl = element
        .with("Unqueriable")
        .map(|cap| create_introspection_impl(element, cap));
    let locatable_impl = element
        .with("Locatable")
        .map(|cap| create_introspection_impl(element, cap));
    let tagged_impl = element
        .with("Tagged")
        .map(|cap| create_introspection_impl(element, cap));
    let mathy_impl = element.can("Mathy").then(|| create_mathy_impl(element));

    // We use a const block to create an anonymous scope, as to not leak any
    // local definitions.
    Ok(quote! {
        #struct_

        const _: () = {
            #inherent_impl
            #native_element_impl
            #(#field_impls)*
            #construct_impl
            #set_impl
            #unqueriable_impl
            #locatable_impl
            #tagged_impl
            #mathy_impl
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
    let Field { i, vis, ident, ty, .. } = field;
    if field.required {
        quote! { #vis #ident: #ty }
    } else if field.synthesized {
        quote! { #vis #ident: ::std::option::Option<#ty> }
    } else {
        quote! { #vis #ident: #foundations::Settable<Self, #i> }
    }
}

/// Create the inherent implementation of the struct.
fn create_inherent_impl(element: &Elem) -> TokenStream {
    let Elem { ident, .. } = element;

    let new_func = create_new_func(element);
    let with_field_methods = element.accessor_fields().map(create_with_field_method);

    let style_consts = element.real_fields().map(|field| {
        let Field { i, vis, ident, .. } = field;
        quote! {
            #vis const #ident: #foundations::Field<Self, #i>
                = #foundations::Field::new();
        }
    });

    quote! {
        impl #ident {
            #new_func
            #(#with_field_methods)*
        }
        #[allow(non_upper_case_globals)]
        impl #ident {
            #(#style_consts)*
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
        } else if field.synthesized {
            quote! { #ident: None }
        } else {
            quote! { #ident: #foundations::Settable::new() }
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
    let Field { vis, ident, with_ident, name, ty, .. } = field;
    let doc = format!("Builder-style setter for the [`{name}`](Self::{ident}) field.");

    let expr = if field.required {
        quote! { self.#ident = #ident }
    } else if field.synthesized {
        quote! { self.#ident = Some(#ident) }
    } else {
        quote! { self.#ident.set(#ident) }
    };

    quote! {
        #[doc = #doc]
        #vis fn #with_ident(mut self, #ident: #ty) -> Self {
            #expr;
            self
        }
    }
}

/// Creates the element's `NativeElement` implementation.
fn create_native_elem_impl(element: &Elem) -> TokenStream {
    let Elem { name, ident, title, scope, keywords, docs, .. } = element;

    let fields = element.fields.iter().filter(|field| !field.internal).map(|field| {
        let i = field.i;
        if field.external {
            quote! { #foundations::ExternalFieldData::<#ident, #i>::vtable() }
        } else if field.variadic {
            quote! { #foundations::RequiredFieldData::<#ident, #i>::vtable_variadic() }
        } else if field.required {
            quote! { #foundations::RequiredFieldData::<#ident, #i>::vtable() }
        } else if field.synthesized {
            quote! { #foundations::SynthesizedFieldData::<#ident, #i>::vtable() }
        } else if field.ghost {
            quote! { #foundations::SettablePropertyData::<#ident, #i>::vtable() }
        } else {
            quote! { #foundations::SettableFieldData::<#ident, #i>::vtable() }
        }
    });

    let field_arms = element
        .fields
        .iter()
        .filter(|field| !field.internal && !field.external)
        .map(|field| {
            let Field { name, i, .. } = field;
            quote! { #name => Some(#i) }
        });
    let field_id = quote! {
        |name| match name {
            #(#field_arms,)*
            _ => None,
        }
    };

    let capable_func = create_capable_func(element);

    let with_keywords =
        (!keywords.is_empty()).then(|| quote! { .with_keywords(&[#(#keywords),*]) });
    let with_repr = element.can("Repr").then(|| quote! { .with_repr() });
    let with_partial_eq = element.can("PartialEq").then(|| quote! { .with_partial_eq() });
    let with_local_name = element.can("LocalName").then(|| quote! { .with_local_name() });
    let with_scope = scope.then(|| quote! { .with_scope() });

    quote! {
        unsafe impl #foundations::NativeElement for #ident {
            const ELEM: #foundations::Element = #foundations::Element::from_vtable({
                static STORE: #foundations::LazyElementStore
                    = #foundations::LazyElementStore::new();
                static VTABLE: #foundations::ContentVtable =
                    #foundations::ContentVtable::new::<#ident>(
                        #name,
                        #title,
                        #docs,
                        &[#(#fields),*],
                        #field_id,
                        #capable_func,
                        || &STORE,
                    ) #with_keywords
                    #with_repr
                    #with_partial_eq
                    #with_local_name
                    #with_scope
                    .erase();
                &VTABLE
            });
        }
    }
}

/// Creates the appropriate trait implementation for a field.
fn create_field_impl(element: &Elem, field: &Field) -> TokenStream {
    let elem_ident = &element.ident;
    let Field { i, ty, ident, default, positional, name, docs, .. } = field;

    let default = match default {
        Some(default) => quote! { || #default },
        None => quote! { std::default::Default::default },
    };

    if field.external {
        quote! {
            impl #foundations::ExternalField<#i> for #elem_ident {
                type Type = #ty;
                const FIELD: #foundations::ExternalFieldData<Self, #i> =
                    #foundations::ExternalFieldData::<Self, #i>::new(
                        #name,
                        #docs,
                        #default,
                    );
            }
        }
    } else if field.required {
        quote! {
            impl #foundations::RequiredField<#i> for #elem_ident {
                type Type = #ty;
                const FIELD: #foundations::RequiredFieldData<Self, #i> =
                    #foundations::RequiredFieldData::<Self, #i>::new(
                        #name,
                        #docs,
                        |elem| &elem.#ident,
                    );
            }
        }
    } else if field.synthesized {
        quote! {
            impl #foundations::SynthesizedField<#i> for #elem_ident {
                type Type = #ty;
                const FIELD: #foundations::SynthesizedFieldData<Self, #i> =
                    #foundations::SynthesizedFieldData::<Self, #i>::new(
                        #name,
                        #docs,
                        |elem| &elem.#ident,
                    );
            }
        }
    } else {
        let slot = quote! {
            || {
                static LOCK: ::std::sync::OnceLock<#ty> = ::std::sync::OnceLock::new();
                &LOCK
            }
        };

        let with_fold = field.fold.then(|| quote! { .with_fold() });
        let refable = (!field.fold).then(|| {
            quote! {
                impl #foundations::RefableProperty<#i> for #elem_ident {}
            }
        });

        if field.ghost {
            quote! {
                impl #foundations::SettableProperty<#i> for #elem_ident {
                    type Type = #ty;
                    const FIELD: #foundations::SettablePropertyData<Self, #i> =
                        #foundations::SettablePropertyData::<Self, #i>::new(
                            #name,
                            #docs,
                            #positional,
                            #default,
                            #slot,
                        ) #with_fold;
                }
                #refable
            }
        } else {
            quote! {
                impl #foundations::SettableField<#i> for #elem_ident {
                    type Type = #ty;
                    const FIELD: #foundations::SettableFieldData<Self, #i> =
                        #foundations::SettableFieldData::<Self, #i>::new(
                            #name,
                            #docs,
                            #positional,
                            |elem| &elem.#ident,
                            |elem| &mut elem.#ident,
                            #default,
                            #slot,
                        ) #with_fold;
                }
                #refable
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
        if field.required {
            quote! { #ident }
        } else if field.synthesized {
            quote! { #ident: None }
        } else {
            quote! { #ident: #foundations::Settable::from(#ident) }
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
        let field_ident = &field.ident;
        let (prefix, value) = create_field_parser(field);
        quote! {
            #prefix
            if let Some(value) = #value {
                styles.set(Self::#field_ident, value);
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
fn create_capable_func(element: &Elem) -> TokenStream {
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
        |capability| {
            let dangling = ::std::ptr::NonNull::<#foundations::Packed<#ident>>::dangling().as_ptr();
            #(#checks)*
            None
        }
    }
}

/// Creates the element's introspection capability implementation.
fn create_introspection_impl(element: &Elem, capability: &Ident) -> TokenStream {
    let ident = &element.ident;
    quote! { impl ::typst_library::introspection::#capability for #foundations::Packed<#ident> {} }
}

/// Creates the element's `Mathy` implementation.
fn create_mathy_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    quote! { impl ::typst_library::math::Mathy for #foundations::Packed<#ident> {} }
}
