use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parse_quote, Result};

use crate::util::{kw, parse_key_value, parse_string};

/// Expand the `#[time(..)]` macro.
pub fn time(stream: TokenStream, item: syn::ItemFn) -> Result<TokenStream> {
    let meta: Meta = syn::parse2(stream)?;
    create(meta, item)
}

/// The `..` in `#[time(..)]`.
pub struct Meta {
    pub span: Option<syn::Expr>,
    pub callsite: Option<syn::Expr>,
    pub name: Option<String>,
}

impl Parse for Meta {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            name: parse_string::<kw::name>(input)?,
            span: parse_key_value::<kw::span, syn::Expr>(input)?,
            callsite: parse_key_value::<kw::callsite, syn::Expr>(input)?,
        })
    }
}

fn create(meta: Meta, mut item: syn::ItemFn) -> Result<TokenStream> {
    let name = meta.name.unwrap_or_else(|| item.sig.ident.to_string());
    let construct = match (meta.span.as_ref(), meta.callsite.as_ref()) {
        (Some(span), Some(callsite)) => quote! {
            with_callsite(#name, Some(#span.into_raw()), Some(#callsite.into_raw()))
        },
        (Some(span), None) => quote! {
            with_span(#name, Some(#span.into_raw()))
        },
        (None, Some(expr)) => {
            bail!(expr, "cannot have a callsite span without a main span")
        }
        (None, None) => quote! { new(#name) },
    };

    item.block.stmts.insert(
        0,
        parse_quote! {
            let __scope = ::typst_timing::TimingScope::#construct;
        },
    );

    Ok(item.into_token_stream())
}
