use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, Result};

use crate::util::{
    determine_name_and_title, documentation, foundations, kw, parse_flag, parse_string,
    parse_string_array, BareType,
};

/// Expand the `#[ty]` macro.
pub fn ty(stream: TokenStream, item: syn::Item) -> Result<TokenStream> {
    let meta: Meta = syn::parse2(stream)?;
    let bare: BareType;
    let (ident, attrs, keep) = match &item {
        syn::Item::Struct(item) => (&item.ident, &item.attrs, true),
        syn::Item::Type(item) => (&item.ident, &item.attrs, true),
        syn::Item::Enum(item) => (&item.ident, &item.attrs, true),
        syn::Item::Verbatim(item) => {
            bare = syn::parse2(item.clone())?;
            (&bare.ident, &bare.attrs, false)
        }
        _ => bail!(item, "invalid type item"),
    };
    let ty = parse(meta, ident.clone(), attrs)?;
    Ok(create(&ty, keep.then_some(&item)))
}

/// Holds all relevant parsed data about a type.
struct Type {
    ident: Ident,
    name: String,
    long: String,
    scope: bool,
    title: String,
    docs: String,
    keywords: Vec<String>,
}

/// The `..` in `#[ty(..)]`.
struct Meta {
    scope: bool,
    name: Option<String>,
    title: Option<String>,
    keywords: Vec<String>,
}

impl Parse for Meta {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            scope: parse_flag::<kw::scope>(input)?,
            name: parse_string::<kw::name>(input)?,
            title: parse_string::<kw::title>(input)?,
            keywords: parse_string_array::<kw::keywords>(input)?,
        })
    }
}

/// Parse details about the type from its definition.
fn parse(meta: Meta, ident: Ident, attrs: &[Attribute]) -> Result<Type> {
    let docs = documentation(attrs);
    let (name, title) = determine_name_and_title(meta.name, meta.title, &ident, None)?;
    let long = title.to_lowercase();
    Ok(Type {
        ident,
        name,
        long,
        scope: meta.scope,
        keywords: meta.keywords,
        title,
        docs,
    })
}

/// Produce the output of the macro.
fn create(ty: &Type, item: Option<&syn::Item>) -> TokenStream {
    let Type {
        ident, name, long, title, docs, keywords, scope, ..
    } = ty;

    let constructor = if *scope {
        quote! { <#ident as #foundations::NativeScope>::constructor() }
    } else {
        quote! { None }
    };

    let scope = if *scope {
        quote! { <#ident as #foundations::NativeScope>::scope() }
    } else {
        quote! { #foundations::Scope::new() }
    };

    let data = quote! {
        #foundations::NativeTypeData {
            name: #name,
            long_name: #long,
            title: #title,
            docs: #docs,
            keywords: &[#(#keywords),*],
            constructor: #foundations::Lazy::new(|| #constructor),
            scope: #foundations::Lazy::new(|| #scope),
        }
    };

    quote! {
        #item

        impl #foundations::NativeType for #ident {
            const NAME: &'static str = #name;

            fn data() -> &'static #foundations::NativeTypeData {
                static DATA: #foundations::NativeTypeData = #data;
                &DATA
            }
        }
    }
}
