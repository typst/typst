use std::cmp::Reverse;
use std::collections::{BTreeMap, HashSet};
use std::ffi::OsStr;

use ecow::{eco_format, EcoString};
use if_chain::if_chain;
use serde::{Deserialize, Serialize};
use typst::foundations::{
    fields_on, repr, AutoValue, CastInfo, Func, Label, NoneValue, ParamInfo, Repr,
    StyleChain, Styles, Type, Value,
};
use typst::layout::{Alignment, Dir, PagedDocument};
use typst::syntax::ast::AstNode;
use typst::syntax::{
    ast, is_id_continue, is_id_start, is_ident, FileId, LinkedNode, Side, Source,
    SyntaxKind,
};
use typst::text::RawElem;
use typst::visualize::Color;
use unscanny::Scanner;

use crate::utils::{
    check_value_recursively, globals, plain_docs_sentence, summarize_font_family,
};
use crate::{analyze_expr, analyze_import, analyze_labels, named_items, IdeWorld};

/// Autocomplete a cursor position in a source file.
///
/// Returns the position from which the completions apply and a list of
/// completions.
///
/// When `explicit` is `true`, the user requested the completion by pressing
/// control and space or something similar.
///
/// Passing a `document` (from a previous compilation) is optional, but enhances
/// the autocompletions. Label completions, for instance, are only generated
/// when the document is available.
pub fn autocomplete(
    world: &dyn IdeWorld,
    document: Option<&PagedDocument>,
    source: &Source,
    cursor: usize,
    explicit: bool,
) -> Option<(usize, Vec<Completion>)> {
    let leaf = LinkedNode::new(source.root()).leaf_at(cursor, Side::Before)?;
    let mut ctx =
        CompletionContext::new(world, document, source, &leaf, cursor, explicit)?;

    let _ = complete_comments(&mut ctx)
        || complete_field_accesses(&mut ctx)
        || complete_open_labels(&mut ctx)
        || complete_imports(&mut ctx)
        || complete_rules(&mut ctx)
        || complete_params(&mut ctx)
        || complete_markup(&mut ctx)
        || complete_math(&mut ctx)
        || complete_code(&mut ctx);

    Some((ctx.from, ctx.completions))
}

/// An autocompletion option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    /// The kind of item this completes to.
    pub kind: CompletionKind,
    /// The label the completion is shown with.
    pub label: EcoString,
    /// The completed version of the input, possibly described with snippet
    /// syntax like `${lhs} + ${rhs}`.
    ///
    /// Should default to the `label` if `None`.
    pub apply: Option<EcoString>,
    /// An optional short description, at most one sentence.
    pub detail: Option<EcoString>,
}

/// A kind of item that can be completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionKind {
    /// A syntactical structure.
    Syntax,
    /// A function.
    Func,
    /// A type.
    Type,
    /// A function parameter.
    Param,
    /// A constant.
    Constant,
    /// A file path.
    Path,
    /// A package.
    Package,
    /// A label.
    Label,
    /// A font family.
    Font,
    /// A symbol.
    Symbol(char),
}

/// Complete in comments. Or rather, don't!
fn complete_comments(ctx: &mut CompletionContext) -> bool {
    matches!(ctx.leaf.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment)
}

/// Complete in markup mode.
fn complete_markup(ctx: &mut CompletionContext) -> bool {
    // Bail if we aren't even in markup.
    if !matches!(
        ctx.leaf.parent_kind(),
        None | Some(SyntaxKind::Markup) | Some(SyntaxKind::Ref)
    ) {
        return false;
    }

    // Start of an interpolated identifier: "#|".
    if ctx.leaf.kind() == SyntaxKind::Hash {
        ctx.from = ctx.cursor;
        code_completions(ctx, true);
        return true;
    }

    // An existing identifier: "#pa|".
    if ctx.leaf.kind() == SyntaxKind::Ident {
        ctx.from = ctx.leaf.offset();
        code_completions(ctx, true);
        return true;
    }

    // Start of a reference: "@|" or "@he|".
    if ctx.leaf.kind() == SyntaxKind::RefMarker {
        ctx.from = ctx.leaf.offset() + 1;
        ctx.label_completions();
        return true;
    }

    // Behind a half-completed binding: "#let x = |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_leaf();
        if prev.kind() == SyntaxKind::Eq;
        if prev.parent_kind() == Some(SyntaxKind::LetBinding);
        then {
            ctx.from = ctx.cursor;
            code_completions(ctx, false);
            return true;
        }
    }

    // Behind a half-completed context block: "#context |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_leaf();
        if prev.kind() == SyntaxKind::Context;
        then {
            ctx.from = ctx.cursor;
            code_completions(ctx, false);
            return true;
        }
    }

    // Directly after a raw block.
    let mut s = Scanner::new(ctx.text);
    s.jump(ctx.leaf.offset());
    if s.eat_if("```") {
        s.eat_while('`');
        let start = s.cursor();
        if s.eat_if(is_id_start) {
            s.eat_while(is_id_continue);
        }
        if s.cursor() == ctx.cursor {
            ctx.from = start;
            ctx.raw_completions();
        }
        return true;
    }

    // Anywhere: "|".
    if ctx.explicit {
        ctx.from = ctx.cursor;
        markup_completions(ctx);
        return true;
    }

    false
}

/// Add completions for markup snippets.
#[rustfmt::skip]
fn markup_completions(ctx: &mut CompletionContext) {
    ctx.snippet_completion(
        "expression",
        "#${}",
        "Variables, function calls, blocks, and more.",
    );

    ctx.snippet_completion(
        "linebreak",
        "\\\n${}",
        "Inserts a forced linebreak.",
    );

    ctx.snippet_completion(
        "strong text",
        "*${strong}*",
        "Strongly emphasizes content by increasing the font weight.",
    );

    ctx.snippet_completion(
        "emphasized text",
        "_${emphasized}_",
        "Emphasizes content by setting it in italic font style.",
    );

    ctx.snippet_completion(
        "raw text",
        "`${text}`",
        "Displays text verbatim, in monospace.",
    );

    ctx.snippet_completion(
        "code listing",
        "```${lang}\n${code}\n```",
        "Inserts computer code with syntax highlighting.",
    );

    ctx.snippet_completion(
        "hyperlink",
        "https://${example.com}",
        "Links to a URL.",
    );

    ctx.snippet_completion(
        "label",
        "<${name}>",
        "Makes the preceding element referenceable.",
    );

    ctx.snippet_completion(
        "reference",
        "@${name}",
        "Inserts a reference to a label.",
    );

    ctx.snippet_completion(
        "heading",
        "= ${title}",
        "Inserts a section heading.",
    );

    ctx.snippet_completion(
        "list item",
        "- ${item}",
        "Inserts an item of a bullet list.",
    );

    ctx.snippet_completion(
        "enumeration item",
        "+ ${item}",
        "Inserts an item of a numbered list.",
    );

    ctx.snippet_completion(
        "enumeration item (numbered)",
        "${number}. ${item}",
        "Inserts an explicitly numbered list item.",
    );

    ctx.snippet_completion(
        "term list item",
        "/ ${term}: ${description}",
        "Inserts an item of a term list.",
    );

    ctx.snippet_completion(
        "math (inline)",
        "$${x}$",
        "Inserts an inline-level mathematical equation.",
    );

    ctx.snippet_completion(
        "math (block)",
        "$ ${sum_x^2} $",
        "Inserts a block-level mathematical equation.",
    );
}

/// Complete in math mode.
fn complete_math(ctx: &mut CompletionContext) -> bool {
    if !matches!(
        ctx.leaf.parent_kind(),
        Some(SyntaxKind::Equation)
            | Some(SyntaxKind::Math)
            | Some(SyntaxKind::MathFrac)
            | Some(SyntaxKind::MathAttach)
    ) {
        return false;
    }

    // Start of an interpolated identifier: "#|".
    if ctx.leaf.kind() == SyntaxKind::Hash {
        ctx.from = ctx.cursor;
        code_completions(ctx, true);
        return true;
    }

    // Behind existing atom or identifier: "$a|$" or "$abc|$".
    if matches!(
        ctx.leaf.kind(),
        SyntaxKind::Text | SyntaxKind::MathText | SyntaxKind::MathIdent
    ) {
        ctx.from = ctx.leaf.offset();
        math_completions(ctx);
        return true;
    }

    // Anywhere: "$|$".
    if ctx.explicit {
        ctx.from = ctx.cursor;
        math_completions(ctx);
        return true;
    }

    false
}

/// Add completions for math snippets.
#[rustfmt::skip]
fn math_completions(ctx: &mut CompletionContext) {
    ctx.scope_completions(true, |_| true);

    ctx.snippet_completion(
        "subscript",
        "${x}_${2:2}",
        "Sets something in subscript.",
    );

    ctx.snippet_completion(
        "superscript",
        "${x}^${2:2}",
        "Sets something in superscript.",
    );

    ctx.snippet_completion(
        "fraction",
        "${x}/${y}",
        "Inserts a fraction.",
    );
}

/// Complete field accesses.
fn complete_field_accesses(ctx: &mut CompletionContext) -> bool {
    // Used to determine whether trivia nodes are allowed before '.'.
    // During an inline expression in markup mode trivia nodes exit the inline expression.
    let in_markup: bool = matches!(
        ctx.leaf.parent_kind(),
        None | Some(SyntaxKind::Markup) | Some(SyntaxKind::Ref)
    );

    // Behind an expression plus dot: "emoji.|".
    if_chain! {
        if ctx.leaf.kind() == SyntaxKind::Dot
            || (matches!(ctx.leaf.kind(), SyntaxKind::Text | SyntaxKind::MathText)
                && ctx.leaf.text() == ".");
        if ctx.leaf.range().end == ctx.cursor;
        if let Some(prev) = ctx.leaf.prev_sibling();
        if !in_markup || prev.range().end == ctx.leaf.range().start;
        if prev.is::<ast::Expr>();
        if prev.parent_kind() != Some(SyntaxKind::Markup) ||
           prev.prev_sibling_kind() == Some(SyntaxKind::Hash);
        if let Some((value, styles)) = analyze_expr(ctx.world, &prev).into_iter().next();
        then {
            ctx.from = ctx.cursor;
            field_access_completions(ctx, &value, &styles);
            return true;
        }
    }

    // Behind a started field access: "emoji.fa|".
    if_chain! {
        if ctx.leaf.kind() == SyntaxKind::Ident;
        if let Some(prev) = ctx.leaf.prev_sibling();
        if prev.kind() == SyntaxKind::Dot;
        if let Some(prev_prev) = prev.prev_sibling();
        if prev_prev.is::<ast::Expr>();
        if let Some((value, styles)) = analyze_expr(ctx.world, &prev_prev).into_iter().next();
        then {
            ctx.from = ctx.leaf.offset();
            field_access_completions(ctx, &value, &styles);
            return true;
        }
    }

    false
}

/// Add completions for all fields on a value.
fn field_access_completions(
    ctx: &mut CompletionContext,
    value: &Value,
    styles: &Option<Styles>,
) {
    let scopes = {
        let ty = value.ty().scope();
        let elem = match value {
            Value::Content(content) => Some(content.elem().scope()),
            _ => None,
        };
        elem.into_iter().chain(Some(ty))
    };

    // Autocomplete methods from the element's or type's scope.
    for (name, binding) in scopes.flat_map(|scope| scope.iter()) {
        ctx.call_completion(name.clone(), binding.read());
    }

    if let Some(scope) = value.scope() {
        for (name, binding) in scope.iter() {
            ctx.call_completion(name.clone(), binding.read());
        }
    }

    for &field in fields_on(value.ty()) {
        // Complete the field name along with its value. Notes:
        // 1. No parentheses since function fields cannot currently be called
        // with method syntax;
        // 2. We can unwrap the field's value since it's a field belonging to
        // this value's type, so accessing it should not fail.
        ctx.value_completion(field, &value.field(field, ()).unwrap());
    }

    match value {
        Value::Symbol(symbol) => {
            for modifier in symbol.modifiers() {
                if let Ok(modified) = symbol.clone().modified(modifier) {
                    ctx.completions.push(Completion {
                        kind: CompletionKind::Symbol(modified.get()),
                        label: modifier.into(),
                        apply: None,
                        detail: None,
                    });
                }
            }
        }
        Value::Content(content) => {
            for (name, value) in content.fields() {
                ctx.value_completion(name, &value);
            }
        }
        Value::Dict(dict) => {
            for (name, value) in dict.iter() {
                ctx.value_completion(name.clone(), value);
            }
        }
        Value::Func(func) => {
            // Autocomplete get rules.
            if let Some((elem, styles)) = func.element().zip(styles.as_ref()) {
                for param in elem.params().iter().filter(|param| !param.required) {
                    if let Some(value) = elem.field_id(param.name).and_then(|id| {
                        elem.field_from_styles(id, StyleChain::new(styles)).ok()
                    }) {
                        ctx.value_completion(param.name, &value);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Complete half-finished labels.
fn complete_open_labels(ctx: &mut CompletionContext) -> bool {
    // A label anywhere in code: "(<la|".
    if ctx.leaf.kind().is_error() && ctx.leaf.text().starts_with('<') {
        ctx.from = ctx.leaf.offset() + 1;
        ctx.label_completions();
        return true;
    }

    false
}

/// Complete imports.
fn complete_imports(ctx: &mut CompletionContext) -> bool {
    // In an import path for a file or package:
    // "#import "|",
    if_chain! {
        if matches!(
            ctx.leaf.parent_kind(),
            Some(SyntaxKind::ModuleImport | SyntaxKind::ModuleInclude)
        );
        if let Some(ast::Expr::Str(str)) = ctx.leaf.cast();
        let value = str.get();
        then {
            ctx.from = ctx.leaf.offset();
            if value.starts_with('@') {
                let all_versions = value.contains(':');
                ctx.package_completions(all_versions);
            } else {
                ctx.file_completions_with_extensions(&["typ"]);
            }
            return true;
        }
    }

    // Behind an import list:
    // "#import "path.typ": |",
    // "#import "path.typ": a, b, |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_sibling();
        if let Some(ast::Expr::Import(import)) = prev.get().cast();
        if let Some(ast::Imports::Items(items)) = import.imports();
        if let Some(source) = prev.children().find(|child| child.is::<ast::Expr>());
        then {
            ctx.from = ctx.cursor;
            import_item_completions(ctx, items, &source);
            return true;
        }
    }

    // Behind a half-started identifier in an import list:
    // "#import "path.typ": thi|",
    if_chain! {
        if ctx.leaf.kind() == SyntaxKind::Ident;
        if let Some(parent) = ctx.leaf.parent();
        if parent.kind() == SyntaxKind::ImportItemPath;
        if let Some(grand) = parent.parent();
        if grand.kind() == SyntaxKind::ImportItems;
        if let Some(great) = grand.parent();
        if let Some(ast::Expr::Import(import)) = great.get().cast();
        if let Some(ast::Imports::Items(items)) = import.imports();
        if let Some(source) = great.children().find(|child| child.is::<ast::Expr>());
        then {
            ctx.from = ctx.leaf.offset();
            import_item_completions(ctx, items, &source);
            return true;
        }
    }

    false
}

/// Add completions for all exports of a module.
fn import_item_completions<'a>(
    ctx: &mut CompletionContext<'a>,
    existing: ast::ImportItems<'a>,
    source: &LinkedNode,
) {
    let Some(value) = analyze_import(ctx.world, source) else { return };
    let Some(scope) = value.scope() else { return };

    if existing.iter().next().is_none() {
        ctx.snippet_completion("*", "*", "Import everything.");
    }

    for (name, binding) in scope.iter() {
        if existing.iter().all(|item| item.original_name().as_str() != name) {
            ctx.value_completion(name.clone(), binding.read());
        }
    }
}

/// Complete set and show rules.
fn complete_rules(ctx: &mut CompletionContext) -> bool {
    // We don't want to complete directly behind the keyword.
    if !ctx.leaf.kind().is_trivia() {
        return false;
    }

    let Some(prev) = ctx.leaf.prev_leaf() else { return false };

    // Behind the set keyword: "set |".
    if matches!(prev.kind(), SyntaxKind::Set) {
        ctx.from = ctx.cursor;
        set_rule_completions(ctx);
        return true;
    }

    // Behind the show keyword: "show |".
    if matches!(prev.kind(), SyntaxKind::Show) {
        ctx.from = ctx.cursor;
        show_rule_selector_completions(ctx);
        return true;
    }

    // Behind a half-completed show rule: "show strong: |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_leaf();
        if matches!(prev.kind(), SyntaxKind::Colon);
        if matches!(prev.parent_kind(), Some(SyntaxKind::ShowRule));
        then {
            ctx.from = ctx.cursor;
            show_rule_recipe_completions(ctx);
            return true;
        }
    }

    false
}

/// Add completions for all functions from the global scope.
fn set_rule_completions(ctx: &mut CompletionContext) {
    ctx.scope_completions(true, |value| {
        matches!(
            value,
            Value::Func(func) if func.params()
                .unwrap_or_default()
                .iter()
                .any(|param| param.settable),
        )
    });
}

/// Add completions for selectors.
fn show_rule_selector_completions(ctx: &mut CompletionContext) {
    ctx.scope_completions(
        false,
        |value| matches!(value, Value::Func(func) if func.element().is_some()),
    );

    ctx.enrich("", ": ");

    ctx.snippet_completion(
        "text selector",
        "\"${text}\": ${}",
        "Replace occurrences of specific text.",
    );

    ctx.snippet_completion(
        "regex selector",
        "regex(\"${regex}\"): ${}",
        "Replace matches of a regular expression.",
    );
}

/// Add completions for recipes.
fn show_rule_recipe_completions(ctx: &mut CompletionContext) {
    ctx.snippet_completion(
        "replacement",
        "[${content}]",
        "Replace the selected element with content.",
    );

    ctx.snippet_completion(
        "replacement (string)",
        "\"${text}\"",
        "Replace the selected element with a string of text.",
    );

    ctx.snippet_completion(
        "transformation",
        "element => [${content}]",
        "Transform the element with a function.",
    );

    ctx.scope_completions(false, |value| matches!(value, Value::Func(_)));
}

/// Complete call and set rule parameters.
fn complete_params(ctx: &mut CompletionContext) -> bool {
    // Ensure that we are in a function call or set rule's argument list.
    let (callee, set, args, args_linked) = if_chain! {
        if let Some(parent) = ctx.leaf.parent();
        if let Some(parent) = match parent.kind() {
            SyntaxKind::Named => parent.parent(),
            _ => Some(parent),
        };
        if let Some(args) = parent.get().cast::<ast::Args>();
        if let Some(grand) = parent.parent();
        if let Some(expr) = grand.get().cast::<ast::Expr>();
        let set = matches!(expr, ast::Expr::Set(_));
        if let Some(callee) = match expr {
            ast::Expr::FuncCall(call) => Some(call.callee()),
            ast::Expr::Set(set) => Some(set.target()),
            _ => None,
        };
        then {
            (callee, set, args, parent)
        } else {
            return false;
        }
    };

    // Find the piece of syntax that decides what we're completing.
    let mut deciding = ctx.leaf.clone();
    while !matches!(
        deciding.kind(),
        SyntaxKind::LeftParen | SyntaxKind::Comma | SyntaxKind::Colon
    ) {
        let Some(prev) = deciding.prev_leaf() else { break };
        deciding = prev;
    }

    // Parameter values: "func(param:|)", "func(param: |)".
    if_chain! {
        if deciding.kind() == SyntaxKind::Colon;
        if let Some(prev) = deciding.prev_leaf();
        if let Some(param) = prev.get().cast::<ast::Ident>();
        then {
            if let Some(next) = deciding.next_leaf() {
                ctx.from = ctx.cursor.min(next.offset());
            }

            named_param_value_completions(ctx, callee, &param);
            return true;
        }
    }

    // Parameters: "func(|)", "func(hi|)", "func(12,|)".
    if_chain! {
        if matches!(deciding.kind(), SyntaxKind::LeftParen | SyntaxKind::Comma);
        if deciding.kind() != SyntaxKind::Comma || deciding.range().end < ctx.cursor;
        then {
            if let Some(next) = deciding.next_leaf() {
                ctx.from = ctx.cursor.min(next.offset());
            }

            param_completions(ctx, callee, set, args, args_linked);
            return true;
        }
    }

    false
}

/// Add completions for the parameters of a function.
fn param_completions<'a>(
    ctx: &mut CompletionContext<'a>,
    callee: ast::Expr<'a>,
    set: bool,
    args: ast::Args<'a>,
    args_linked: &'a LinkedNode<'a>,
) {
    let Some(func) = resolve_global_callee(ctx, callee) else { return };
    let Some(params) = func.params() else { return };

    // Determine which arguments are already present.
    let mut existing_positional = 0;
    let mut existing_named = HashSet::new();
    for arg in args.items() {
        match arg {
            ast::Arg::Pos(_) => {
                let Some(node) = args_linked.find(arg.span()) else { continue };
                if node.range().end < ctx.cursor {
                    existing_positional += 1;
                }
            }
            ast::Arg::Named(named) => {
                existing_named.insert(named.name().as_str());
            }
            _ => {}
        }
    }

    let mut skipped_positional = 0;
    for param in params {
        if set && !param.settable {
            continue;
        }

        if param.positional {
            if skipped_positional < existing_positional && !param.variadic {
                skipped_positional += 1;
                continue;
            }

            param_value_completions(ctx, func, param);
        }

        if param.named {
            if existing_named.contains(&param.name) {
                continue;
            }

            let apply = if param.name == "caption" {
                eco_format!("{}: [${{}}]", param.name)
            } else {
                eco_format!("{}: ${{}}", param.name)
            };

            ctx.completions.push(Completion {
                kind: CompletionKind::Param,
                label: param.name.into(),
                apply: Some(apply),
                detail: Some(plain_docs_sentence(param.docs)),
            });
        }
    }

    if ctx.before.ends_with(',') {
        ctx.enrich(" ", "");
    }
}

/// Add completions for the values of a named function parameter.
fn named_param_value_completions<'a>(
    ctx: &mut CompletionContext<'a>,
    callee: ast::Expr<'a>,
    name: &str,
) {
    let Some(func) = resolve_global_callee(ctx, callee) else { return };
    let Some(param) = func.param(name) else { return };
    if !param.named {
        return;
    }

    param_value_completions(ctx, func, param);

    if ctx.before.ends_with(':') {
        ctx.enrich(" ", "");
    }
}

/// Add completions for the values of a parameter.
fn param_value_completions<'a>(
    ctx: &mut CompletionContext<'a>,
    func: &Func,
    param: &'a ParamInfo,
) {
    if param.name == "font" {
        ctx.font_completions();
    } else if let Some(extensions) = path_completion(func, param) {
        ctx.file_completions_with_extensions(extensions);
    } else if func.name() == Some("figure") && param.name == "body" {
        ctx.snippet_completion("image", "image(\"${}\"),", "An image in a figure.");
        ctx.snippet_completion("table", "table(\n  ${}\n),", "A table in a figure.");
    }

    ctx.cast_completions(&param.input);
}

/// Returns which file extensions to complete for the given parameter if any.
fn path_completion(func: &Func, param: &ParamInfo) -> Option<&'static [&'static str]> {
    Some(match (func.name(), param.name) {
        (Some("image"), "source") => &["png", "jpg", "jpeg", "gif", "svg", "svgz"],
        (Some("csv"), "source") => &["csv"],
        (Some("plugin"), "source") => &["wasm"],
        (Some("cbor"), "source") => &["cbor"],
        (Some("json"), "source") => &["json"],
        (Some("toml"), "source") => &["toml"],
        (Some("xml"), "source") => &["xml"],
        (Some("yaml"), "source") => &["yml", "yaml"],
        (Some("bibliography"), "sources") => &["bib", "yml", "yaml"],
        (Some("bibliography"), "style") => &["csl"],
        (Some("cite"), "style") => &["csl"],
        (Some("raw"), "syntaxes") => &["sublime-syntax"],
        (Some("raw"), "theme") => &["tmtheme"],
        (Some("embed"), "path") => &[],
        (None, "path") => &[],
        _ => return None,
    })
}

/// Resolve a callee expression to a global function.
fn resolve_global_callee<'a>(
    ctx: &CompletionContext<'a>,
    callee: ast::Expr<'a>,
) -> Option<&'a Func> {
    let globals = globals(ctx.world, ctx.leaf);
    let value = match callee {
        ast::Expr::Ident(ident) => globals.get(&ident)?.read(),
        ast::Expr::FieldAccess(access) => match access.target() {
            ast::Expr::Ident(target) => {
                globals.get(&target)?.read().scope()?.get(&access.field())?.read()
            }
            _ => return None,
        },
        _ => return None,
    };

    match value {
        Value::Func(func) => Some(func),
        _ => None,
    }
}

/// Complete in code mode.
fn complete_code(ctx: &mut CompletionContext) -> bool {
    if matches!(
        ctx.leaf.parent_kind(),
        None | Some(SyntaxKind::Markup)
            | Some(SyntaxKind::Math)
            | Some(SyntaxKind::MathFrac)
            | Some(SyntaxKind::MathAttach)
            | Some(SyntaxKind::MathRoot)
    ) {
        return false;
    }

    // An existing identifier: "{ pa| }".
    if ctx.leaf.kind() == SyntaxKind::Ident {
        ctx.from = ctx.leaf.offset();
        code_completions(ctx, false);
        return true;
    }

    // A potential label (only at the start of an argument list): "(<|".
    if ctx.before.ends_with("(<") {
        ctx.from = ctx.cursor;
        ctx.label_completions();
        return true;
    }

    // Anywhere: "{ | }".
    // But not within or after an expression.
    if ctx.explicit
        && (ctx.leaf.kind().is_trivia()
            || matches!(ctx.leaf.kind(), SyntaxKind::LeftParen | SyntaxKind::LeftBrace))
    {
        ctx.from = ctx.cursor;
        code_completions(ctx, false);
        return true;
    }

    false
}

/// Add completions for expression snippets.
#[rustfmt::skip]
fn code_completions(ctx: &mut CompletionContext, hash: bool) {
    if hash {
        ctx.scope_completions(true, |value| {
            // If we are in markup, ignore colors, directions, and alignments.
            // They are useless and bloat the autocomplete results.
            let ty = value.ty();
            ty != Type::of::<Color>()
                && ty != Type::of::<Dir>()
                && ty != Type::of::<Alignment>()
        });
    } else {
        ctx.scope_completions(true, |_| true);
    }

    ctx.snippet_completion(
        "function call",
        "${function}(${arguments})[${body}]",
        "Evaluates a function.",
    );

    ctx.snippet_completion(
        "code block",
        "{ ${} }",
        "Inserts a nested code block.",
    );

    ctx.snippet_completion(
        "content block",
        "[${content}]",
        "Switches into markup mode.",
    );

    ctx.snippet_completion(
        "set rule",
        "set ${}",
        "Sets style properties on an element.",
    );

    ctx.snippet_completion(
        "show rule",
        "show ${}",
        "Redefines the look of an element.",
    );

    ctx.snippet_completion(
        "show rule (everything)",
        "show: ${}",
        "Transforms everything that follows.",
    );

    ctx.snippet_completion(
        "context expression",
        "context ${}",
        "Provides contextual data.",
    );

    ctx.snippet_completion(
        "let binding",
        "let ${name} = ${value}",
        "Saves a value in a variable.",
    );

    ctx.snippet_completion(
        "let binding (function)",
        "let ${name}(${params}) = ${output}",
        "Defines a function.",
    );

    ctx.snippet_completion(
        "if conditional",
        "if ${1 < 2} {\n\t${}\n}",
        "Computes or inserts something conditionally.",
    );

    ctx.snippet_completion(
        "if-else conditional",
        "if ${1 < 2} {\n\t${}\n} else {\n\t${}\n}",
        "Computes or inserts different things based on a condition.",
    );

    ctx.snippet_completion(
        "while loop",
        "while ${1 < 2} {\n\t${}\n}",
        "Computes or inserts something while a condition is met.",
    );

    ctx.snippet_completion(
        "for loop",
        "for ${value} in ${(1, 2, 3)} {\n\t${}\n}",
        "Computes or inserts something for each value in a collection.",
    );

    ctx.snippet_completion(
        "for loop (with key)",
        "for (${key}, ${value}) in ${(a: 1, b: 2)} {\n\t${}\n}",
        "Computes or inserts something for each key and value in a collection.",
    );

    ctx.snippet_completion(
        "break",
        "break",
        "Exits early from a loop.",
    );

    ctx.snippet_completion(
        "continue",
        "continue",
        "Continues with the next iteration of a loop.",
    );

    ctx.snippet_completion(
        "return",
        "return ${output}",
        "Returns early from a function.",
    );

    ctx.snippet_completion(
        "import (file)",
        "import \"${}\": ${}",
        "Imports variables from another file.",
    );

    ctx.snippet_completion(
        "import (package)",
        "import \"@${}\": ${}",
        "Imports variables from a package.",
    );

    ctx.snippet_completion(
        "include (file)",
        "include \"${}\"",
        "Includes content from another file.",
    );

    ctx.snippet_completion(
        "array literal",
        "(${1, 2, 3})",
        "Creates a sequence of values.",
    );

    ctx.snippet_completion(
        "dictionary literal",
        "(${a: 1, b: 2})",
        "Creates a mapping from names to value.",
    );

    if !hash {
        ctx.snippet_completion(
            "function",
            "(${params}) => ${output}",
            "Creates an unnamed function.",
        );
    }
}

/// Context for autocompletion.
struct CompletionContext<'a> {
    world: &'a (dyn IdeWorld + 'a),
    document: Option<&'a PagedDocument>,
    text: &'a str,
    before: &'a str,
    after: &'a str,
    leaf: &'a LinkedNode<'a>,
    cursor: usize,
    explicit: bool,
    from: usize,
    completions: Vec<Completion>,
    seen_casts: HashSet<u128>,
}

impl<'a> CompletionContext<'a> {
    /// Create a new autocompletion context.
    fn new(
        world: &'a (dyn IdeWorld + 'a),
        document: Option<&'a PagedDocument>,
        source: &'a Source,
        leaf: &'a LinkedNode<'a>,
        cursor: usize,
        explicit: bool,
    ) -> Option<Self> {
        let text = source.text();
        Some(Self {
            world,
            document,
            text,
            before: &text[..cursor],
            after: &text[cursor..],
            leaf,
            cursor,
            explicit,
            from: cursor,
            completions: vec![],
            seen_casts: HashSet::new(),
        })
    }

    /// A small window of context before the cursor.
    fn before_window(&self, size: usize) -> &str {
        Scanner::new(self.before).get(self.cursor.saturating_sub(size)..self.cursor)
    }

    /// Add a prefix and suffix to all applications.
    fn enrich(&mut self, prefix: &str, suffix: &str) {
        for Completion { label, apply, .. } in &mut self.completions {
            let current = apply.as_ref().unwrap_or(label);
            *apply = Some(eco_format!("{prefix}{current}{suffix}"));
        }
    }

    /// Add a snippet completion.
    fn snippet_completion(
        &mut self,
        label: &'static str,
        snippet: &'static str,
        docs: &'static str,
    ) {
        self.completions.push(Completion {
            kind: CompletionKind::Syntax,
            label: label.into(),
            apply: Some(snippet.into()),
            detail: Some(docs.into()),
        });
    }

    /// Add completions for all font families.
    fn font_completions(&mut self) {
        let equation = self.before_window(25).contains("equation");
        for (family, iter) in self.world.book().families() {
            let detail = summarize_font_family(iter);
            if !equation || family.contains("Math") {
                self.str_completion(
                    family,
                    Some(CompletionKind::Font),
                    Some(detail.as_str()),
                );
            }
        }
    }

    /// Add completions for all available packages.
    fn package_completions(&mut self, all_versions: bool) {
        let mut packages: Vec<_> = self.world.packages().iter().collect();
        packages.sort_by_key(|(spec, _)| {
            (&spec.namespace, &spec.name, Reverse(spec.version))
        });
        if !all_versions {
            packages.dedup_by_key(|(spec, _)| (&spec.namespace, &spec.name));
        }
        for (package, description) in packages {
            self.str_completion(
                eco_format!("{package}"),
                Some(CompletionKind::Package),
                description.as_deref(),
            );
        }
    }

    /// Add completions for all available files.
    fn file_completions(&mut self, mut filter: impl FnMut(FileId) -> bool) {
        let Some(base_id) = self.leaf.span().id() else { return };
        let Some(base_path) = base_id.vpath().as_rooted_path().parent() else { return };

        let mut paths: Vec<EcoString> = self
            .world
            .files()
            .iter()
            .filter(|&&file_id| file_id != base_id && filter(file_id))
            .filter_map(|file_id| {
                let file_path = file_id.vpath().as_rooted_path();
                pathdiff::diff_paths(file_path, base_path)
            })
            .map(|path| path.to_string_lossy().replace('\\', "/").into())
            .collect();

        paths.sort();

        for path in paths {
            self.str_completion(path, Some(CompletionKind::Path), None);
        }
    }

    /// Add completions for all files with any of the given extensions.
    ///
    /// If the array is empty, all extensions are allowed.
    fn file_completions_with_extensions(&mut self, extensions: &[&str]) {
        if extensions.is_empty() {
            self.file_completions(|_| true);
        }
        self.file_completions(|id| {
            let ext = id
                .vpath()
                .as_rooted_path()
                .extension()
                .and_then(OsStr::to_str)
                .map(EcoString::from)
                .unwrap_or_default()
                .to_lowercase();
            extensions.contains(&ext.as_str())
        });
    }

    /// Add completions for raw block tags.
    fn raw_completions(&mut self) {
        for (name, mut tags) in RawElem::languages() {
            let lower = name.to_lowercase();
            if !tags.contains(&lower.as_str()) {
                tags.push(lower.as_str());
            }

            tags.retain(|tag| is_ident(tag));
            if tags.is_empty() {
                continue;
            }

            self.completions.push(Completion {
                kind: CompletionKind::Constant,
                label: name.into(),
                apply: Some(tags[0].into()),
                detail: Some(repr::separated_list(&tags, " or ").into()),
            });
        }
    }

    /// Add completions for labels and references.
    fn label_completions(&mut self) {
        let Some(document) = self.document else { return };
        let (labels, split) = analyze_labels(document);

        let head = &self.text[..self.from];
        let at = head.ends_with('@');
        let open = !at && !head.ends_with('<');
        let close = !at && !self.after.starts_with('>');
        let citation = !at && self.before_window(15).contains("cite");

        let (skip, take) = if at {
            (0, usize::MAX)
        } else if citation {
            (split, usize::MAX)
        } else {
            (0, split)
        };

        for (label, detail) in labels.into_iter().skip(skip).take(take) {
            self.completions.push(Completion {
                kind: CompletionKind::Label,
                apply: (open || close).then(|| {
                    eco_format!(
                        "{}{}{}",
                        if open { "<" } else { "" },
                        label.resolve(),
                        if close { ">" } else { "" }
                    )
                }),
                label: label.resolve().as_str().into(),
                detail,
            });
        }
    }

    /// Add a completion for an arbitrary value.
    fn value_completion(&mut self, label: impl Into<EcoString>, value: &Value) {
        self.value_completion_full(Some(label.into()), value, false, None, None);
    }

    /// Add a completion for an arbitrary value, adding parentheses if it's a function.
    fn call_completion(&mut self, label: impl Into<EcoString>, value: &Value) {
        self.value_completion_full(Some(label.into()), value, true, None, None);
    }

    /// Add a completion for a specific string literal.
    fn str_completion(
        &mut self,
        string: impl Into<EcoString>,
        kind: Option<CompletionKind>,
        detail: Option<&str>,
    ) {
        let string = string.into();
        self.value_completion_full(None, &Value::Str(string.into()), false, kind, detail);
    }

    /// Add a completion for a specific value.
    fn value_completion_full(
        &mut self,
        label: Option<EcoString>,
        value: &Value,
        parens: bool,
        kind: Option<CompletionKind>,
        detail: Option<&str>,
    ) {
        let at = label.as_deref().is_some_and(|field| !is_ident(field));
        let label = label.unwrap_or_else(|| value.repr());

        let detail = detail.map(Into::into).or_else(|| match value {
            Value::Symbol(_) => None,
            Value::Func(func) => func.docs().map(plain_docs_sentence),
            Value::Type(ty) => Some(plain_docs_sentence(ty.docs())),
            v => {
                let repr = v.repr();
                (repr.as_str() != label).then_some(repr)
            }
        });

        let mut apply = None;
        if parens
            && matches!(value, Value::Func(_))
            && !self.after.starts_with(['(', '['])
        {
            if let Value::Func(func) = value {
                apply = Some(match BracketMode::of(func) {
                    BracketMode::RoundAfter => eco_format!("{label}()${{}}"),
                    BracketMode::RoundWithin => eco_format!("{label}(${{}})"),
                    BracketMode::RoundNewline => eco_format!("{label}(\n  ${{}}\n)"),
                    BracketMode::SquareWithin => eco_format!("{label}[${{}}]"),
                });
            }
        } else if at {
            apply = Some(eco_format!("at(\"{label}\")"));
        } else if label.starts_with('"') && self.after.starts_with('"') {
            if let Some(trimmed) = label.strip_suffix('"') {
                apply = Some(trimmed.into());
            }
        }

        self.completions.push(Completion {
            kind: kind.unwrap_or_else(|| match value {
                Value::Func(_) => CompletionKind::Func,
                Value::Type(_) => CompletionKind::Type,
                Value::Symbol(s) => CompletionKind::Symbol(s.get()),
                _ => CompletionKind::Constant,
            }),
            label,
            apply,
            detail,
        });
    }

    /// Add completions for a castable.
    fn cast_completions(&mut self, cast: &'a CastInfo) {
        // Prevent duplicate completions from appearing.
        if !self.seen_casts.insert(typst::utils::hash128(cast)) {
            return;
        }

        match cast {
            CastInfo::Any => {}
            CastInfo::Value(value, docs) => {
                self.value_completion_full(None, value, false, None, Some(docs));
            }
            CastInfo::Type(ty) => {
                if *ty == Type::of::<NoneValue>() {
                    self.snippet_completion("none", "none", "Nothing.")
                } else if *ty == Type::of::<AutoValue>() {
                    self.snippet_completion("auto", "auto", "A smart default.");
                } else if *ty == Type::of::<bool>() {
                    self.snippet_completion("false", "false", "No / Disabled.");
                    self.snippet_completion("true", "true", "Yes / Enabled.");
                } else if *ty == Type::of::<Color>() {
                    self.snippet_completion(
                        "luma()",
                        "luma(${v})",
                        "A custom grayscale color.",
                    );
                    self.snippet_completion(
                        "rgb()",
                        "rgb(${r}, ${g}, ${b}, ${a})",
                        "A custom RGBA color.",
                    );
                    self.snippet_completion(
                        "cmyk()",
                        "cmyk(${c}, ${m}, ${y}, ${k})",
                        "A custom CMYK color.",
                    );
                    self.snippet_completion(
                        "oklab()",
                        "oklab(${l}, ${a}, ${b}, ${alpha})",
                        "A custom Oklab color.",
                    );
                    self.snippet_completion(
                        "oklch()",
                        "oklch(${l}, ${chroma}, ${hue}, ${alpha})",
                        "A custom Oklch color.",
                    );
                    self.snippet_completion(
                        "color.linear-rgb()",
                        "color.linear-rgb(${r}, ${g}, ${b}, ${a})",
                        "A custom linear RGBA color.",
                    );
                    self.snippet_completion(
                        "color.hsv()",
                        "color.hsv(${h}, ${s}, ${v}, ${a})",
                        "A custom HSVA color.",
                    );
                    self.snippet_completion(
                        "color.hsl()",
                        "color.hsl(${h}, ${s}, ${l}, ${a})",
                        "A custom HSLA color.",
                    );
                    self.scope_completions(false, |value| value.ty() == *ty);
                } else if *ty == Type::of::<Label>() {
                    self.label_completions()
                } else if *ty == Type::of::<Func>() {
                    self.snippet_completion(
                        "function",
                        "(${params}) => ${output}",
                        "A custom function.",
                    );
                } else {
                    self.completions.push(Completion {
                        kind: CompletionKind::Syntax,
                        label: ty.long_name().into(),
                        apply: Some(eco_format!("${{{ty}}}")),
                        detail: Some(eco_format!("A value of type {ty}.")),
                    });
                    self.scope_completions(false, |value| value.ty() == *ty);
                }
            }
            CastInfo::Union(union) => {
                for info in union {
                    self.cast_completions(info);
                }
            }
        }
    }

    /// Add completions for definitions that are available at the cursor.
    ///
    /// Filters the global/math scope with the given filter.
    fn scope_completions(&mut self, parens: bool, filter: impl Fn(&Value) -> bool) {
        // When any of the constituent parts of the value matches the filter,
        // that's ok as well. For example, when autocompleting `#rect(fill: |)`,
        // we propose colors, but also dictionaries and modules that contain
        // colors.
        let filter = |value: &Value| check_value_recursively(value, &filter);

        let mut defined = BTreeMap::<EcoString, Option<Value>>::new();
        named_items(self.world, self.leaf.clone(), |item| {
            let name = item.name();
            if !name.is_empty() && item.value().as_ref().map_or(true, filter) {
                defined.insert(name.clone(), item.value());
            }

            None::<()>
        });

        for (name, value) in &defined {
            if let Some(value) = value {
                self.value_completion(name.clone(), value);
            } else {
                self.completions.push(Completion {
                    kind: CompletionKind::Constant,
                    label: name.clone(),
                    apply: None,
                    detail: None,
                });
            }
        }

        for (name, binding) in globals(self.world, self.leaf).iter() {
            let value = binding.read();
            if filter(value) && !defined.contains_key(name) {
                self.value_completion_full(Some(name.clone()), value, parens, None, None);
            }
        }
    }
}

/// What kind of parentheses to autocomplete for a function.
enum BracketMode {
    /// Round parenthesis, with the cursor within: `(|)`.
    RoundWithin,
    /// Round parenthesis, with the cursor after them: `()|`.
    RoundAfter,
    /// Round parenthesis, with newlines and indent.
    RoundNewline,
    /// Square brackets, with the cursor within: `[|]`.
    SquareWithin,
}

impl BracketMode {
    fn of(func: &Func) -> Self {
        if func
            .params()
            .is_some_and(|params| params.iter().all(|param| param.name == "self"))
        {
            return Self::RoundAfter;
        }

        match func.name() {
            Some(
                "emph" | "footnote" | "quote" | "strong" | "highlight" | "overline"
                | "underline" | "smallcaps" | "strike" | "sub" | "super",
            ) => Self::SquareWithin,
            Some("colbreak" | "parbreak" | "linebreak" | "pagebreak") => Self::RoundAfter,
            Some("figure" | "table" | "grid" | "stack") => Self::RoundNewline,
            _ => Self::RoundWithin,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;
    use std::collections::BTreeSet;

    use typst::layout::PagedDocument;

    use super::{autocomplete, Completion};
    use crate::tests::{FilePos, TestWorld, WorldLike};

    /// Quote a string.
    macro_rules! q {
        ($s:literal) => {
            concat!("\"", $s, "\"")
        };
    }

    type Response = Option<(usize, Vec<Completion>)>;

    trait ResponseExt {
        fn completions(&self) -> &[Completion];
        fn labels(&self) -> BTreeSet<&str>;
        fn must_include<'a>(&self, includes: impl IntoIterator<Item = &'a str>) -> &Self;
        fn must_exclude<'a>(&self, excludes: impl IntoIterator<Item = &'a str>) -> &Self;
        fn must_apply<'a>(&self, label: &str, apply: impl Into<Option<&'a str>>)
            -> &Self;
    }

    impl ResponseExt for Response {
        fn completions(&self) -> &[Completion] {
            match self {
                Some((_, completions)) => completions.as_slice(),
                None => &[],
            }
        }

        fn labels(&self) -> BTreeSet<&str> {
            self.completions().iter().map(|c| c.label.as_str()).collect()
        }

        #[track_caller]
        fn must_include<'a>(&self, includes: impl IntoIterator<Item = &'a str>) -> &Self {
            let labels = self.labels();
            for item in includes {
                assert!(
                    labels.contains(item),
                    "{item:?} was not contained in {labels:?}",
                );
            }
            self
        }

        #[track_caller]
        fn must_exclude<'a>(&self, excludes: impl IntoIterator<Item = &'a str>) -> &Self {
            let labels = self.labels();
            for item in excludes {
                assert!(
                    !labels.contains(item),
                    "{item:?} was wrongly contained in {labels:?}",
                );
            }
            self
        }

        #[track_caller]
        fn must_apply<'a>(
            &self,
            label: &str,
            apply: impl Into<Option<&'a str>>,
        ) -> &Self {
            let Some(completion) = self.completions().iter().find(|c| c.label == label)
            else {
                panic!("found no completion for {label:?}");
            };
            assert_eq!(completion.apply.as_deref(), apply.into());
            self
        }
    }

    #[track_caller]
    fn test(world: impl WorldLike, pos: impl FilePos) -> Response {
        let world = world.acquire();
        let world = world.borrow();
        let doc = typst::compile(world).output.ok();
        test_with_doc(world, pos, doc.as_ref())
    }

    #[track_caller]
    fn test_with_doc(
        world: impl WorldLike,
        pos: impl FilePos,
        doc: Option<&PagedDocument>,
    ) -> Response {
        let world = world.acquire();
        let world = world.borrow();
        let (source, cursor) = pos.resolve(world);
        autocomplete(world, doc, &source, cursor, true)
    }

    #[test]
    fn test_autocomplete_hash_expr() {
        test("#i", -1).must_include(["int", "if conditional"]);
    }

    #[test]
    fn test_autocomplete_array_method() {
        test("#().", -1).must_include(["insert", "remove", "len", "all"]);
        test("#{ let x = (1, 2, 3); x. }", -3).must_include(["at", "push", "pop"]);
    }

    /// Test that extra space before '.' is handled correctly.
    #[test]
    fn test_autocomplete_whitespace() {
        test("#() .", -1).must_exclude(["insert", "remove", "len", "all"]);
        test("#{() .}", -2).must_include(["insert", "remove", "len", "all"]);
        test("#() .a", -1).must_exclude(["insert", "remove", "len", "all"]);
        test("#{() .a}", -2).must_include(["at", "any", "all"]);
    }

    /// Test that the `before_window` doesn't slice into invalid byte
    /// boundaries.
    #[test]
    fn test_autocomplete_before_window_char_boundary() {
        test("😀😀     #text(font: \"\")", -3);
    }

    /// Ensure that autocompletion for `#cite(|)` completes bibligraphy labels,
    /// but no other labels.
    #[test]
    fn test_autocomplete_cite_function() {
        // First compile a working file to get a document.
        let mut world =
            TestWorld::new("#bibliography(\"works.bib\") <bib>").with_asset("works.bib");
        let doc = typst::compile(&world).output.ok();

        // Then, add the invalid `#cite` call. Had the document been invalid
        // initially, we would have no populated document to autocomplete with.
        let end = world.main.len_bytes();
        world.main.edit(end..end, " #cite()");

        test_with_doc(&world, -2, doc.as_ref())
            .must_include(["netwok", "glacier-melt", "supplement"])
            .must_exclude(["bib"]);
    }

    /// Test what kind of brackets we autocomplete for function calls depending
    /// on the function and existing parens.
    #[test]
    fn test_autocomplete_bracket_mode() {
        test("#", 1).must_apply("list", "list(${})");
        test("#", 1).must_apply("linebreak", "linebreak()${}");
        test("#", 1).must_apply("strong", "strong[${}]");
        test("#", 1).must_apply("footnote", "footnote[${}]");
        test("#", 1).must_apply("figure", "figure(\n  ${}\n)");
        test("#", 1).must_apply("table", "table(\n  ${}\n)");
        test("#()", 1).must_apply("list", None);
        test("#[]", 1).must_apply("strong", None);
    }

    /// Test that we only complete positional parameters if they aren't
    /// already present.
    #[test]
    fn test_autocomplete_positional_param() {
        // No string given yet.
        test("#numbering()", -2).must_include(["string", "integer"]);
        // String is already given.
        test("#numbering(\"foo\", )", -2)
            .must_include(["integer"])
            .must_exclude(["string"]);
        // Integer is already given, but numbering is variadic.
        test("#numbering(\"foo\", 1, )", -2)
            .must_include(["integer"])
            .must_exclude(["string"]);
    }

    /// Test that autocompletion for values of known type picks up nested
    /// values.
    #[test]
    fn test_autocomplete_value_filter() {
        let world = TestWorld::new("#import \"design.typ\": clrs; #rect(fill: )")
            .with_source(
                "design.typ",
                "#let clrs = (a: red, b: blue); #let nums = (a: 1, b: 2)",
            );

        test(&world, -2)
            .must_include(["clrs", "aqua"])
            .must_exclude(["nums", "a", "b"]);
    }

    #[test]
    fn test_autocomplete_packages() {
        test("#import \"@\"", -2).must_include([q!("@preview/example:0.1.0")]);
    }

    #[test]
    fn test_autocomplete_file_path() {
        let world = TestWorld::new("#include \"\"")
            .with_source("utils.typ", "")
            .with_source("content/a.typ", "#image()")
            .with_source("content/b.typ", "#csv(\"\")")
            .with_source("content/c.typ", "#include \"\"")
            .with_asset_at("assets/tiger.jpg", "tiger.jpg")
            .with_asset_at("assets/rhino.png", "rhino.png")
            .with_asset_at("data/example.csv", "example.csv");

        test(&world, -2)
            .must_include([q!("content/a.typ"), q!("content/b.typ"), q!("utils.typ")])
            .must_exclude([q!("assets/tiger.jpg")]);

        test(&world, ("content/c.typ", -2))
            .must_include([q!("../main.typ"), q!("a.typ"), q!("b.typ")])
            .must_exclude([q!("c.typ")]);

        test(&world, ("content/a.typ", -2))
            .must_include([q!("../assets/tiger.jpg"), q!("../assets/rhino.png")])
            .must_exclude([q!("../data/example.csv"), q!("b.typ")]);

        test(&world, ("content/b.typ", -3)).must_include([q!("../data/example.csv")]);
    }

    #[test]
    fn test_autocomplete_figure_snippets() {
        test("#figure()", -2)
            .must_apply("image", "image(\"${}\"),")
            .must_apply("table", "table(\n  ${}\n),");

        test("#figure(cap)", -2).must_apply("caption", "caption: [${}]");
    }

    #[test]
    fn test_autocomplete_import_items() {
        let world = TestWorld::new("#import \"other.typ\": ")
            .with_source("second.typ", "#import \"other.typ\": th")
            .with_source("other.typ", "#let this = 1; #let that = 2");

        test(&world, ("main.typ", 21))
            .must_include(["*", "this", "that"])
            .must_exclude(["figure"]);
        test(&world, ("second.typ", 23))
            .must_include(["this", "that"])
            .must_exclude(["*", "figure"]);
    }

    #[test]
    fn test_autocomplete_type_methods() {
        test("#\"hello\".", -1).must_include(["len", "contains"]);
    }

    #[test]
    fn test_autocomplete_content_methods() {
        test("#show outline.entry: it => it.\n#outline()\n= Hi", 30)
            .must_include(["indented", "body", "page"]);
    }

    #[test]
    fn test_autocomplete_symbol_variants() {
        test("#sym.arrow.", -1)
            .must_include(["r", "dashed"])
            .must_exclude(["cases"]);
        test("$ arrow. $", -3)
            .must_include(["r", "dashed"])
            .must_exclude(["cases"]);
    }
}
