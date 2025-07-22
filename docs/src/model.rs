use ecow::EcoString;
use heck::ToKebabCase;
use serde::Serialize;

use crate::html::Html;

/// Details about a documentation page and its children.
#[derive(Debug, Serialize)]
pub struct PageModel {
    pub route: EcoString,
    pub title: EcoString,
    pub description: EcoString,
    pub part: Option<&'static str>,
    pub outline: Vec<OutlineItem>,
    pub body: BodyModel,
    pub children: Vec<Self>,
}

impl PageModel {
    pub fn with_route(self, route: &str) -> Self {
        Self { route: route.into(), ..self }
    }

    pub fn with_part(self, part: &'static str) -> Self {
        Self { part: Some(part), ..self }
    }
}

/// An element in the "On This Page" outline.
#[derive(Debug, Clone, Serialize)]
pub struct OutlineItem {
    pub id: EcoString,
    pub name: EcoString,
    pub children: Vec<Self>,
}

impl OutlineItem {
    /// Create an outline item from a name with auto-generated id.
    pub fn from_name(name: &str) -> Self {
        Self {
            id: name.to_kebab_case().into(),
            name: name.into(),
            children: vec![],
        }
    }
}

/// Details about the body of a documentation page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind", content = "content")]
#[allow(clippy::large_enum_variant)]
pub enum BodyModel {
    Html(Html),
    Category(CategoryModel),
    Func(FuncModel),
    Group(GroupModel),
    Type(TypeModel),
    Symbols(SymbolsModel),
    Packages(Html),
}

/// Details about a category.
#[derive(Debug, Serialize)]
pub struct CategoryModel {
    pub name: &'static str,
    pub title: EcoString,
    pub details: Html,
    pub items: Vec<CategoryItem>,
    pub shorthands: Option<ShorthandsModel>,
}

/// Details about a category item.
#[derive(Debug, Serialize)]
pub struct CategoryItem {
    pub name: EcoString,
    pub route: EcoString,
    pub oneliner: EcoString,
    pub code: bool,
}

/// Details about a function.
#[derive(Debug, Serialize)]
pub struct FuncModel {
    pub path: Vec<EcoString>,
    pub name: EcoString,
    pub title: &'static str,
    pub keywords: &'static [&'static str],
    pub oneliner: EcoString,
    pub element: bool,
    pub contextual: bool,
    pub deprecation: Option<&'static str>,
    pub details: Html,
    /// This example is only for nested function models. Others can have
    /// their example directly in their details.
    pub example: Option<Html>,
    #[serde(rename = "self")]
    pub self_: bool,
    pub params: Vec<ParamModel>,
    pub returns: Vec<&'static str>,
    pub scope: Vec<FuncModel>,
}

/// Details about a function parameter.
#[derive(Debug, Serialize)]
pub struct ParamModel {
    pub name: &'static str,
    pub details: Html,
    pub example: Option<Html>,
    pub types: Vec<&'static str>,
    pub strings: Vec<StrParam>,
    pub default: Option<Html>,
    pub positional: bool,
    pub named: bool,
    pub required: bool,
    pub variadic: bool,
    pub settable: bool,
}

/// A specific string that can be passed as an argument.
#[derive(Debug, Serialize)]
pub struct StrParam {
    pub string: EcoString,
    pub details: Html,
}

/// Details about a group of functions.
#[derive(Debug, Serialize)]
pub struct GroupModel {
    pub name: EcoString,
    pub title: EcoString,
    pub details: Html,
    pub functions: Vec<FuncModel>,
}

/// Details about a type.
#[derive(Debug, Serialize)]
pub struct TypeModel {
    pub name: &'static str,
    pub title: &'static str,
    pub keywords: &'static [&'static str],
    pub oneliner: EcoString,
    pub details: Html,
    pub constructor: Option<FuncModel>,
    pub scope: Vec<FuncModel>,
}

/// A collection of symbols.
#[derive(Debug, Serialize)]
pub struct SymbolsModel {
    pub name: EcoString,
    pub title: EcoString,
    pub details: Html,
    pub list: Vec<SymbolModel>,
}

/// Details about a symbol.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolModel {
    pub name: EcoString,
    pub codepoint: u32,
    pub accent: bool,
    pub alternates: Vec<EcoString>,
    pub markup_shorthand: Option<&'static str>,
    pub math_shorthand: Option<&'static str>,
    pub math_class: Option<&'static str>,
    pub deprecation: Option<&'static str>,
}

/// Shorthands listed on a category page.
#[derive(Debug, Serialize)]
pub struct ShorthandsModel {
    pub markup: Vec<SymbolModel>,
    pub math: Vec<SymbolModel>,
}
