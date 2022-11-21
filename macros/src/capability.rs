use super::*;

/// Expand the `#[capability]` macro.
pub fn expand(body: syn::ItemTrait) -> Result<TokenStream> {
    let ident = &body.ident;
    Ok(quote! {
        #body
        impl ::typst::model::Capability for dyn #ident {}
    })
}
