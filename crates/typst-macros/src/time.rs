use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Result, parse_quote};

use crate::util::{kw, parse_key_value, parse_string};

/// Expand the `#[time(..)]` macro.
pub fn time(stream: TokenStream, item: syn::ItemFn) -> Result<TokenStream> {
    let meta: Meta = syn::parse2(stream)?;
    Ok(create(meta, item))
}

/// The `..` in `#[time(..)]`.
pub struct Meta {
    pub span: Option<syn::Expr>,
    pub name: Option<String>,
}

impl Parse for Meta {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            name: parse_string::<kw::name>(input)?,
            span: parse_key_value::<kw::span, syn::Expr>(input)?,
        })
    }
}

fn create(meta: Meta, mut item: syn::ItemFn) -> TokenStream {
    let name = meta.name.unwrap_or_else(|| item.sig.ident.to_string());
    let construct = match meta.span.as_ref() {
        Some(span) => quote! { with_span(#name, Some(#span.into_raw())) },
        None => quote! { new(#name) },
    };

    item.block.stmts.insert(
        0,
        parse_quote! {
            let __scope = ::typst_timing::TimingScope::#construct;
        },
    );

    item.into_token_stream()
}
