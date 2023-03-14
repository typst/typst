use super::*;

/// Expand the `#[node]` macro.
pub fn node(stream: TokenStream, body: syn::ItemStruct) -> Result<TokenStream> {
    let node = prepare(stream, &body)?;
    Ok(create(&node))
}

struct Node {
    name: String,
    display: String,
    category: String,
    docs: String,
    vis: syn::Visibility,
    ident: Ident,
    capable: Vec<Ident>,
    fields: Vec<Field>,
}

struct Field {
    name: String,
    docs: String,
    internal: bool,
    external: bool,
    positional: bool,
    required: bool,
    variadic: bool,
    synthesized: bool,
    fold: bool,
    resolve: bool,
    parse: Option<FieldParser>,
    default: syn::Expr,
    vis: syn::Visibility,
    ident: Ident,
    ident_in: Ident,
    with_ident: Ident,
    push_ident: Ident,
    set_ident: Ident,
    ty: syn::Type,
    output: syn::Type,
}

impl Field {
    fn inherent(&self) -> bool {
        self.required || self.variadic
    }

    fn settable(&self) -> bool {
        !self.inherent()
    }
}

struct FieldParser {
    prefix: Vec<syn::Stmt>,
    expr: syn::Stmt,
}

impl Parse for FieldParser {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut stmts = syn::Block::parse_within(input)?;
        let Some(expr) = stmts.pop() else {
            return Err(input.error("expected at least on expression"));
        };
        Ok(Self { prefix: stmts, expr })
    }
}

/// Preprocess the node's definition.
fn prepare(stream: TokenStream, body: &syn::ItemStruct) -> Result<Node> {
    let syn::Fields::Named(named) = &body.fields else {
        bail!(body, "expected named fields");
    };

    let mut fields = vec![];
    for field in &named.named {
        let Some(ident) = field.ident.clone() else {
            bail!(field, "expected named field");
        };

        let mut attrs = field.attrs.clone();
        let variadic = has_attr(&mut attrs, "variadic");
        let required = has_attr(&mut attrs, "required") || variadic;
        let positional = has_attr(&mut attrs, "positional") || required;

        if ident == "label" {
            bail!(ident, "invalid field name");
        }

        let mut field = Field {
            name: kebab_case(&ident),
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
        fields.push(field);
    }

    let capable = Punctuated::<Ident, Token![,]>::parse_terminated
        .parse2(stream)?
        .into_iter()
        .collect();

    let docs = documentation(&body.attrs);
    let mut lines = docs.split("\n").collect();
    let category = meta_line(&mut lines, "Category")?.into();
    let display = meta_line(&mut lines, "Display")?.into();
    let docs = lines.join("\n").trim().into();

    let node = Node {
        name: body.ident.to_string().trim_end_matches("Node").to_lowercase(),
        display,
        category,
        docs,
        vis: body.vis.clone(),
        ident: body.ident.clone(),
        capable,
        fields,
    };

    validate_attrs(&body.attrs)?;
    Ok(node)
}

/// Produce the node's definition.
fn create(node: &Node) -> TokenStream {
    let Node { vis, ident, docs, .. } = node;
    let all = node.fields.iter().filter(|field| !field.external);
    let settable = all.clone().filter(|field| !field.synthesized && field.settable());

    // Inherent methods and functions.
    let new = create_new_func(node);
    let field_methods = all.clone().map(create_field_method);
    let field_in_methods = settable.clone().map(create_field_in_method);
    let with_field_methods = all.clone().map(create_with_field_method);
    let push_field_methods = all.map(create_push_field_method);
    let field_style_methods = settable.map(create_set_field_method);

    // Trait implementations.
    let node_impl = create_node_impl(node);
    let construct_impl = node
        .capable
        .iter()
        .all(|capability| capability != "Construct")
        .then(|| create_construct_impl(node));
    let set_impl = create_set_impl(node);
    let locatable_impl = node
        .capable
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

            /// The node's span.
            pub fn span(&self) -> ::typst::syntax::Span {
                self.0.span()
            }
        }

        #node_impl
        #construct_impl
        #set_impl
        #locatable_impl

        impl From<#ident> for ::typst::eval::Value {
            fn from(value: #ident) -> Self {
                value.0.into()
            }
        }
    }
}

/// Create the `new` function for the node.
fn create_new_func(node: &Node) -> TokenStream {
    let relevant = node
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
        /// Create a new node.
        pub fn new(#(#params),*) -> Self {
            Self(::typst::model::Content::new::<Self>())
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
        let access =
            create_style_chain_access(field, quote! { self.0.field(#name).cloned() });
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
            ::typst::model::NodeId::of::<Self>(),
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
        #vis fn #set_ident(#ident: #ty) -> ::typst::model::Property {
            ::typst::model::Property::new(
                ::typst::model::NodeId::of::<Self>(),
                #name.into(),
                #ident.into()
            )
        }
    }
}

/// Create the node's `Node` implementation.
fn create_node_impl(node: &Node) -> TokenStream {
    let Node { ident, name, display, category, docs, .. } = node;
    let vtable_func = create_vtable_func(node);
    let infos = node
        .fields
        .iter()
        .filter(|field| !field.internal && !field.synthesized)
        .map(create_param_info);
    quote! {
        impl ::typst::model::Node for #ident {
            fn id() -> ::typst::model::NodeId {
                static META: ::typst::model::NodeMeta = ::typst::model::NodeMeta {
                    name: #name,
                    vtable: #vtable_func,
                    construct: <#ident as ::typst::model::Construct>::construct,
                    set: <#ident as ::typst::model::Set>::set,
                    info: ::typst::eval::Lazy::new(|| typst::eval::FuncInfo {
                        name: #name,
                        display: #display,
                        docs: #docs,
                        params: ::std::vec![#(#infos),*],
                        returns: ::std::vec!["content"],
                        category: #category,
                    }),
                };
                ::typst::model::NodeId(&META)
            }

            fn pack(self) -> ::typst::model::Content {
                self.0
            }
        }
    }
}

/// Create the node's casting vtable.
fn create_vtable_func(node: &Node) -> TokenStream {
    let ident = &node.ident;
    let relevant = node.capable.iter().filter(|&ident| ident != "Construct");
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
            let null = Self(::typst::model::Content::new::<#ident>());
            #(#checks)*
            None
        }
    }
}

/// Create a parameter info for a field.
fn create_param_info(field: &Field) -> TokenStream {
    let Field { name, docs, positional, variadic, required, ty, .. } = field;
    let named = !positional;
    let settable = field.settable();
    let ty = if *variadic {
        quote! { <#ty as ::typst::eval::Variadics>::Inner }
    } else {
        quote! { #ty }
    };
    quote! {
        ::typst::eval::ParamInfo {
            name: #name,
            docs: #docs,
            cast: <#ty as ::typst::eval::Cast<
                ::typst::syntax::Spanned<::typst::eval::Value>
            >>::describe(),
            positional: #positional,
            named: #named,
            variadic: #variadic,
            required: #required,
            settable: #settable,
        }
    }
}

/// Create the node's `Construct` implementation.
fn create_construct_impl(node: &Node) -> TokenStream {
    let ident = &node.ident;
    let handlers = node
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
                        node.#push_ident(value);
                    }
                }
            } else {
                quote! {
                    #prefix
                    node.#push_ident(#value);
                }
            }
        });

    quote! {
        impl ::typst::model::Construct for #ident {
            fn construct(
                vm: &::typst::eval::Vm,
                args: &mut ::typst::eval::Args,
            ) -> ::typst::diag::SourceResult<::typst::model::Content> {
                let mut node = Self(::typst::model::Content::new::<Self>());
                #(#handlers)*
                Ok(node.0)
            }
        }
    }
}

/// Create the node's `Set` implementation.
fn create_set_impl(node: &Node) -> TokenStream {
    let ident = &node.ident;
    let handlers = node
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
                args: &mut ::typst::eval::Args,
            ) -> ::typst::diag::SourceResult<::typst::model::StyleMap> {
                let mut styles = ::typst::model::StyleMap::new();
                #(#handlers)*
                Ok(styles)
            }
        }
    }
}

/// Create argument parsing code for a field.
fn create_field_parser(field: &Field) -> (TokenStream, TokenStream) {
    if let Some(FieldParser { prefix, expr }) = &field.parse {
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
