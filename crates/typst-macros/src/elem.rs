use heck::ToKebabCase;

use super::*;

/// Expand the `#[elem]` macro.
pub fn elem(stream: TokenStream, body: syn::ItemStruct) -> Result<TokenStream> {
    let element = parse(stream, &body)?;
    Ok(create(&element))
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
    capabilities: Vec<Ident>,
    fields: Vec<Field>,
}

/// Details about an element field.
struct Field {
    ident: Ident,
    ident_in: Ident,
    with_ident: Ident,
    push_ident: Ident,
    set_ident: Ident,
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
    parse: Option<BlockWithReturn>,
    default: syn::Expr,
}

impl Field {
    /// Whether the field is present on every instance of the element.
    fn inherent(&self) -> bool {
        self.required || self.variadic
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
        synthesized: has_attr(&mut attrs, "synthesized"),
        fold: has_attr(&mut attrs, "fold"),
        resolve: has_attr(&mut attrs, "resolve"),
        parse: parse_attr(&mut attrs, "parse")?.flatten(),
        default: parse_attr(&mut attrs, "default")?
            .flatten()
            .unwrap_or_else(|| parse_quote! { ::std::default::Default::default() }),
        vis: field.vis.clone(),
        ident: ident.clone(),
        ident_in: Ident::new(&format!("{}_in", ident), ident.span()),
        with_ident: Ident::new(&format!("with_{}", ident), ident.span()),
        push_ident: Ident::new(&format!("push_{}", ident), ident.span()),
        set_ident: Ident::new(&format!("set_{}", ident), ident.span()),
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
        field.output = parse_quote! { <#output as ::typst::model::Resolve>::Output };
    }

    if field.fold {
        let output = &field.output;
        field.output = parse_quote! { <#output as ::typst::model::Fold>::Output };
    }

    validate_attrs(&attrs)?;

    Ok(field)
}

/// Produce the element's definition.
fn create(element: &Elem) -> TokenStream {
    let Elem { vis, ident, docs, .. } = element;
    let all = element.fields.iter().filter(|field| !field.external);
    let settable = all.clone().filter(|field| !field.synthesized && field.settable());

    // Inherent methods and functions.
    let new = create_new_func(element);
    let field_methods = all.clone().map(create_field_method);
    let field_in_methods = settable.clone().map(create_field_in_method);
    let with_field_methods = all.clone().map(create_with_field_method);
    let push_field_methods = all.map(create_push_field_method);
    let field_style_methods = settable.map(create_set_field_method);

    // Trait implementations.
    let element_impl = create_pack_impl(element);
    let construct_impl = element
        .capabilities
        .iter()
        .all(|capability| capability != "Construct")
        .then(|| create_construct_impl(element));
    let set_impl = create_set_impl(element);
    let locatable_impl = element
        .capabilities
        .iter()
        .any(|capability| capability == "Locatable")
        .then(|| quote! { impl ::typst::model::Locatable for #ident {} });

    quote! {
        #[doc = #docs]
        #[derive(Debug, Clone, Hash)]
        #[repr(transparent)]
        #vis struct #ident(pub ::typst::model::Content);

        impl #ident {
            #new
            #(#field_methods)*
            #(#field_in_methods)*
            #(#with_field_methods)*
            #(#push_field_methods)*
            #(#field_style_methods)*

            /// The element's span.
            pub fn span(&self) -> ::typst::syntax::Span {
                self.0.span()
            }

             /// Set the element's span.
             pub fn spanned(self, span: ::typst::syntax::Span) -> Self {
                Self(self.0.spanned(span))
            }
        }

        #element_impl
        #construct_impl
        #set_impl
        #locatable_impl

        impl ::typst::eval::IntoValue for #ident {
            fn into_value(self) -> ::typst::eval::Value {
                ::typst::eval::Value::Content(self.0)
            }
        }
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
    let builder_calls = relevant.map(|Field { ident, with_ident, .. }| {
        quote! { .#with_ident(#ident) }
    });
    quote! {
        /// Create a new element.
        pub fn new(#(#params),*) -> Self {
            Self(::typst::model::Content::new(
                <Self as ::typst::model::NativeElement>::elem()
            ))
            #(#builder_calls)*
        }
    }
}

/// Create an accessor methods for a field.
fn create_field_method(field: &Field) -> TokenStream {
    let Field { vis, docs, ident, name, output, .. } = field;
    if field.inherent() || field.synthesized {
        quote! {
            #[doc = #docs]
            #[track_caller]
            #vis fn #ident(&self) -> #output {
                self.0.expect_field(#name)
            }
        }
    } else {
        let access = create_style_chain_access(field, quote! { self.0.field(#name) });
        quote! {
            #[doc = #docs]
            #vis fn #ident(&self, styles: ::typst::model::StyleChain) -> #output {
                #access
            }
        }
    }
}

/// Create a style chain access method for a field.
fn create_field_in_method(field: &Field) -> TokenStream {
    let Field { vis, ident_in, name, output, .. } = field;
    let doc = format!("Access the `{}` field in the given style chain.", name);
    let access = create_style_chain_access(field, quote! { None });
    quote! {
        #[doc = #doc]
        #vis fn #ident_in(styles: ::typst::model::StyleChain) -> #output {
            #access
        }
    }
}

/// Create a style chain access method for a field.
fn create_style_chain_access(field: &Field, inherent: TokenStream) -> TokenStream {
    let Field { name, ty, default, .. } = field;
    let getter = match (field.fold, field.resolve) {
        (false, false) => quote! { get },
        (false, true) => quote! { get_resolve },
        (true, false) => quote! { get_fold },
        (true, true) => quote! { get_resolve_fold },
    };

    quote! {
        styles.#getter::<#ty>(
            <Self as ::typst::model::NativeElement>::elem(),
            #name,
            #inherent,
            || #default,
        )
    }
}

/// Create a builder pattern method for a field.
fn create_with_field_method(field: &Field) -> TokenStream {
    let Field { vis, ident, with_ident, name, ty, .. } = field;
    let doc = format!("Set the [`{}`](Self::{}) field.", name, ident);
    quote! {
        #[doc = #doc]
        #vis fn #with_ident(mut self, #ident: #ty) -> Self {
            Self(self.0.with_field(#name, #ident))
        }
    }
}

/// Create a set-style method for a field.
fn create_push_field_method(field: &Field) -> TokenStream {
    let Field { vis, ident, push_ident, name, ty, .. } = field;
    let doc = format!("Push the [`{}`](Self::{}) field.", name, ident);
    quote! {
        #[doc = #doc]
        #vis fn #push_ident(&mut self, #ident: #ty) {
            self.0.push_field(#name, #ident);
        }
    }
}

/// Create a setter method for a field.
fn create_set_field_method(field: &Field) -> TokenStream {
    let Field { vis, ident, set_ident, name, ty, .. } = field;
    let doc = format!("Create a style property for the `{}` field.", name);
    quote! {
        #[doc = #doc]
        #vis fn #set_ident(#ident: #ty) -> ::typst::model::Style {
            ::typst::model::Style::Property(::typst::model::Property::new(
                <Self as ::typst::model::NativeElement>::elem(),
                #name,
                #ident,
            ))
        }
    }
}

/// Create the element's `Pack` implementation.
fn create_pack_impl(element: &Elem) -> TokenStream {
    let eval = quote! { ::typst::eval };
    let model = quote! { ::typst::model };

    let Elem { name, ident, title, scope, keywords, docs, .. } = element;
    let vtable_func = create_vtable_func(element);
    let params = element
        .fields
        .iter()
        .filter(|field| !field.internal && !field.synthesized)
        .map(create_param_info);

    let scope = if *scope {
        quote! { <#ident as #eval::NativeScope>::scope() }
    } else {
        quote! { #eval::Scope::new() }
    };

    let data = quote! {
        #model::NativeElementData {
            name: #name,
            title: #title,
            docs: #docs,
            keywords: &[#(#keywords),*],
            construct: <#ident as #model::Construct>::construct,
            set: <#ident as #model::Set>::set,
            vtable: #vtable_func,
            scope: #eval::Lazy::new(|| #scope),
            params: #eval::Lazy::new(|| ::std::vec![#(#params),*])
        }
    };

    quote! {
        impl #model::NativeElement for #ident {
            fn data() -> &'static #model::NativeElementData {
                static DATA: #model::NativeElementData = #data;
                &DATA
            }

            fn pack(self) -> #model::Content {
                self.0
            }

            fn unpack(content: &#model::Content) -> ::std::option::Option<&Self> {
                // Safety: Elements are #[repr(transparent)].
                content.is::<Self>().then(|| unsafe {
                    ::std::mem::transmute(content)
                })
            }
        }
    }
}

/// Create the element's casting vtable.
fn create_vtable_func(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let relevant = element.capabilities.iter().filter(|&ident| ident != "Construct");
    let checks = relevant.map(|capability| {
        quote! {
            if id == ::std::any::TypeId::of::<dyn #capability>() {
                return Some(unsafe {
                    ::typst::util::fat::vtable(&null as &dyn #capability)
                });
            }
        }
    });

    quote! {
        |id| {
            let null = Self(::typst::model::Content::new(
                <#ident as ::typst::model::NativeElement>::elem()
            ));
            #(#checks)*
            None
        }
    }
}

/// Create a parameter info for a field.
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
        quote! {
            || {
                let typed: #default_ty = #default;
                ::typst::eval::IntoValue::into_value(typed)
            }
        }
    }));
    let ty = if *variadic {
        quote! { <#ty as ::typst::eval::Container>::Inner }
    } else {
        quote! { #ty }
    };
    quote! {
        ::typst::eval::ParamInfo {
            name: #name,
            docs: #docs,
            input: <#ty as ::typst::eval::Reflect>::input(),
            default: #default,
            positional: #positional,
            named: #named,
            variadic: #variadic,
            required: #required,
            settable: #settable,
        }
    }
}

/// Create the element's `Construct` implementation.
fn create_construct_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let handlers = element
        .fields
        .iter()
        .filter(|field| {
            !field.external
                && !field.synthesized
                && (!field.internal || field.parse.is_some())
        })
        .map(|field| {
            let push_ident = &field.push_ident;
            let (prefix, value) = create_field_parser(field);
            if field.settable() {
                quote! {
                    #prefix
                    if let Some(value) = #value {
                        element.#push_ident(value);
                    }
                }
            } else {
                quote! {
                    #prefix
                    element.#push_ident(#value);
                }
            }
        });

    quote! {
        impl ::typst::model::Construct for #ident {
            fn construct(
                vm: &mut ::typst::eval::Vm,
                args: &mut ::typst::eval::Args,
            ) -> ::typst::diag::SourceResult<::typst::model::Content> {
                let mut element = Self(::typst::model::Content::new(
                    <Self as ::typst::model::NativeElement>::elem()
                ));
                #(#handlers)*
                Ok(element.0)
            }
        }
    }
}

/// Create the element's `Set` implementation.
fn create_set_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let handlers = element
        .fields
        .iter()
        .filter(|field| {
            !field.external
                && !field.synthesized
                && field.settable()
                && (!field.internal || field.parse.is_some())
        })
        .map(|field| {
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
        impl ::typst::model::Set for #ident {
            fn set(
                vm: &mut Vm,
                args: &mut ::typst::eval::Args,
            ) -> ::typst::diag::SourceResult<::typst::model::Styles> {
                let mut styles = ::typst::model::Styles::new();
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
