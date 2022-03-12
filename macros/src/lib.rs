extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
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
        let sets = properties.into_iter().filter(|p| !p.skip).map(|property| {
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

/// A style property.
struct Property {
    name: Ident,
    shorthand: bool,
    variadic: bool,
    skip: bool,
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
    // The display name, e.g. `TextNode::STRONG`.
    let name = format!("{}::{}", self_name, &item.ident);

    // The type of the property's value is what the user of our macro wrote
    // as type of the const ...
    let value_ty = &item.ty;

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

    // The default value of the property is what the user wrote as
    // initialization value of the const.
    let default = &item.expr;

    let mut fold = None;
    let mut property = Property {
        name: item.ident.clone(),
        shorthand: false,
        variadic: false,
        skip: false,
    };

    for attr in std::mem::take(&mut item.attrs) {
        match attr.path.get_ident().map(ToString::to_string).as_deref() {
            Some("fold") => {
                // Look for a folding function like `#[fold(u64::add)]`.
                let func: syn::Expr = attr.parse_args()?;
                fold = Some(quote! {
                    const FOLDING: bool = true;

                    fn fold(inner: Self::Value, outer: Self::Value) -> Self::Value {
                        let f: fn(Self::Value, Self::Value) -> Self::Value = #func;
                        f(inner, outer)
                    }
                });
            }
            Some("shorthand") => property.shorthand = true,
            Some("variadic") => property.variadic = true,
            Some("skip") => property.skip = true,
            _ => item.attrs.push(attr),
        }
    }

    if property.shorthand && property.variadic {
        return Err(Error::new(
            property.name.span(),
            "shorthand and variadic are mutually exclusive",
        ));
    }

    let referencable = fold.is_none().then(|| {
        quote! { impl<#params> eval::Referencable for #key {} }
    });

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

            impl<#params> eval::Key for #key {
                type Value = #value_ty;

                const NAME: &'static str = #name;

                fn node_id() -> TypeId {
                    TypeId::of::<#self_ty>()
                }

                fn default() -> Self::Value {
                    #default
                }

                fn default_ref() -> &'static Self::Value {
                    static LAZY: Lazy<#value_ty> = Lazy::new(|| #default);
                    &*LAZY
                }

                #fold
            }

            #referencable
        }
    };

    // Replace type and initializer expression with the `Key`.
    item.ty = parse_quote! { #module_name::#key };
    item.expr = parse_quote! { #module_name::Key(PhantomData) };

    Ok((property, module))
}
