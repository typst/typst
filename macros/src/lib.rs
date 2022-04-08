extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{quote, quote_spanned};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, Ident, Result};

#[proc_macro_attribute]
pub fn node(stream: TokenStream, item: TokenStream) -> TokenStream {
    let impl_block = syn::parse_macro_input!(item as syn::ItemImpl);
    expand(TokenStream2::from(stream), impl_block)
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

    let construct =
        construct.ok_or_else(|| Error::new(impl_block.span(), "missing constructor"))?;

    let set = set.unwrap_or_else(|| {
        let sets = properties.into_iter().filter(|p| !p.hidden).map(|property| {
            let name = property.name;
            let string = name.to_string().replace("_", "-").to_lowercase();

            let value = if property.variadic {
                quote! {
                    match args.named(#string)? {
                        Some(value) => value,
                        None => {
                            let list: Vec<_> = args.all()?;
                            (!list.is_empty()).then(|| list)
                        }
                    }
                }
            } else if property.shorthand {
                quote! { args.named_or_find(#string)? }
            } else {
                quote! { args.named(#string)? }
            };

            quote! { styles.set_opt(Self::#name, #value); }
        });

        parse_quote! {
            fn set(args: &mut Args) -> TypResult<StyleMap> {
                let mut styles = StyleMap::new();
                #(#sets)*
                Ok(styles)
            }
        }
    });

    let showable = match stream.to_string().as_str() {
        "" => false,
        "showable" => true,
        _ => return Err(Error::new(stream.span(), "unrecognized argument")),
    };

    // Put everything into a module with a hopefully unique type to isolate
    // it from the outside.
    Ok(quote! {
        #[allow(non_snake_case)]
        mod #module {
            use std::any::TypeId;
            use std::marker::PhantomData;
            use once_cell::sync::Lazy;
            use crate::eval;
            use super::*;

            #impl_block

            impl<#params> eval::Node for #self_ty {
                const SHOWABLE: bool = #showable;
                #construct
                #set
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

    // The display name, e.g. `TextNode::STRONG`.
    let name = format!("{}::{}", self_name, &item.ident);

    // The type of the property's value is what the user of our macro wrote
    // as type of the const ...
    let value_ty = &item.ty;
    let output_ty = if property.referenced {
        parse_quote!(&'a #value_ty)
    } else if property.fold {
        parse_quote!(<#value_ty as eval::Fold>::Output)
    } else if property.resolve {
        parse_quote!(<#value_ty as eval::Resolve>::Output)
    } else {
        value_ty.clone()
    };

    // ... but the real type of the const becomes this..
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

    // Ensure that the type is either `Copy` or that the property is referenced
    // or that the property isn't copy but can't be referenced because it needs
    // folding.
    let get;
    let mut copy = None;

    if property.referenced {
        get = quote! {
            values.next().unwrap_or_else(|| {
                static LAZY: Lazy<#value_ty> = Lazy::new(|| #default);
                &*LAZY
            })
        };
    } else if property.fold {
        get = quote! {
            match values.next().cloned() {
                Some(inner) => eval::Fold::fold(inner, Self::get(chain, values)),
                None => #default,
            }
        };
    } else if property.resolve {
        get = quote! {
            let value = values.next().cloned().unwrap_or(#default);
            eval::Resolve::resolve(value, chain)
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

            impl<'a, #params> eval::Key<'a> for #key {
                type Value = #value_ty;
                type Output = #output_ty;

                const NAME: &'static str = #name;

                fn node() -> TypeId {
                    TypeId::of::<#self_ty>()
                }

                fn get(
                    chain: StyleChain<'a>,
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
    hidden: bool,
    referenced: bool,
    shorthand: bool,
    variadic: bool,
    fold: bool,
    resolve: bool,
}

/// Parse a style property attribute.
fn parse_property(item: &mut syn::ImplItemConst) -> Result<Property> {
    let mut property = Property {
        name: item.ident.clone(),
        hidden: false,
        referenced: false,
        shorthand: false,
        variadic: false,
        fold: false,
        resolve: false,
    };

    if let Some(idx) = item
        .attrs
        .iter()
        .position(|attr| attr.path.get_ident().map_or(false, |name| name == "property"))
    {
        let attr = item.attrs.remove(idx);
        for token in attr.parse_args::<TokenStream2>()? {
            match token {
                TokenTree::Ident(ident) => match ident.to_string().as_str() {
                    "hidden" => property.hidden = true,
                    "shorthand" => property.shorthand = true,
                    "referenced" => property.referenced = true,
                    "variadic" => property.variadic = true,
                    "fold" => property.fold = true,
                    "resolve" => property.resolve = true,
                    _ => return Err(Error::new(ident.span(), "invalid attribute")),
                },
                TokenTree::Punct(_) => {}
                _ => return Err(Error::new(token.span(), "invalid token")),
            }
        }
    }

    let span = property.name.span();
    if property.shorthand && property.variadic {
        return Err(Error::new(
            span,
            "shorthand and variadic are mutually exclusive",
        ));
    }

    if property.referenced as u8 + property.fold as u8 + property.resolve as u8 > 1 {
        return Err(Error::new(
            span,
            "referenced, fold and resolve are mutually exclusive",
        ));
    }

    Ok(property)
}
