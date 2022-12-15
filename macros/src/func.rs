use super::*;

/// Expand the `#[func]` macro.
pub fn func(item: syn::Item) -> Result<TokenStream> {
    let doc_comment = match &item {
        syn::Item::Struct(item) => doc_comment(&item.attrs),
        syn::Item::Enum(item) => doc_comment(&item.attrs),
        syn::Item::Fn(item) => doc_comment(&item.attrs),
        _ => String::new(),
    };

    let mut tags = vec![];
    let mut kept = vec![];
    for line in doc_comment.lines() {
        let line = line.trim();
        if let Some(suffix) = line.trim_end_matches(".").strip_prefix("Tags: ") {
            tags.extend(suffix.split(", "));
        } else {
            kept.push(line);
        }
    }

    while kept.last().map_or(false, |line| line.is_empty()) {
        kept.pop();
    }

    let docs = kept.join("\n");
    let info = quote! {
        ::typst::model::FuncInfo {
            name,
            docs: #docs,
            tags: &[#(#tags),*],
            params: ::std::vec![],
        }
    };

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
                    ::typst::model::Func::from_fn(name, #full, #info)
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
                    ::typst::model::Func::from_node::<Self>(name, #info)
                }
            }
        })
    }
}
