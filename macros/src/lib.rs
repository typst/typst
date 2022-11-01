extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{quote, quote_spanned};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, Ident, Result};

/// Turn a struct into a node / a function with settable properties.
#[proc_macro_attribute]
pub fn node(stream: TokenStream, item: TokenStream) -> TokenStream {
    let impl_block = syn::parse_macro_input!(item as syn::ItemImpl);
    expand(stream.into(), impl_block)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Expand an impl block for a node.
fn expand(stream: TokenStream2, mut impl_block: syn::ItemImpl) -> Result<TokenStream2> {
    // Split the node type into name and generic type arguments.
    let params = &impl_block.generics.params;
    let self_ty = &*impl_block.self_ty;
    let (self_name, self_args) = parse_self(self_ty)?;

    let module = quote::format_ident!("{}_types", self_name);

    let mut key_modules = vec![];
    let mut properties = vec![];
    let mut construct = None;
    let mut set = None;

    for item in std::mem::take(&mut impl_block.items) {
        match item {
            syn::ImplItem::Const(mut item) => {
                let (property, module) =
                    process_const(&mut item, params, self_ty, &self_name, &self_args)?;
                properties.push(property);
                key_modules.push(module);
                impl_block.items.push(syn::ImplItem::Const(item));
            }
            syn::ImplItem::Method(method) => {
                match method.sig.ident.to_string().as_str() {
                    "construct" => construct = Some(method),
                    "set" => set = Some(method),
                    _ => return Err(Error::new(method.span(), "unexpected method")),
                }
            }
            _ => return Err(Error::new(item.span(), "unexpected item")),
        }
    }

    let construct = construct.unwrap_or_else(|| {
        parse_quote! {
            fn construct(
                _: &mut model::Vm,
                _: &mut model::Args,
            ) -> crate::diag::SourceResult<model::Content> {
                unimplemented!()
            }
        }
    });

    let set = generate_set(&properties, set);

    let items: syn::punctuated::Punctuated<Ident, syn::Token![,]> =
        parse_quote! { #stream };

    let checks = items.iter().map(|cap| {
        quote! {
            if id == TypeId::of::<dyn #cap>() {
                return Some(unsafe { crate::util::fat::vtable(self as &dyn #cap) });
            }
        }
    });

    let vtable = quote! {
        fn vtable(&self, id: TypeId) -> Option<*const ()> {
            #(#checks)*
            None
        }
    };

    // Put everything into a module with a hopefully unique type to isolate
    // it from the outside.
    Ok(quote! {
        #[allow(non_snake_case)]
        mod #module {
            use std::any::TypeId;
            use std::marker::PhantomData;
            use once_cell::sync::Lazy;
            use crate::model;
            use super::*;

            #impl_block

            impl<#params> model::Node for #self_ty {
                #construct
                #set
                #vtable

                fn id(&self) -> model::NodeId {
                    model::NodeId::of::<Self>()
                }
            }

            #(#key_modules)*
        }
    })
}

/// Parse the name and generic type arguments of the node type.
fn parse_self(
    self_ty: &syn::Type,
) -> Result<(String, Punctuated<syn::GenericArgument, syn::Token![,]>)> {
    // Extract the node type for which we want to generate properties.
    let path = match self_ty {
        syn::Type::Path(path) => path,
        ty => return Err(Error::new(ty.span(), "must be a path type")),
    };

    // Split up the type into its name and its generic type arguments.
    let last = path.path.segments.last().unwrap();
    let self_name = last.ident.to_string();
    let self_args = match &last.arguments {
        syn::PathArguments::AngleBracketed(args) => args.args.clone(),
        _ => Punctuated::new(),
    };

    Ok((self_name, self_args))
}

/// Process a single const item.
fn process_const(
    item: &mut syn::ImplItemConst,
    params: &Punctuated<syn::GenericParam, syn::Token![,]>,
    self_ty: &syn::Type,
    self_name: &str,
    self_args: &Punctuated<syn::GenericArgument, syn::Token![,]>,
) -> Result<(Property, syn::ItemMod)> {
    let property = parse_property(item)?;

    // The display name, e.g. `TextNode::BOLD`.
    let name = format!("{}::{}", self_name, &item.ident);

    // The type of the property's value is what the user of our macro wrote
    // as type of the const ...
    let value_ty = &item.ty;
    let output_ty = if property.referenced {
        parse_quote!(&'a #value_ty)
    } else if property.fold && property.resolve {
        parse_quote!(<<#value_ty as model::Resolve>::Output as model::Fold>::Output)
    } else if property.fold {
        parse_quote!(<#value_ty as model::Fold>::Output)
    } else if property.resolve {
        parse_quote!(<#value_ty as model::Resolve>::Output)
    } else {
        value_ty.clone()
    };

    // ... but the real type of the const becomes this ...
    let key = quote! { Key<#value_ty, #self_args> };
    let phantom_args = self_args.iter().filter(|arg| match arg {
        syn::GenericArgument::Type(syn::Type::Path(path)) => {
            params.iter().all(|param| match param {
                syn::GenericParam::Const(c) => !path.path.is_ident(&c.ident),
                _ => true,
            })
        }
        _ => true,
    });

    let default = &item.expr;

    // Ensure that the type is
    // - either `Copy`, or
    // - that the property is referenced, or
    // - that the property isn't copy but can't be referenced because it needs
    //   folding.
    let get;
    let mut copy = None;

    if property.referenced {
        get = quote! {
            values.next().unwrap_or_else(|| {
                static LAZY: Lazy<#value_ty> = Lazy::new(|| #default);
                &*LAZY
            })
        };
    } else if property.resolve && property.fold {
        get = quote! {
            match values.next().cloned() {
                Some(value) => model::Fold::fold(
                    model::Resolve::resolve(value, chain),
                    Self::get(chain, values),
                ),
                None => #default,
            }
        };
    } else if property.resolve {
        get = quote! {
            let value = values.next().cloned().unwrap_or_else(|| #default);
            model::Resolve::resolve(value, chain)
        };
    } else if property.fold {
        get = quote! {
            match values.next().cloned() {
                Some(value) => model::Fold::fold(value, Self::get(chain, values)),
                None => #default,
            }
        };
    } else {
        get = quote! {
            values.next().copied().unwrap_or(#default)
        };

        copy = Some(quote_spanned! { item.ty.span() =>
            const _: fn() -> () = || {
                fn type_must_be_copy_or_fold_or_referenced<T: Copy>() {}
                type_must_be_copy_or_fold_or_referenced::<#value_ty>();
            };
        });
    }

    // Generate the module code.
    let module_name = &item.ident;
    let module = parse_quote! {
        #[allow(non_snake_case)]
        mod #module_name {
            use super::*;

            pub struct Key<VALUE, #params>(pub PhantomData<(VALUE, #(#phantom_args,)*)>);

            impl<#params> Copy for #key {}
            impl<#params> Clone for #key {
                fn clone(&self) -> Self {
                    *self
                }
            }

            impl<'a, #params> model::Key<'a> for #key {
                type Value = #value_ty;
                type Output = #output_ty;

                const NAME: &'static str = #name;

                fn node() -> model::NodeId {
                    model::NodeId::of::<#self_ty>()
                }

                fn get(
                    chain: model::StyleChain<'a>,
                    mut values: impl Iterator<Item = &'a Self::Value>,
                ) -> Self::Output {
                    #get
                }
            }

            #copy
        }
    };

    // Replace type and initializer expression with the `Key`.
    item.ty = parse_quote! { #module_name::#key };
    item.expr = parse_quote! { #module_name::Key(PhantomData) };

    Ok((property, module))
}

/// A style property.
struct Property {
    name: Ident,
    skip: bool,
    referenced: bool,
    shorthand: Option<Shorthand>,
    resolve: bool,
    fold: bool,
}

enum Shorthand {
    Positional,
    Named(Ident),
}

/// Parse a style property attribute.
fn parse_property(item: &mut syn::ImplItemConst) -> Result<Property> {
    let mut property = Property {
        name: item.ident.clone(),
        skip: false,
        shorthand: None,
        referenced: false,
        resolve: false,
        fold: false,
    };

    if let Some(idx) = item
        .attrs
        .iter()
        .position(|attr| attr.path.get_ident().map_or(false, |name| name == "property"))
    {
        let attr = item.attrs.remove(idx);
        let mut stream = attr.parse_args::<TokenStream2>()?.into_iter().peekable();
        while let Some(token) = stream.next() {
            match token {
                TokenTree::Ident(ident) => match ident.to_string().as_str() {
                    "skip" => property.skip = true,
                    "shorthand" => {
                        let short = if let Some(TokenTree::Group(group)) = stream.peek() {
                            let span = group.span();
                            let repr = group.to_string();
                            let ident = repr.trim_matches(|c| matches!(c, '(' | ')'));
                            if !ident.chars().all(|c| c.is_ascii_alphabetic()) {
                                return Err(Error::new(span, "invalid args"));
                            }
                            stream.next();
                            Shorthand::Named(Ident::new(ident, span))
                        } else {
                            Shorthand::Positional
                        };
                        property.shorthand = Some(short);
                    }
                    "referenced" => property.referenced = true,
                    "resolve" => property.resolve = true,
                    "fold" => property.fold = true,
                    _ => return Err(Error::new(ident.span(), "invalid attribute")),
                },
                TokenTree::Punct(_) => {}
                _ => return Err(Error::new(token.span(), "invalid token")),
            }
        }
    }

    let span = property.name.span();
    if property.skip && property.shorthand.is_some() {
        return Err(Error::new(
            span,
            "skip and shorthand are mutually exclusive",
        ));
    }

    if property.referenced && (property.fold || property.resolve) {
        return Err(Error::new(
            span,
            "referenced is mutually exclusive with fold and resolve",
        ));
    }

    Ok(property)
}

/// Auto-generate a `set` function from properties.
fn generate_set(
    properties: &[Property],
    user: Option<syn::ImplItemMethod>,
) -> syn::ImplItemMethod {
    let user = user.map(|method| {
        let block = &method.block;
        quote! { (|| -> crate::diag::SourceResult<()> { #block; Ok(()) } )()?; }
    });

    let mut shorthands = vec![];
    let sets: Vec<_> = properties
        .iter()
        .filter(|p| !p.skip)
        .map(|property| {
            let name = &property.name;
            let string = name.to_string().replace("_", "-").to_lowercase();

            let value = if let Some(short) = &property.shorthand {
                match short {
                    Shorthand::Positional => quote! { args.named_or_find(#string)? },
                    Shorthand::Named(named) => {
                        shorthands.push(named);
                        quote! { args.named(#string)?.or_else(|| #named.clone()) }
                    }
                }
            } else {
                quote! { args.named(#string)? }
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
            args: &mut model::Args,
            constructor: bool,
        ) -> crate::diag::SourceResult<model::StyleMap> {
            let mut styles = model::StyleMap::new();
            #user
            #(#bindings)*
            #(#sets)*
            Ok(styles)
        }
    }
}
