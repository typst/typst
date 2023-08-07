//! Documentation provider for Typst.

mod contribs;
mod html;

pub use contribs::{contributors, Author, Commit};
pub use html::Html;

use std::fmt::{self, Debug, Formatter};
use std::path::Path;

use comemo::Prehashed;
use heck::ToTitleCase;
use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_yaml as yaml;
use typst::doc::Frame;
use typst::eval::{CastInfo, Func, FuncInfo, Library, Module, ParamInfo, Value};
use typst::font::{Font, FontBook};
use typst::geom::{Abs, Smart};
use typst_library::layout::{Margin, PageElem};
use unscanny::Scanner;

static DOCS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../docs");
static FILES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../assets/files");
static DETAILS: Lazy<yaml::Mapping> = Lazy::new(|| yaml("reference/details.yml"));
static GROUPS: Lazy<Vec<GroupData>> = Lazy::new(|| yaml("reference/groups.yml"));

static FONTS: Lazy<(Prehashed<FontBook>, Vec<Font>)> = Lazy::new(|| {
    static DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../assets/fonts");
    let fonts: Vec<_> = DIR
        .files()
        .flat_map(|file| Font::iter(file.contents().into()))
        .collect();
    let book = FontBook::from_fonts(&fonts);
    (Prehashed::new(book), fonts)
});

static LIBRARY: Lazy<Prehashed<Library>> = Lazy::new(|| {
    let mut lib = typst_library::build();
    // Hack for documenting the `mix` function in the color module.
    // Will be superseded by proper associated functions.
    lib.global
        .scope_mut()
        .define("mix", typst_library::compute::mix_func());
    lib.styles
        .set(PageElem::set_width(Smart::Custom(Abs::pt(240.0).into())));
    lib.styles.set(PageElem::set_height(Smart::Auto));
    lib.styles.set(PageElem::set_margin(Margin::splat(Some(Smart::Custom(
        Abs::pt(15.0).into(),
    )))));
    typst::eval::set_lang_items(lib.items.clone());
    Prehashed::new(lib)
});

/// Build documentation pages.
pub fn provide(resolver: &dyn Resolver) -> Vec<PageModel> {
    vec![
        markdown_page(resolver, "/docs/", "overview.md").with_route("/docs/"),
        tutorial_pages(resolver),
        reference_pages(resolver),
        guides_pages(resolver),
        packages_page(resolver),
        markdown_page(resolver, "/docs/", "changelog.md"),
        markdown_page(resolver, "/docs/", "roadmap.md"),
        markdown_page(resolver, "/docs/", "community.md"),
    ]
}

/// Resolve consumer dependencies.
pub trait Resolver {
    /// Try to resolve a link that the system cannot resolve itself.
    fn link(&self, link: &str) -> Option<String>;

    /// Produce an URL for an image file.
    fn image(&self, filename: &str, data: &[u8]) -> String;

    /// Produce HTML for an example.
    fn example(&self, hash: u128, source: Html, frames: &[Frame]) -> Html;

    /// Determine the commits between two tags.
    fn commits(&self, from: &str, to: &str) -> Vec<Commit>;
}

/// Details about a documentation page and its children.
#[derive(Debug, Serialize)]
pub struct PageModel {
    pub route: String,
    pub title: String,
    pub description: String,
    pub part: Option<&'static str>,
    pub outline: Vec<OutlineItem>,
    pub body: BodyModel,
    pub children: Vec<Self>,
}

impl PageModel {
    fn with_route(self, route: &str) -> Self {
        Self { route: route.into(), ..self }
    }

    fn with_part(self, part: &'static str) -> Self {
        Self { part: Some(part), ..self }
    }
}

/// An element in the "On This Page" outline.
#[derive(Debug, Clone, Serialize)]
pub struct OutlineItem {
    id: String,
    name: String,
    children: Vec<Self>,
}

/// Details about the body of a documentation page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind", content = "content")]
pub enum BodyModel {
    Html(Html),
    Category(CategoryModel),
    Func(FuncModel),
    Funcs(FuncsModel),
    Type(TypeModel),
    Symbols(SymbolsModel),
    Packages(Html),
}

/// Build the tutorial.
fn tutorial_pages(resolver: &dyn Resolver) -> PageModel {
    let mut page = markdown_page(resolver, "/docs/", "tutorial/welcome.md");
    page.children = DOCS
        .get_dir("tutorial")
        .unwrap()
        .files()
        .filter(|file| file.path() != Path::new("tutorial/welcome.md"))
        .map(|file| markdown_page(resolver, "/docs/tutorial/", file.path()))
        .collect();
    page
}

/// Build the reference.
fn reference_pages(resolver: &dyn Resolver) -> PageModel {
    let mut page = markdown_page(resolver, "/docs/", "reference/welcome.md");
    page.children = vec![
        markdown_page(resolver, "/docs/reference/", "reference/syntax.md")
            .with_part("Language"),
        markdown_page(resolver, "/docs/reference/", "reference/styling.md"),
        markdown_page(resolver, "/docs/reference/", "reference/scripting.md"),
        types_page(resolver, "/docs/reference/"),
        category_page(resolver, "text").with_part("Content"),
        category_page(resolver, "math"),
        category_page(resolver, "layout"),
        category_page(resolver, "visualize"),
        category_page(resolver, "meta"),
        category_page(resolver, "symbols"),
        category_page(resolver, "foundations").with_part("Compute"),
        category_page(resolver, "calculate"),
        category_page(resolver, "construct"),
        category_page(resolver, "data-loading"),
    ];
    page
}

/// Build the guides section.
fn guides_pages(resolver: &dyn Resolver) -> PageModel {
    let mut page = markdown_page(resolver, "/docs/", "guides/welcome.md");
    page.children =
        vec![markdown_page(resolver, "/docs/guides/", "guides/guide-for-latex-users.md")];
    page
}

/// Build the packages section.
fn packages_page(resolver: &dyn Resolver) -> PageModel {
    PageModel {
        route: "/docs/packages/".into(),
        title: "Packages".into(),
        description: "Packages for Typst.".into(),
        part: None,
        outline: vec![],
        body: BodyModel::Packages(Html::markdown(resolver, details("packages"))),
        children: vec![],
    }
}

/// Create a page from a markdown file.
#[track_caller]
fn markdown_page(
    resolver: &dyn Resolver,
    parent: &str,
    path: impl AsRef<Path>,
) -> PageModel {
    assert!(parent.starts_with('/') && parent.ends_with('/'));
    let md = DOCS.get_file(path).unwrap().contents_utf8().unwrap();
    let html = Html::markdown(resolver, md);
    let title = html.title().expect("chapter lacks a title").to_string();
    PageModel {
        route: format!("{parent}{}/", urlify(&title)),
        title,
        description: html.description().unwrap(),
        part: None,
        outline: html.outline(),
        body: BodyModel::Html(html),
        children: vec![],
    }
}

/// Details about a category.
#[derive(Debug, Serialize)]
pub struct CategoryModel {
    pub name: String,
    pub details: Html,
    pub kind: &'static str,
    pub items: Vec<CategoryItem>,
    pub markup_shorthands: Option<Vec<SymbolModel>>,
    pub math_shorthands: Option<Vec<SymbolModel>>,
}

/// Details about a category item.
#[derive(Debug, Serialize)]
pub struct CategoryItem {
    pub name: String,
    pub route: String,
    pub oneliner: String,
    pub code: bool,
}

/// Create a page for a category.
#[track_caller]
fn category_page(resolver: &dyn Resolver, category: &str) -> PageModel {
    let route = format!("/docs/reference/{category}/");
    let mut children = vec![];
    let mut items = vec![];

    let focus = match category {
        "math" => &LIBRARY.math,
        "calculate" => module(&LIBRARY.global, "calc").unwrap(),
        _ => &LIBRARY.global,
    };

    let parents: &[&str] = match category {
        "math" => &[],
        "calculate" => &["calc"],
        _ => &[],
    };

    let grouped = match category {
        "math" => GROUPS.as_slice(),
        _ => &[],
    };

    // Add functions.
    for (_, value) in focus.scope().iter() {
        let Value::Func(func) = value else { continue };
        let Some(info) = func.info() else { continue };
        if info.category != category {
            continue;
        }

        // Skip grouped functions.
        if grouped
            .iter()
            .flat_map(|group| &group.functions)
            .any(|f| f == info.name)
        {
            continue;
        }

        let subpage = function_page(resolver, &route, func, info, parents);
        items.push(CategoryItem {
            name: info.name.into(),
            route: subpage.route.clone(),
            oneliner: oneliner(info.docs).into(),
            code: true,
        });
        children.push(subpage);
    }

    // Add grouped functions.
    for group in grouped {
        let mut functions = vec![];
        let mut outline = vec![OutlineItem {
            id: "summary".into(),
            name: "Summary".into(),
            children: vec![],
        }];

        for name in &group.functions {
            let value = focus.get(name).unwrap();
            let Value::Func(func) = value else { panic!("not a function") };
            let info = func.info().unwrap();
            let func = func_model(resolver, func, info, &[], info.name);
            let id = urlify(&func.path.join("-"));
            let children = func_outline(&func, &id, false);
            outline.push(OutlineItem { id, name: func.display.into(), children });
            functions.push(func);
        }

        let route = format!("{}{}/", route, group.name);
        items.push(CategoryItem {
            name: group.name.clone(),
            route: route.clone(),
            oneliner: oneliner(&group.description).into(),
            code: false,
        });

        children.push(PageModel {
            route,
            title: group.display.clone(),
            description: format!("Documentation for {} group of functions.", group.name),
            part: None,
            outline,
            body: BodyModel::Funcs(FuncsModel {
                name: group.name.clone(),
                display: group.display.clone(),
                details: Html::markdown(resolver, &group.description),
                functions,
            }),
            children: vec![],
        });
    }

    children.sort_by_cached_key(|child| child.title.clone());
    items.sort_by_cached_key(|item| item.name.clone());

    // Add symbol pages. These are ordered manually.
    let mut markup_shorthands = vec![];
    let mut math_shorthands = vec![];
    if category == "symbols" {
        for module in ["sym", "emoji"] {
            let subpage = symbol_page(resolver, &route, module);
            let BodyModel::Symbols(model) = &subpage.body else { continue };
            let list = &model.list;
            markup_shorthands.extend(
                list.iter()
                    .filter(|symbol| symbol.markup_shorthand.is_some())
                    .cloned(),
            );
            math_shorthands.extend(
                list.iter().filter(|symbol| symbol.math_shorthand.is_some()).cloned(),
            );

            items.push(CategoryItem {
                name: module.into(),
                route: subpage.route.clone(),
                oneliner: oneliner(details(module)).into(),
                code: true,
            });
            children.push(subpage);
        }
    }

    let name = category.to_title_case();
    let kind = match category {
        "symbols" => "Modules",
        _ => "Functions",
    };

    PageModel {
        route,
        title: name.clone(),
        description: format!("Documentation for functions related to {name} in Typst."),
        part: None,
        outline: category_outline(kind),
        body: BodyModel::Category(CategoryModel {
            name,
            details: Html::markdown(resolver, details(category)),
            kind,
            items,
            markup_shorthands: Some(markup_shorthands),
            math_shorthands: Some(math_shorthands),
        }),
        children,
    }
}

/// Produce an outline for a category page.
fn category_outline(kind: &str) -> Vec<OutlineItem> {
    vec![
        OutlineItem {
            id: "summary".into(),
            name: "Summary".into(),
            children: vec![],
        },
        OutlineItem {
            id: urlify(kind),
            name: kind.into(),
            children: vec![],
        },
    ]
}

/// Details about a function.
#[derive(Debug, Serialize)]
pub struct FuncModel {
    pub path: Vec<&'static str>,
    pub display: &'static str,
    pub keywords: Option<&'static str>,
    pub oneliner: &'static str,
    pub element: bool,
    pub details: Html,
    pub params: Vec<ParamModel>,
    pub returns: Vec<&'static str>,
    pub methods: Vec<MethodModel>,
    pub scope: Vec<Self>,
}

/// Details about a group of functions.
#[derive(Debug, Serialize)]
pub struct FuncsModel {
    pub name: String,
    pub display: String,
    pub details: Html,
    pub functions: Vec<FuncModel>,
}

/// Create a page for a function.
fn function_page(
    resolver: &dyn Resolver,
    parent: &str,
    func: &Func,
    info: &FuncInfo,
    parents: &[&'static str],
) -> PageModel {
    let model = func_model(resolver, func, info, parents, "");
    PageModel {
        route: format!("{parent}{}/", urlify(info.name)),
        title: info.display.to_string(),
        description: format!("Documentation for the `{}` function.", info.name),
        part: None,
        outline: func_outline(&model, "", true),
        body: BodyModel::Func(model),
        children: vec![],
    }
}

/// Produce a function's model.
fn func_model(
    resolver: &dyn Resolver,
    func: &Func,
    info: &FuncInfo,
    parents: &[&'static str],
    id_base: &str,
) -> FuncModel {
    let mut s = unscanny::Scanner::new(info.docs);
    let docs = s.eat_until("\n## Methods").trim();

    let mut path = parents.to_vec();
    let mut name = info.name;
    for parent in parents.iter().rev() {
        name = name
            .strip_prefix(parent)
            .or(name.strip_prefix(parent.strip_suffix('s').unwrap_or(parent)))
            .unwrap_or(name)
            .trim_matches('-');
    }
    path.push(name);

    let scope = info
        .scope
        .iter()
        .filter_map(|(_, value)| {
            let Value::Func(func) = value else { return None };
            let info = func.info().unwrap();
            Some(func_model(resolver, func, info, &path, id_base))
        })
        .collect();

    let mut returns = vec![];
    casts(resolver, &mut returns, &mut vec![], &info.returns);
    returns.sort_by_key(|ty| type_index(ty));
    if returns == ["none"] {
        returns.clear();
    }

    FuncModel {
        path,
        display: info.display,
        keywords: info.keywords,
        oneliner: oneliner(docs),
        element: func.element().is_some(),
        details: Html::markdown_with_id_base(resolver, docs, id_base),
        params: info.params.iter().map(|param| param_model(resolver, param)).collect(),
        returns,
        methods: method_models(resolver, info.docs),
        scope,
    }
}

/// Produce an outline for a function page.
fn func_outline(model: &FuncModel, base: &str, summary: bool) -> Vec<OutlineItem> {
    let mut outline = vec![];

    if summary {
        outline.push(OutlineItem {
            id: "summary".into(),
            name: "Summary".into(),
            children: vec![],
        });
    }

    outline.extend(model.details.outline());

    if !model.params.is_empty() {
        let join = if base.is_empty() { "" } else { "-" };
        outline.push(OutlineItem {
            id: format!("{base}{join}parameters"),
            name: "Parameters".into(),
            children: model
                .params
                .iter()
                .map(|param| OutlineItem {
                    id: format!("{base}{join}parameters-{}", urlify(param.name)),
                    name: param.name.into(),
                    children: vec![],
                })
                .collect(),
        });
    }

    for func in &model.scope {
        let id = urlify(&func.path.join("-"));
        let children = func_outline(func, &id, false);
        outline.push(OutlineItem { id, name: func.display.into(), children })
    }

    outline.extend(methods_outline(&model.methods));
    outline
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
    pub string: String,
    pub details: Html,
}

/// Produce a parameter's model.
fn param_model(resolver: &dyn Resolver, info: &ParamInfo) -> ParamModel {
    let mut types = vec![];
    let mut strings = vec![];
    casts(resolver, &mut types, &mut strings, &info.cast);
    if !strings.is_empty() && !types.contains(&"string") {
        types.push("string");
    }
    types.sort_by_key(|ty| type_index(ty));

    let mut details = info.docs;
    let mut example = None;
    if let Some(mut i) = info.docs.find("```example") {
        while info.docs[..i].ends_with('`') {
            i -= 1;
        }
        details = &info.docs[..i];
        example = Some(&info.docs[i..]);
    }

    ParamModel {
        name: info.name,
        details: Html::markdown(resolver, details),
        example: example.map(|md| Html::markdown(resolver, md)),
        types,
        strings,
        default: info.default.map(|default| {
            let node = typst::syntax::parse_code(&default().repr());
            Html::new(typst::ide::highlight_html(&node))
        }),
        positional: info.positional,
        named: info.named,
        required: info.required,
        variadic: info.variadic,
        settable: info.settable,
    }
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
            string: string.to_string(),
            details: Html::markdown(resolver, docs),
        }),
        CastInfo::Value(..) => {}
        CastInfo::Type(ty) => types.push(ty),
        CastInfo::Union(options) => {
            for option in options {
                casts(resolver, types, strings, option);
            }
        }
    }
}

/// A collection of symbols.
#[derive(Debug, Serialize)]
pub struct TypeModel {
    pub name: String,
    pub oneliner: &'static str,
    pub details: Html,
    pub methods: Vec<MethodModel>,
}

/// Details about a built-in method on a type.
#[derive(Debug, Serialize)]
pub struct MethodModel {
    pub name: &'static str,
    pub details: Html,
    pub params: Vec<ParamModel>,
    pub returns: Vec<&'static str>,
}

/// Create a page for the types.
fn types_page(resolver: &dyn Resolver, parent: &str) -> PageModel {
    let route = format!("{parent}types/");
    let mut children = vec![];
    let mut items = vec![];

    for model in type_models(resolver) {
        let route = format!("{route}{}/", urlify(&model.name));
        items.push(CategoryItem {
            name: model.name.clone(),
            route: route.clone(),
            oneliner: model.oneliner.into(),
            code: true,
        });
        children.push(PageModel {
            route,
            title: model.name.to_title_case(),
            description: format!("Documentation for the `{}` type.", model.name),
            part: None,
            outline: type_outline(&model),
            body: BodyModel::Type(model),
            children: vec![],
        });
    }

    PageModel {
        route,
        title: "Types".into(),
        description: "Documentation for Typst's built-in types.".into(),
        part: None,
        outline: category_outline("Types"),
        body: BodyModel::Category(CategoryModel {
            name: "Types".into(),
            details: Html::markdown(resolver, details("types")),
            kind: "Types",
            items,
            markup_shorthands: None,
            math_shorthands: None,
        }),
        children,
    }
}

/// Produce the types' models.
fn type_models(resolver: &dyn Resolver) -> Vec<TypeModel> {
    let file = DOCS.get_file("reference/types.md").unwrap();
    let text = file.contents_utf8().unwrap();

    let mut s = unscanny::Scanner::new(text);
    let mut types = vec![];

    while s.eat_if("# ") {
        let part = s.eat_until("\n# ");
        types.push(type_model(resolver, part));
        s.eat_if('\n');
    }

    types
}

/// Produce a type's model.
fn type_model(resolver: &dyn Resolver, part: &'static str) -> TypeModel {
    let mut s = unscanny::Scanner::new(part);
    let display = s.eat_until('\n').trim();
    let docs = s.eat_until("\n## Methods").trim();
    TypeModel {
        name: display.to_lowercase(),
        oneliner: oneliner(docs),
        details: Html::markdown(resolver, docs),
        methods: method_models(resolver, part),
    }
}

/// Produce multiple methods' models.
fn method_models(resolver: &dyn Resolver, docs: &'static str) -> Vec<MethodModel> {
    let mut s = unscanny::Scanner::new(docs);
    s.eat_until("\n## Methods");
    s.eat_whitespace();

    let mut methods = vec![];
    if s.eat_if("## Methods") {
        s.eat_until("\n### ");
        while s.eat_if("\n### ") {
            methods.push(method_model(resolver, s.eat_until("\n### ")));
        }
    }

    methods
}

/// Produce a method's model.
fn method_model(resolver: &dyn Resolver, part: &'static str) -> MethodModel {
    let mut s = unscanny::Scanner::new(part);
    let mut params = vec![];
    let mut returns = vec![];

    let name = s.eat_until('(').trim();
    s.expect("()");
    let docs = s.eat_until("\n- ").trim();

    while s.eat_if("\n- ") {
        let name = s.eat_until(':');
        s.expect(": ");
        let types: Vec<_> =
            s.eat_until(['(', '\n']).split(" or ").map(str::trim).collect();
        if !types.iter().all(|ty| type_index(ty) != usize::MAX) {
            panic!(
                "unknown type in method {} parameter {}",
                name,
                types.iter().find(|ty| type_index(ty) == usize::MAX).unwrap()
            )
        }

        if name == "returns" {
            returns = types;
            continue;
        }

        s.expect('(');

        let mut named = false;
        let mut positional = false;
        let mut required = false;
        let mut variadic = false;
        for part in s.eat_until(')').split(',').map(str::trim) {
            match part {
                "named" => named = true,
                "positional" => positional = true,
                "required" => required = true,
                "variadic" => variadic = true,
                _ => panic!("unknown parameter flag {:?}", part),
            }
        }

        s.expect(')');

        params.push(ParamModel {
            name,
            details: Html::markdown(resolver, s.eat_until("\n- ").trim()),
            example: None,
            types,
            strings: vec![],
            default: None,
            positional,
            named,
            required,
            variadic,
            settable: false,
        });
    }

    MethodModel {
        name,
        details: Html::markdown(resolver, docs),
        params,
        returns,
    }
}

/// Produce an outline for a type page.
fn type_outline(model: &TypeModel) -> Vec<OutlineItem> {
    let mut outline = vec![OutlineItem {
        id: "summary".into(),
        name: "Summary".into(),
        children: vec![],
    }];

    outline.extend(methods_outline(&model.methods));
    outline
}

/// Produce an outline for a type's method.
fn methods_outline(methods: &[MethodModel]) -> Option<OutlineItem> {
    (!methods.is_empty()).then(|| OutlineItem {
        id: "methods".into(),
        name: "Methods".into(),
        children: methods.iter().map(method_outline).collect(),
    })
}

/// Produce an outline for a type's method.
fn method_outline(model: &MethodModel) -> OutlineItem {
    OutlineItem {
        id: format!("methods-{}", urlify(model.name)),
        name: model.name.into(),
        children: model
            .params
            .iter()
            .map(|param| OutlineItem {
                id: format!(
                    "methods-{}-parameters-{}",
                    urlify(model.name),
                    urlify(param.name)
                ),
                name: param.name.into(),
                children: vec![],
            })
            .collect(),
    }
}

/// A collection of symbols.
#[derive(Debug, Serialize)]
pub struct SymbolsModel {
    pub name: &'static str,
    pub details: Html,
    pub list: Vec<SymbolModel>,
}

/// Details about a symbol.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolModel {
    pub name: String,
    pub markup_shorthand: Option<&'static str>,
    pub math_shorthand: Option<&'static str>,
    pub codepoint: u32,
    pub accent: bool,
    pub unicode_name: Option<String>,
    pub alternates: Vec<String>,
}

/// Create a page for symbols.
fn symbol_page(resolver: &dyn Resolver, parent: &str, name: &str) -> PageModel {
    let module = module(&LIBRARY.global, name).unwrap();

    let mut list = vec![];
    for (name, value) in module.scope().iter() {
        let Value::Symbol(symbol) = value else { continue };
        let complete = |variant: &str| {
            if variant.is_empty() {
                name.into()
            } else {
                format!("{}.{}", name, variant)
            }
        };

        for (variant, c) in symbol.variants() {
            let shorthand = |list: &[(&'static str, char)]| {
                list.iter().copied().find(|&(_, x)| x == c).map(|(s, _)| s)
            };

            list.push(SymbolModel {
                name: complete(variant),
                markup_shorthand: shorthand(typst::syntax::ast::Shorthand::MARKUP_LIST),
                math_shorthand: shorthand(typst::syntax::ast::Shorthand::MATH_LIST),
                codepoint: c as u32,
                accent: typst::eval::Symbol::combining_accent(c).is_some(),
                unicode_name: unicode_names2::name(c)
                    .map(|s| s.to_string().to_title_case()),
                alternates: symbol
                    .variants()
                    .filter(|(other, _)| other != &variant)
                    .map(|(other, _)| complete(other))
                    .collect(),
            });
        }
    }

    let title = match name {
        "sym" => "General",
        "emoji" => "Emoji",
        _ => unreachable!(),
    };

    PageModel {
        route: format!("{parent}{name}/"),
        title: title.into(),
        description: format!("Documentation for the `{name}` module."),
        part: None,
        outline: vec![],
        body: BodyModel::Symbols(SymbolsModel {
            name: title,
            details: Html::markdown(resolver, details(name)),
            list,
        }),
        children: vec![],
    }
}

/// Data about a collection of functions.
#[derive(Debug, Deserialize)]
struct GroupData {
    name: String,
    display: String,
    functions: Vec<String>,
    description: String,
}

/// Extract a module from another module.
#[track_caller]
fn module<'a>(parent: &'a Module, name: &str) -> Result<&'a Module, String> {
    match parent.scope().get(name) {
        Some(Value::Module(module)) => Ok(module),
        _ => Err(format!("module doesn't contain module `{name}`")),
    }
}

/// Load YAML from a path.
#[track_caller]
fn yaml<T: DeserializeOwned>(path: &str) -> T {
    let file = DOCS.get_file(path).unwrap();
    yaml::from_slice(file.contents()).unwrap()
}

/// Load details for an identifying key.
#[track_caller]
fn details(key: &str) -> &str {
    DETAILS
        .get(&yaml::Value::String(key.into()))
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| panic!("missing details for {key}"))
}

/// Turn a title into an URL fragment.
pub fn urlify(title: &str) -> String {
    title
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| match c {
            'a'..='z' | '0'..='9' => c,
            _ => '-',
        })
        .collect()
}

/// Extract the first line of documentation.
fn oneliner(docs: &str) -> &str {
    docs.lines().next().unwrap_or_default()
}

/// The order of types in the documentation.
fn type_index(ty: &str) -> usize {
    TYPE_ORDER.iter().position(|&v| v == ty).unwrap_or(usize::MAX)
}

const TYPE_ORDER: &[&str] = &[
    "any",
    "none",
    "auto",
    "boolean",
    "integer",
    "float",
    "length",
    "angle",
    "ratio",
    "relative length",
    "fraction",
    "color",
    "datetime",
    "string",
    "bytes",
    "regex",
    "label",
    "content",
    "array",
    "dictionary",
    "function",
    "arguments",
    "location",
    "dir",
    "alignment",
    "2d alignment",
    "selector",
    "stroke",
];

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

        fn example(&self, _: u128, _: Html, _: &[Frame]) -> Html {
            Html::new(String::new())
        }

        fn image(&self, _: &str, _: &[u8]) -> String {
            String::new()
        }

        fn commits(&self, _: &str, _: &str) -> Vec<Commit> {
            vec![]
        }
    }
}
