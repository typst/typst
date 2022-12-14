use super::*;

use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::Token;

/// Expand the `#[capability]` macro.
pub fn capability(item: syn::ItemTrait) -> Result<TokenStream> {
    let ident = &item.ident;
    Ok(quote! {
        #item
        impl ::typst::model::Capability for dyn #ident {}
    })
}

/// Expand the `#[capable(..)]` macro.
pub fn capable(attr: TokenStream, item: syn::Item) -> Result<TokenStream> {
    let (ident, generics) = match &item {
        syn::Item::Struct(s) => (&s.ident, &s.generics),
        syn::Item::Enum(s) => (&s.ident, &s.generics),
        _ => bail!(item, "only structs and enums are supported"),
    };

    let (params, args, clause) = generics.split_for_impl();
    let checks = Punctuated::<Ident, Token![,]>::parse_terminated
        .parse2(attr)?
        .into_iter()
        .map(|capability| {
            quote! {
                if id == ::std::any::TypeId::of::<dyn #capability>() {
                    return Some(unsafe {
                        ::typst::util::fat::vtable(self as &dyn #capability)
                    });
                }
            }
        });

    Ok(quote! {
        #item

        unsafe impl #params ::typst::model::Capable for #ident #args #clause {
            fn vtable(&self, id: ::std::any::TypeId) -> ::std::option::Option<*const ()> {
                #(#checks)*
                None
            }
        }
    })
}
