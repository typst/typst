use super::*;

/// Expand the `#[func]` macro.
pub fn func(mut item: syn::Item) -> Result<TokenStream> {
    let attrs = match &mut item {
        syn::Item::Struct(item) => &mut item.attrs,
        syn::Item::Fn(item) => &mut item.attrs,
        _ => bail!(item, "expected struct or fn"),
    };

    let docs = documentation(&attrs);

    let mut lines: Vec<_> = docs.lines().collect();
    let Some(category) = lines.pop().and_then(|s| s.strip_prefix("Category: ")) else {
        bail!(item, "expected category");
    };
    let Some(display) = lines.pop().and_then(|s| s.strip_prefix("Display: ")) else {
        bail!(item, "expected display name");
    };

    let mut docs = lines.join("\n");
    let (params, returns) = params(&mut docs)?;
    let docs = docs.trim();
    attrs.retain(|attr| !attr.path.is_ident("doc"));
    attrs.push(parse_quote! { #[doc = #docs] });

    let info = quote! {
        ::typst::eval::FuncInfo {
            name,
            display: #display,
            category: #category,
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
                "positional" => positional = true,
                "named" => named = true,
                "required" => required = true,
                "variadic" => variadic = true,
                "settable" => settable = true,
                _ => bail!(callsite, "unknown parameter flag {:?}", part),
            }
        }

        if (!named && !positional) || (variadic && !positional) {
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
                positional: #positional,
                named: #named,
                variadic: #variadic,
                required: #required,
                settable: #settable,
            }
        });

        s.eat_whitespace();
    }

    Ok((infos, returns))
}
