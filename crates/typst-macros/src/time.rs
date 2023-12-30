use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::Result;

use crate::util::{kw, parse_key_value, parse_string};

/// Expand the `#[time(..)]` macro.
pub fn time(stream: TokenStream, item: &syn::ItemFn) -> Result<TokenStream> {
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
            #[cfg(not(target_arch = "wasm32"))]
            let __scope = ::typst_timing::TimingScope::new(#name, {
                use ::typst::foundations::NativeElement;
                #span
            });

            #block
        }
    }
}
