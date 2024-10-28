use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Ident, LitChar, Path, Result, Token};

use crate::util::foundations;

/// Expand the `symbols!` macro.
pub fn symbols(stream: TokenStream) -> Result<TokenStream> {
    let list: Punctuated<Symbol, Token![,]> =
        Punctuated::parse_terminated.parse2(stream)?;
    let pairs = list.iter().map(|symbol| {
        let name = symbol.name.to_string();
        let kind = match &symbol.kind {
            Kind::Single(c, h) => {
                let symbol = construct_sym_char(c, h);
                quote! { #foundations::Symbol::single(#symbol), }
            }
            Kind::Multiple(variants) => {
                let variants = variants.iter().map(|variant| {
                    let name = &variant.name;
                    let c = &variant.c;
                    let symbol = construct_sym_char(c, &variant.handler);
                    quote! { (#name, #symbol) }
                });
                quote! {
                    #foundations::Symbol::list(&[#(#variants),*])
                }
            }
        };
        quote! { (#name, #kind) }
    });
    Ok(quote! { &[#(#pairs),*] })
}

fn construct_sym_char(ch: &LitChar, handler: &Handler) -> TokenStream {
    match &handler.0 {
        None => quote! {  #foundations::SymChar::pure(#ch), },
        Some(path) => quote! {
             #foundations::SymChar::with_func(
                #ch,
                <#path as ::typst_library::foundations::NativeFunc>::func,
            ),
        },
    }
}

struct Symbol {
    name: syn::Ident,
    kind: Kind,
}

enum Kind {
    Single(syn::LitChar, Handler),
    Multiple(Punctuated<Variant, Token![,]>),
}

struct Variant {
    name: String,
    c: syn::LitChar,
    handler: Handler,
}

struct Handler(Option<Path>);

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
        let handler = input.parse::<Handler>()?;
        if input.peek(syn::LitChar) {
            Ok(Self::Single(input.parse()?, handler))
        } else {
            if handler.0.is_some() {
                return Err(input.error("unexpected handler"));
            }
            let content;
            syn::bracketed!(content in input);
            Ok(Self::Multiple(Punctuated::parse_terminated(&content)?))
        }
    }
}

impl Parse for Variant {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = String::new();
        let handler = input.parse::<Handler>()?;
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
        Ok(Self { name, c, handler })
    }
}

impl Parse for Handler {
    fn parse(input: ParseStream) -> Result<Self> {
        let Ok(attrs) = input.call(syn::Attribute::parse_outer) else {
            return Ok(Self(None));
        };
        let handler = attrs
            .iter()
            .find_map(|attr| {
                if attr.path().is_ident("call") {
                    if let Ok(path) = attr.parse_args::<Path>() {
                        return Some(Self(Some(path)));
                    }
                }
                None
            })
            .unwrap_or(Self(None));
        Ok(handler)
    }
}
