use super::*;

/// Expand the `#[node]` macro.
pub fn node(stream: TokenStream, body: syn::ItemStruct) -> Result<TokenStream> {
    let node = prepare(stream, &body)?;
    Ok(create(&node))
}

struct Node {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: Ident,
    name: String,
    capable: Vec<Ident>,
    set: Option<syn::Block>,
    fields: Vec<Field>,
}

struct Field {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: Ident,
    with_ident: Ident,
    name: String,

    positional: bool,
    required: bool,
    variadic: bool,

    named: bool,
    shorthand: Option<Shorthand>,

    settable: bool,
    fold: bool,
    resolve: bool,
    skip: bool,

    ty: syn::Type,
    default: Option<syn::Expr>,
}

enum Shorthand {
    Positional,
    Named(Ident),
}

impl Node {
    fn inherent(&self) -> impl Iterator<Item = &Field> + Clone {
        self.fields.iter().filter(|field| !field.settable)
    }

    fn settable(&self) -> impl Iterator<Item = &Field> + Clone {
        self.fields.iter().filter(|field| field.settable)
    }
}

/// Preprocess the node's definition.
fn prepare(stream: TokenStream, body: &syn::ItemStruct) -> Result<Node> {
    let syn::Fields::Named(named) = &body.fields else {
        bail!(body, "expected named fields");
    };

    let mut fields = vec![];
    for field in &named.named {
        let Some(mut ident) = field.ident.clone() else {
            bail!(field, "expected named field");
        };

        let mut attrs = field.attrs.clone();
        let settable = has_attr(&mut attrs, "settable");
        if settable {
            ident = Ident::new(&ident.to_string().to_uppercase(), ident.span());
        }

        let field = Field {
            vis: field.vis.clone(),
            ident: ident.clone(),
            with_ident: Ident::new(&format!("with_{}", ident), ident.span()),
            name: kebab_case(&ident),

            positional: has_attr(&mut attrs, "positional"),
            required: has_attr(&mut attrs, "required"),
            variadic: has_attr(&mut attrs, "variadic"),

            named: has_attr(&mut attrs, "named"),
            shorthand: parse_attr(&mut attrs, "shorthand")?.map(|v| match v {
                None => Shorthand::Positional,
                Some(ident) => Shorthand::Named(ident),
            }),

            settable,
            fold: has_attr(&mut attrs, "fold"),
            resolve: has_attr(&mut attrs, "resolve"),
            skip: has_attr(&mut attrs, "skip"),

            ty: field.ty.clone(),
            default: parse_attr(&mut attrs, "default")?.map(|opt| {
                opt.unwrap_or_else(|| parse_quote! { ::std::default::Default::default() })
            }),

            attrs: {
                validate_attrs(&attrs)?;
                attrs
            },
        };

        if !field.positional && !field.named && !field.variadic && !field.settable {
            bail!(ident, "expected positional, named, variadic, or settable");
        }

        if !field.required && !field.variadic && field.default.is_none() {
            bail!(ident, "non-required fields must have a default value");
        }

        fields.push(field);
    }

    let capable = Punctuated::<Ident, Token![,]>::parse_terminated
        .parse2(stream)?
        .into_iter()
        .collect();

    let mut attrs = body.attrs.clone();
    Ok(Node {
        vis: body.vis.clone(),
        ident: body.ident.clone(),
        name: body.ident.to_string().trim_end_matches("Node").to_lowercase(),
        capable,
        fields,
        set: parse_attr(&mut attrs, "set")?.flatten(),
        attrs: {
            validate_attrs(&attrs)?;
            attrs
        },
    })
}

/// Produce the node's definition.
fn create(node: &Node) -> TokenStream {
    let attrs = &node.attrs;
    let vis = &node.vis;
    let ident = &node.ident;
    let name = &node.name;
    let new = create_new_func(node);
    let construct = node
        .capable
        .iter()
        .all(|capability| capability != "Construct")
        .then(|| create_construct_impl(node));
    let set = create_set_impl(node);
    let builders = node.inherent().map(create_builder_method);
    let accessors = node.inherent().map(create_accessor_method);
    let vtable = create_vtable(node);

    let mut modules = vec![];
    let mut items = vec![];
    let scope = quote::format_ident!("__{}_keys", ident);

    for field in node.settable() {
        let ident = &field.ident;
        let attrs = &field.attrs;
        let vis = &field.vis;
        let ty = &field.ty;
        modules.push(create_field_module(node, field));
        items.push(quote! {
            #(#attrs)*
            #vis const #ident: #scope::#ident::Key<#ty>
                = #scope::#ident::Key(::std::marker::PhantomData);
        });
    }

    quote! {
        #(#attrs)*
        #[::typst::eval::func]
        #[derive(Debug, Clone, Hash)]
        #[repr(transparent)]
        #vis struct #ident(::typst::model::Content);

        impl #ident {
            #new
            #(#builders)*

            /// The node's span.
            pub fn span(&self) -> Option<::typst::syntax::Span> {
                self.0.span()
            }
        }

        impl #ident {
            #(#accessors)*
            #(#items)*
        }

        impl ::typst::model::Node for #ident {
            fn id() -> ::typst::model::NodeId {
                static META: ::typst::model::NodeMeta = ::typst::model::NodeMeta {
                    name: #name,
                    vtable: #vtable,
                };
                ::typst::model::NodeId::from_meta(&META)
            }

            fn pack(self) -> ::typst::model::Content {
                self.0
            }
        }

        #construct
        #set

        impl From<#ident> for ::typst::eval::Value {
            fn from(value: #ident) -> Self {
                value.0.into()
            }
        }

        #[allow(non_snake_case)]
        mod #scope {
            use super::*;
            #(#modules)*
        }
    }
}

/// Create the `new` function for the node.
fn create_new_func(node: &Node) -> TokenStream {
    let relevant = node.inherent().filter(|field| field.required || field.variadic);
    let params = relevant.clone().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        quote! { #ident: #ty }
    });
    let pushes = relevant.map(|field| {
        let ident = &field.ident;
        let with_ident = &field.with_ident;
        quote! { .#with_ident(#ident) }
    });
    let defaults = node
        .inherent()
        .filter_map(|field| field.default.as_ref().map(|default| (field, default)))
        .map(|(field, default)| {
            let with_ident = &field.with_ident;
            quote! { .#with_ident(#default) }
        });
    quote! {
        /// Create a new node.
        pub fn new(#(#params),*) -> Self {
            Self(::typst::model::Content::new::<Self>())
            #(#pushes)*
            #(#defaults)*
        }
    }
}

/// Create a builder pattern method for a field.
fn create_builder_method(field: &Field) -> TokenStream {
    let Field { with_ident, ident, name, ty, .. } = field;
    let doc = format!("Set the [`{}`](Self::{}) field.", name, ident);
    quote! {
        #[doc = #doc]
        pub fn #with_ident(mut self, #ident: #ty) -> Self {
            Self(self.0.with_field(#name, #ident))
        }
    }
}

/// Create an accessor methods for a field.
fn create_accessor_method(field: &Field) -> TokenStream {
    let Field { attrs, vis, ident, name, ty, .. } = field;
    quote! {
        #(#attrs)*
        #vis fn #ident(&self) -> #ty {
            self.0.cast_field(#name)
        }
    }
}

/// Create the node's `Construct` implementation.
fn create_construct_impl(node: &Node) -> TokenStream {
    let ident = &node.ident;
    let shorthands = create_construct_shorthands(node);
    let builders = node.inherent().map(create_construct_builder_call);
    quote! {
        impl ::typst::model::Construct for #ident {
            fn construct(
                _: &::typst::eval::Vm,
                args: &mut ::typst::eval::Args,
            ) -> ::typst::diag::SourceResult<::typst::model::Content> {
                #(#shorthands)*
                Ok(::typst::model::Node::pack(
                    Self(::typst::model::Content::new::<Self>())
                        #(#builders)*))
            }
        }
    }
}

/// Create let bindings for shorthands in the constructor.
fn create_construct_shorthands(node: &Node) -> impl Iterator<Item = TokenStream> + '_ {
    let mut shorthands = vec![];
    for field in node.inherent() {
        if let Some(Shorthand::Named(named)) = &field.shorthand {
            shorthands.push(named);
        }
    }

    shorthands.sort();
    shorthands.dedup_by_key(|ident| ident.to_string());
    shorthands.into_iter().map(|ident| {
        let string = ident.to_string();
        quote! { let #ident = args.named(#string)?; }
    })
}

/// Create a builder call for the constructor.
fn create_construct_builder_call(field: &Field) -> TokenStream {
    let name = &field.name;
    let with_ident = &field.with_ident;

    let mut value = if field.variadic {
        quote! { args.all()? }
    } else if field.required {
        quote! { args.expect(#name)? }
    } else if let Some(shorthand) = &field.shorthand {
        match shorthand {
            Shorthand::Positional => quote! { args.named_or_find(#name)? },
            Shorthand::Named(named) => {
                quote! { args.named(#name)?.or_else(|| #named.clone()) }
            }
        }
    } else if field.named {
        quote! { args.named(#name)? }
    } else {
        quote! { args.find()? }
    };

    if let Some(default) = &field.default {
        value = quote! { #value.unwrap_or(#default) };
    }

    quote! { .#with_ident(#value) }
}

/// Create the node's `Set` implementation.
fn create_set_impl(node: &Node) -> TokenStream {
    let ident = &node.ident;
    let custom = node.set.as_ref().map(|block| {
        quote! { (|| -> typst::diag::SourceResult<()> { #block; Ok(()) } )()?; }
    });

    let mut shorthands = vec![];
    let sets: Vec<_> = node
        .settable()
        .filter(|field| !field.skip)
        .map(|field| {
            let ident = &field.ident;
            let name = &field.name;
            let value = match &field.shorthand {
                Some(Shorthand::Positional) => quote! { args.named_or_find(#name)? },
                Some(Shorthand::Named(named)) => {
                    shorthands.push(named);
                    quote! { args.named(#name)?.or_else(|| #named.clone()) }
                }
                None => quote! { args.named(#name)? },
            };

            quote! { styles.set_opt(Self::#ident, #value); }
        })
        .collect();

    shorthands.sort();
    shorthands.dedup_by_key(|ident| ident.to_string());

    let bindings = shorthands.into_iter().map(|ident| {
        let string = ident.to_string();
        quote! { let #ident = args.named(#string)?; }
    });

    let infos = node.fields.iter().filter(|p| !p.skip).map(|field| {
        let name = &field.name;
        let value_ty = &field.ty;
        let shorthand = matches!(field.shorthand, Some(Shorthand::Positional));
        let docs = documentation(&field.attrs);
        let docs = docs.trim();
        quote! {
            ::typst::eval::ParamInfo {
                name: #name,
                docs: #docs,
                cast: <#value_ty as ::typst::eval::Cast<
                    ::typst::syntax::Spanned<::typst::eval::Value>
                >>::describe(),
                named: true,
                positional: #shorthand,
                required: false,
                variadic: false,
                settable: true,
            }
        }
    });

    quote! {
        impl ::typst::model::Set for #ident {
            fn set(
                args: &mut ::typst::eval::Args,
                constructor: bool,
            ) -> ::typst::diag::SourceResult<::typst::model::StyleMap> {
                let mut styles = ::typst::model::StyleMap::new();
                #custom
                #(#bindings)*
                #(#sets)*
                Ok(styles)
            }

            fn properties() -> ::std::vec::Vec<::typst::eval::ParamInfo> {
                ::std::vec![#(#infos),*]
            }
        }
    }
}

/// Create the module for a single field.
fn create_field_module(node: &Node, field: &Field) -> TokenStream {
    let node_ident = &node.ident;
    let ident = &field.ident;
    let name = &field.name;
    let ty = &field.ty;
    let default = &field.default;

    let mut output = quote! { #ty };
    if field.resolve {
        output = quote! { <#output as ::typst::model::Resolve>::Output };
    }
    if field.fold {
        output = quote! { <#output as ::typst::model::Fold>::Output };
    }

    let value = if field.resolve && field.fold {
        quote! {
            values
                .next()
                .map(|value| {
                    ::typst::model::Fold::fold(
                        ::typst::model::Resolve::resolve(value, chain),
                        Self::get(chain, values),
                    )
                })
                .unwrap_or(#default)
        }
    } else if field.resolve {
        quote! {
            ::typst::model::Resolve::resolve(
                values.next().unwrap_or(#default),
                chain
            )
        }
    } else if field.fold {
        quote! {
            values
                .next()
                .map(|value| {
                    ::typst::model::Fold::fold(
                        value,
                        Self::get(chain, values),
                    )
                })
                .unwrap_or(#default)
        }
    } else {
        quote! {
            values.next().unwrap_or(#default)
        }
    };

    // Generate the contents of the module.
    let scope = quote! {
        use super::*;

        pub struct Key<T>(pub ::std::marker::PhantomData<T>);
        impl ::std::marker::Copy for Key<#ty> {}
        impl ::std::clone::Clone for Key<#ty> {
            fn clone(&self) -> Self { *self }
        }

        impl ::typst::model::Key for Key<#ty> {
            type Value = #ty;
            type Output = #output;

            fn id() -> ::typst::model::KeyId {
                static META: ::typst::model::KeyMeta = ::typst::model::KeyMeta {
                    name: #name,
                };
                ::typst::model::KeyId::from_meta(&META)
            }

            fn node() -> ::typst::model::NodeId {
                ::typst::model::NodeId::of::<#node_ident>()
            }

            fn get(
                chain: ::typst::model::StyleChain,
                mut values: impl ::std::iter::Iterator<Item = Self::Value>,
            ) -> Self::Output {
                #value
            }
        }
    };

    // Generate the module code.
    quote! {
        pub mod #ident { #scope }
    }
}

/// Create the node's metadata vtable.
fn create_vtable(node: &Node) -> TokenStream {
    let ident = &node.ident;
    let checks =
        node.capable
            .iter()
            .filter(|&ident| ident != "Construct")
            .map(|capability| {
                quote! {
                    if id == ::std::any::TypeId::of::<dyn #capability>() {
                        return Some(unsafe {
                            ::typst::util::fat::vtable(&
                                Self(::typst::model::Content::new::<#ident>()) as &dyn #capability
                            )
                        });
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
