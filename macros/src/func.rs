use super::*;

/// Expand the `#[func]` macro.
pub fn func(item: syn::Item) -> Result<TokenStream> {
    let doc = documentation(&item)?;

    if let syn::Item::Fn(item) = &item {
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let s = ident.to_string();
        let mut chars = s.trim_end_matches("_").chars();
        let ty = quote::format_ident!(
            "{}{}Func",
            chars.next().unwrap().to_ascii_uppercase(),
            chars.as_str()
        );

        let full = if item.sig.inputs.len() == 1 {
            quote! { |_, args| #ident(args) }
        } else {
            quote! { #ident }
        };

        Ok(quote! {
            #item

            #[doc(hidden)]
            #vis enum #ty {}

            impl::typst::model::FuncType for #ty {
                fn create_func(name: &'static str) -> ::typst::model::Func {
                    ::typst::model::Func::from_fn(name, #full, #doc)
                }
            }
        })
    } else {
        let (ident, generics) = match &item {
            syn::Item::Struct(s) => (&s.ident, &s.generics),
            syn::Item::Enum(s) => (&s.ident, &s.generics),
            _ => bail!(item, "only structs, enums, and functions are supported"),
        };

        let (params, args, clause) = generics.split_for_impl();

        Ok(quote! {
            #item

            impl #params ::typst::model::FuncType for #ident #args #clause {
                fn create_func(name: &'static str) -> ::typst::model::Func {
                    ::typst::model::Func::from_node::<Self>(name, #doc)
                }
            }
        })
    }
}

/// Extract the item's documentation.
fn documentation(item: &syn::Item) -> Result<String> {
    let mut doc = String::new();

    // Extract attributes.
    let attrs = match item {
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        _ => return Ok(doc),
    };

    // Parse doc comments.
    for attr in attrs {
        if let syn::Meta::NameValue(meta) = attr.parse_meta()? {
            if meta.path.is_ident("doc") {
                if let syn::Lit::Str(string) = &meta.lit {
                    doc.push_str(&string.value());
                    doc.push('\n');
                }
            }
        }
    }

    Ok(doc.trim().into())
}
