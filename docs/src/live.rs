//! Cooperates with `docs/components/live.typ`.

use std::ops::Range;

use syn::spanned::Spanned as _;
use typst::diag::bail;
use typst::foundations::{Array, Dict, IntoValue, Str, array, cast, func};

/// Takes a string of Rust source code and provides all doc comments in it that
/// belong to native definitions, keyed by the definitions def site key.
///
/// Returns a dictionary from items keys to pairs of (strings, ranges).
///
/// The item keys are per-file unique string that identify specific items. The
/// key structure is agreed upon between this function and the `key` file of
/// `def_site` fields on native type/element/function/param so that for a
/// specific native definition, the live docs can be located. Also see
/// `DefSite::key` for more information.
///
/// The ranges are represented as arrays of (start, end) pairs. They plug right
/// into `eval_mapped`'s `ranges` parameter.
#[func]
pub fn docs_in_source(source: Str) -> Dict {
    let mut v = DocsVisitor { items: Dict::new() };
    let file = syn::parse_file(&source);
    if let Ok(file) = &file {
        for item in &file.items {
            v.visit_item(item);
        }
    }
    v.items
}

/// Processes a Rust file and collects doc comments for items. Internally this
/// accumulates the collected documentation for all recognized items.
struct DocsVisitor {
    /// See `docs_in_source` for more information on the structure of this.
    items: Dict,
}

impl DocsVisitor {
    fn visit_item(&mut self, item: &syn::Item) {
        match item {
            syn::Item::Struct(s) if has_attr(&s.attrs, "ty") => {
                let key = s.ident.to_string();
                self.save_docs(key, &s.attrs);
            }
            syn::Item::Struct(s) if has_attr(&s.attrs, "elem") => {
                let key = s.ident.to_string();
                for field in &s.fields {
                    let Some(ident) = &field.ident else { continue };
                    let key = format!("{key}::{ident}");
                    self.save_docs(key, &field.attrs);
                }
                self.save_docs(key, &s.attrs);
            }
            syn::Item::Enum(e) if has_attr(&e.attrs, "ty") => {
                let key = e.ident.to_string();
                self.save_docs(key, &e.attrs);
            }
            syn::Item::Fn(f) if has_attr(&f.attrs, "func") => {
                self.visit_func(&f.attrs, &f.sig, None);
            }
            syn::Item::Impl(i) => {
                if let syn::Type::Path(path) = &*i.self_ty
                    && let Some(parent) = path.path.get_ident()
                {
                    for item in &i.items {
                        if let syn::ImplItem::Fn(item) = item
                            && has_attr(&item.attrs, "func")
                        {
                            self.visit_func(&item.attrs, &item.sig, Some(parent));
                        }
                    }
                }
            }
            syn::Item::Verbatim(s) => {
                if let Ok(t) = syn::parse2::<BareType>(s.clone())
                    && has_attr(&t.attrs, "ty")
                {
                    let key = t.ident.to_string();
                    self.save_docs(key, &t.attrs);
                }
            }
            _ => {}
        }
    }

    fn visit_func(
        &mut self,
        attrs: &[syn::Attribute],
        sig: &syn::Signature,
        parent: Option<&syn::Ident>,
    ) {
        let ident = &sig.ident;
        let key = match parent {
            Some(parent) => format!("{parent}::{ident}"),
            None => ident.to_string(),
        };

        for input in &sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input
                && let syn::Pat::Ident(pat) = &*pat_type.pat
            {
                let key = format!("{key}::{}", pat.ident);
                self.save_docs(key, &pat_type.attrs);
            }
        }

        self.save_docs(key, attrs);
    }

    fn save_docs(&mut self, key: String, attrs: &[syn::Attribute]) {
        let mut docs = String::new();
        let mut ranges = Vec::new();

        // Parse doc comments.
        for attr in attrs {
            if let syn::Meta::NameValue(meta) = &attr.meta
                && meta.path.is_ident("doc")
                && let syn::Expr::Lit(lit) = &meta.value
                && let syn::Lit::Str(string) = &lit.lit
            {
                let full = string.value();
                let line = full.strip_prefix(' ').unwrap_or(&full);
                docs.push_str(line);
                docs.push('\n'); // TODO: No trailing \n

                let start = attr.span().byte_range().start + 3 + full.len() - line.len();
                let end = start + line.len() + 1;
                ranges.push(RangePair(start..end));
            }
        }

        self.items.insert(key.into(), array![docs, ranges].into_value());
    }
}

/// Whether the attribute list contains an attribute with the given identifier.
fn has_attr(attrs: &[syn::Attribute], ident: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(ident))
}

/// Parse a bare `type Name;` item.
#[allow(dead_code)]
struct BareType {
    attrs: Vec<syn::Attribute>,
    type_token: syn::Token![type],
    ident: syn::Ident,
    semi_token: syn::Token![;],
}

impl syn::parse::Parse for BareType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(syn::Attribute::parse_outer)?,
            type_token: input.parse()?,
            ident: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

/// Represents a `(start, end)` pair, with `end` exclusive.
pub struct RangePair(pub Range<usize>);

cast! {
    RangePair,
    self => array![self.0.start, self.0.end].into_value(),
    array: Array => {
         let mut iter = array.into_iter();
         match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Self(a.cast()?.. b.cast()?),
            _ => bail!("array must contain exactly two items"),
        }
    }
}
