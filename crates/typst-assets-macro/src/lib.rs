//! Proc-macro for build time embedding of assets in Typst.
//!
//! This is only used in tests and CLI, not in the main crate.

extern crate proc_macro;

use proc_macro::TokenStream as BoundaryStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Includes an asset at compile time.
#[proc_macro]
pub fn include_asset(stream: BoundaryStream) -> BoundaryStream {
    let lit = syn::parse_macro_input!(stream as syn::LitStr);
    include_asset_impl(lit)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Expands the `include_asset!` macro.
fn include_asset_impl(lit: syn::LitStr) -> Result<TokenStream> {
    let filename = lit.value();

    let path = match typst_assets::path(&filename) {
        Ok(path) => path,
        Err(err) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("typst-assets: failed to include asset: {err:#}"),
            ))
        }
    };

    let path = path.display().to_string();

    Ok(quote! {
        include_bytes!(#path)
    })
}
