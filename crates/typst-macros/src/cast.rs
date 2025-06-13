use heck::ToKebabCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{DeriveInput, Ident, Result, Token};

use crate::util::{documentation, foundations};

/// Expand the `#[derive(Cast)]` macro.
pub fn derive_cast(item: DeriveInput) -> Result<TokenStream> {
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
            variant.attrs.iter().find(|attr| attr.path().is_ident("string"))
        {
            attr.parse_args::<syn::LitStr>()?.value()
        } else {
            variant.ident.to_string().to_kebab_case()
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
        #foundations::cast! {
            #ty,
            self => #foundations::IntoValue::into_value(match self {
                #(#variants_to_strs),*
            }),
            #(#strs_to_variants),*
        }
    })
}

/// An enum variant in a `derive(Cast)`.
struct Variant {
    ident: Ident,
    string: String,
    docs: String,
}

/// Expand the `cast!` macro.
pub fn cast(stream: TokenStream) -> Result<TokenStream> {
    let input: CastInput = syn::parse2(stream)?;
    let ty = &input.ty;
    let castable_body = create_castable_body(&input);
    let input_body = create_input_body(&input);
    let output_body = create_output_body(&input);
    let into_value_body = create_into_value_body(&input);
    let from_value_body = create_from_value_body(&input);

    let reflect = (!input.from_value.is_empty() || input.dynamic).then(|| {
        quote! {
            impl #foundations::Reflect for #ty {
                fn input() -> #foundations::CastInfo {
                    #input_body
                }

                fn output() -> #foundations::CastInfo {
                    #output_body
                }

                fn castable(value: &#foundations::Value) -> bool {
                    #castable_body
                }
            }
        }
    });

    let into_value = (input.into_value.is_some() || input.dynamic).then(|| {
        quote! {
            impl #foundations::IntoValue for #ty {
                fn into_value(self) -> #foundations::Value {
                    #into_value_body
                }
            }
        }
    });

    let from_value = (!input.from_value.is_empty() || input.dynamic).then(|| {
        quote! {
            impl #foundations::FromValue for #ty {
                fn from_value(value: #foundations::Value) -> ::typst_library::diag::HintedStrResult<Self> {
                    #from_value_body
                }
            }
        }
    });

    Ok(quote! {
        #reflect
        #into_value
        #from_value
    })
}

/// The input to `cast!`.
struct CastInput {
    ty: syn::Type,
    dynamic: bool,
    into_value: Option<syn::Expr>,
    from_value: Punctuated<Cast, Token![,]>,
}

impl Parse for CastInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut dynamic = false;
        if input.peek(syn::Token![type]) {
            let _: syn::Token![type] = input.parse()?;
            dynamic = true;
        }

        let ty = input.parse()?;
        let _: syn::Token![,] = input.parse()?;

        let mut to_value = None;
        if input.peek(syn::Token![self]) {
            let _: syn::Token![self] = input.parse()?;
            let _: syn::Token![=>] = input.parse()?;
            to_value = Some(input.parse()?);
            let _: syn::Token![,] = input.parse()?;
        }

        let from_value = Punctuated::parse_terminated(input)?;
        Ok(Self { ty, dynamic, into_value: to_value, from_value })
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
            let pat = syn::Pat::parse_single(input)?;
            let _: syn::Token![:] = input.parse()?;
            let ty = input.parse()?;
            Ok(Pattern::Ty(pat, ty))
        }
    }
}

/// A single cast, e.g. `v: i64 => Self::Int(v)`.
struct Cast {
    attrs: Vec<syn::Attribute>,
    pattern: Pattern,
    expr: syn::Expr,
}

/// A pattern in a cast, e.g.`"ascender"` or `v: i64`.
#[allow(clippy::large_enum_variant)]
enum Pattern {
    Str(syn::LitStr),
    Ty(syn::Pat, syn::Type),
}

fn create_castable_body(input: &CastInput) -> TokenStream {
    let mut strings = vec![];
    let mut casts = vec![];

    for cast in &input.from_value {
        match &cast.pattern {
            Pattern::Str(lit) => {
                strings.push(quote! { #lit => return true });
            }
            Pattern::Ty(_, ty) => {
                casts.push(quote! {
                    if <#ty as #foundations::Reflect>::castable(value) {
                        return true;
                    }
                });
            }
        }
    }

    let dynamic_check = input.dynamic.then(|| {
        quote! {
            if let #foundations::Value::Dyn(dynamic) = &value {
                if dynamic.is::<Self>() {
                    return true;
                }
            }
        }
    });

    let str_check = (!strings.is_empty()).then(|| {
        quote! {
            if let #foundations::Value::Str(string) = &value {
                match string.as_str() {
                    #(#strings,)*
                    _ => {}
                }
            }
        }
    });

    quote! {
        #dynamic_check
        #str_check
        #(#casts)*
        false
    }
}

fn create_input_body(input: &CastInput) -> TokenStream {
    let mut infos = vec![];

    for cast in &input.from_value {
        let docs = documentation(&cast.attrs);
        infos.push(match &cast.pattern {
            Pattern::Str(lit) => {
                quote! {
                    #foundations::CastInfo::Value(
                        #foundations::IntoValue::into_value(#lit),
                        #docs,
                    )
                }
            }
            Pattern::Ty(_, ty) => {
                quote! { <#ty as #foundations::Reflect>::input() }
            }
        });
    }

    if input.dynamic {
        infos.push(quote! {
            #foundations::CastInfo::Type(#foundations::Type::of::<Self>())
        });
    }

    quote! {
        #(#infos)+*
    }
}

fn create_output_body(input: &CastInput) -> TokenStream {
    if input.dynamic {
        quote! { #foundations::CastInfo::Type(#foundations::Type::of::<Self>()) }
    } else {
        quote! { <Self as #foundations::Reflect>::input() }
    }
}

fn create_into_value_body(input: &CastInput) -> TokenStream {
    if let Some(expr) = &input.into_value {
        quote! { #expr }
    } else {
        quote! { #foundations::Value::dynamic(self) }
    }
}

fn create_from_value_body(input: &CastInput) -> TokenStream {
    let mut string_arms = vec![];
    let mut cast_checks = vec![];

    for cast in &input.from_value {
        let expr = &cast.expr;
        match &cast.pattern {
            Pattern::Str(lit) => {
                string_arms.push(quote! { #lit => return Ok(#expr) });
            }
            Pattern::Ty(binding, ty) => {
                cast_checks.push(quote! {
                    if <#ty as #foundations::Reflect>::castable(&value) {
                        let #binding = <#ty as #foundations::FromValue>::from_value(value)?;
                        return Ok(#expr);
                    }
                });
            }
        }
    }

    let dynamic_check = input.dynamic.then(|| {
        quote! {
            if let #foundations::Value::Dyn(dynamic) = &value {
                if let Some(concrete) = dynamic.downcast::<Self>() {
                    return Ok(concrete.clone());
                }
            }
        }
    });

    let str_check = (!string_arms.is_empty()).then(|| {
        quote! {
            if let #foundations::Value::Str(string) = &value {
                match string.as_str() {
                    #(#string_arms,)*
                    _ => {}
                }
            }
        }
    });

    quote! {
        #dynamic_check
        #str_check
        #(#cast_checks)*
        Err(<Self as #foundations::Reflect>::error(&value))
    }
}
