use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, Result};

use crate::util::{
    BareType, determine_name_and_title, documentation, foundations, kw, parse_flag,
    parse_string, parse_string_array,
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
    meta: Meta,
    /// The name for this type given in Rust.
    ident: Ident,
    /// The type's identifier as exposed to Typst.
    name: String,
    long: String,
    /// The type's title case name.
    title: String,
    /// The documentation for this type as a string.
    docs: String,
}

/// The `..` in `#[ty(..)]`.
struct Meta {
    /// Whether this element has an associated scope defined by the `#[scope]` macro.
    scope: bool,
    /// Whether a custom cast implementation will be defined for this type.
    cast: bool,
    name: Option<String>,
    title: Option<String>,
    keywords: Vec<String>,
}

impl Parse for Meta {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            scope: parse_flag::<kw::scope>(input)?,
            cast: parse_flag::<kw::cast>(input)?,
            name: parse_string::<kw::name>(input)?,
            title: parse_string::<kw::title>(input)?,
            keywords: parse_string_array::<kw::keywords>(input)?,
        })
    }
}

/// Parse details about the type from its definition.
fn parse(meta: Meta, ident: Ident, attrs: &[Attribute]) -> Result<Type> {
    let docs = documentation(attrs);
    let (name, title) =
        determine_name_and_title(meta.name.clone(), meta.title.clone(), &ident, None)?;
    let long = title.to_lowercase();
    Ok(Type { meta, ident, name, long, title, docs })
}

/// Produce the output of the macro.
fn create(ty: &Type, item: Option<&syn::Item>) -> TokenStream {
    let Type { ident, name, long, title, docs, meta, .. } = ty;
    let Meta { keywords, .. } = meta;

    let constructor = if meta.scope {
        quote! { <#ident as #foundations::NativeScope>::constructor() }
    } else {
        quote! { None }
    };

    let scope = if meta.scope {
        quote! { <#ident as #foundations::NativeScope>::scope() }
    } else {
        quote! { #foundations::Scope::new() }
    };

    let cast = (!meta.cast).then(|| {
        quote! {
            #foundations::cast! { type #ident, }
        }
    });

    let data = quote! {
        #foundations::NativeTypeData {
            name: #name,
            long_name: #long,
            title: #title,
            docs: #docs,
            keywords: &[#(#keywords),*],
            constructor: ::std::sync::LazyLock::new(|| #constructor),
            scope: ::std::sync::LazyLock::new(|| #scope),
        }
    };

    let attr = item.map(|_| {
        quote! {
            #[allow(rustdoc::broken_intra_doc_links)]
        }
    });

    quote! {
        #attr
        #item
        #cast

        impl #foundations::NativeType for #ident {
            const NAME: &'static str = #name;

            fn data() -> &'static #foundations::NativeTypeData {
                static DATA: #foundations::NativeTypeData = #data;
                &DATA
            }
        }
    }
}
