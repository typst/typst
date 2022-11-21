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

mod capability;
mod node;

use proc_macro::TokenStream as BoundaryStream;
use proc_macro2::{TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::parse_quote;
use syn::{Error, Ident, Result};

/// Implement `Node` for a struct.
#[proc_macro_attribute]
pub fn node(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemImpl);
    node::expand(stream.into(), item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implement `Capability` for a trait.
#[proc_macro_attribute]
pub fn capability(_: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemTrait);
    capability::expand(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
