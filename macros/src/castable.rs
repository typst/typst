use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::Token;

use super::*;

/// Expand the `castable!` macro.
pub fn castable(stream: TokenStream) -> Result<TokenStream> {
    let castable: Castable = syn::parse2(stream)?;
    let ty = &castable.ty;

    if castable.casts.is_empty() && castable.name.is_none() {
        bail!(castable.ty, "expected at least one pattern");
    }

    let is_func = create_is_func(&castable);
    let cast_func = create_cast_func(&castable);
    let describe_func = create_describe_func(&castable);
    let dynamic_impls = castable.name.as_ref().map(|name| {
        quote! {
            impl ::typst::model::Type for #ty {
                const TYPE_NAME: &'static str = #name;
            }

            impl From<#ty> for ::typst::model::Value {
                fn from(v: #ty) -> Self {
                    ::typst::model::Value::Dyn(::typst::model::Dynamic::new(v))
                }
            }
        }
    });

    Ok(quote! {
        impl ::typst::model::Cast for #ty {
            #is_func
            #cast_func
            #describe_func
        }

        #dynamic_impls
    })
}

/// Create the castable's `is` function.
fn create_is_func(castable: &Castable) -> TokenStream {
    let mut string_arms = vec![];
    let mut cast_checks = vec![];

    for cast in &castable.casts {
        match &cast.pattern {
            Pattern::Str(lit) => {
                string_arms.push(quote! { #lit => return true });
            }
            Pattern::Ty(_, ty) => {
                cast_checks.push(quote! {
                    if <#ty as ::typst::model::Cast>::is(value) {
                        return true;
                    }
                });
            }
        }
    }

    let dynamic_check = castable.name.is_some().then(|| {
        quote! {
            if let ::typst::model::Value::Dyn(dynamic) = &value {
                if dynamic.is::<Self>() {
                    return true;
                }
            }
        }
    });

    let str_check = (!string_arms.is_empty()).then(|| {
        quote! {
            if let ::typst::model::Value::Str(string) = &value {
                match string.as_str() {
                    #(#string_arms,)*
                    _ => {}
                }
            }
        }
    });

    quote! {
        fn is(value: &typst::model::Value) -> bool {
            #dynamic_check
            #str_check
            #(#cast_checks)*
            false
        }
    }
}

/// Create the castable's `cast` function.
fn create_cast_func(castable: &Castable) -> TokenStream {
    let mut string_arms = vec![];
    let mut cast_checks = vec![];

    for cast in &castable.casts {
        let expr = &cast.expr;
        match &cast.pattern {
            Pattern::Str(lit) => {
                string_arms.push(quote! { #lit => return Ok(#expr) });
            }
            Pattern::Ty(binding, ty) => {
                cast_checks.push(quote! {
                    if <#ty as ::typst::model::Cast>::is(&value) {
                        let #binding = <#ty as ::typst::model::Cast>::cast(value)?;
                        return Ok(#expr);
                    }
                });
            }
        }
    }

    let dynamic_check = castable.name.is_some().then(|| {
        quote! {
            if let ::typst::model::Value::Dyn(dynamic) = &value {
                if let Some(concrete) = dynamic.downcast::<Self>() {
                    return Ok(concrete.clone());
                }
            }
        }
    });

    let str_check = (!string_arms.is_empty()).then(|| {
        quote! {
            if let ::typst::model::Value::Str(string) = &value {
                match string.as_str() {
                    #(#string_arms,)*
                    _ => {}
                }
            }
        }
    });

    quote! {
        fn cast(value: ::typst::model::Value) -> ::typst::diag::StrResult<Self> {
            #dynamic_check
            #str_check
            #(#cast_checks)*
            <Self as ::typst::model::Cast>::error(value)
        }
    }
}

/// Create the castable's `describe` function.
fn create_describe_func(castable: &Castable) -> TokenStream {
    let mut infos = vec![];

    for cast in &castable.casts {
        let docs = documentation(&cast.attrs);
        infos.push(match &cast.pattern {
            Pattern::Str(lit) => {
                quote! { ::typst::model::CastInfo::Value(#lit.into(), #docs) }
            }
            Pattern::Ty(_, ty) => {
                quote! { <#ty as ::typst::model::Cast>::describe() }
            }
        });
    }

    if let Some(name) = &castable.name {
        infos.push(quote! {
            CastInfo::Type(#name)
        });
    }

    quote! {
        fn describe() -> ::typst::model::CastInfo {
            #(#infos)+*
        }
    }
}

struct Castable {
    ty: syn::Type,
    name: Option<syn::LitStr>,
    casts: Punctuated<Cast, Token![,]>,
}

impl Parse for Castable {
    fn parse(input: ParseStream) -> Result<Self> {
        let ty = input.parse()?;
        let mut name = None;
        if input.peek(Token![:]) {
            let _: syn::Token![:] = input.parse()?;
            name = Some(input.parse()?);
        }
        let _: syn::Token![,] = input.parse()?;
        let casts = Punctuated::parse_terminated(input)?;
        Ok(Self { ty, name, casts })
    }
}

struct Cast {
    attrs: Vec<syn::Attribute>,
    pattern: Pattern,
    expr: syn::Expr,
}

impl Parse for Cast {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let pattern = input.parse()?;
        let _: syn::Token![=>] = input.parse()?;
        let expr = input.parse()?;
        Ok(Self { attrs, pattern, expr })
    }
}

enum Pattern {
    Str(syn::LitStr),
    Ty(syn::Pat, syn::Type),
}

impl Parse for Pattern {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::LitStr) {
            Ok(Pattern::Str(input.parse()?))
        } else {
            let pat = input.parse()?;
            let _: syn::Token![:] = input.parse()?;
            let ty = input.parse()?;
            Ok(Pattern::Ty(pat, ty))
        }
    }
}
