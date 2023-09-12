use super::*;

use heck::ToKebabCase;

/// Expand the `#[func]` macro.
pub fn func(stream: TokenStream, item: &syn::ItemFn) -> Result<TokenStream> {
    let func = parse(stream, item)?;
    Ok(create(&func, item))
}

/// Details about a function.
struct Func {
    name: String,
    title: String,
    scope: bool,
    constructor: bool,
    keywords: Vec<String>,
    parent: Option<syn::Type>,
    docs: String,
    vis: syn::Visibility,
    ident: Ident,
    special: SpecialParams,
    params: Vec<Param>,
    returns: syn::Type,
}

/// Special parameters provided by the runtime.
#[derive(Default)]
struct SpecialParams {
    self_: Option<Param>,
    vm: bool,
    vt: bool,
    args: bool,
    span: bool,
}

/// Details about a function parameter.
struct Param {
    binding: Binding,
    ident: Ident,
    ty: syn::Type,
    name: String,
    docs: String,
    named: bool,
    variadic: bool,
    external: bool,
    default: Option<syn::Expr>,
}

/// How a parameter is bound.
enum Binding {
    /// Normal parameter.
    Owned,
    /// `&self`.
    Ref,
    /// `&mut self`.
    RefMut,
}

/// The `..` in `#[func(..)]`.
pub struct Meta {
    pub scope: bool,
    pub name: Option<String>,
    pub title: Option<String>,
    pub constructor: bool,
    pub keywords: Vec<String>,
    pub parent: Option<syn::Type>,
}

impl Parse for Meta {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            scope: parse_flag::<kw::scope>(input)?,
            name: parse_string::<kw::name>(input)?,
            title: parse_string::<kw::title>(input)?,
            constructor: parse_flag::<kw::constructor>(input)?,
            keywords: parse_string_array::<kw::keywords>(input)?,
            parent: parse_key_value::<kw::parent, _>(input)?,
        })
    }
}

/// Parse details about the function from the fn item.
fn parse(stream: TokenStream, item: &syn::ItemFn) -> Result<Func> {
    let meta: Meta = syn::parse2(stream)?;
    let (name, title) =
        determine_name_and_title(meta.name, meta.title, &item.sig.ident, None)?;

    let docs = documentation(&item.attrs);

    let mut special = SpecialParams::default();
    let mut params = vec![];
    for input in &item.sig.inputs {
        parse_param(&mut special, &mut params, meta.parent.as_ref(), input)?;
    }

    let returns = match &item.sig.output {
        syn::ReturnType::Default => parse_quote! { () },
        syn::ReturnType::Type(_, ty) => ty.as_ref().clone(),
    };

    if meta.parent.is_some() && meta.scope {
        bail!(item, "scoped function cannot have a scope");
    }

    Ok(Func {
        name,
        title,
        scope: meta.scope,
        constructor: meta.constructor,
        keywords: meta.keywords,
        parent: meta.parent,
        docs,
        vis: item.vis.clone(),
        ident: item.sig.ident.clone(),
        special,
        params,
        returns,
    })
}

/// Parse details about a functino parameter.
fn parse_param(
    special: &mut SpecialParams,
    params: &mut Vec<Param>,
    parent: Option<&syn::Type>,
    input: &syn::FnArg,
) -> Result<()> {
    let typed = match input {
        syn::FnArg::Receiver(recv) => {
            let mut binding = Binding::Owned;
            if recv.reference.is_some() {
                if recv.mutability.is_some() {
                    binding = Binding::RefMut
                } else {
                    binding = Binding::Ref
                }
            };

            special.self_ = Some(Param {
                binding,
                ident: syn::Ident::new("self_", recv.self_token.span),
                ty: match parent {
                    Some(ty) => ty.clone(),
                    None => bail!(recv, "explicit parent type required"),
                },
                name: "self".into(),
                docs: documentation(&recv.attrs),
                named: false,
                variadic: false,
                external: false,
                default: None,
            });
            return Ok(());
        }
        syn::FnArg::Typed(typed) => typed,
    };

    let syn::Pat::Ident(syn::PatIdent { by_ref: None, mutability: None, ident, .. }) =
        &*typed.pat
    else {
        bail!(typed.pat, "expected identifier");
    };

    match ident.to_string().as_str() {
        "vm" => special.vm = true,
        "vt" => special.vt = true,
        "args" => special.args = true,
        "span" => special.span = true,
        _ => {
            let mut attrs = typed.attrs.clone();
            params.push(Param {
                binding: Binding::Owned,
                ident: ident.clone(),
                ty: (*typed.ty).clone(),
                name: ident.to_string().to_kebab_case(),
                docs: documentation(&attrs),
                named: has_attr(&mut attrs, "named"),
                variadic: has_attr(&mut attrs, "variadic"),
                external: has_attr(&mut attrs, "external"),
                default: parse_attr(&mut attrs, "default")?.map(|expr| {
                    expr.unwrap_or_else(
                        || parse_quote! { ::std::default::Default::default() },
                    )
                }),
            });
            validate_attrs(&attrs)?;
        }
    }

    Ok(())
}

/// Produce the function's definition.
fn create(func: &Func, item: &syn::ItemFn) -> TokenStream {
    let eval = quote! { ::typst::eval };

    let Func { docs, vis, ident, .. } = func;
    let item = rewrite_fn_item(item);
    let ty = create_func_ty(func);
    let data = create_func_data(func);

    let creator = if ty.is_some() {
        quote! {
            impl #eval::NativeFunc for #ident {
                fn data() -> &'static #eval::NativeFuncData {
                    static DATA: #eval::NativeFuncData = #data;
                    &DATA
                }
            }
        }
    } else {
        let ident_data = quote::format_ident!("{ident}_data");
        quote! {
            #[doc(hidden)]
            #vis fn #ident_data() -> &'static #eval::NativeFuncData {
                static DATA: #eval::NativeFuncData = #data;
                &DATA
            }
        }
    };

    quote! {
        #[doc = #docs]
        #[allow(dead_code)]
        #item

        #[doc(hidden)]
        #ty
        #creator
    }
}

/// Create native function data for the function.
fn create_func_data(func: &Func) -> TokenStream {
    let eval = quote! { ::typst::eval };

    let Func {
        ident,
        name,
        title,
        docs,
        keywords,
        returns,
        scope,
        parent,
        constructor,
        ..
    } = func;

    let scope = if *scope {
        quote! { <#ident as #eval::NativeScope>::scope() }
    } else {
        quote! { #eval::Scope::new() }
    };

    let closure = create_wrapper_closure(func);
    let params = func.special.self_.iter().chain(&func.params).map(create_param_info);

    let name = if *constructor {
        quote! { <#parent as #eval::NativeType>::NAME }
    } else {
        quote! { #name }
    };

    quote! {
        #eval::NativeFuncData {
            function: #closure,
            name: #name,
            title: #title,
            docs: #docs,
            keywords: &[#(#keywords),*],
            scope: #eval::Lazy::new(|| #scope),
            params: #eval::Lazy::new(|| ::std::vec![#(#params),*]),
            returns:  #eval::Lazy::new(|| <#returns as #eval::Reflect>::output()),
        }
    }
}

/// Create a type that shadows the function.
fn create_func_ty(func: &Func) -> Option<TokenStream> {
    if func.parent.is_some() {
        return None;
    }

    let Func { vis, ident, .. } = func;
    Some(quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #vis enum #ident {}
    })
}

/// Create the runtime-compatible wrapper closure that parses arguments.
fn create_wrapper_closure(func: &Func) -> TokenStream {
    // These handlers parse the arguments.
    let handlers = {
        let func_handlers = func
            .params
            .iter()
            .filter(|param| !param.external)
            .map(create_param_parser);
        let self_handler = func.special.self_.as_ref().map(create_param_parser);
        quote! {
            #self_handler
            #(#func_handlers)*
        }
    };

    // This is the actual function call.
    let call = {
        let self_ = func
            .special
            .self_
            .as_ref()
            .map(bind)
            .map(|tokens| quote! { #tokens, });
        let vm_ = func.special.vm.then(|| quote! { vm, });
        let vt_ = func.special.vt.then(|| quote! { &mut vm.vt, });
        let args_ = func.special.args.then(|| quote! { args.take(), });
        let span_ = func.special.span.then(|| quote! { args.span, });
        let forwarded = func.params.iter().filter(|param| !param.external).map(bind);
        quote! {
            __typst_func(#self_ #vm_ #vt_ #args_ #span_ #(#forwarded,)*)
        }
    };

    // This is the whole wrapped closure.
    let ident = &func.ident;
    let parent = func.parent.as_ref().map(|ty| quote! { #ty:: });
    quote! {
        |vm, args| {
            let __typst_func = #parent #ident;
            #handlers
            let output = #call;
            ::typst::eval::IntoResult::into_result(output, args.span)
        }
    }
}

/// Create a parameter info for a field.
fn create_param_info(param: &Param) -> TokenStream {
    let Param { name, docs, named, variadic, ty, default, .. } = param;
    let positional = !named;
    let required = !named && default.is_none();
    let ty = if *variadic || (*named && default.is_none()) {
        quote! { <#ty as ::typst::eval::Container>::Inner }
    } else {
        quote! { #ty }
    };
    let default = quote_option(&default.as_ref().map(|_default| {
        quote! {
            || {
                let typed: #ty = #default;
                ::typst::eval::IntoValue::into_value(typed)
            }
        }
    }));
    quote! {
        ::typst::eval::ParamInfo {
            name: #name,
            docs: #docs,
            input: <#ty as ::typst::eval::Reflect>::input(),
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

/// Apply the binding to a parameter.
fn bind(param: &Param) -> TokenStream {
    let ident = &param.ident;
    match param.binding {
        Binding::Owned => quote! { #ident },
        Binding::Ref => quote! { &#ident },
        Binding::RefMut => quote! { &mut #ident },
    }
}

/// Removes attributes and so on from the native function.
fn rewrite_fn_item(item: &syn::ItemFn) -> syn::ItemFn {
    let inputs = item.sig.inputs.iter().cloned().filter_map(|mut input| {
        if let syn::FnArg::Typed(typed) = &mut input {
            if typed.attrs.iter().any(|attr| attr.path().is_ident("external")) {
                return None;
            }
            typed.attrs.clear();
        }
        Some(input)
    });
    let mut item = item.clone();
    item.attrs.clear();
    item.sig.inputs = parse_quote! { #(#inputs),* };
    item
}
