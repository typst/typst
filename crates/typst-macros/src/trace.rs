use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::Result;

use crate::util::{kw, parse_key_value, parse_string};

/// Expand the `#[trace(..)]` macro.
pub fn trace(stream: TokenStream, item: &syn::ItemFn) -> Result<TokenStream> {
    let meta: Meta = syn::parse2(stream)?;
    Ok(create(meta, item))
}

/// The `..` in `#[trace(..)]`.
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

fn create(meta: Meta, item: &syn::ItemFn) -> TokenStream {
    let name = meta.name.unwrap_or_else(|| item.sig.ident.to_string());
    let span = meta
        .span
        .as_ref()
        .map(|span| quote! { Some(#span) })
        .unwrap_or_else(|| quote! { None });

    let sig = &item.sig;
    let vis = &item.vis;
    let block = &item.block;
    quote! {
        #vis #sig {
            use ::typst::foundations::NativeElement;
            #[cfg(not(target_arch = "wasm32"))]
            return ::typst_trace::record(
                #name,
                #span,
                move || #block
            );

            #[cfg(target_arch = "wasm32")]
            return #block;
        }
    }
}
