use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parse_quote, Result};

use crate::util::{eat_comma, kw, parse_key_value, parse_string};

/// Expand the `#[time(..)]` macro.
pub fn time(stream: TokenStream, item: syn::ItemFn) -> Result<TokenStream> {
    let meta: Meta = syn::parse2(stream)?;
    create(meta, item)
}

/// The `..` in `#[time(..)]`.
pub struct Meta {
    pub name: Option<String>,
    pub span: Option<syn::Expr>,
    pub callsite: Option<syn::Expr>,
    pub func: Option<syn::Expr>,
    pub extras: Vec<(syn::Ident, Mode, syn::Expr)>,
}

impl Parse for Meta {
    fn parse(input: ParseStream) -> Result<Self> {
        let out = Self {
            name: parse_string::<kw::name>(input)?,
            span: parse_key_value::<kw::span, syn::Expr>(input)?,
            callsite: parse_key_value::<kw::callsite, syn::Expr>(input)?,
            func: parse_key_value::<kw::func, syn::Expr>(input)?,
            extras: {
                let mut pairs = Vec::new();
                while input.peek(syn::Ident) {
                    let key: syn::Ident = input.parse()?;
                    let _: syn::Token![=] = input.parse()?;

                    // Get the mode of this extra argument.
                    let mode = Mode::parse(input)?;

                    let value = input.parse()?;
                    eat_comma(input);

                    pairs.push((key, mode, value));
                }
                pairs
            },
        };

        let mut keys = HashSet::new();
        keys.insert("name".to_string());
        keys.insert("span".to_string());
        keys.insert("callsite".to_string());
        keys.insert("func".to_string());

        // Check that the keys are unique.
        for (key, _, _) in &out.extras {
            if !keys.insert(key.to_string()) {
                bail!(key, "Duplicate key in #[time(..)]: `{}`", key);
            }
        }

        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum Mode {
    Span,
    Serialize,
    Debug,
    Display,
}

impl Parse for Mode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::Token![$]) {
            input.parse::<syn::Token![$]>()?;
            Ok(Self::Span)
        } else if input.peek(syn::Token![?]) {
            input.parse::<syn::Token![?]>()?;
            Ok(Self::Debug)
        } else if input.peek(syn::Token![#]) {
            input.parse::<syn::Token![#]>()?;
            Ok(Self::Display)
        } else {
            Ok(Self::Serialize)
        }
    }
}

fn create(meta: Meta, mut item: syn::ItemFn) -> Result<TokenStream> {
    let name = meta.name.unwrap_or_else(|| item.sig.ident.to_string());
    let mut extras = Vec::new();
    if let Some(func) = &meta.func {
        extras.push(quote! { .with_func(#func) });
    }

    if let Some(span) = &meta.span {
        extras.push(quote! { .with_span(#span.into_raw()) });
    }

    if let Some(callsite) = &meta.callsite {
        extras.push(quote! { .with_callsite(#callsite.into_raw()) });
    }

    for (key, mode, value) in &meta.extras {
        let (method, transform) = match mode {
            Mode::Span => {
                (format_ident!("with_named_span"), Some(quote! { .into_raw() }))
            }
            Mode::Debug => (format_ident!("with_debug"), None),
            Mode::Display => (format_ident!("with_display"), None),
            Mode::Serialize => (format_ident!("with_arg"), None),
        };

        let key = key.to_string();
        extras.push(quote! { .#method(#key, (#value) #transform) });
        if matches!(mode, Mode::Serialize) {
            let error_msg = format!("failed to serialize {key}");
            extras.push(quote! { .expect(#error_msg) })
        }
    }

    item.block.stmts.insert(
        0,
        parse_quote! {
            let __scope = ::typst_timing::TimingScope::new(#name).map(|__scope| {
                __scope
                    #(#extras)*
                    .build()
            });
        },
    );

    Ok(item.into_token_stream())
}
