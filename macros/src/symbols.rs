use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::Token;

use super::*;

/// Expand the `symbols!` macro.
pub fn symbols(stream: TokenStream) -> Result<TokenStream> {
    let list: List = syn::parse2(stream)?;
    let pairs = list.0.iter().map(Symbol::expand);
    Ok(quote! { &[#(#pairs),*] })
}

struct List(Punctuated<Symbol, Token![,]>);

impl Parse for List {
    fn parse(input: ParseStream) -> Result<Self> {
        Punctuated::parse_terminated(input).map(Self)
    }
}

struct Symbol {
    name: syn::Ident,
    kind: Kind,
}

impl Parse for Symbol {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.call(Ident::parse_any)?;
        input.parse::<Token![:]>()?;
        let kind = input.parse()?;
        Ok(Self { name, kind })
    }
}

impl Symbol {
    fn expand(&self) -> TokenStream {
        let name = self.name.to_string();
        let kind = self.kind.expand();
        quote! { (#name, #kind) }
    }
}

enum Kind {
    Single(syn::LitChar),
    Multiple(Punctuated<Variant, Token![,]>),
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

impl Kind {
    fn expand(&self) -> TokenStream {
        match self {
            Self::Single(c) => quote! { typst::eval::Symbol::new(#c), },
            Self::Multiple(variants) => {
                let variants = variants.iter().map(Variant::expand);
                quote! {
                    typst::eval::Symbol::list(&[#(#variants),*])
                }
            }
        }
    }
}

struct Variant {
    name: String,
    c: syn::LitChar,
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

impl Variant {
    fn expand(&self) -> TokenStream {
        let name = &self.name;
        let c = &self.c;
        quote! { (#name, #c) }
    }
}
