use super::*;

/// Return an error at the given item.
macro_rules! bail {
    (callsite, $fmt:literal $($tts:tt)*) => {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(concat!("typst: ", $fmt) $($tts)*)
        ))
    };
    ($item:expr, $fmt:literal $($tts:tt)*) => {
        return Err(syn::Error::new_spanned(
            &$item,
            format!(concat!("typst: ", $fmt) $($tts)*)
        ))
    };
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
        .map(|attr| (!attr.tokens.is_empty()).then(|| attr.parse_args()).transpose())
        .transpose()
}

/// Whether an attribute list has a specified attribute.
pub fn take_attr(
    attrs: &mut Vec<syn::Attribute>,
    target: &str,
) -> Option<syn::Attribute> {
    attrs
        .iter()
        .position(|attr| attr.path.is_ident(target))
        .map(|i| attrs.remove(i))
}

/// Ensure that no unrecognized attributes remain.
pub fn validate_attrs(attrs: &[syn::Attribute]) -> Result<()> {
    for attr in attrs {
        if !attr.path.is_ident("doc") {
            let ident = attr.path.get_ident().unwrap();
            bail!(ident, "unrecognized attribute: {:?}", ident.to_string());
        }
    }
    Ok(())
}

/// Convert an identifier to a kebab-case string.
pub fn kebab_case(name: &Ident) -> String {
    name.to_string().to_lowercase().replace('_', "-")
}

/// Dedent documentation text.
pub fn dedent(text: &str) -> String {
    text.lines()
        .map(|s| s.strip_prefix("  ").unwrap_or(s))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract documentation comments from an attribute list.
pub fn documentation(attrs: &[syn::Attribute]) -> String {
    let mut doc = String::new();

    // Parse doc comments.
    for attr in attrs {
        if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
            if meta.path.is_ident("doc") {
                if let syn::Lit::Str(string) = &meta.lit {
                    let full = string.value();
                    let line = full.strip_prefix(' ').unwrap_or(&full);
                    doc.push_str(line);
                    doc.push('\n');
                }
            }
        }
    }

    doc.trim().into()
}
