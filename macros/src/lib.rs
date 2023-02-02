//! Procedural macros for Typst.

extern crate proc_macro;

/// Return an error at the given item.
macro_rules! bail {
    (callsite, $fmt:literal $($tts:tt)*) => {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(concat!("typst: ", $fmt) $($tts)*)
        ))
    };
    ($item:expr, $fmt:literal $($tts:tt)*) => {
        return Err(syn::Error::new_spanned(
            &$item,
            format!(concat!("typst: ", $fmt) $($tts)*)
        ))
    };
}

mod capable;
mod castable;
mod func;
mod node;
mod symbols;

use proc_macro::TokenStream as BoundaryStream;
use proc_macro2::{TokenStream, TokenTree};
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_quote, Ident, Result};

/// Implement `FuncType` for a type or function.
#[proc_macro_attribute]
pub fn func(_: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::Item);
    func::func(item).unwrap_or_else(|err| err.to_compile_error()).into()
}

/// Implement `Node` for a struct.
#[proc_macro_attribute]
pub fn node(_: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemImpl);
    node::node(item).unwrap_or_else(|err| err.to_compile_error()).into()
}

/// Implement `Capability` for a trait.
#[proc_macro_attribute]
pub fn capability(_: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemTrait);
    capable::capability(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implement `Capable` for a type.
#[proc_macro_attribute]
pub fn capable(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::Item);
    capable::capable(stream.into(), item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implement `Cast` and optionally `Type` for a type.
#[proc_macro]
pub fn castable(stream: BoundaryStream) -> BoundaryStream {
    castable::castable(stream.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Define a list of symbols.
#[proc_macro]
pub fn symbols(stream: BoundaryStream) -> BoundaryStream {
    symbols::symbols(stream.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Extract documentation comments from an attribute list.
fn documentation(attrs: &[syn::Attribute]) -> String {
    let mut doc = String::new();

    // Parse doc comments.
    for attr in attrs {
        if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
            if meta.path.is_ident("doc") {
                if let syn::Lit::Str(string) = &meta.lit {
                    let full = string.value();
                    let line = full.strip_prefix(' ').unwrap_or(&full);
                    doc.push_str(line);
                    doc.push('\n');
                }
            }
        }
    }

    doc.trim().into()
}

/// Dedent documentation text.
fn dedent(text: &str) -> String {
    text.lines()
        .map(|s| s.strip_prefix("  ").unwrap_or(s))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Quote an optional value.
fn quote_option<T: ToTokens>(option: Option<T>) -> TokenStream {
    match option {
        Some(value) => quote! { Some(#value) },
        None => quote! { None },
    }
}
