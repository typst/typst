use quote::ToTokens;

use super::*;

/// Expand the `#[func]` macro.
pub fn func(item: syn::ItemFn) -> Result<TokenStream> {
    let func = prepare(&item)?;
    Ok(create(&func))
}

struct Func {
    name: String,
    display: String,
    category: String,
    keywords: Option<String>,
    docs: String,
    vis: syn::Visibility,
    ident: Ident,
    params: Vec<Param>,
    returns: Vec<String>,
    body: syn::Block,
    scope: Option<BlockWithReturn>,
}

struct Param {
    name: String,
    docs: String,
    external: bool,
    named: bool,
    variadic: bool,
    default: Option<syn::Expr>,
    ident: Ident,
    ty: syn::Type,
}

fn prepare(item: &syn::ItemFn) -> Result<Func> {
    let sig = &item.sig;

    let mut params = vec![];
    for input in &sig.inputs {
        let syn::FnArg::Typed(typed) = input else {
            bail!(input, "self is not allowed here");
        };

        let syn::Pat::Ident(syn::PatIdent {
            by_ref: None,
            mutability: None,
            ident,
            ..
        }) = &*typed.pat else {
            bail!(typed.pat, "expected identifier");
        };

        if sig.output.to_token_stream().to_string() != "-> Value" {
            bail!(sig.output, "must return `Value`");
        }

        let mut attrs = typed.attrs.clone();
        params.push(Param {
            name: kebab_case(ident),
            docs: documentation(&attrs),
            external: has_attr(&mut attrs, "external"),
            named: has_attr(&mut attrs, "named"),
            variadic: has_attr(&mut attrs, "variadic"),
            default: parse_attr(&mut attrs, "default")?.map(|expr| {
                expr.unwrap_or_else(
                    || parse_quote! { ::std::default::Default::default() },
                )
            }),
            ident: ident.clone(),
            ty: (*typed.ty).clone(),
        });

        validate_attrs(&attrs)?;
    }

    let mut attrs = item.attrs.clone();
    let docs = documentation(&attrs);
    let mut lines = docs.split('\n').collect();
    let returns = meta_line(&mut lines, "Returns")?
        .split(" or ")
        .map(Into::into)
        .collect();
    let keywords = meta_line(&mut lines, "Keywords").ok().map(Into::into);
    let category = meta_line(&mut lines, "Category")?.into();
    let display = meta_line(&mut lines, "Display")?.into();
    let docs = lines.join("\n").trim().into();

    let func = Func {
        name: sig.ident.to_string().replace('_', ""),
        display,
        category,
        keywords,
        docs,
        vis: item.vis.clone(),
        ident: sig.ident.clone(),
        params,
        returns,
        body: (*item.block).clone(),
        scope: parse_attr(&mut attrs, "scope")?.flatten(),
    };

    validate_attrs(&attrs)?;
    Ok(func)
}

fn create(func: &Func) -> TokenStream {
    let Func {
        name,
        display,
        keywords,
        category,
        docs,
        vis,
        ident,
        params,
        returns,
        body,
        ..
    } = func;
    let handlers = params.iter().filter(|param| !param.external).map(create_param_parser);
    let params = params.iter().map(create_param_info);
    let scope = create_scope_builder(func.scope.as_ref());
    let keywords = quote_option(keywords);
    quote! {
        #[doc = #docs]
        #vis fn #ident() -> &'static ::typst::eval::NativeFunc {
            static FUNC: ::typst::eval::NativeFunc = ::typst::eval::NativeFunc {
                func: |vm, args| {
                    #(#handlers)*
                    #[allow(unreachable_code)]
                    Ok(#body)
                },
                info: ::typst::eval::Lazy::new(|| typst::eval::FuncInfo {
                    name: #name,
                    display: #display,
                    keywords: #keywords,
                    docs: #docs,
                    params: ::std::vec![#(#params),*],
                    returns: ::std::vec![#(#returns),*],
                    category: #category,
                    scope: #scope,
                }),
            };
            &FUNC
        }
    }
}

/// Create a parameter info for a field.
fn create_param_info(param: &Param) -> TokenStream {
    let Param { name, docs, named, variadic, ty, default, .. } = param;
    let positional = !named;
    let required = default.is_none();
    let default = quote_option(&default.as_ref().map(|_default| {
        quote! {
            || {
                let typed: #ty = #default;
                ::typst::eval::Value::from(typed)
            }
        }
    }));
    let ty = if *variadic {
        quote! { <#ty as ::typst::eval::Variadics>::Inner }
    } else {
        quote! { #ty }
    };
    quote! {
        ::typst::eval::ParamInfo {
            name: #name,
            docs: #docs,
            cast: <#ty as ::typst::eval::Cast<
                ::typst::syntax::Spanned<::typst::eval::Value>
            >>::describe(),
            default: #default,
            positional: #positional,
            named: #named,
            variadic: #variadic,
            required: #required,
            settable: false,
        }
    }
}

/// Create argument parsing code for a parameter.
fn create_param_parser(param: &Param) -> TokenStream {
    let Param { name, ident, ty, .. } = param;

    let mut value = if param.variadic {
        quote! { args.all()? }
    } else if param.named {
        quote! { args.named(#name)? }
    } else if param.default.is_some() {
        quote! { args.eat()? }
    } else {
        quote! { args.expect(#name)? }
    };

    if let Some(default) = &param.default {
        value = quote! { #value.unwrap_or_else(|| #default) }
    }

    quote! { let #ident: #ty = #value; }
}
