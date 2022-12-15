//! Procedural macros for Typst.

extern crate proc_macro;

/// Return an error at the given item.
macro_rules! bail {
    ($item:expr, $fmt:literal $($tts:tt)*) => {
        return Err(Error::new_spanned(
            &$item,
            format!(concat!("typst: ", $fmt) $($tts)*)
        ))
    }
}

mod capable;
mod castable;
mod func;
mod node;

use proc_macro::TokenStream as BoundaryStream;
use proc_macro2::{TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::parse_quote;
use syn::{Error, Ident, Result};

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

/// Extract documentation comments from an attribute list.
fn doc_comment(attrs: &[syn::Attribute]) -> String {
    let mut doc = String::new();

    // Parse doc comments.
    for attr in attrs {
        if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
            if meta.path.is_ident("doc") {
                if let syn::Lit::Str(string) = &meta.lit {
                    doc.push_str(&string.value());
                    doc.push('\n');
                }
            }
        }
    }

    doc.trim().into()
}
