use heck::{ToKebabCase, ToPascalCase};

use super::*;

/// Expand the `#[selem]` macro.
pub fn selem(stream: TokenStream, body: syn::ItemStruct) -> Result<TokenStream> {
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
    enum_ident: Ident,
    capabilities: Vec<Ident>,
    fields: Vec<Field>,
}

struct Field {
    ident: Ident,
    ident_in: Ident,
    with_ident: Ident,
    push_ident: Ident,
    set_ident: Ident,
    enum_ident: Ident,
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
    not_hash: bool,
    children: bool,
    parse: Option<BlockWithReturn>,
    default: syn::Expr,
    empty: syn::Expr,
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

/// Produce the element's definition.
fn create(element: &Elem) -> TokenStream {
    let Elem { vis, ident, docs, name, .. } = element;
    let all = element.fields.iter().filter(|field| !field.external);
    let settable = all.clone().filter(|field| !field.synthesized && field.settable());

    let new = create_new_func(element);
    let fields = all.clone().map(create_field);
    let field_methods = all.clone().map(create_field_method);
    let field_in_methods = settable.clone().map(create_field_in_method);
    let with_field_methods = all.clone().map(create_with_field_method);
    let push_field_methods = all.clone().map(create_push_field_method);
    let field_style_methods = settable.clone().map(create_set_field_method);

    // Trait implementations.
    let element_impl = create_pack_impl(element);
    let default_impl = create_default_impl(element);
    let hash_impl = create_hash_impl(element);
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
    let partial_eq_impl = element
        .capabilities
        .iter()
        .all(|capability| capability != "PartialEq")
        .then(|| create_partial_eq_impl(element));
    let repr_impl = element
        .capabilities
        .iter()
        .all(|capability| capability != "Repr")
        .then(|| create_repr_impl(element));
    let debug_impl = element
        .capabilities
        .iter()
        .all(|capability| capability != "Debug")
        .then(|| create_debug_impl(element));

    let field_matches = all.clone().filter(|field| !field.internal).map(|field| {
        let name = &field.name;
        let field_ident = &field.ident;

        quote! {
            #name => Some(::typst::eval::IntoValue::into_value(self.#field_ident.clone())),
        }
    });

    let field_set_matches = settable.map(|field| {
        let name = &field.name;
        let field_ident = &field.ident;

        quote! {
            #name => {
                self.#field_ident = Some(::typst::eval::FromValue::from_value(value)?);
                return Ok(());
            }
        }
    });

    let field_inherent_matches = all.clone().filter(|field| !field.internal && !field.settable()).map(|field| {
        let name = &field.name;
        let field_ident = &field.ident;

        quote! {
            #name => {
                self.#field_ident = ::typst::eval::FromValue::from_value(value)?;
                return Ok(());
            }
        }
    });

    let needs_preparation = element
        .capabilities
        .iter()
        .any(|capability| capability == "Locatable" || capability == "Synthesize")
        .then(|| quote! { true })
        .unwrap_or_else(|| quote! { false });

    let field_dict = element
        .fields
        .iter()
        .filter(|field| !field.external && !field.synthesized && field.inherent())
        .clone()
        .map(|field| {
            let name = &field.name;
            let field_ident = &field.ident;

            quote! {
                fields.insert(
                    EcoString::inline(#name).into(),
                    ::typst::eval::IntoValue::into_value(self.#field_ident.clone())
                );
            }
        });

    let field_opt_dict = element
        .fields
        .iter()
        .filter(|field| {
            !field.external && !field.internal && (field.synthesized || !field.inherent())
        })
        .clone()
        .map(|field| {
            let name = &field.name;
            let field_ident = &field.ident;

            quote! {
                if let Some(value) = &self.#field_ident {
                    fields.insert(
                        EcoString::inline(#name).into(),
                        ::typst::eval::IntoValue::into_value(value.clone())
                    );
                }
            }
        });

    let unknown_field = format!("unknown field {{}} on {}", name);

    let children = element
        .fields
        .iter()
        .find(|field| field.children)
        .map(|field| {
            let ident = &field.ident;
            quote! { &self.#ident }
        }).unwrap_or_else(|| quote! { &[] });

    quote! {
        #[doc = #docs]
        #[derive(Clone)]
        #vis struct #ident {
            span: ::typst::syntax::Span,
            location: Option<::typst::model::Location>,
            label: Option<::typst::model::Label>,
            prepared: bool,
            guards: ::std::vec::Vec<::typst::model::Guard>,

            #(#fields,)*
        }

        impl #ident {
            #new
            #(#field_methods)*
            #(#field_in_methods)*
            #(#with_field_methods)*
            #(#push_field_methods)*
            #(#field_style_methods)*

            /// Set the element's span.
            pub fn spanned(mut self, span: ::typst::syntax::Span) -> Self {
                self.span = span;
                self
            }

            /// Set the element's location.
            pub fn located(mut self, location: ::typst::model::Location) -> Self {
                self.location = Some(location);
                self
            }

            /// Set the element's label.
            pub fn labelled(mut self, label: ::typst::model::Label) -> Self {
                self.label = Some(label);
                self
            }
        }

        #default_impl
        #element_impl
        #hash_impl
        #construct_impl
        #set_impl
        #locatable_impl
        #partial_eq_impl
        #repr_impl
        #debug_impl

        impl ::typst::model::Element for #ident {
            fn data(&self) -> ::typst::model::ElementData {
                ::typst::model::ElementData::of::<Self>()
            }

            fn span(&self) -> ::typst::syntax::Span {
                self.span
            }

            fn set_span(&mut self, span: ::typst::syntax::Span) {
                if self.span().is_detached() {
                    self.span = span;
                }
            }

            fn location(&self) -> Option<::typst::model::Location> {
                self.location
            }

            fn set_location(&mut self, location: ::typst::model::Location) {
                self.location = Some(location);
            }

            fn label(&self) -> Option<&::typst::model::Label> {
                self.label.as_ref()
            }

            fn set_label(&mut self, label: ::typst::model::Label) {
                self.label = Some(label);
            }

            fn push_guard(&mut self, guard: ::typst::model::Guard) {
                self.guards.push(guard);
            }

            fn is_guarded(&self, guard: ::typst::model::Guard) -> bool {
                self.guards.contains(&guard)
            }

            fn is_pristine(&self) -> bool {
                self.guards.is_empty()
            }

            fn guards(&self) -> &[::typst::model::Guard] {
                &self.guards
            }

            fn mark_prepared(&mut self) {
                self.prepared = true;
            }

            fn needs_preparation(&self) -> bool {
                (#needs_preparation || self.label().is_some()) && !self.prepared
            }

            fn is_prepared(&self) -> bool {
                self.prepared
            }

            fn dyn_hash(&self, mut hasher: &mut dyn ::std::hash::Hasher) {
                <Self as ::std::hash::Hash>::hash(self, &mut hasher);
            }

            fn dyn_eq(&self, other: &dyn ::std::any::Any) -> bool {
                if let Some(other) = other.downcast_ref::<Self>() {
                    <Self as ::std::cmp::PartialEq>::eq(self, other)
                } else {
                    false
                }
            }

            fn dyn_clone(&self) -> ::std::sync::Arc<dyn ::typst::model::Element> {
                ::std::sync::Arc::new(Clone::clone(self))
            }

            fn field(&self, name: &str) -> Option<::typst::eval::Value> {
                match name {
                    "label" => self.label().cloned().map(::typst::eval::Value::Label),
                    #(
                        #field_matches
                    )*
                    _ => None,
                }
            }

            fn fields(&self) -> Dict {
                let mut fields = Dict::new();
                #(#field_dict)*
                #(#field_opt_dict)*
                fields
            }

            /// Set the fields of the element.
            fn set_field(&mut self, name: &str, value: Value) -> ::typst::diag::StrResult<()> {
                match name {
                    #(
                        #field_set_matches
                    )*
                    #(
                        #field_inherent_matches
                    )*
                    _ => ::typst::diag::bail!(#unknown_field, name),
                }
            }

            fn children(&self) -> &[::comemo::Prehashed<Content>] {
                #children
            }
        }

        impl ::typst::eval::IntoValue for #ident {
            fn into_value(self) -> ::typst::eval::Value {
                ::typst::eval::Value::Content(::typst::model::Content::static_(self))
            }
        }
    }
}

fn create_partial_eq_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let all = element
        .fields
        .iter()
        .filter(|field| {
            !field.external
                && !field.synthesized
                && (!field.internal || field.parse.is_some())
                && !field.fold
        })
        .map(|field| &field.ident)
        .collect::<Vec<_>>();

    let empty = all.is_empty().then(|| quote! { true });
    quote! {
        impl PartialEq for #ident {
            fn eq(&self, other: &Self) -> bool {
                #empty
                #(
                    &self.#all == &other.#all
                )&&*
            }
        }
    }
}

fn create_debug_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let name = &element.name;

    let all = element
        .fields
        .iter()
        .filter(|field| {
            !field.external
                && !field.synthesized
                && (!field.internal || field.parse.is_some())
                && !field.fold
        })
        .map(|field| &field.ident)
        .collect::<Vec<_>>();

    quote! {
        impl ::std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(#name)
                    .field("span", &self.span)
                    .field("location", &self.location)
                    .field("label", &self.label)
                    #(
                        .field(stringify!(#all), &self.#all)
                    )*
                    .finish()
            }
        }
    }
}

fn create_repr_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let all = element
        .fields
        .iter()
        .filter(|field| {
            !field.external
                && !field.synthesized
                && (!field.internal || field.parse.is_some())
        })
        .map(|field| {
            let ident = &field.ident;
            let fmt_str = format!("{}: {{}}, ", field.name.to_kebab_case());
            quote! {
                ::std::write!(&mut buf, eco_format!(#fmt_str, self.#ident.repr())).unwrap();
            }
        });

    let opt = element
        .fields
        .iter()
        .filter(|field| {
            !field.external
                || ((field.synthesized || field.parse.is_some()) && !field.internal)
        })
        .map(|field| {
            let ident = &field.ident;
            let fmt_str = format!("{}: {{:?}}, ", field.name.to_kebab_case());
            quote! {
                if let Some(value) = &self.#ident {
                    ::std::write!(&mut buf, eco_format!(#fmt_str, value)).unwrap();
                }
            }
        });

    quote! {
        impl ::typst::eval::Repr for #ident {
            fn repr(&self) -> ::ecow::EcoString {
                use std::io::Write;
                use ::ecow::eco_format;

                let mut buf = ::ecow::EcoString::new();

                /*#(#all)*
                #(#opt)**/

                buf
            }
        }
    }
}

/// Create the element's `Construct` implementation.
fn create_construct_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let all = element.fields.iter().filter(|field| {
        !field.external
            && !field.synthesized
            && (!field.internal || field.parse.is_some())
    });

    let pre = all.clone().map(|field| {
        let (prefix, value) = create_field_parser(field);
        let ident = &field.ident;
        quote! {
            #prefix
            let #ident = #value;
        }
    });

    let defaults = all
        .clone()
        .filter(|field| !field.settable())
        .map(|field| &field.ident);

    let handlers = all.filter(|field| field.settable()).map(|field| {
        let push_ident = &field.push_ident;
        let ident = &field.ident;
        quote! {
            if let Some(value) = #ident {
                element.#push_ident(value);
            }
        }
    });

    quote! {
        impl ::typst::model::Construct for #ident {
            type Output = ::typst::model::Content;

            fn construct(
                vm: &mut ::typst::eval::Vm,
                args: &mut ::typst::eval::Args,
            ) -> ::typst::diag::SourceResult<Self::Output> {
                #(#pre)*

                let mut element = Self::new(#(#defaults),*);

                #(#handlers)*

                Ok(::typst::model::Content::static_(element))
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

/// Create the element's casting vtable.
fn create_vtable_func(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let relevant = element.capabilities.iter().filter(|&ident| ident != "Construct");
    let checks = relevant.map(|capability| {
        quote! {
            if id == ::std::any::TypeId::of::<dyn #capability>() {
                let vtable = unsafe {
                    ::typst::util::fat::vtable(&null as &dyn #capability)
                };
                std::mem::forget(null);
                return Some(vtable);
            }
        }
    });

    quote! {
        |id| {
            // Safety: The null pointer is never dereferenced.
            #[allow(invalid_value)]
            let null = unsafe { ::std::mem::MaybeUninit::<#ident>::uninit().assume_init() };
            #(#checks)*

            std::mem::forget(null);
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
            static_: true,
            keywords: &[#(#keywords),*],
            empty: || #model::Content::static_(<#ident as ::std::default::Default>::default()),
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
                #model::Content::static_(self)
            }

            fn unpack(content: &#model::Content) -> ::std::option::Option<&Self> {
                content.is::<Self>().then(|| unsafe {
                    // Safety: we checked that we are `Self`.
                    unsafe {
                        &*(::std::sync::Arc::as_ptr(&content.0) as *const () as *const Self)
                    }
                })
            }

            fn unpack_mut(content: &mut #model::Content) -> Option<&mut Self> {
                content.is::<Self>().then(|| unsafe {
                    // Make sure we're mutable
                    ::typst::model::swap_with_mut(&mut content.0);

                    // Safety: we checked that we are `Self` and mutable.
                    unsafe {
                        &mut *(::std::sync::Arc::as_ptr(&mut content.0) as *const () as *mut () as *mut Self)
                    }
                })
            }
        }
    }
}

/// Create a builder pattern method for a field.
fn create_with_field_method(field: &Field) -> TokenStream {
    let Field { vis, ident, with_ident, name, ty, .. } = field;
    let doc = format!("Set the [`{}`](Self::{}) field.", name, ident);

    let set = if field.inherent() {
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
    let Field { vis, ident, push_ident, name, ty, .. } = field;
    let doc = format!("Push the [`{}`](Self::{}) field.", name, ident);
    let set = if field.inherent() && !field.synthesized {
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

/// Create an accessor methods for a field.
fn create_field_method(field: &Field) -> TokenStream {
    let Field { vis, docs, ident, output, .. } = field;
    if field.inherent() && !field.synthesized {
        quote! {
            #[doc = #docs]
            #[track_caller]
            #vis fn #ident(&self) -> #output {
                self.#ident.clone()
            }
        }
    } else if field.synthesized {
        quote! {
            #[doc = #docs]
            #[track_caller]
            #vis fn #ident(&self) -> #output {
                self.#ident.clone().unwrap()
            }
        }
    } else {
        let access = create_style_chain_access(
            field,
            quote! { self.#ident.clone().map(::typst::eval::IntoValue::into_value) },
        );
        quote! {
            #[doc = #docs]
            #vis fn #ident(&self, styles: ::typst::model::StyleChain) -> #output {
                #access
            }
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

fn create_field(field: &Field) -> TokenStream {
    let Field { ident, ty, docs, required, .. } = field;

    let ty = required
        .then(|| quote! { #ty })
        .unwrap_or_else(|| quote! { Option<#ty> });
    quote! {
        #[doc = #docs]
        #ident: #ty
    }
}

fn create_hash_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let all = element
        .fields
        .iter()
        .filter(|field| !field.not_hash && !field.external && field.required)
        .map(|field| &field.ident)
        .collect::<Vec<_>>();

    let opts = element
        .fields
        .iter()
        .filter(|field| !field.not_hash && !field.external && !field.required)
        .map(|field| &field.ident)
        .collect::<Vec<_>>();

    quote! {
        impl ::std::hash::Hash for #ident {
            fn hash<H: ::std::hash::Hasher>(&self, hasher: &mut H) {
                if let Some(location) = &self.location {
                    location.hash(hasher);
                }

                if let Some(label) = &self.label {
                    label.hash(hasher);
                }

                if !self.span.is_detached() {
                    self.span.hash(hasher);
                }

                if self.prepared {
                    self.prepared.hash(hasher);
                }

                if !self.guards.is_empty() {
                    self.guards.hash(hasher);
                }

                #(
                    self.#all.hash(hasher);
                )*

                #(
                    if let Some(#opts) = &self.#opts {
                        #opts.hash(hasher);
                    }
                )*
            }
        }
    }
}

fn create_default_impl(element: &Elem) -> TokenStream {
    let ident = &element.ident;
    let relevant = element
        .fields
        .iter()
        .filter(|field| !field.external && !field.synthesized && field.inherent())
        .map(|Field { ident, empty, .. }| {
            quote! { #ident: #empty }
        });
    let defaults = element
        .fields
        .iter()
        .filter(|field| !field.external && (field.synthesized || !field.inherent()))
        .map(|Field { ident, .. }| {
            quote! { #ident: None }
        });

    quote! {
        impl ::std::default::Default for #ident {
            fn default() -> Self {
                Self {
                    span: ::typst::syntax::Span::detached(),
                    location: None,
                    label: None,
                    prepared: false,
                    guards: ::std::vec::Vec::with_capacity(0),
                    #(#relevant,)*
                    #(#defaults,)*
                }
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
    let required = relevant.map(|Field { ident, .. }| {
        quote! { #ident }
    });
    let defaults = element
        .fields
        .iter()
        .filter(|field| !field.external && (field.synthesized || !field.inherent()))
        .map(|Field { ident, .. }| {
            quote! { #ident: None }
        });

    quote! {
        /// Create a new element.
        pub fn new(#(#params),*) -> Self {
            Self {
                span: ::typst::syntax::Span::detached(),
                location: None,
                label: None,
                prepared: false,
                guards: ::std::vec::Vec::with_capacity(0),
                #(#required,)*
                #(#defaults,)*
            }
        }
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
    let mut has_children = false;
    let fields = named.named.iter().map(|field| parse_field(field, &mut has_children)).collect::<Result<_>>()?;

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

fn parse_field(field: &syn::Field, has_children: &mut bool) -> Result<Field> {
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
    let children = has_attr(&mut attrs, "children");
    if children && *has_children {
        bail!(ident, "only one field can be marked as children");
    }
    *has_children |= children;

    let mut field = Field {
        name: ident.to_string().to_kebab_case(),
        docs: documentation(&attrs),
        internal: has_attr(&mut attrs, "internal"),
        external: has_attr(&mut attrs, "external"),
        positional,
        required,
        variadic,
        not_hash: has_attr(&mut attrs, "not_hash"),
        synthesized: has_attr(&mut attrs, "synthesized"),
        children,
        fold: has_attr(&mut attrs, "fold"),
        resolve: has_attr(&mut attrs, "resolve"),
        parse: parse_attr(&mut attrs, "parse")?.flatten(),
        default: parse_attr::<syn::Expr>(&mut attrs, "default")?
            .flatten()
            .unwrap_or_else(|| parse_quote! { ::std::default::Default::default() }),
        empty: parse_attr::<syn::Expr>(&mut attrs, "empty")?
            .flatten()
            .unwrap_or_else(|| parse_quote! { ::std::default::Default::default() }),
        vis: field.vis.clone(),
        ident: ident.clone(),
        ident_in: Ident::new(&format!("{}_in", ident), ident.span()),
        with_ident: Ident::new(&format!("with_{}", ident), ident.span()),
        push_ident: Ident::new(&format!("push_{}", ident), ident.span()),
        set_ident: Ident::new(&format!("set_{}", ident), ident.span()),
        enum_ident: Ident::new(
            &format!("set_{}", heck::AsUpperCamelCase(&ident.to_string())),
            ident.span(),
        ),
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
