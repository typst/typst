use super::*;

/// Expand the `#[derive(Cast)]` macro.
pub fn cast(item: DeriveInput) -> Result<TokenStream> {
    let ty = &item.ident;

    let syn::Data::Enum(data) = &item.data else {
        bail!(item, "only enums are supported");
    };

    let mut variants = vec![];
    for variant in &data.variants {
        if let Some((_, expr)) = &variant.discriminant {
            bail!(expr, "explicit discriminant is not allowed");
        }

        let string = if let Some(attr) =
            variant.attrs.iter().find(|attr| attr.path.is_ident("string"))
        {
            attr.parse_args::<syn::LitStr>()?.value()
        } else {
            kebab_case(&variant.ident)
        };

        variants.push(Variant {
            ident: variant.ident.clone(),
            string,
            docs: documentation(&variant.attrs),
        });
    }

    let strs_to_variants = variants.iter().map(|Variant { ident, string, docs }| {
        quote! {
            #[doc = #docs]
            #string => Self::#ident
        }
    });

    let variants_to_strs = variants.iter().map(|Variant { ident, string, .. }| {
        quote! {
            #ty::#ident => #string
        }
    });

    Ok(quote! {
        ::typst::eval::cast_from_value! {
            #ty,
            #(#strs_to_variants),*
        }

        ::typst::eval::cast_to_value! {
            v: #ty => ::typst::eval::Value::from(match v {
                #(#variants_to_strs),*
            })
        }
    })
}

struct Variant {
    ident: Ident,
    string: String,
    docs: String,
}

/// Expand the `cast_from_value!` macro.
pub fn cast_from_value(stream: TokenStream) -> Result<TokenStream> {
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
            impl ::typst::eval::Type for #ty {
                const TYPE_NAME: &'static str = #name;
            }

            impl From<#ty> for ::typst::eval::Value {
                fn from(v: #ty) -> Self {
                    ::typst::eval::Value::Dyn(::typst::eval::Dynamic::new(v))
                }
            }
        }
    });

    Ok(quote! {
        impl ::typst::eval::Cast for #ty {
            #is_func
            #cast_func
            #describe_func
        }

        #dynamic_impls
    })
}

/// Expand the `cast_to_value!` macro.
pub fn cast_to_value(stream: TokenStream) -> Result<TokenStream> {
    let cast: Cast = syn::parse2(stream)?;
    let Pattern::Ty(pat, ty) = &cast.pattern else {
        bail!(callsite, "expected pattern");
    };

    let expr = &cast.expr;
    Ok(quote! {
        impl ::std::convert::From<#ty> for ::typst::eval::Value {
            fn from(#pat: #ty) -> Self {
                #expr
            }
        }
    })
}

struct Castable {
    ty: syn::Type,
    name: Option<syn::LitStr>,
    casts: Punctuated<Cast, Token![,]>,
}

struct Cast {
    attrs: Vec<syn::Attribute>,
    pattern: Pattern,
    expr: syn::Expr,
}

enum Pattern {
    Str(syn::LitStr),
    Ty(syn::Pat, syn::Type),
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

impl Parse for Cast {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let pattern = input.parse()?;
        let _: syn::Token![=>] = input.parse()?;
        let expr = input.parse()?;
        Ok(Self { attrs, pattern, expr })
    }
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
                    if <#ty as ::typst::eval::Cast>::is(value) {
                        return true;
                    }
                });
            }
        }
    }

    let dynamic_check = castable.name.is_some().then(|| {
        quote! {
            if let ::typst::eval::Value::Dyn(dynamic) = &value {
                if dynamic.is::<Self>() {
                    return true;
                }
            }
        }
    });

    let str_check = (!string_arms.is_empty()).then(|| {
        quote! {
            if let ::typst::eval::Value::Str(string) = &value {
                match string.as_str() {
                    #(#string_arms,)*
                    _ => {}
                }
            }
        }
    });

    quote! {
        fn is(value: &::typst::eval::Value) -> bool {
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
                    if <#ty as ::typst::eval::Cast>::is(&value) {
                        let #binding = <#ty as ::typst::eval::Cast>::cast(value)?;
                        return Ok(#expr);
                    }
                });
            }
        }
    }

    let dynamic_check = castable.name.is_some().then(|| {
        quote! {
            if let ::typst::eval::Value::Dyn(dynamic) = &value {
                if let Some(concrete) = dynamic.downcast::<Self>() {
                    return Ok(concrete.clone());
                }
            }
        }
    });

    let str_check = (!string_arms.is_empty()).then(|| {
        quote! {
            if let ::typst::eval::Value::Str(string) = &value {
                match string.as_str() {
                    #(#string_arms,)*
                    _ => {}
                }
            }
        }
    });

    quote! {
        fn cast(value: ::typst::eval::Value) -> ::typst::diag::StrResult<Self> {
            #dynamic_check
            #str_check
            #(#cast_checks)*
            <Self as ::typst::eval::Cast>::error(value)
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
                quote! { ::typst::eval::CastInfo::Value(#lit.into(), #docs) }
            }
            Pattern::Ty(_, ty) => {
                quote! { <#ty as ::typst::eval::Cast>::describe() }
            }
        });
    }

    if let Some(name) = &castable.name {
        infos.push(quote! {
            ::typst::eval::CastInfo::Type(#name)
        });
    }

    quote! {
        fn describe() -> ::typst::eval::CastInfo {
            #(#infos)+*
        }
    }
}
