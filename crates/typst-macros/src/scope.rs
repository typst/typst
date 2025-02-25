use heck::ToKebabCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Result};

use crate::util::{foundations, BareType};

/// Expand the `#[scope]` macro.
pub fn scope(_: TokenStream, item: syn::Item) -> Result<TokenStream> {
    let syn::Item::Impl(mut item) = item else {
        bail!(item, "expected module or impl item");
    };

    let self_ty = &item.self_ty;

    let mut primitive_ident_ext = None;
    if let syn::Type::Path(syn::TypePath { path, .. }) = self_ty.as_ref() {
        if let Some(ident) = path.get_ident() {
            if is_primitive(ident) {
                let ident_ext = quote::format_ident!("{ident}Ext");
                primitive_ident_ext = Some(ident_ext);
            }
        }
    }

    let self_ty_expr = match &primitive_ident_ext {
        None => quote! { #self_ty },
        Some(ident_ext) => quote! { <#self_ty as #ident_ext> },
    };

    let mut definitions = vec![];
    let mut constructor = quote! { None };
    for child in &mut item.items {
        let bare: BareType;
        let (mut def, attrs) = match child {
            syn::ImplItem::Const(item) => {
                (handle_const(&self_ty_expr, item)?, &item.attrs)
            }
            syn::ImplItem::Fn(item) => (
                match handle_fn(self_ty, item)? {
                    FnKind::Member(tokens) => tokens,
                    FnKind::Constructor(tokens) => {
                        constructor = tokens;
                        continue;
                    }
                },
                &item.attrs,
            ),
            syn::ImplItem::Verbatim(item) => {
                bare = syn::parse2(item.clone())?;
                (handle_type_or_elem(&bare)?, &bare.attrs)
            }
            _ => bail!(child, "unexpected item in scope"),
        };

        if let Some(message) = attrs.iter().find_map(|attr| match &attr.meta {
            syn::Meta::NameValue(pair) if pair.path.is_ident("deprecated") => {
                Some(&pair.value)
            }
            _ => None,
        }) {
            def = quote! { #def.deprecated(#message) }
        }

        definitions.push(def);
    }

    item.items.retain(|item| !matches!(item, syn::ImplItem::Verbatim(_)));

    let base = match &primitive_ident_ext {
        None => quote! { #item },
        Some(ident_ext) => rewrite_primitive_base(&item, ident_ext),
    };

    Ok(quote! {
        #base

        impl #foundations::NativeScope for #self_ty {
            fn constructor() -> ::std::option::Option<&'static #foundations::NativeFuncData> {
                #constructor
            }

            #[allow(deprecated)]
            fn scope() -> #foundations::Scope {
                let mut scope = #foundations::Scope::deduplicating();
                #(#definitions;)*
                scope
            }
        }
    })
}

/// Process a const item and returns its definition.
fn handle_const(self_ty: &TokenStream, item: &syn::ImplItemConst) -> Result<TokenStream> {
    let ident = &item.ident;
    let name = ident.to_string().to_kebab_case();
    Ok(quote! { scope.define(#name, #self_ty::#ident) })
}

/// Process a type item.
fn handle_type_or_elem(item: &BareType) -> Result<TokenStream> {
    let ident = &item.ident;
    let define = if item.attrs.iter().any(|attr| attr.path().is_ident("elem")) {
        quote! { define_elem }
    } else {
        quote! { define_type }
    };
    Ok(quote! { scope.#define::<#ident>() })
}

/// Process a function, return its definition, and register it as a constructor
/// if applicable.
fn handle_fn(self_ty: &syn::Type, item: &mut syn::ImplItemFn) -> Result<FnKind> {
    let Some(attr) = item.attrs.iter_mut().find(|attr| attr.meta.path().is_ident("func"))
    else {
        bail!(item, "scope function is missing #[func] attribute");
    };

    let ident_data = quote::format_ident!("{}_data", item.sig.ident);

    match &mut attr.meta {
        syn::Meta::Path(_) => {
            *attr = parse_quote! { #[func(parent = #self_ty)] };
        }
        syn::Meta::List(list) => {
            let tokens = &list.tokens;
            let meta: crate::func::Meta = syn::parse2(tokens.clone())?;
            list.tokens = quote! { #tokens, parent = #self_ty };
            if meta.constructor {
                return Ok(FnKind::Constructor(quote! { Some(#self_ty::#ident_data()) }));
            }
        }
        syn::Meta::NameValue(_) => bail!(attr.meta, "invalid func attribute"),
    }

    Ok(FnKind::Member(quote! { scope.define_func_with_data(#self_ty::#ident_data()) }))
}

enum FnKind {
    Constructor(TokenStream),
    Member(TokenStream),
}

/// Whether the identifier describes a primitive type.
fn is_primitive(ident: &syn::Ident) -> bool {
    ident == "bool" || ident == "i64" || ident == "f64"
}

/// Rewrite an impl block for a primitive into a trait + trait impl.
fn rewrite_primitive_base(item: &syn::ItemImpl, ident_ext: &syn::Ident) -> TokenStream {
    let mut sigs = vec![];
    let mut items = vec![];
    for sub in &item.items {
        match sub.clone() {
            syn::ImplItem::Fn(mut func) => {
                func.vis = syn::Visibility::Inherited;
                items.push(func.clone());

                let mut sig = func.sig;
                let inputs = sig.inputs.iter().cloned().map(|mut input| {
                    if let syn::FnArg::Typed(typed) = &mut input {
                        typed.attrs.clear();
                    }
                    input
                });
                sig.inputs = parse_quote! { #(#inputs),* };

                let ident_data = quote::format_ident!("{}_data", sig.ident);
                sigs.push(quote! { #sig; });
                sigs.push(quote! {
                    fn #ident_data() -> &'static #foundations::NativeFuncData;
                });
            }

            syn::ImplItem::Const(cons) => {
                sigs.push(quote! { #cons });
            }

            _ => {}
        }
    }

    let self_ty = &item.self_ty;
    quote! {
        #[allow(non_camel_case_types)]
        trait #ident_ext {
            #(#sigs)*
        }

        impl #ident_ext for #self_ty {
            #(#items)*
        }
    }
}
