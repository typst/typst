use heck::ToKebabCase;

use super::*;

/// Expand the `#[scope]` macro.
pub fn scope(_: TokenStream, item: syn::Item) -> Result<TokenStream> {
    let syn::Item::Impl(mut item) = item else {
        bail!(item, "expected module or impl item");
    };

    let eval = quote! { ::typst::eval };
    let self_ty = &item.self_ty;

    let mut definitions = vec![];
    let mut constructor = quote! { None };
    for child in &mut item.items {
        let def = match child {
            syn::ImplItem::Const(item) => handle_const(self_ty, item)?,
            syn::ImplItem::Fn(item) => match handle_fn(self_ty, item)? {
                FnKind::Member(tokens) => tokens,
                FnKind::Constructor(tokens) => {
                    constructor = tokens;
                    continue;
                }
            },
            syn::ImplItem::Verbatim(item) => handle_type_or_elem(item)?,
            _ => bail!(child, "unexpected item in scope"),
        };
        definitions.push(def);
    }

    item.items.retain(|item| !matches!(item, syn::ImplItem::Verbatim(_)));

    let mut base = quote! { #item };
    if let syn::Type::Path(syn::TypePath { path, .. }) = self_ty.as_ref() {
        if let Some(ident) = path.get_ident() {
            if is_primitive(ident) {
                base = rewrite_primitive_base(&item, ident);
            }
        }
    }

    Ok(quote! {
        #base

        impl #eval::NativeScope for #self_ty {
            fn constructor() -> ::std::option::Option<&'static #eval::NativeFuncData> {
                #constructor
            }

            fn scope() -> #eval::Scope {
                let mut scope = #eval::Scope::deduplicating();
                #(#definitions;)*
                scope
            }
        }
    })
}

/// Process a const item and returns its definition.
fn handle_const(self_ty: &syn::Type, item: &syn::ImplItemConst) -> Result<TokenStream> {
    let ident = &item.ident;
    let name = ident.to_string().to_kebab_case();
    Ok(quote! { scope.define(#name, #self_ty::#ident) })
}

/// Process a type item.
fn handle_type_or_elem(item: &TokenStream) -> Result<TokenStream> {
    let item: BareType = syn::parse2(item.clone())?;
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
            let meta: super::func::Meta = syn::parse2(tokens.clone())?;
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
fn rewrite_primitive_base(item: &syn::ItemImpl, ident: &syn::Ident) -> TokenStream {
    let mut sigs = vec![];
    let mut items = vec![];
    for sub in &item.items {
        let syn::ImplItem::Fn(mut func) = sub.clone() else { continue };
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
            fn #ident_data() -> &'static ::typst::eval::NativeFuncData;
        });
    }

    let ident_ext = quote::format_ident!("{ident}Ext");
    let self_ty = &item.self_ty;
    quote! {
        trait #ident_ext {
            #(#sigs)*
        }

        impl #ident_ext for #self_ty {
            #(#items)*
        }
    }
}
