//! Documentation provider for Typst.

mod contribs;
mod html;
mod link;
mod model;

pub use self::contribs::*;
pub use self::html::*;
pub use self::model::*;

use ecow::{EcoString, eco_format};
use heck::ToTitleCase;
use rustc_hash::FxHashSet;
use serde::Deserialize;
use serde_yaml as yaml;
use std::sync::LazyLock;
use typst::diag::{StrResult, bail};
use typst::foundations::Deprecation;
use typst::foundations::{
    AutoValue, Binding, Bytes, CastInfo, Func, Module, NoneValue, ParamInfo, Repr, Scope,
    Smart, Type, Value,
};
use typst::layout::{Abs, Margin, PageElem, PagedDocument};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Category, Feature, Library, LibraryExt};
use unicode_math_class::MathClass;

macro_rules! load {
    ($path:literal) => {
        include_str!(concat!("../", $path))
    };
}

static GROUPS: LazyLock<Vec<GroupData>> = LazyLock::new(|| {
    let mut groups: Vec<GroupData> =
        yaml::from_str(load!("reference/groups.yml")).unwrap();
    for group in &mut groups {
        if group.filter.is_empty() && group.name != "std" {
            group.filter = group
                .module()
                .scope()
                .iter()
                .filter(|(_, b)| matches!(b.read(), Value::Func(_)))
                .map(|(k, _)| k.clone())
                .collect();
        }
    }
    groups
});

static LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| {
    let mut lib = Library::builder()
        .with_features([Feature::Html, Feature::A11yExtras].into_iter().collect())
        .build();
    let scope = lib.global.scope_mut();

    // Add those types, so that they show up in the docs.
    scope.start_category(Category::Foundations);
    scope.define_type::<NoneValue>();
    scope.define_type::<AutoValue>();
    scope.reset_category();

    // Adjust the default look.
    lib.styles.set(PageElem::width, Smart::Custom(Abs::pt(240.0).into()));
    lib.styles.set(PageElem::height, Smart::Auto);
    lib.styles
        .set(PageElem::margin, Margin::splat(Some(Smart::Custom(Abs::pt(15.0).into()))));

    LazyHash::new(lib)
});

static FONTS: LazyLock<(LazyHash<FontBook>, Vec<Font>)> = LazyLock::new(|| {
    let fonts: Vec<_> = typst_assets::fonts()
        .chain(typst_dev_assets::fonts())
        .flat_map(|data| Font::iter(Bytes::new(data)))
        .collect();
    let book = FontBook::from_fonts(&fonts);
    (LazyHash::new(book), fonts)
});

/// Build documentation pages.
pub fn provide(resolver: &dyn Resolver) -> Vec<PageModel> {
    let base = resolver.base();
    vec![
        md_page(resolver, base, load!("overview.md")).with_route(base),
        tutorial_pages(resolver),
        reference_pages(resolver),
        guide_pages(resolver),
        changelog_pages(resolver),
    ]
}

/// Resolve consumer dependencies.
pub trait Resolver {
    /// Try to resolve a link. If this returns `None`, the system will try to
    /// resolve the link itself.
    fn link(&self, link: &str) -> Option<String>;

    /// Produce an URL for an image file.
    fn image(&self, filename: &str, data: &[u8]) -> String;

    /// Produce HTML for an example.
    fn example(&self, hash: u128, source: Option<Html>, document: &PagedDocument)
    -> Html;

    /// Determine the commits between two tags.
    fn commits(&self, from: &str, to: &str) -> Vec<Commit>;

    /// Get the base URL for the routes and links. This must end with a slash.
    fn base(&self) -> &str;
}

/// Create a page from a markdown file.
#[track_caller]
fn md_page(resolver: &dyn Resolver, parent: &str, md: &str) -> PageModel {
    md_page_with_title(resolver, parent, md, None)
}

/// Create a page from a markdown file.
#[track_caller]
fn md_page_with_title(
    resolver: &dyn Resolver,
    parent: &str,
    md: &str,
    title: Option<&str>,
) -> PageModel {
    assert!(parent.starts_with('/') && parent.ends_with('/'));
    let html = Html::markdown(resolver, md, Some(0));
    let title = title.or(html.title()).expect("chapter lacks a title");
    PageModel {
        route: eco_format!("{parent}{}/", urlify(title)),
        title: title.into(),
        description: html.description().expect("chapter lacks a description"),
        part: None,
        outline: html.outline(),
        body: BodyModel::Html(html),
        children: vec![],
    }
}

/// Build the tutorial.
fn tutorial_pages(resolver: &dyn Resolver) -> PageModel {
    let mut page = md_page(resolver, resolver.base(), load!("tutorial/welcome.md"));
    let base = format!("{}tutorial/", resolver.base());
    page.children = vec![
        md_page(resolver, &base, load!("tutorial/1-writing.md")),
        md_page(resolver, &base, load!("tutorial/2-formatting.md")),
        md_page(resolver, &base, load!("tutorial/3-advanced.md")),
        md_page(resolver, &base, load!("tutorial/4-template.md")),
    ];
    page
}

/// Build the reference.
fn reference_pages(resolver: &dyn Resolver) -> PageModel {
    let mut page = md_page(resolver, resolver.base(), load!("reference/welcome.md"));
    let base = format!("{}reference/", resolver.base());
    page.children = vec![
        md_page(resolver, &base, load!("reference/language/syntax.md"))
            .with_part("Language"),
        md_page(resolver, &base, load!("reference/language/styling.md")),
        md_page(resolver, &base, load!("reference/language/scripting.md")),
        md_page(resolver, &base, load!("reference/language/context.md")),
        category_page(resolver, Category::Foundations).with_part("Library"),
        category_page(resolver, Category::Model),
        category_page(resolver, Category::Text),
        category_page(resolver, Category::Math),
        category_page(resolver, Category::Symbols),
        category_page(resolver, Category::Layout),
        category_page(resolver, Category::Visualize),
        category_page(resolver, Category::Introspection),
        category_page(resolver, Category::DataLoading),
        category_page(resolver, Category::Pdf).with_part("Export"),
        category_page(resolver, Category::Html),
        category_page(resolver, Category::Png),
        category_page(resolver, Category::Svg),
    ];
    page
}

/// Build the guides section.
fn guide_pages(resolver: &dyn Resolver) -> PageModel {
    let mut page = md_page(resolver, resolver.base(), load!("guides/welcome.md"));
    let base = format!("{}guides/", resolver.base());
    page.children = vec![
        md_page_with_title(
            resolver,
            &base,
            load!("guides/guide-for-latex-users.md"),
            Some("For LaTeX Users"),
        ),
        md_page_with_title(
            resolver,
            &base,
            load!("guides/page-setup.md"),
            Some("Page Setup"),
        ),
        md_page_with_title(resolver, &base, load!("guides/tables.md"), Some("Tables")),
        md_page_with_title(
            resolver,
            &base,
            load!("guides/accessibility.md"),
            Some("Accessibility"),
        ),
    ];
    page
}

/// Build the changelog section.
fn changelog_pages(resolver: &dyn Resolver) -> PageModel {
    let mut page = md_page(resolver, resolver.base(), load!("changelog/welcome.md"));
    let base = format!("{}changelog/", resolver.base());
    page.children = vec![
        md_page(resolver, &base, load!("changelog/0.14.0.md")),
        md_page(resolver, &base, load!("changelog/0.13.1.md")),
        md_page(resolver, &base, load!("changelog/0.13.0.md")),
        md_page(resolver, &base, load!("changelog/0.12.0.md")),
        md_page(resolver, &base, load!("changelog/0.11.1.md")),
        md_page(resolver, &base, load!("changelog/0.11.0.md")),
        md_page(resolver, &base, load!("changelog/0.10.0.md")),
        md_page(resolver, &base, load!("changelog/0.9.0.md")),
        md_page(resolver, &base, load!("changelog/0.8.0.md")),
        md_page(resolver, &base, load!("changelog/0.7.0.md")),
        md_page(resolver, &base, load!("changelog/0.6.0.md")),
        md_page(resolver, &base, load!("changelog/0.5.0.md")),
        md_page(resolver, &base, load!("changelog/0.4.0.md")),
        md_page(resolver, &base, load!("changelog/0.3.0.md")),
        md_page(resolver, &base, load!("changelog/0.2.0.md")),
        md_page(resolver, &base, load!("changelog/0.1.0.md")),
        md_page(resolver, &base, load!("changelog/earlier.md")),
    ];
    page
}

/// Create a page for a category.
#[track_caller]
fn category_page(resolver: &dyn Resolver, category: Category) -> PageModel {
    let route = eco_format!("{}reference/{}/", resolver.base(), category.name());
    let mut children = vec![];
    let mut items = vec![];
    let mut shorthands = None;
    let mut markup = vec![];
    let mut math = vec![];

    let docs = category_docs(category);
    let (module, path): (&Module, &[&str]) = match category {
        Category::Math => (&LIBRARY.math, &["math"]),
        Category::Pdf => (get_module(&LIBRARY.global, "pdf").unwrap(), &["pdf"]),
        Category::Html => (get_module(&LIBRARY.global, "html").unwrap(), &["html"]),
        _ => (&LIBRARY.global, &[]),
    };

    // Add groups.
    for group in GROUPS.iter().filter(|g| g.category == category).cloned() {
        if matches!(group.name.as_str(), "sym" | "emoji") {
            let subpage = symbols_page(resolver, &route, &group);
            let BodyModel::Symbols(model) = &subpage.body else { continue };
            let list = &model.list;
            markup.extend(
                list.iter()
                    .filter(|symbol| symbol.markup_shorthand.is_some())
                    .cloned(),
            );
            math.extend(
                list.iter().filter(|symbol| symbol.math_shorthand.is_some()).cloned(),
            );

            items.push(CategoryItem {
                name: group.name.clone(),
                route: subpage.route.clone(),
                oneliner: oneliner(docs),
                code: true,
            });
            children.push(subpage);
            continue;
        }

        let (child, item) = group_page(resolver, &route, &group);
        children.push(child);
        items.push(item);
    }

    // Add symbol pages. These are ordered manually.
    if category == Category::Symbols {
        shorthands = Some(ShorthandsModel { markup, math });
    }

    let mut skip = FxHashSet::default();
    if category == Category::Math {
        skip = GROUPS
            .iter()
            .filter(|g| g.category == category)
            .flat_map(|g| &g.filter)
            .map(|s| s.as_str())
            .collect();

        // Already documented in the text category.
        skip.insert("text");
    }

    // Tiling would be duplicate otherwise.
    if category == Category::Visualize {
        skip.insert("pattern");
    }

    // PDF attach would be duplicate otherwise.
    if category == Category::Pdf {
        skip.insert("embed");
    }

    // Add values and types.
    let scope = module.scope();
    for (name, binding) in scope.iter() {
        if binding.category() != Some(category) {
            continue;
        }

        if skip.contains(name.as_str()) {
            continue;
        }

        match binding.read() {
            Value::Func(func) => {
                let name = func.name().unwrap();
                let subpage =
                    func_page(resolver, &route, func, path, binding.deprecation());
                items.push(CategoryItem {
                    name: name.into(),
                    route: subpage.route.clone(),
                    oneliner: oneliner(func.docs().unwrap_or_default()),
                    code: true,
                });
                children.push(subpage);
            }
            Value::Type(ty) => {
                let subpage = type_page(resolver, &route, ty);
                items.push(CategoryItem {
                    name: ty.short_name().into(),
                    route: subpage.route.clone(),
                    oneliner: oneliner(ty.docs()),
                    code: true,
                });
                children.push(subpage);
            }
            _ => {}
        }
    }

    if category != Category::Symbols {
        children.sort_by_cached_key(|child| child.title.clone());
        items.sort_by_cached_key(|item| item.name.clone());
    }

    let title = EcoString::from(match category {
        Category::Pdf | Category::Html | Category::Png | Category::Svg => {
            category.name().to_uppercase()
        }
        _ => category.name().to_title_case(),
    });

    let details = Html::markdown(resolver, docs, Some(1));
    let mut outline = vec![OutlineItem::from_name("Summary")];
    outline.extend(details.outline());
    if !items.is_empty() {
        outline.push(OutlineItem::from_name("Definitions"));
    }
    if shorthands.is_some() {
        outline.push(OutlineItem::from_name("Shorthands"));
    }

    PageModel {
        route,
        title: title.clone(),
        description: eco_format!(
            "Documentation for functions related to {title} in Typst."
        ),
        part: None,
        outline,
        body: BodyModel::Category(CategoryModel {
            name: category.name(),
            title,
            details,
            items,
            shorthands,
        }),
        children,
    }
}

/// Retrieve the docs for a category.
fn category_docs(category: Category) -> &'static str {
    match category {
        Category::Foundations => load!("reference/library/foundations.md"),
        Category::Introspection => load!("reference/library/introspection.md"),
        Category::Layout => load!("reference/library/layout.md"),
        Category::DataLoading => load!("reference/library/data-loading.md"),
        Category::Math => load!("reference/library/math.md"),
        Category::Model => load!("reference/library/model.md"),
        Category::Symbols => load!("reference/library/symbols.md"),
        Category::Text => load!("reference/library/text.md"),
        Category::Visualize => load!("reference/library/visualize.md"),
        Category::Pdf => load!("reference/export/pdf.md"),
        Category::Html => load!("reference/export/html.md"),
        Category::Svg => load!("reference/export/svg.md"),
        Category::Png => load!("reference/export/png.md"),
    }
}

/// Create a page for a function.
fn func_page(
    resolver: &dyn Resolver,
    parent: &str,
    func: &Func,
    path: &[&str],
    deprecation: Option<&Deprecation>,
) -> PageModel {
    let model = func_model(resolver, func, path, false, deprecation);
    let name = func.name().unwrap();
    PageModel {
        route: eco_format!("{parent}{}/", urlify(name)),
        title: func.title().unwrap().into(),
        description: eco_format!("Documentation for the `{name}` function."),
        part: None,
        outline: func_outline(&model, ""),
        body: BodyModel::Func(model),
        children: vec![],
    }
}

/// Produce a function's model.
fn func_model(
    resolver: &dyn Resolver,
    func: &Func,
    path: &[&str],
    nested: bool,
    deprecation: Option<&Deprecation>,
) -> FuncModel {
    let name = func.name().unwrap();
    let scope = func.scope().unwrap();
    let docs = func.docs().unwrap();

    let mut self_ = false;
    let mut params = func.params().unwrap();
    if params.first().is_some_and(|first| first.name == "self") {
        self_ = true;
        params = &params[1..];
    }

    let mut returns = vec![];
    let mut strings = vec![];
    casts(resolver, &mut returns, &mut strings, func.returns().unwrap());
    if !strings.is_empty() && !returns.contains(&"str") {
        returns.push("str");
    }
    returns.sort_by_key(|ty| type_index(ty));
    if returns == ["none"] {
        returns.clear();
    }

    let nesting = if nested { None } else { Some(1) };
    let items =
        if nested { details_blocks(docs) } else { vec![RawDetailsBlock::Markdown(docs)] };

    let Some(first_md) = items.iter().find_map(|item| {
        if let RawDetailsBlock::Markdown(md) = item { Some(md) } else { None }
    }) else {
        panic!("function lacks any details")
    };

    FuncModel {
        path: path.iter().copied().map(Into::into).collect(),
        name: name.into(),
        title: func.title().unwrap(),
        keywords: func.keywords(),
        oneliner: oneliner(first_md),
        element: func.element().is_some(),
        contextual: func.contextual().unwrap_or(false),
        deprecation_message: deprecation.map(Deprecation::message),
        deprecation_until: deprecation.and_then(Deprecation::until),
        details: items
            .into_iter()
            .map(|proto| proto.into_model(resolver, nesting))
            .collect(),
        self_,
        params: params.iter().map(|param| param_model(resolver, param)).collect(),
        returns,
        scope: scope_models(resolver, name, scope),
    }
}

/// Produce a parameter's model.
fn param_model(resolver: &dyn Resolver, info: &ParamInfo) -> ParamModel {
    let mut types = vec![];
    let mut strings = vec![];
    casts(resolver, &mut types, &mut strings, &info.input);
    if !strings.is_empty() && !types.contains(&"str") {
        types.push("str");
    }
    types.sort_by_key(|ty| type_index(ty));

    ParamModel {
        name: info.name,
        details: details_blocks(info.docs)
            .into_iter()
            .map(|proto| proto.into_model(resolver, None))
            .collect(),
        types,
        strings,
        default: info.default.map(|default| {
            let node = typst::syntax::parse_code(&default().repr());
            Html::new(typst::syntax::highlight_html(&node))
        }),
        positional: info.positional,
        named: info.named,
        required: info.required,
        variadic: info.variadic,
        settable: info.settable,
    }
}

/// A details block that has not yet been processed.
enum RawDetailsBlock<'a> {
    /// Raw Markdown.
    Markdown(&'a str),
    /// An example with an optional title.
    Example { body: &'a str, title: Option<&'a str> },
}

impl<'a> RawDetailsBlock<'a> {
    fn into_model(self, resolver: &dyn Resolver, nesting: Option<usize>) -> DetailsBlock {
        match self {
            RawDetailsBlock::Markdown(md) => {
                DetailsBlock::Html(Html::markdown(resolver, md, nesting))
            }
            RawDetailsBlock::Example { body, title } => DetailsBlock::Example {
                body: Html::markdown(resolver, body, None),
                title: title.map(Into::into),
            },
        }
    }
}

/// Split up documentation into Markdown blocks and examples.
fn details_blocks(docs: &str) -> Vec<RawDetailsBlock<'_>> {
    let mut i = 0;
    let mut res = Vec::new();

    while i < docs.len() {
        match find_fence_start(&docs[i..]) {
            Some((found, fence_len)) => {
                let fence_idx = i + found;

                // Find the language tag of the fence, if any.
                let lang_tag_end = docs[fence_idx + fence_len..]
                    .find('\n')
                    .map(|end| fence_idx + fence_len + end)
                    .unwrap_or(docs.len());

                let tag = &docs[fence_idx + fence_len..lang_tag_end].trim();
                let title = ExampleArgs::from_tag(tag).title;

                // First, push non-fenced content.
                if found > 0 {
                    res.push(RawDetailsBlock::Markdown(&docs[i..fence_idx]));
                }

                // Then, find the end of the fence.
                let offset = fence_idx + fence_len;
                let Some(fence_end) = docs[offset..]
                    .find(&"`".repeat(fence_len))
                    .map(|end| offset + end + fence_len)
                else {
                    panic!(
                        "unclosed code fence in docs at position {}: {}",
                        fence_idx,
                        &docs[fence_idx..]
                    );
                };

                res.push(RawDetailsBlock::Example {
                    body: &docs[fence_idx..fence_end],
                    title,
                });
                i = fence_end;
            }
            None => {
                res.push(RawDetailsBlock::Markdown(&docs[i..]));
                break;
            }
        }
    }

    res
}

/// Returns the start of a code fence and how many backticks it uses.
fn find_fence_start(md: &str) -> Option<(usize, usize)> {
    let start = md.find("```")?;
    let mut count = 3;
    while md[start + count..].starts_with('`') {
        count += 1;
    }
    Some((start, count))
}

/// Process cast information into types and strings.
fn casts(
    resolver: &dyn Resolver,
    types: &mut Vec<&'static str>,
    strings: &mut Vec<StrParam>,
    info: &CastInfo,
) {
    match info {
        CastInfo::Any => types.push("any"),
        CastInfo::Value(Value::Str(string), docs) => strings.push(StrParam {
            string: string.clone().into(),
            details: Html::markdown(resolver, docs, None),
        }),
        CastInfo::Value(..) => {}
        CastInfo::Type(ty) => types.push(ty.short_name()),
        CastInfo::Union(options) => {
            for option in options {
                casts(resolver, types, strings, option);
            }
        }
    }
}

/// Produce models for a function's scope.
fn scope_models(resolver: &dyn Resolver, name: &str, scope: &Scope) -> Vec<FuncModel> {
    scope
        .iter()
        .filter_map(|(_, binding)| {
            let Value::Func(func) = binding.read() else { return None };
            Some(func_model(resolver, func, &[name], true, binding.deprecation()))
        })
        .collect()
}

/// Produce an outline for a function page.
fn func_outline(model: &FuncModel, id_base: &str) -> Vec<OutlineItem> {
    let mut outline = vec![];

    if id_base.is_empty() {
        outline.push(OutlineItem::from_name("Summary"));
        for block in &model.details {
            if let DetailsBlock::Html(html) = block {
                outline.extend(html.outline());
            }
        }

        if !model.params.is_empty() {
            outline.push(OutlineItem {
                id: "parameters".into(),
                name: "Parameters".into(),
                children: model
                    .params
                    .iter()
                    .map(|param| OutlineItem {
                        id: eco_format!("parameters-{}", urlify(param.name)),
                        name: param.name.into(),
                        children: vec![],
                    })
                    .collect(),
            });
        }
    } else {
        outline.extend(model.params.iter().map(|param| OutlineItem {
            id: eco_format!("{id_base}-{}", urlify(param.name)),
            name: param.name.into(),
            children: vec![],
        }));
    }

    outline.extend(scope_outline(&model.scope, id_base));

    outline
}

/// Produce an outline for a function scope.
fn scope_outline(scope: &[FuncModel], id_base: &str) -> Option<OutlineItem> {
    if scope.is_empty() {
        return None;
    }

    let dash = if id_base.is_empty() { "" } else { "-" };
    let id = eco_format!("{id_base}{dash}definitions");

    let children = scope
        .iter()
        .map(|func| {
            let id = urlify(&eco_format!("{id}-{}", func.name));
            let children = func_outline(func, &id);
            OutlineItem { id, name: func.title.into(), children }
        })
        .collect();

    Some(OutlineItem { id, name: "Definitions".into(), children })
}

/// Create a page for a group of functions.
fn group_page(
    resolver: &dyn Resolver,
    parent: &str,
    group: &GroupData,
) -> (PageModel, CategoryItem) {
    let mut functions = vec![];
    let mut outline = vec![OutlineItem::from_name("Summary")];

    let path: Vec<_> = group.path.iter().map(|s| s.as_str()).collect();
    let details = Html::markdown(resolver, &group.details, Some(1));
    outline.extend(details.outline());

    let mut outline_items = vec![];
    for name in &group.filter {
        let binding = group.module().scope().get(name).unwrap();
        let Ok(ref func) = binding.read().clone().cast::<Func>() else {
            panic!("not a function")
        };
        let func = func_model(resolver, func, &path, true, binding.deprecation());
        let id_base = urlify(&eco_format!("functions-{}", func.name));
        let children = func_outline(&func, &id_base);
        outline_items.push(OutlineItem {
            id: id_base,
            name: func.title.into(),
            children,
        });
        functions.push(func);
    }

    outline.push(OutlineItem {
        id: "functions".into(),
        name: "Functions".into(),
        children: outline_items,
    });

    let model = PageModel {
        route: eco_format!("{parent}{}/", group.name),
        title: group.title.clone(),
        description: eco_format!("Documentation for the {} functions.", group.name),
        part: None,
        outline,
        body: BodyModel::Group(GroupModel {
            name: group.name.clone(),
            title: group.title.clone(),
            details,
            functions,
        }),
        children: vec![],
    };

    let item = CategoryItem {
        name: group.name.clone(),
        route: model.route.clone(),
        oneliner: oneliner(&group.details),
        code: false,
    };

    (model, item)
}

/// Create a page for a type.
fn type_page(resolver: &dyn Resolver, parent: &str, ty: &Type) -> PageModel {
    let model = type_model(resolver, ty);
    PageModel {
        route: eco_format!("{parent}{}/", urlify(ty.short_name())),
        title: ty.title().into(),
        description: eco_format!("Documentation for the {} type.", ty.title()),
        part: None,
        outline: type_outline(&model),
        body: BodyModel::Type(model),
        children: vec![],
    }
}

/// Produce a type's model.
fn type_model(resolver: &dyn Resolver, ty: &Type) -> TypeModel {
    TypeModel {
        name: ty.short_name(),
        title: ty.title(),
        keywords: ty.keywords(),
        oneliner: oneliner(ty.docs()),
        details: Html::markdown(resolver, ty.docs(), Some(1)),
        constructor: ty
            .constructor()
            .ok()
            .map(|func| func_model(resolver, &func, &[], true, None)),
        scope: scope_models(resolver, ty.short_name(), ty.scope()),
    }
}

/// Produce an outline for a type page.
fn type_outline(model: &TypeModel) -> Vec<OutlineItem> {
    let mut outline = vec![OutlineItem::from_name("Summary")];
    outline.extend(model.details.outline());

    if let Some(func) = &model.constructor {
        outline.push(OutlineItem {
            id: "constructor".into(),
            name: "Constructor".into(),
            children: func_outline(func, "constructor"),
        });
    }

    outline.extend(scope_outline(&model.scope, ""));
    outline
}

/// Create a page for symbols.
fn symbols_page(resolver: &dyn Resolver, parent: &str, group: &GroupData) -> PageModel {
    let model = symbols_model(resolver, group);
    PageModel {
        route: eco_format!("{parent}{}/", group.name),
        title: group.title.clone(),
        description: eco_format!("Documentation for the `{}` module.", group.name),
        part: None,
        outline: vec![],
        body: BodyModel::Symbols(model),
        children: vec![],
    }
}

/// Produce a symbol list's model.
fn symbols_model(resolver: &dyn Resolver, group: &GroupData) -> SymbolsModel {
    let mut list = vec![];
    for (name, binding) in group.module().scope().iter() {
        let Value::Symbol(symbol) = binding.read() else { continue };
        let complete = |variant: codex::ModifierSet<&str>| {
            if variant.is_empty() {
                name.clone()
            } else {
                eco_format!("{}.{}", name, variant.as_str())
            }
        };

        for (variant, value, deprecation_message) in symbol.variants() {
            let value_char = value.parse::<char>().ok();

            let shorthand = |list: &[(&'static str, char)]| {
                value_char.and_then(|c| {
                    list.iter().copied().find(|&(_, x)| x == c).map(|(s, _)| s)
                })
            };

            let name = complete(variant);

            list.push(SymbolModel {
                name,
                markup_shorthand: shorthand(typst::syntax::ast::Shorthand::LIST),
                math_shorthand: shorthand(typst::syntax::ast::MathShorthand::LIST),
                // Matches `typst_layout::math::GlyphFragment::new`
                math_class: value.chars().next().and_then(|c| {
                    typst_utils::default_math_class(c).map(math_class_name)
                }),
                value: value.into(),
                // Matches casting `Symbol` to `Accent`
                accent: value_char
                    .is_some_and(|c| typst::math::Accent::combine(c).is_some()),
                alternates: symbol
                    .variants()
                    .filter(|(other, _, _)| other != &variant)
                    .map(|(other, _, _)| complete(other))
                    .collect(),
                deprecation_message: deprecation_message
                    .or_else(|| binding.deprecation().map(Deprecation::message)),
                deprecation_until: binding.deprecation().and_then(Deprecation::until),
            });
        }
    }

    SymbolsModel {
        name: group.name.clone(),
        title: group.title.clone(),
        details: Html::markdown(resolver, &group.details, Some(1)),
        list,
    }
}

/// Extract a module from another module.
#[track_caller]
fn get_module<'a>(parent: &'a Module, name: &str) -> StrResult<&'a Module> {
    match parent.scope().get(name).map(Binding::read) {
        Some(Value::Module(module)) => Ok(module),
        _ => bail!("module doesn't contain module `{name}`"),
    }
}

/// Turn a title into an URL fragment.
pub fn urlify(title: &str) -> EcoString {
    title
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '.' => c,
            _ => '-',
        })
        .collect()
}

/// Extract the first line of documentation.
fn oneliner(docs: &str) -> EcoString {
    let paragraph = docs.split("\n\n").next().unwrap_or_default();
    let mut depth = 0;
    let mut period = false;
    let mut end = paragraph.len();
    for (i, c) in paragraph.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '.' if depth == 0 => period = true,
            c if period && c.is_whitespace() && !docs[..i].ends_with("e.g.") => {
                end = i;
                break;
            }
            _ => period = false,
        }
    }
    EcoString::from(&docs[..end]).replace("\r\n", " ").replace("\n", " ")
}

/// The order of types in the documentation.
fn type_index(ty: &str) -> usize {
    TYPE_ORDER.iter().position(|&v| v == ty).unwrap_or(usize::MAX)
}

const TYPE_ORDER: &[&str] = &[
    "any",
    "none",
    "auto",
    "bool",
    "int",
    "float",
    "length",
    "angle",
    "ratio",
    "relative",
    "fraction",
    "color",
    "gradient",
    "datetime",
    "duration",
    "str",
    "bytes",
    "regex",
    "label",
    "content",
    "array",
    "dict",
    "func",
    "args",
    "selector",
    "location",
    "direction",
    "alignment",
    "alignment2d",
    "stroke",
];

fn math_class_name(class: MathClass) -> &'static str {
    match class {
        MathClass::Normal => "Normal",
        MathClass::Alphabetic => "Alphabetic",
        MathClass::Binary => "Binary",
        MathClass::Closing => "Closing",
        MathClass::Diacritic => "Diacritic",
        MathClass::Fence => "Fence",
        MathClass::GlyphPart => "Glyph Part",
        MathClass::Large => "Large",
        MathClass::Opening => "Opening",
        MathClass::Punctuation => "Punctuation",
        MathClass::Relation => "Relation",
        MathClass::Space => "Space",
        MathClass::Unary => "Unary",
        MathClass::Vary => "Vary",
        MathClass::Special => "Special",
    }
}

/// Data about a collection of functions.
#[derive(Debug, Clone, Deserialize)]
struct GroupData {
    name: EcoString,
    title: EcoString,
    category: Category,
    #[serde(default)]
    path: Vec<EcoString>,
    #[serde(default)]
    filter: Vec<EcoString>,
    details: EcoString,
}

impl GroupData {
    fn module(&self) -> &'static Module {
        let mut focus = &LIBRARY.global;
        for path in &self.path {
            focus = get_module(focus, path).unwrap();
        }
        focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docs() {
        provide(&TestResolver);
    }

    struct TestResolver;

    impl Resolver for TestResolver {
        fn link(&self, _: &str) -> Option<String> {
            None
        }

        fn example(&self, _: u128, _: Option<Html>, _: &PagedDocument) -> Html {
            Html::new(String::new())
        }

        fn image(&self, _: &str, _: &[u8]) -> String {
            String::new()
        }

        fn commits(&self, _: &str, _: &str) -> Vec<Commit> {
            vec![]
        }

        fn base(&self) -> &str {
            "/"
        }
    }
}
