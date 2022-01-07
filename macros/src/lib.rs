extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::{Error, Result};

/// Generate node properties.
#[proc_macro_attribute]
pub fn properties(_: TokenStream, item: TokenStream) -> TokenStream {
    let impl_block = syn::parse_macro_input!(item as syn::ItemImpl);
    expand(impl_block).unwrap_or_else(|err| err.to_compile_error()).into()
}

/// Expand a property impl block for a node.
fn expand(mut impl_block: syn::ItemImpl) -> Result<TokenStream2> {
    // Split the node type into name and generic type arguments.
    let self_ty = &*impl_block.self_ty;
    let (self_name, self_args) = parse_self(self_ty)?;

    // Rewrite the const items from values to keys.
    let mut modules = vec![];
    for item in &mut impl_block.items {
        if let syn::ImplItem::Const(item) = item {
            let module = process_const(
                item,
                &impl_block.generics,
                self_ty,
                &self_name,
                &self_args,
            )?;
            modules.push(module);
        }
    }

    // Put everything into a module with a hopefully unique type to isolate
    // it from the outside.
    let module = quote::format_ident!("{}_types", self_name);
    Ok(quote! {
        #[allow(non_snake_case)]
        mod #module {
            use std::any::TypeId;
            use std::marker::PhantomData;
            use once_cell::sync::Lazy;
            use crate::eval::{Nonfolding, Property};
            use super::*;

            #impl_block
            #(#modules)*
        }
    })
}

/// Parse the name and generic type arguments of the node type.
fn parse_self(self_ty: &syn::Type) -> Result<(String, Vec<&syn::Type>)> {
    // Extract the node type for which we want to generate properties.
    let path = match self_ty {
        syn::Type::Path(path) => path,
        ty => return Err(Error::new(ty.span(), "must be a path type")),
    };

    // Split up the type into its name and its generic type arguments.
    let last = path.path.segments.last().unwrap();
    let self_name = last.ident.to_string();
    let self_args = match &last.arguments {
        syn::PathArguments::AngleBracketed(args) => args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .collect(),
        _ => vec![],
    };

    Ok((self_name, self_args))
}

/// Process a single const item.
fn process_const(
    item: &mut syn::ImplItemConst,
    impl_generics: &syn::Generics,
    self_ty: &syn::Type,
    self_name: &str,
    self_args: &[&syn::Type],
) -> Result<syn::ItemMod> {
    // The module that will contain the `Key` type.
    let module_name = &item.ident;

    // The type of the property's value is what the user of our macro wrote
    // as type of the const ...
    let value_ty = &item.ty;

    // ... but the real type of the const becomes Key<#key_args>.
    let key_params = &impl_generics.params;
    let key_args = quote! { #value_ty #(, #self_args)* };

    // The display name, e.g. `TextNode::STRONG`.
    let name = format!("{}::{}", self_name, &item.ident);

    // The default value of the property is what the user wrote as
    // initialization value of the const.
    let default = &item.expr;

    let mut folder = None;
    let mut nonfolding = Some(quote! {
        impl<#key_params> Nonfolding for Key<#key_args> {}
    });

    // Look for a folding function like `#[fold(u64::add)]`.
    for attr in &item.attrs {
        if attr.path.is_ident("fold") {
            let func: syn::Expr = attr.parse_args()?;
            folder = Some(quote! {
                const FOLDABLE: bool = true;

                fn fold(inner: Self::Value, outer: Self::Value) -> Self::Value {
                    let f: fn(Self::Value, Self::Value) -> Self::Value = #func;
                    f(inner, outer)
                }
            });
            nonfolding = None;
        }
    }

    // Generate the module code.
    let module = parse_quote! {
        #[allow(non_snake_case)]
        mod #module_name {
            use super::*;

            pub struct Key<T, #key_params>(pub PhantomData<(T, #key_args)>);

            impl<#key_params> Copy for Key<#key_args> {}
            impl<#key_params> Clone for Key<#key_args> {
                fn clone(&self) -> Self {
                    *self
                }
            }

            impl<#key_params> Property for Key<#key_args> {
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

                #folder
            }

            #nonfolding
        }
    };

    // Replace type and initializer expression with the `Key`.
    item.attrs.retain(|attr| !attr.path.is_ident("fold"));
    item.ty = parse_quote! { #module_name::Key<#key_args> };
    item.expr = parse_quote! { #module_name::Key(PhantomData) };

    Ok(module)
}
