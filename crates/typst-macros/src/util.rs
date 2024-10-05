use heck::{ToKebabCase, ToTitleCase};
use quote::ToTokens;
use syn::token::Token;
use syn::Attribute;

use super::*;

/// Return an error at the given item.
macro_rules! bail {
    (callsite, $($tts:tt)*) => {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("typst: {}", format!($($tts)*))
        ))
    };
    ($item:expr, $($tts:tt)*) => {
        return Err(syn::Error::new_spanned(
            &$item,
            format!("typst: {}", format!($($tts)*))
        ))
    };
}

/// Extract documentation comments from an attribute list.
pub fn documentation(attrs: &[syn::Attribute]) -> String {
    let mut doc = String::new();

    // Parse doc comments.
    for attr in attrs {
        if let syn::Meta::NameValue(meta) = &attr.meta {
            if meta.path.is_ident("doc") {
                if let syn::Expr::Lit(lit) = &meta.value {
                    if let syn::Lit::Str(string) = &lit.lit {
                        let full = string.value();
                        let line = full.strip_prefix(' ').unwrap_or(&full);
                        doc.push_str(line);
                        doc.push('\n');
                    }
                }
            }
        }
    }

    doc.trim().into()
}

/// Whether an attribute list has a specified attribute.
pub fn has_attr(attrs: &mut Vec<syn::Attribute>, target: &str) -> bool {
    take_attr(attrs, target).is_some()
}

/// Whether an attribute list has a specified attribute.
pub fn parse_attr<T: Parse>(
    attrs: &mut Vec<syn::Attribute>,
    target: &str,
) -> Result<Option<Option<T>>> {
    take_attr(attrs, target)
        .map(|attr| {
            Ok(match attr.meta {
                syn::Meta::Path(_) => None,
                syn::Meta::List(list) => Some(list.parse_args()?),
                syn::Meta::NameValue(meta) => bail!(meta, "not valid here"),
            })
        })
        .transpose()
}

/// Whether an attribute list has a specified attribute.
pub fn take_attr(
    attrs: &mut Vec<syn::Attribute>,
    target: &str,
) -> Option<syn::Attribute> {
    attrs
        .iter()
        .position(|attr| attr.path().is_ident(target))
        .map(|i| attrs.remove(i))
}

/// Ensure that no unrecognized attributes remain.
pub fn validate_attrs(attrs: &[syn::Attribute]) -> Result<()> {
    for attr in attrs {
        if !attr.path().is_ident("doc") && !attr.path().is_ident("derive") {
            let ident = attr.path().get_ident().unwrap();
            bail!(ident, "unrecognized attribute: {ident}");
        }
    }
    Ok(())
}

/// Quotes an option literally.
pub fn quote_option<T: ToTokens>(option: &Option<T>) -> TokenStream {
    if let Some(value) = option {
        quote! { Some(#value) }
    } else {
        quote! { None }
    }
}

/// Parse a metadata key-value pair, separated by `=`.
pub fn parse_key_value<K: Token + Default + Parse, V: Parse>(
    input: ParseStream,
) -> Result<Option<V>> {
    if !input.peek(|_| K::default()) {
        return Ok(None);
    }

    let _: K = input.parse()?;
    let _: Token![=] = input.parse()?;
    let value: V = input.parse::<V>()?;
    eat_comma(input);
    Ok(Some(value))
}

/// Parse a metadata key-array pair, separated by `=`.
pub fn parse_key_value_array<K: Token + Default + Parse, V: Parse>(
    input: ParseStream,
) -> Result<Vec<V>> {
    Ok(parse_key_value::<K, Array<V>>(input)?.map_or(vec![], |array| array.0))
}

/// Parse a metadata key-string pair, separated by `=`.
pub fn parse_string<K: Token + Default + Parse>(
    input: ParseStream,
) -> Result<Option<String>> {
    Ok(parse_key_value::<K, syn::LitStr>(input)?.map(|s| s.value()))
}

/// Parse a metadata key-string pair, separated by `=`.
pub fn parse_string_array<K: Token + Default + Parse>(
    input: ParseStream,
) -> Result<Vec<String>> {
    Ok(parse_key_value_array::<K, syn::LitStr>(input)?
        .into_iter()
        .map(|lit| lit.value())
        .collect())
}

/// Parse a metadata flag that can be present or not.
pub fn parse_flag<K: Token + Default + Parse>(input: ParseStream) -> Result<bool> {
    if input.peek(|_| K::default()) {
        let _: K = input.parse()?;
        eat_comma(input);
        return Ok(true);
    }
    Ok(false)
}

/// Parse a comma if there is one.
pub fn eat_comma(input: ParseStream) {
    if input.peek(Token![,]) {
        let _: Token![,] = input.parse().unwrap();
    }
}

/// Determine the normal and title case name of a function, type, or element.
pub fn determine_name_and_title(
    specified_name: Option<String>,
    specified_title: Option<String>,
    ident: &syn::Ident,
    trim: Option<fn(&str) -> &str>,
) -> Result<(String, String)> {
    let name = {
        let trim = trim.unwrap_or(|s| s);
        let default = trim(&ident.to_string()).to_kebab_case();
        if specified_name.as_ref() == Some(&default) {
            bail!(ident, "name was specified unnecessarily");
        }
        specified_name.unwrap_or(default)
    };

    let title = {
        let default = name.to_title_case();
        if specified_title.as_ref() == Some(&default) {
            bail!(ident, "title was specified unnecessarily");
        }
        specified_title.unwrap_or(default)
    };

    Ok((name, title))
}

/// A generic parseable array.
struct Array<T>(Vec<T>);

impl<T: Parse> Parse for Array<T> {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        syn::bracketed!(content in input);

        let mut elems = Vec::new();
        while !content.is_empty() {
            let first: T = content.parse()?;
            elems.push(first);
            if !content.is_empty() {
                let _: Token![,] = content.parse()?;
            }
        }

        Ok(Self(elems))
    }
}

/// For parsing attributes of the form:
/// #[attr(
///   statement;
///   statement;
///   returned_expression
/// )]
pub struct BlockWithReturn {
    pub prefix: Vec<syn::Stmt>,
    pub expr: syn::Stmt,
}

impl Parse for BlockWithReturn {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut stmts = syn::Block::parse_within(input)?;
        let Some(expr) = stmts.pop() else {
            return Err(input.error("expected at least one expression"));
        };
        Ok(Self { prefix: stmts, expr })
    }
}

pub mod kw {
    syn::custom_keyword!(name);
    syn::custom_keyword!(title);
    syn::custom_keyword!(scope);
    syn::custom_keyword!(constructor);
    syn::custom_keyword!(keywords);
    syn::custom_keyword!(parent);
}

/// Parse a bare `type Name;` item.
pub struct BareType {
    pub attrs: Vec<Attribute>,
    pub type_token: Token![type],
    pub ident: Ident,
    pub semi_token: Token![;],
}

impl Parse for BareType {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(BareType {
            attrs: input.call(Attribute::parse_outer)?,
            type_token: input.parse()?,
            ident: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}
