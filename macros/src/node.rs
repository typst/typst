use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;

use super::*;

/// Expand the `#[node]` macro.
pub fn expand(attr: TokenStream, body: syn::ItemImpl) -> Result<TokenStream> {
    let node = prepare(attr, body)?;
    create(&node)
}

/// Details about a node.
struct Node {
    body: syn::ItemImpl,
    params: Punctuated<syn::GenericParam, Token![,]>,
    self_ty: syn::Type,
    self_name: String,
    self_args: Punctuated<syn::GenericArgument, Token![,]>,
    capabilities: Vec<syn::Ident>,
    properties: Vec<Property>,
    construct: Option<syn::ImplItemMethod>,
    set: Option<syn::ImplItemMethod>,
    field: Option<syn::ImplItemMethod>,
}

/// A style property.
struct Property {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    name: Ident,
    value_ty: syn::Type,
    output_ty: syn::Type,
    default: syn::Expr,
    skip: bool,
    referenced: bool,
    shorthand: Option<Shorthand>,
    resolve: bool,
    fold: bool,
}

/// The shorthand form of a style property.
enum Shorthand {
    Positional,
    Named(Ident),
}

/// Preprocess the impl block of a node.
fn prepare(attr: TokenStream, body: syn::ItemImpl) -> Result<Node> {
    // Extract the generic type arguments.
    let params = body.generics.params.clone();

    // Extract the node type for which we want to generate properties.
    let self_ty = (*body.self_ty).clone();
    let self_path = match &self_ty {
        syn::Type::Path(path) => path,
        ty => bail!(ty, "must be a path type"),
    };

    // Split up the type into its name and its generic type arguments.
    let last = self_path.path.segments.last().unwrap();
    let self_name = last.ident.to_string();
    let self_args = match &last.arguments {
        syn::PathArguments::AngleBracketed(args) => args.args.clone(),
        _ => Punctuated::new(),
    };

    // Parse the capabilities.
    let capabilities: Vec<_> = Punctuated::<Ident, Token![,]>::parse_terminated
        .parse2(attr)?
        .into_iter()
        .collect();

    let mut properties = vec![];
    let mut construct = None;
    let mut set = None;
    let mut field = None;

    // Parse the properties and methods.
    for item in &body.items {
        match item {
            syn::ImplItem::Const(item) => {
                properties.push(prepare_property(item)?);
            }
            syn::ImplItem::Method(method) => {
                match method.sig.ident.to_string().as_str() {
                    "construct" => construct = Some(method.clone()),
                    "set" => set = Some(method.clone()),
                    "field" => field = Some(method.clone()),
                    _ => bail!(method, "unexpected method"),
                }
            }
            _ => bail!(item, "unexpected item"),
        }
    }

    Ok(Node {
        body,
        params,
        self_ty,
        self_name,
        self_args,
        capabilities,
        properties,
        construct,
        set,
        field,
    })
}

/// Preprocess and validate a property constant.
fn prepare_property(item: &syn::ImplItemConst) -> Result<Property> {
    let mut attrs = item.attrs.clone();
    let tokens = match attrs
        .iter()
        .position(|attr| attr.path.is_ident("property"))
        .map(|i| attrs.remove(i))
    {
        Some(attr) => attr.parse_args::<TokenStream>()?,
        None => TokenStream::default(),
    };

    let mut skip = false;
    let mut shorthand = None;
    let mut referenced = false;
    let mut resolve = false;
    let mut fold = false;

    // Parse the `#[property(..)]` attribute.
    let mut stream = tokens.into_iter().peekable();
    while let Some(token) = stream.next() {
        let ident = match token {
            TokenTree::Ident(ident) => ident,
            TokenTree::Punct(_) => continue,
            _ => bail!(token, "invalid token"),
        };

        let mut arg = None;
        if let Some(TokenTree::Group(group)) = stream.peek() {
            let span = group.span();
            let string = group.to_string();
            let ident = string.trim_start_matches('(').trim_end_matches(')');
            if !ident.chars().all(|c| c.is_ascii_alphabetic()) {
                bail!(group, "invalid arguments");
            }
            arg = Some(Ident::new(ident, span));
            stream.next();
        };

        match ident.to_string().as_str() {
            "skip" => skip = true,
            "shorthand" => {
                shorthand = Some(match arg {
                    Some(name) => Shorthand::Named(name),
                    None => Shorthand::Positional,
                });
            }
            "referenced" => referenced = true,
            "resolve" => resolve = true,
            "fold" => fold = true,
            _ => bail!(ident, "invalid attribute"),
        }
    }

    if skip && shorthand.is_some() {
        bail!(item.ident, "skip and shorthand are mutually exclusive");
    }

    if referenced && (fold || resolve) {
        bail!(item.ident, "referenced is mutually exclusive with fold and resolve");
    }

    // The type of the property's value is what the user of our macro wrote as
    // type of the const, but the real type of the const will be a unique `Key`
    // type.
    let value_ty = item.ty.clone();
    let output_ty = if referenced {
        parse_quote! { &'a #value_ty }
    } else if fold && resolve {
        parse_quote! {
            <<#value_ty as ::typst::model::Resolve>::Output
                as ::typst::model::Fold>::Output
        }
    } else if fold {
        parse_quote! { <#value_ty as ::typst::model::Fold>::Output }
    } else if resolve {
        parse_quote! { <#value_ty as ::typst::model::Resolve>::Output }
    } else {
        value_ty.clone()
    };

    Ok(Property {
        attrs,
        vis: item.vis.clone(),
        name: item.ident.clone(),
        value_ty,
        output_ty,
        default: item.expr.clone(),
        skip,
        shorthand,
        referenced,
        resolve,
        fold,
    })
}

/// Produce the necessary items for a type to become a node.
fn create(node: &Node) -> Result<TokenStream> {
    let params = &node.params;
    let self_ty = &node.self_ty;

    let id_method = create_node_id_method();
    let name_method = create_node_name_method(node);
    let construct_func = create_node_construct_func(node);
    let set_func = create_node_set_func(node);
    let field_method = create_node_field_method(node);
    let vtable_method = create_node_vtable_method(node);

    let node_impl = quote! {
        impl<#params> ::typst::model::Node for #self_ty {
            #id_method
            #name_method
            #construct_func
            #set_func
            #field_method
        }

        unsafe impl<#params> ::typst::model::Capable for #self_ty {
            #vtable_method
        }
    };

    let mut modules: Vec<syn::ItemMod> = vec![];
    let mut items: Vec<syn::ImplItem> = vec![];
    let scope = quote::format_ident!("__{}_keys", node.self_name);

    for property in &node.properties {
        let (key, module) = create_property_module(node, &property);
        modules.push(module);

        let name = &property.name;
        let attrs = &property.attrs;
        let vis = &property.vis;
        items.push(parse_quote! {
            #(#attrs)*
            #vis const #name: #scope::#name::#key
                = #scope::#name::Key(::std::marker::PhantomData);
        });
    }

    let mut body = node.body.clone();
    body.items = items;

    Ok(quote! {
        #body
        mod #scope {
            use super::*;
            #node_impl
            #(#modules)*
        }
    })
}

/// Create the node's id method.
fn create_node_id_method() -> syn::ImplItemMethod {
    parse_quote! {
        fn id(&self) -> ::typst::model::NodeId {
            ::typst::model::NodeId::of::<Self>()
        }
    }
}

/// Create the node's name method.
fn create_node_name_method(node: &Node) -> syn::ImplItemMethod {
    let name = node.self_name.trim_end_matches("Node").to_lowercase();
    parse_quote! {
        fn name(&self) -> &'static str {
            #name
        }
    }
}

/// Create the node's `construct` function.
fn create_node_construct_func(node: &Node) -> syn::ImplItemMethod {
    node.construct.clone().unwrap_or_else(|| {
        parse_quote! {
            fn construct(
                _: &::typst::model::Vm,
                _: &mut ::typst::model::Args,
            ) -> ::typst::diag::SourceResult<::typst::model::Content> {
                unimplemented!()
            }
        }
    })
}

/// Create the node's `set` function.
fn create_node_set_func(node: &Node) -> syn::ImplItemMethod {
    let user = node.set.as_ref().map(|method| {
        let block = &method.block;
        quote! { (|| -> typst::diag::SourceResult<()> { #block; Ok(()) } )()?; }
    });

    let mut shorthands = vec![];
    let sets: Vec<_> = node
        .properties
        .iter()
        .filter(|p| !p.skip)
        .map(|property| {
            let name = &property.name;
            let string = name.to_string().replace('_', "-").to_lowercase();
            let value = match &property.shorthand {
                Some(Shorthand::Positional) => quote! { args.named_or_find(#string)? },
                Some(Shorthand::Named(named)) => {
                    shorthands.push(named);
                    quote! { args.named(#string)?.or_else(|| #named.clone()) }
                }
                None => quote! { args.named(#string)? },
            };

            quote! { styles.set_opt(Self::#name, #value); }
        })
        .collect();

    shorthands.sort();
    shorthands.dedup_by_key(|ident| ident.to_string());

    let bindings = shorthands.into_iter().map(|ident| {
        let string = ident.to_string();
        quote! { let #ident = args.named(#string)?; }
    });

    parse_quote! {
        fn set(
            args: &mut ::typst::model::Args,
            constructor: bool,
        ) -> ::typst::diag::SourceResult<::typst::model::StyleMap> {
            let mut styles = ::typst::model::StyleMap::new();
            #user
            #(#bindings)*
            #(#sets)*
            Ok(styles)
        }
    }
}

/// Create the node's `field` method.
fn create_node_field_method(node: &Node) -> syn::ImplItemMethod {
    node.field.clone().unwrap_or_else(|| {
        parse_quote! {
            fn field(
                &self,
                _: &str,
            ) -> ::std::option::Option<::typst::model::Value> {
                None
            }
        }
    })
}

/// Create the node's capability accessor method.
fn create_node_vtable_method(node: &Node) -> syn::ImplItemMethod {
    let checks = node.capabilities.iter().map(|capability| {
        quote! {
            if id == ::std::any::TypeId::of::<dyn #capability>() {
                return Some(unsafe {
                    ::typst::util::fat::vtable(self as &dyn #capability)
                });
            }
        }
    });

    parse_quote! {
        fn vtable(&self, id: ::std::any::TypeId) -> ::std::option::Option<*const ()> {
            #(#checks)*
            None
        }
    }
}

/// Process a single const item.
fn create_property_module(node: &Node, property: &Property) -> (syn::Type, syn::ItemMod) {
    let params = &node.params;
    let self_args = &node.self_args;
    let name = &property.name;
    let value_ty = &property.value_ty;
    let output_ty = &property.output_ty;

    let key = parse_quote! { Key<#value_ty, #self_args> };
    let phantom_args = self_args.iter().filter(|arg| match arg {
        syn::GenericArgument::Type(syn::Type::Path(path)) => {
            node.params.iter().all(|param| match param {
                syn::GenericParam::Const(c) => !path.path.is_ident(&c.ident),
                _ => true,
            })
        }
        _ => true,
    });

    let name_const = create_property_name_const(node, property);
    let node_func = create_property_node_func(node);
    let get_method = create_property_get_method(property);
    let copy_assertion = create_property_copy_assertion(property);

    // Generate the contents of the module.
    let scope = quote! {
        use super::*;

        pub struct Key<__T, #params>(
            pub ::std::marker::PhantomData<(__T, #(#phantom_args,)*)>
        );

        impl<#params> ::std::marker::Copy for #key {}
        impl<#params> ::std::clone::Clone for #key {
            fn clone(&self) -> Self { *self }
        }

        impl<#params> ::typst::model::Key for #key {
            type Value = #value_ty;
            type Output<'a> = #output_ty;
            #name_const
            #node_func
            #get_method
        }

        #copy_assertion
    };

    // Generate the module code.
    let module = parse_quote! {
        #[allow(non_snake_case)]
        pub mod #name { #scope }
    };

    (key, module)
}

/// Create the property's node method.
fn create_property_name_const(node: &Node, property: &Property) -> syn::ImplItemConst {
    // The display name, e.g. `TextNode::BOLD`.
    let name = format!("{}::{}", node.self_name, &property.name);
    parse_quote! {
        const NAME: &'static str = #name;
    }
}

/// Create the property's node method.
fn create_property_node_func(node: &Node) -> syn::ImplItemMethod {
    let self_ty = &node.self_ty;
    parse_quote! {
        fn node() -> ::typst::model::NodeId {
            ::typst::model::NodeId::of::<#self_ty>()
        }
    }
}

/// Create the property's get method.
fn create_property_get_method(property: &Property) -> syn::ImplItemMethod {
    let default = &property.default;
    let value_ty = &property.value_ty;

    let value = if property.referenced {
        quote! {
            values.next().unwrap_or_else(|| {
                static LAZY: ::typst::model::once_cell::sync::Lazy<#value_ty>
                    = ::typst::model::once_cell::sync::Lazy::new(|| #default);
                &*LAZY
            })
        }
    } else if property.resolve && property.fold {
        quote! {
            match values.next().cloned() {
                Some(value) => ::typst::model::Fold::fold(
                    ::typst::model::Resolve::resolve(value, chain),
                    Self::get(chain, values),
                ),
                None => #default,
            }
        }
    } else if property.resolve {
        quote! {
            let value = values.next().cloned().unwrap_or_else(|| #default);
            ::typst::model::Resolve::resolve(value, chain)
        }
    } else if property.fold {
        quote! {
            match values.next().cloned() {
                Some(value) => ::typst::model::Fold::fold(value, Self::get(chain, values)),
                None => #default,
            }
        }
    } else {
        quote! {
            values.next().copied().unwrap_or(#default)
        }
    };

    parse_quote! {
        fn get<'a>(
            chain: ::typst::model::StyleChain<'a>,
            mut values: impl ::std::iter::Iterator<Item = &'a Self::Value>,
        ) -> Self::Output<'a> {
            #value
        }
    }
}

/// Create the assertion if the property's value must be copyable.
fn create_property_copy_assertion(property: &Property) -> Option<TokenStream> {
    let value_ty = &property.value_ty;
    let must_be_copy = !property.fold && !property.resolve && !property.referenced;
    must_be_copy.then(|| {
        quote_spanned! { value_ty.span() =>
            const _: fn() -> () = || {
                fn must_be_copy_fold_resolve_or_referenced<T: ::std::marker::Copy>() {}
                must_be_copy_fold_resolve_or_referenced::<#value_ty>();
            };
        }
    })
}
