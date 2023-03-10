//! Procedural macros for Typst.

extern crate proc_macro;

#[macro_use]
mod util;
mod castable;
mod func;
mod node;
mod symbols;

use proc_macro::TokenStream as BoundaryStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{parse_quote, Ident, Result, Token};

use self::util::*;

/// Turns a function into a `NativeFunc`.
#[proc_macro_attribute]
pub fn func(_: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemFn);
    func::func(item).unwrap_or_else(|err| err.to_compile_error()).into()
}

/// Implement `Node` for a struct.
#[proc_macro_attribute]
pub fn node(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemStruct);
    node::node(stream.into(), item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implement `Cast` and optionally `Type` for a type.
#[proc_macro]
pub fn cast_from_value(stream: BoundaryStream) -> BoundaryStream {
    castable::cast_from_value(stream.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implement `From<T> for Value` for a type `T`.
#[proc_macro]
pub fn cast_to_value(stream: BoundaryStream) -> BoundaryStream {
    castable::cast_to_value(stream.into())
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
