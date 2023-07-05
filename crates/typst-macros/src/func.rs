use super::*;

/// Expand the `#[func]` macro.
pub fn func(stream: TokenStream, item: &syn::ItemFn) -> Result<TokenStream> {
    let func = prepare(stream, item)?;
    Ok(create(&func, item))
}

struct Func {
    name: String,
    display: String,
    category: String,
    keywords: Option<String>,
    docs: String,
    vis: syn::Visibility,
    ident: Ident,
    ident_func: Ident,
    parent: Option<syn::Type>,
    vm: bool,
    vt: bool,
    args: bool,
    span: bool,
    params: Vec<Param>,
    returns: syn::Type,
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

fn prepare(stream: TokenStream, item: &syn::ItemFn) -> Result<Func> {
    let sig = &item.sig;

    let Parent(parent) = syn::parse2(stream)?;

    let mut vm = false;
    let mut vt = false;
    let mut args = false;
    let mut span = false;
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

        match ident.to_string().as_str() {
            "vm" => vm = true,
            "vt" => vt = true,
            "args" => args = true,
            "span" => span = true,
            _ => {
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
        }
    }

    let mut attrs = item.attrs.clone();
    let docs = documentation(&attrs);
    let mut lines = docs.split('\n').collect();
    let keywords = meta_line(&mut lines, "Keywords").ok().map(Into::into);
    let category = meta_line(&mut lines, "Category")?.into();
    let display = meta_line(&mut lines, "Display")?.into();
    let docs = lines.join("\n").trim().into();

    let func = Func {
        name: sig.ident.to_string().trim_end_matches('_').replace('_', "-"),
        display,
        category,
        keywords,
        docs,
        vis: item.vis.clone(),
        ident: sig.ident.clone(),
        ident_func: Ident::new(
            &format!("{}_func", sig.ident.to_string().trim_end_matches('_')),
            sig.ident.span(),
        ),
        parent,
        params,
        returns: match &sig.output {
            syn::ReturnType::Default => parse_quote! { () },
            syn::ReturnType::Type(_, ty) => ty.as_ref().clone(),
        },
        scope: parse_attr(&mut attrs, "scope")?.flatten(),
        vm,
        vt,
        args,
        span,
    };

    Ok(func)
}

fn create(func: &Func, item: &syn::ItemFn) -> TokenStream {
    let Func {
        name,
        display,
        category,
        docs,
        vis,
        ident,
        ident_func,
        returns,
        ..
    } = func;

    let handlers = func
        .params
        .iter()
        .filter(|param| !param.external)
        .map(create_param_parser);

    let args = func
        .params
        .iter()
        .filter(|param| !param.external)
        .map(|param| &param.ident);

    let parent = func.parent.as_ref().map(|ty| quote! { #ty:: });
    let vm_ = func.vm.then(|| quote! { vm, });
    let vt_ = func.vt.then(|| quote! { &mut vm.vt, });
    let args_ = func.args.then(|| quote! { args.take(), });
    let span_ = func.span.then(|| quote! { args.span, });
    let wrapper = quote! {
        |vm, args| {
            let __typst_func = #parent #ident;
            #(#handlers)*
            let output = __typst_func(#(#args,)* #vm_ #vt_ #args_ #span_);
            ::typst::eval::IntoResult::into_result(output, args.span)
        }
    };

    let mut item = item.clone();
    item.attrs.clear();

    let inputs = item.sig.inputs.iter().cloned().filter_map(|mut input| {
        if let syn::FnArg::Typed(typed) = &mut input {
            if typed.attrs.iter().any(|attr| attr.path().is_ident("external")) {
                return None;
            }
            typed.attrs.clear();
        }
        Some(input)
    });

    item.sig.inputs = parse_quote! { #(#inputs),* };

    let keywords = quote_option(&func.keywords);
    let params = func.params.iter().map(create_param_info);
    let scope = create_scope_builder(func.scope.as_ref());

    quote! {
        #[doc(hidden)]
        #vis fn #ident_func() -> &'static ::typst::eval::NativeFunc {
            static FUNC: ::typst::eval::NativeFunc = ::typst::eval::NativeFunc {
                func: #wrapper,
                info: ::typst::eval::Lazy::new(|| typst::eval::FuncInfo {
                    name: #name,
                    display: #display,
                    keywords: #keywords,
                    category: #category,
                    docs: #docs,
                    params: ::std::vec![#(#params),*],
                    returns: <#returns as ::typst::eval::Reflect>::describe(),
                    scope: #scope,
                }),
            };
            &FUNC
        }

        #[doc = #docs]
        #item
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
                ::typst::eval::IntoValue::into_value(typed)
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
            cast: <#ty as ::typst::eval::Reflect>::describe(),
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

    quote! { let mut #ident: #ty = #value; }
}

struct Parent(Option<syn::Type>);

impl Parse for Parent {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(if !input.is_empty() { Some(input.parse()?) } else { None }))
    }
}
