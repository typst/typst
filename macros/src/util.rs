use heck::ToKebabCase;
use quote::ToTokens;

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

/// Convert an identifier to a kebab-case string.
pub fn kebab_case(name: &Ident) -> String {
    name.to_string().to_kebab_case()
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

/// Extract a line of metadata from documentation.
pub fn meta_line<'a>(lines: &mut Vec<&'a str>, key: &str) -> Result<&'a str> {
    match lines.last().and_then(|line| line.strip_prefix(&format!("{key}:"))) {
        Some(value) => {
            lines.pop();
            Ok(value.trim())
        }
        None => bail!(callsite, "missing metadata key: {key}"),
    }
}

/// Creates a block responsible for building a `Scope`.
pub fn create_scope_builder(scope_block: Option<&BlockWithReturn>) -> TokenStream {
    if let Some(BlockWithReturn { prefix, expr }) = scope_block {
        quote! { {
            let mut scope = ::typst::eval::Scope::deduplicating();
            #(#prefix);*
            #expr
        } }
    } else {
        quote! { ::typst::eval::Scope::new() }
    }
}

/// Quotes an option literally.
pub fn quote_option<T: ToTokens>(option: &Option<T>) -> TokenStream {
    if let Some(value) = option {
        quote! { Some(#value) }
    } else {
        quote! { None }
    }
}
