use std::error::Error;

use askama::Template;
use ecow::EcoString;
use typst_docs::{
    BodyModel, CategoryModel, FuncModel, GroupModel, Html, PageModel, SymbolsModel,
    TypeModel,
};

const BUST: &'static str = "20231030";
const D: &'static str = "2024-02-14";

#[derive(Template)]
#[template(path = "category.html")]
pub struct CategoryTemplate<'a> {
    page: &'a PageModel,
    prev: Option<&'a PageModel>,
    next: Option<&'a PageModel>,
    breadcrumbs: Vec<&'a PageModel>,
    // all_pages: &'a [&'a PageModel],
    root_pages: &'a [&'a PageModel],
    bust: &'a str,
    d: &'a str,
    category: &'a CategoryModel,
}

#[derive(Template)]
#[template(path = "func.html")]
pub struct FuncTemplate<'a> {
    page: &'a PageModel,
    prev: Option<&'a PageModel>,
    next: Option<&'a PageModel>,
    breadcrumbs: Vec<&'a PageModel>,
    // all_pages: &'a [&'a PageModel],
    root_pages: &'a [&'a PageModel],
    bust: &'a str,
    d: &'a str,
    func: &'a FuncModel,
}

#[derive(Template)]
#[template(path = "group.html")]
pub struct GroupTemplate<'a> {
    page: &'a PageModel,
    prev: Option<&'a PageModel>,
    next: Option<&'a PageModel>,
    breadcrumbs: Vec<&'a PageModel>,
    // all_pages: &'a [&'a PageModel],
    root_pages: &'a [&'a PageModel],
    bust: &'a str,
    d: &'a str,
    group: &'a GroupModel,
}

#[derive(Template)]
#[template(path = "html.html")]
pub struct HtmlTemplate<'a> {
    page: &'a PageModel,
    prev: Option<&'a PageModel>,
    next: Option<&'a PageModel>,
    breadcrumbs: Vec<&'a PageModel>,
    // all_pages: &'a [&'a PageModel],
    root_pages: &'a [&'a PageModel],
    bust: &'a str,
    d: &'a str,
    html: &'a Html,
}

#[derive(Template)]
#[template(path = "packages.html")]
pub struct PackagesTemplate<'a> {
    page: &'a PageModel,
    prev: Option<&'a PageModel>,
    next: Option<&'a PageModel>,
    breadcrumbs: Vec<&'a PageModel>,
    // all_pages: &'a [&'a PageModel],
    root_pages: &'a [&'a PageModel],
    bust: &'a str,
    d: &'a str,
    packages: &'a Html,
}

#[derive(Template)]
#[template(path = "symbols.html")]
pub struct SymbolsTemplate<'a> {
    page: &'a PageModel,
    prev: Option<&'a PageModel>,
    next: Option<&'a PageModel>,
    breadcrumbs: Vec<&'a PageModel>,
    // all_pages: &'a [&'a PageModel],
    root_pages: &'a [&'a PageModel],
    bust: &'a str,
    d: &'a str,
    symbols: &'a SymbolsModel,
}

#[derive(Template)]
#[template(path = "type.html")]
pub struct TypeTemplate<'a> {
    page: &'a PageModel,
    prev: Option<&'a PageModel>,
    next: Option<&'a PageModel>,
    breadcrumbs: Vec<&'a PageModel>,
    // all_pages: &'a [&'a PageModel],
    root_pages: &'a [&'a PageModel],
    bust: &'a str,
    d: &'a str,
    type_: &'a TypeModel,
}

/// Get the breadcrumbs for a page. The "breadcrumbs" are the links at the top of the page that
/// show the hierarchy of the page. The returned vector is in order from the highest level to the
/// lowest level. It includes the current page.
pub fn get_breadcrumbs<'a>(
    page: &'a PageModel,
    all_pages: &'a [&'a PageModel],
) -> Vec<&'a PageModel> {
    let mut breadcrumbs = Vec::new();
    for &p in all_pages {
        if page.route.starts_with(p.route.as_str()) {
            breadcrumbs.push(p);
        }
    }
    breadcrumbs
}

pub fn render_page<'a>(
    page: &'a PageModel,
    all_pages: &'a [&'a PageModel],
    root_pages: &'a [&'a PageModel],
) -> Result<EcoString, Box<dyn Error>> {
    let page_index = all_pages.iter().position(|p| p.route == page.route).unwrap();
    let prev = if page_index > 0 { Some(all_pages[page_index - 1]) } else { None };
    let next = if page_index < all_pages.len() - 1 {
        Some(all_pages[page_index + 1])
    } else {
        None
    };

    let breadcrumbs = get_breadcrumbs(page, all_pages);

    let html_string = match &page.body {
        BodyModel::Category(category) => CategoryTemplate {
            page,
            prev,
            next,
            breadcrumbs,
            // all_pages,
            root_pages,
            bust: BUST,
            d: D,
            category,
        }
        .render()?,
        BodyModel::Func(func) => FuncTemplate {
            page,
            prev,
            next,
            breadcrumbs,
            // all_pages,
            root_pages,
            bust: BUST,
            d: D,
            func,
        }
        .render()?,
        BodyModel::Group(group) => GroupTemplate {
            page,
            prev,
            next,
            breadcrumbs,
            // all_pages,
            root_pages,
            bust: BUST,
            d: D,
            group,
        }
        .render()?,
        BodyModel::Html(html) => HtmlTemplate {
            page,
            prev,
            next,
            breadcrumbs,
            // all_pages,
            root_pages,
            bust: BUST,
            d: D,
            html,
        }
        .render()?,
        BodyModel::Packages(packages) => PackagesTemplate {
            page,
            prev,
            next,
            breadcrumbs,
            // all_pages,
            root_pages,
            bust: BUST,
            d: D,
            packages,
        }
        .render()?,
        BodyModel::Symbols(symbols) => SymbolsTemplate {
            page,
            prev,
            next,
            breadcrumbs,
            // all_pages,
            root_pages,
            bust: BUST,
            d: D,
            symbols,
        }
        .render()?,
        BodyModel::Type(type_) => TypeTemplate {
            page,
            prev,
            next,
            breadcrumbs,
            // all_pages,
            root_pages,
            bust: BUST,
            d: D,
            type_,
        }
        .render()?,
    };
    Ok(EcoString::from(html_string))
}
