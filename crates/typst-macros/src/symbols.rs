use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Ident, Result, Token};

/// Expand the `symbols!` macro.
pub fn symbols(stream: TokenStream) -> Result<TokenStream> {
    let list: Punctuated<Symbol, Token![,]> =
        Punctuated::parse_terminated.parse2(stream)?;
    let pairs = list.iter().map(|symbol| {
        let name = symbol.name.to_string();
        let kind = match &symbol.kind {
            Kind::Single(c) => quote! { ::typst::symbols::Symbol::single(#c), },
            Kind::Multiple(variants) => {
                let variants = variants.iter().map(|variant| {
                    let name = &variant.name;
                    let c = &variant.c;
                    quote! { (#name, #c) }
                });
                quote! {
                    ::typst::symbols::Symbol::list(&[#(#variants),*])
                }
            }
        };
        quote! { (#name, #kind) }
    });
    Ok(quote! { &[#(#pairs),*] })
}

struct Symbol {
    name: syn::Ident,
    kind: Kind,
}

enum Kind {
    Single(syn::LitChar),
    Multiple(Punctuated<Variant, Token![,]>),
}

struct Variant {
    name: String,
    c: syn::LitChar,
}

impl Parse for Symbol {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.call(Ident::parse_any)?;
        input.parse::<Token![:]>()?;
        let kind = input.parse()?;
        Ok(Self { name, kind })
    }
}

impl Parse for Kind {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::LitChar) {
            Ok(Self::Single(input.parse()?))
        } else {
            let content;
            syn::bracketed!(content in input);
            Ok(Self::Multiple(Punctuated::parse_terminated(&content)?))
        }
    }
}

impl Parse for Variant {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = String::new();
        if input.peek(syn::Ident::peek_any) {
            name.push_str(&input.call(Ident::parse_any)?.to_string());
            while input.peek(Token![.]) {
                input.parse::<Token![.]>()?;
                name.push('.');
                name.push_str(&input.call(Ident::parse_any)?.to_string());
            }
            input.parse::<Token![:]>()?;
        }
        let c = input.parse()?;
        Ok(Self { name, c })
    }
}
