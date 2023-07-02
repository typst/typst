//! Procedural macros for Typst.

extern crate proc_macro;

#[macro_use]
mod util;
mod castable;
mod element;
mod func;
mod symbols;

use proc_macro::TokenStream as BoundaryStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{parse_quote, DeriveInput, Ident, Result, Token};

use self::util::*;

/// Turns a function into a `NativeFunc`.
#[proc_macro_attribute]
pub fn func(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemFn);
    func::func(stream.into(), &item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Turns a type into an `Element`.
#[proc_macro_attribute]
pub fn element(stream: BoundaryStream, item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as syn::ItemStruct);
    element::element(stream.into(), &item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implements `Reflect`, `FromValue`, and `IntoValue` for an enum.
#[proc_macro_derive(Cast, attributes(string))]
pub fn derive_cast(item: BoundaryStream) -> BoundaryStream {
    let item = syn::parse_macro_input!(item as DeriveInput);
    castable::derive_cast(&item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Implements `Reflect`, `FromValue`, and `IntoValue` for a type.
#[proc_macro]
pub fn cast(stream: BoundaryStream) -> BoundaryStream {
    castable::cast(stream.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Defines a list of `Symbol`s.
#[proc_macro]
pub fn symbols(stream: BoundaryStream) -> BoundaryStream {
    symbols::symbols(stream.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
