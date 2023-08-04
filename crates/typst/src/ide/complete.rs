use std::cmp::Reverse;
use std::collections::{BTreeSet, HashSet};

use ecow::{eco_format, EcoString};
use if_chain::if_chain;
use unscanny::Scanner;

use super::analyze::analyze_labels;
use super::{analyze_expr, analyze_import, plain_docs_sentence, summarize_font_family};
use crate::doc::Frame;
use crate::eval::{fields_on, format_str, methods_on, CastInfo, Library, Scope, Value};
use crate::syntax::{
    ast, is_id_continue, is_id_start, is_ident, LinkedNode, Source, SyntaxKind,
};
use crate::util::separated_list;
use crate::World;

/// Autocomplete a cursor position in a source file.
///
/// Returns the position from which the completions apply and a list of
/// completions.
///
/// When `explicit` is `true`, the user requested the completion by pressing
/// control and space or something similar.
pub fn autocomplete(
    world: &(dyn World + 'static),
    frames: &[Frame],
    source: &Source,
    cursor: usize,
    explicit: bool,
) -> Option<(usize, Vec<Completion>)> {
    let mut ctx = CompletionContext::new(world, frames, source, cursor, explicit)?;

    let _ = complete_comments(&mut ctx)
        || complete_field_accesses(&mut ctx)
        || complete_imports(&mut ctx)
        || complete_rules(&mut ctx)
        || complete_params(&mut ctx)
        || complete_markup(&mut ctx)
        || complete_math(&mut ctx)
        || complete_code(&mut ctx);

    Some((ctx.from, ctx.completions))
}

/// An autocompletion option.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum CompletionKind {
    /// A syntactical structure.
    Syntax,
    /// A function.
    Func,
    /// A function parameter.
    Param,
    /// A constant.
    Constant,
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
    if ctx.leaf.kind() == SyntaxKind::Hashtag {
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

    // Start of an reference: "@|" or "@he|".
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
    if ctx.leaf.kind() == SyntaxKind::Hashtag {
        ctx.from = ctx.cursor;
        code_completions(ctx, true);
        return true;
    }

    // Behind existing atom or identifier: "$a|$" or "$abc|$".
    if matches!(ctx.leaf.kind(), SyntaxKind::Text | SyntaxKind::MathIdent) {
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
    // Behind an expression plus dot: "emoji.|".
    if_chain! {
        if ctx.leaf.kind() == SyntaxKind::Dot
            || (ctx.leaf.kind() == SyntaxKind::Text
                && ctx.leaf.text() == ".");
        if ctx.leaf.range().end == ctx.cursor;
        if let Some(prev) = ctx.leaf.prev_sibling();
        if prev.is::<ast::Expr>();
        if prev.parent_kind() != Some(SyntaxKind::Markup) ||
           prev.prev_sibling_kind() == Some(SyntaxKind::Hashtag);
        if let Some(value) = analyze_expr(ctx.world, &prev).into_iter().next();
        then {
            ctx.from = ctx.cursor;
            field_access_completions(ctx, &value);
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
        if let Some(value) = analyze_expr(ctx.world, &prev_prev).into_iter().next();
        then {
            ctx.from = ctx.leaf.offset();
            field_access_completions(ctx, &value);
            return true;
        }
    }

    false
}

/// Add completions for all fields on a value.
fn field_access_completions(ctx: &mut CompletionContext, value: &Value) {
    for &(method, args) in methods_on(value.type_name()) {
        ctx.completions.push(Completion {
            kind: CompletionKind::Func,
            label: method.into(),
            apply: Some(if args {
                eco_format!("{method}(${{}})")
            } else {
                eco_format!("{method}()${{}}")
            }),
            detail: None,
        })
    }

    for &field in fields_on(value.type_name()) {
        // Complete the field name along with its value. Notes:
        // 1. No parentheses since function fields cannot currently be called
        // with method syntax;
        // 2. We can unwrap the field's value since it's a field belonging to
        // this value's type, so accessing it should not fail.
        ctx.value_completion(
            Some(field.into()),
            &value.field(field).unwrap(),
            false,
            None,
        );
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
                ctx.value_completion(Some(name.clone()), &value, false, None);
            }
        }
        Value::Dict(dict) => {
            for (name, value) in dict.iter() {
                ctx.value_completion(Some(name.clone().into()), value, false, None);
            }
        }
        Value::Module(module) => {
            for (name, value) in module.scope().iter() {
                ctx.value_completion(Some(name.clone()), value, true, None);
            }
        }
        Value::Func(func) => {
            if let Some(info) = func.info() {
                // Consider all names from the function's scope.
                for (name, value) in info.scope.iter() {
                    ctx.value_completion(Some(name.clone()), value, true, None);
                }
            }
        }
        _ => {}
    }
}

/// Complete imports.
fn complete_imports(ctx: &mut CompletionContext) -> bool {
    // In an import path for a package:
    // "#import "@|",
    if_chain! {
        if matches!(
            ctx.leaf.parent_kind(),
            Some(SyntaxKind::ModuleImport | SyntaxKind::ModuleInclude)
        );
        if let Some(ast::Expr::Str(str)) = ctx.leaf.cast();
        let value = str.get();
        if value.starts_with('@');
        then {
            let all_versions = value.contains(':');
            ctx.from = ctx.leaf.offset();
            ctx.package_completions(all_versions);
            return true;
        }
    }

    // Behind an import list:
    // "#import "path.typ": |",
    // "#import "path.typ": a, b, |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_sibling();
        if let Some(ast::Expr::Import(import)) = prev.cast();
        if let Some(ast::Imports::Items(items)) = import.imports();
        if let Some(source) = prev.children().find(|child| child.is::<ast::Expr>());
        if let Some(value) = analyze_expr(ctx.world, &source).into_iter().next();
        then {
            ctx.from = ctx.cursor;
            import_item_completions(ctx, &items, &value);
            return true;
        }
    }

    // Behind a half-started identifier in an import list:
    // "#import "path.typ": thi|",
    if_chain! {
        if ctx.leaf.kind() == SyntaxKind::Ident;
        if let Some(parent) = ctx.leaf.parent();
        if parent.kind() == SyntaxKind::ImportItems;
        if let Some(grand) = parent.parent();
        if let Some(ast::Expr::Import(import)) = grand.cast();
        if let Some(ast::Imports::Items(items)) = import.imports();
        if let Some(source) = grand.children().find(|child| child.is::<ast::Expr>());
        if let Some(value) = analyze_expr(ctx.world, &source).into_iter().next();
        then {
            ctx.from = ctx.leaf.offset();
            import_item_completions(ctx, &items, &value);
            return true;
        }
    }

    false
}

/// Add completions for all exports of a module.
fn import_item_completions(
    ctx: &mut CompletionContext,
    existing: &[ast::Ident],
    value: &Value,
) {
    let module = match value {
        Value::Str(path) => match analyze_import(ctx.world, ctx.source, path) {
            Some(module) => module,
            None => return,
        },
        Value::Module(module) => module.clone(),
        _ => return,
    };

    if existing.is_empty() {
        ctx.snippet_completion("*", "*", "Import everything.");
    }

    for (name, value) in module.scope().iter() {
        if existing.iter().all(|ident| ident.as_str() != name) {
            ctx.value_completion(Some(name.clone()), value, false, None);
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
            Value::Func(func) if func.info().map_or(false, |info| {
                info.params.iter().any(|param| param.settable)
            }),
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
    let (callee, set, args) = if_chain! {
        if let Some(parent) = ctx.leaf.parent();
        if let Some(parent) = match parent.kind() {
            SyntaxKind::Named => parent.parent(),
            _ => Some(parent),
        };
        if let Some(args) = parent.cast::<ast::Args>();
        if let Some(grand) = parent.parent();
        if let Some(expr) = grand.cast::<ast::Expr>();
        let set = matches!(expr, ast::Expr::Set(_));
        if let Some(ast::Expr::Ident(callee)) = match expr {
            ast::Expr::FuncCall(call) => Some(call.callee()),
            ast::Expr::Set(set) => Some(set.target()),
            _ => None,
        };
        then {
            (callee, set, args)
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
        if let Some(param) = prev.cast::<ast::Ident>();
        then {
            if let Some(next) = deciding.next_leaf() {
                ctx.from = ctx.cursor.min(next.offset());
            }

            named_param_value_completions(ctx, &callee, &param);
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

            // Exclude arguments which are already present.
            let exclude: Vec<_> = args.items().filter_map(|arg| match arg {
                ast::Arg::Named(named) => Some(named.name()),
                _ => None,
            }).collect();

            param_completions(ctx, &callee, set, &exclude);
            return true;
        }
    }

    false
}

/// Add completions for the parameters of a function.
fn param_completions(
    ctx: &mut CompletionContext,
    callee: &ast::Ident,
    set: bool,
    exclude: &[ast::Ident],
) {
    let info = if_chain! {
        if let Some(Value::Func(func)) = ctx.global.get(callee);
        if let Some(info) = func.info();
        then { info }
        else { return; }
    };

    for param in &info.params {
        if exclude.iter().any(|ident| ident.as_str() == param.name) {
            continue;
        }

        if set && !param.settable {
            continue;
        }

        if param.named {
            ctx.completions.push(Completion {
                kind: CompletionKind::Param,
                label: param.name.into(),
                apply: Some(eco_format!("{}: ${{}}", param.name)),
                detail: Some(plain_docs_sentence(param.docs)),
            });
        }

        if param.positional {
            ctx.cast_completions(&param.cast);
        }
    }

    if ctx.before.ends_with(',') {
        ctx.enrich(" ", "");
    }
}

/// Add completions for the values of a named function parameter.
fn named_param_value_completions(
    ctx: &mut CompletionContext,
    callee: &ast::Ident,
    name: &str,
) {
    let param = if_chain! {
        if let Some(Value::Func(func)) = ctx.global.get(callee);
        if let Some(info) = func.info();
        if let Some(param) = info.param(name);
        if param.named;
        then { param }
        else { return; }
    };

    ctx.cast_completions(&param.cast);

    if callee.as_str() == "text" && name == "font" {
        ctx.font_completions();
    }

    if ctx.before.ends_with(':') {
        ctx.enrich(" ", "");
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
fn code_completions(ctx: &mut CompletionContext, hashtag: bool) {
    ctx.scope_completions(true, |value| !hashtag || {
        matches!(value, Value::Symbol(_) | Value::Func(_) | Value::Module(_))
    });

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
        "import \"${file}.typ\": ${items}",
        "Imports variables from another file.",
    );

    ctx.snippet_completion(
        "import (package)",
        "import \"@${}\": ${items}",
        "Imports variables from another file.",
    );

    ctx.snippet_completion(
        "include (file)",
        "include \"${file}.typ\"",
        "Includes content from another file.",
    );

    ctx.snippet_completion(
        "include (package)",
        "include \"@${}\"",
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

    if !hashtag {
        ctx.snippet_completion(
            "function",
            "(${params}) => ${output}",
            "Creates an unnamed function.",
        );
    }
}

/// Context for autocompletion.
struct CompletionContext<'a> {
    world: &'a (dyn World + 'static),
    frames: &'a [Frame],
    library: &'a Library,
    source: &'a Source,
    global: &'a Scope,
    math: &'a Scope,
    text: &'a str,
    before: &'a str,
    after: &'a str,
    leaf: LinkedNode<'a>,
    cursor: usize,
    explicit: bool,
    from: usize,
    completions: Vec<Completion>,
    seen_casts: HashSet<u128>,
}

impl<'a> CompletionContext<'a> {
    /// Create a new autocompletion context.
    fn new(
        world: &'a (dyn World + 'static),
        frames: &'a [Frame],
        source: &'a Source,
        cursor: usize,
        explicit: bool,
    ) -> Option<Self> {
        let text = source.text();
        let library = world.library();
        let leaf = LinkedNode::new(source.root()).leaf_at(cursor)?;
        Some(Self {
            world,
            frames,
            library,
            source,
            global: library.global.scope(),
            math: library.math.scope(),
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
        let equation = self.before[self.cursor.saturating_sub(25)..].contains("equation");
        for (family, iter) in self.world.book().families() {
            let detail = summarize_font_family(iter);
            if !equation || family.contains("Math") {
                self.value_completion(
                    None,
                    &Value::Str(family.into()),
                    false,
                    Some(detail.as_str()),
                );
            }
        }
    }

    /// Add completions for all available packages.
    fn package_completions(&mut self, all_versions: bool) {
        let mut packages: Vec<_> = self.world.packages().iter().collect();
        packages.sort_by_key(|(spec, _)| (&spec.name, Reverse(spec.version)));
        if !all_versions {
            packages.dedup_by_key(|(spec, _)| &spec.name);
        }
        for (package, description) in packages {
            self.value_completion(
                None,
                &Value::Str(format_str!("{package}")),
                false,
                description.as_deref(),
            );
        }
    }

    /// Add completions for raw block tags.
    fn raw_completions(&mut self) {
        for (name, mut tags) in (self.library.items.raw_languages)() {
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
                detail: Some(separated_list(&tags, " or ").into()),
            });
        }
    }

    /// Add completions for all labels.
    fn label_completions(&mut self) {
        for (label, detail) in analyze_labels(self.world, self.frames).0 {
            self.completions.push(Completion {
                kind: CompletionKind::Constant,
                label: label.0,
                apply: None,
                detail,
            });
        }
    }

    /// Add a completion for a specific value.
    fn value_completion(
        &mut self,
        label: Option<EcoString>,
        value: &Value,
        parens: bool,
        docs: Option<&str>,
    ) {
        let at = label.as_deref().map_or(false, |field| !is_ident(field));
        let label = label.unwrap_or_else(|| value.repr().into());

        let detail = docs.map(Into::into).or_else(|| match value {
            Value::Symbol(_) => None,
            Value::Func(func) => func.info().map(|info| plain_docs_sentence(info.docs)),
            v => {
                let repr = v.repr();
                (repr.as_str() != label).then(|| repr.into())
            }
        });

        let mut apply = None;
        if parens && matches!(value, Value::Func(_)) {
            apply = Some(eco_format!("{label}(${{}})"));
        } else if at {
            apply = Some(eco_format!("at(\"{label}\")"));
        } else if label.starts_with('"') && self.after.starts_with('"') {
            if let Some(trimmed) = label.strip_suffix('"') {
                apply = Some(trimmed.into());
            }
        }

        self.completions.push(Completion {
            kind: match value {
                Value::Func(_) => CompletionKind::Func,
                Value::Symbol(s) => CompletionKind::Symbol(s.get()),
                _ => CompletionKind::Constant,
            },
            label,
            apply,
            detail,
        });
    }

    /// Add completions for a castable.
    fn cast_completions(&mut self, cast: &'a CastInfo) {
        // Prevent duplicate completions from appearing.
        if !self.seen_casts.insert(crate::util::hash128(cast)) {
            return;
        }

        match cast {
            CastInfo::Any => {}
            CastInfo::Value(value, docs) => {
                self.value_completion(None, value, true, Some(docs));
            }
            CastInfo::Type("none") => self.snippet_completion("none", "none", "Nothing."),
            CastInfo::Type("auto") => {
                self.snippet_completion("auto", "auto", "A smart default.");
            }
            CastInfo::Type("boolean") => {
                self.snippet_completion("false", "false", "No / Disabled.");
                self.snippet_completion("true", "true", "Yes / Enabled.");
            }
            CastInfo::Type("color") => {
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
                self.scope_completions(false, |value| value.type_name() == "color");
            }
            CastInfo::Type("function") => {
                self.snippet_completion(
                    "function",
                    "(${params}) => ${output}",
                    "A custom function.",
                );
            }
            CastInfo::Type(ty) => {
                self.completions.push(Completion {
                    kind: CompletionKind::Syntax,
                    label: (*ty).into(),
                    apply: Some(eco_format!("${{{ty}}}")),
                    detail: Some(eco_format!("A value of type {ty}.")),
                });
                self.scope_completions(false, |value| value.type_name() == *ty);
            }
            CastInfo::Union(union) => {
                for info in union {
                    self.cast_completions(info);
                }
            }
        }
    }

    /// Add completions for definitions that are available at the cursor.
    /// Filters the global/math scope with the given filter.
    fn scope_completions(&mut self, parens: bool, filter: impl Fn(&Value) -> bool) {
        let mut defined = BTreeSet::new();

        let mut ancestor = Some(self.leaf.clone());
        while let Some(node) = &ancestor {
            let mut sibling = Some(node.clone());
            while let Some(node) = &sibling {
                if let Some(v) = node.cast::<ast::LetBinding>() {
                    for ident in v.kind().idents() {
                        defined.insert(ident.take());
                    }
                }
                sibling = node.prev_sibling();
            }

            if let Some(parent) = node.parent() {
                if let Some(v) = parent.cast::<ast::ForLoop>() {
                    if node.prev_sibling_kind() != Some(SyntaxKind::In) {
                        let pattern = v.pattern();
                        for ident in pattern.idents() {
                            defined.insert(ident.take());
                        }
                    }
                }

                ancestor = Some(parent.clone());
                continue;
            }

            break;
        }

        let in_math = matches!(
            self.leaf.parent_kind(),
            Some(SyntaxKind::Equation)
                | Some(SyntaxKind::Math)
                | Some(SyntaxKind::MathFrac)
                | Some(SyntaxKind::MathAttach)
        );

        let scope = if in_math { self.math } else { self.global };
        for (name, value) in scope.iter() {
            if filter(value) && !defined.contains(name) {
                self.value_completion(Some(name.clone()), value, parens, None);
            }
        }

        for name in defined {
            if !name.is_empty() {
                self.completions.push(Completion {
                    kind: CompletionKind::Constant,
                    label: name,
                    apply: None,
                    detail: None,
                });
            }
        }
    }
}
