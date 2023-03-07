use super::*;

/// Expand the `#[func]` macro.
pub fn func(item: syn::Item) -> Result<TokenStream> {
    let mut docs = match &item {
        syn::Item::Struct(item) => documentation(&item.attrs),
        syn::Item::Enum(item) => documentation(&item.attrs),
        syn::Item::Fn(item) => documentation(&item.attrs),
        _ => String::new(),
    };

    let (params, returns) = params(&mut docs)?;
    let docs = docs.trim();

    let info = quote! {
        ::typst::eval::FuncInfo {
            name,
            display: "TODO",
            category: "TODO",
            docs: #docs,
            params: ::std::vec![#(#params),*],
            returns: ::std::vec![#(#returns),*]
        }
    };

    if let syn::Item::Fn(item) = &item {
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let s = ident.to_string();
        let mut chars = s.trim_end_matches('_').chars();
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

            impl::typst::eval::FuncType for #ty {
                fn create_func(name: &'static str) -> ::typst::eval::Func {
                    ::typst::eval::Func::from_fn(#full, #info)
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

            impl #params ::typst::eval::FuncType for #ident #args #clause {
                fn create_func(name: &'static str) -> ::typst::eval::Func {
                    ::typst::eval::Func::from_node::<Self>(#info)
                }
            }
        })
    }
}

/// Extract a section.
fn section(docs: &mut String, title: &str, level: usize) -> Option<String> {
    let hashtags = "#".repeat(level);
    let needle = format!("\n{hashtags} {title}\n");
    let start = docs.find(&needle)?;
    let rest = &docs[start..];
    let len = rest[1..]
        .find("\n# ")
        .or_else(|| rest[1..].find("\n## "))
        .or_else(|| rest[1..].find("\n### "))
        .map(|x| 1 + x)
        .unwrap_or(rest.len());
    let end = start + len;
    let section = docs[start + needle.len()..end].trim().to_owned();
    docs.replace_range(start..end, "");
    Some(section)
}

/// Parse the parameter section.
fn params(docs: &mut String) -> Result<(Vec<TokenStream>, Vec<String>)> {
    let Some(section) = section(docs, "Parameters", 2) else {
        return Ok((vec![], vec![]));
    };

    let mut s = Scanner::new(&section);
    let mut infos = vec![];
    let mut returns = vec![];

    while s.eat_if('-') {
        let mut named = false;
        let mut positional = false;
        let mut required = false;
        let mut variadic = false;
        let mut settable = false;

        s.eat_whitespace();
        let name = s.eat_until(':');
        s.expect(": ");

        if name == "returns" {
            returns = s
                .eat_until('\n')
                .split(" or ")
                .map(str::trim)
                .map(Into::into)
                .collect();
            s.eat_whitespace();
            continue;
        }

        s.expect('`');
        let ty: syn::Type = syn::parse_str(s.eat_until('`'))?;
        s.expect('`');
        s.eat_whitespace();
        s.expect('(');

        for part in s.eat_until(')').split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match part {
                "named" => named = true,
                "positional" => positional = true,
                "required" => required = true,
                "variadic" => variadic = true,
                "settable" => settable = true,
                _ => bail!(callsite, "unknown parameter flag {:?}", part),
            }
        }

        if (!named && !positional) || (variadic && !positional) || (required && variadic)
        {
            bail!(callsite, "invalid combination of parameter flags");
        }

        s.expect(')');

        let docs = dedent(s.eat_until("\n-").trim());
        let docs = docs.trim();

        infos.push(quote! {
            ::typst::eval::ParamInfo {
                name: #name,
                docs: #docs,
                cast: <#ty as ::typst::eval::Cast<
                    ::typst::syntax::Spanned<::typst::eval::Value>
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

    Ok((infos, returns))
}
