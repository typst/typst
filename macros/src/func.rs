use proc_macro2::Span;
use unscanny::Scanner;

use super::*;

/// Expand the `#[func]` macro.
pub fn func(item: syn::Item) -> Result<TokenStream> {
    let mut docs = match &item {
        syn::Item::Struct(item) => documentation(&item.attrs),
        syn::Item::Enum(item) => documentation(&item.attrs),
        syn::Item::Fn(item) => documentation(&item.attrs),
        _ => String::new(),
    };

    let tags = tags(&mut docs);
    let params = params(&mut docs)?;
    let docs = docs.trim();
    let info = quote! {
        ::typst::model::FuncInfo {
            name,
            docs: #docs,
            tags: &[#(#tags),*],
            params: ::std::vec![#(#params),*],
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

/// Extract a section.
pub fn section(docs: &mut String, title: &str) -> Option<String> {
    let needle = format!("# {title}\n");
    let start = docs.find(&needle)?;
    let rest = &docs[start..];
    let len = rest[1..].find('#').map(|x| 1 + x).unwrap_or(rest.len());
    let end = start + len;
    let section = docs[start + needle.len()..].to_owned();
    docs.replace_range(start..end, "");
    Some(section)
}

/// Parse the tag section.
pub fn tags(docs: &mut String) -> Vec<String> {
    section(docs, "Tags")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix('-'))
        .map(|s| s.trim().into())
        .collect()
}

/// Parse the parameter section.
pub fn params(docs: &mut String) -> Result<Vec<TokenStream>> {
    let Some(section) = section(docs, "Parameters") else { return Ok(vec![]) };
    let mut s = Scanner::new(&section);
    let mut infos = vec![];

    while s.eat_if('-') {
        s.eat_whitespace();
        let name = s.eat_until(':');
        s.expect(": ");
        let ty: syn::Type = syn::parse_str(s.eat_until(char::is_whitespace))?;
        s.eat_whitespace();
        let mut named = false;
        let mut positional = false;
        let mut required = false;
        let mut variadic = false;
        let mut settable = false;
        s.expect('(');
        for part in s.eat_until(')').split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match part {
                "named" => named = true,
                "positional" => positional = true,
                "required" => required = true,
                "variadic" => variadic = true,
                "settable" => settable = true,
                _ => {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!("unknown parameter flag {:?}", part),
                    ))
                }
            }
        }

        if (!named && !positional)
            || (variadic && !positional)
            || (named && variadic)
            || (required && variadic)
        {
            return Err(syn::Error::new(
                Span::call_site(),
                "invalid combination of parameter flags",
            ));
        }

        s.expect(')');
        let docs = dedent(s.eat_until("\n-").trim());
        infos.push(quote! {
            ::typst::model::ParamInfo {
                name: #name,
                docs: #docs,
                cast: <#ty as ::typst::model::Cast<
                    ::typst::syntax::Spanned<::typst::model::Value>
                >>::describe(),
                named: #named,
                positional: #positional,
                required: #required,
                variadic: #variadic,
                settable: #settable,
            }
        });

        s.eat_whitespace();
    }

    Ok(infos)
}
