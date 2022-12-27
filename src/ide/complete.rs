use std::collections::HashSet;

use if_chain::if_chain;

use super::{plain_docs_sentence, summarize_font_family};
use crate::model::{CastInfo, Scope, Value};
use crate::syntax::ast::AstNode;
use crate::syntax::{ast, LinkedNode, Source, SyntaxKind};
use crate::util::{format_eco, EcoString};
use crate::World;

/// Autocomplete a cursor position in a source file.
///
/// Returns the position from which the completions apply and a list of
/// completions.
///
/// When `explicit` is `true`, the user requested the completion by pressing
/// control and space or something similar.
pub fn autocomplete(
    world: &dyn World,
    source: &Source,
    cursor: usize,
    explicit: bool,
) -> Option<(usize, Vec<Completion>)> {
    let mut ctx = CompletionContext::new(world, source, cursor, explicit)?;

    let _ = complete_rules(&mut ctx)
        || complete_params(&mut ctx)
        || complete_symbols(&mut ctx)
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
    /// A font family.
    Font,
    /// A symmie symbol.
    Symbol(char),
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
        ctx.set_rule_completions();
        return true;
    }

    // Behind the show keyword: "show |".
    if matches!(prev.kind(), SyntaxKind::Show) {
        ctx.from = ctx.cursor;
        ctx.show_rule_selector_completions();
        return true;
    }

    // Behind a half-completed show rule: "show strong: |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_leaf();
        if matches!(prev.kind(), SyntaxKind::Colon);
        if matches!(prev.parent_kind(), Some(SyntaxKind::ShowRule));
        then {
            ctx.from = ctx.cursor;
            ctx.show_rule_recipe_completions();
            return true;
        }
    }

    false
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
        if let Some(callee) = match expr {
            ast::Expr::FuncCall(call) => call.callee().as_untyped().cast(),
            ast::Expr::Set(set) => Some(set.target()),
            _ => None,
        };
        then {
            (callee, set, args)
        } else {
            return false;
        }
    };

    // Parameter values: "func(param:|)", "func(param: |)".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_leaf();
        if let Some(before_colon) = match (prev.kind(), ctx.leaf.kind()) {
            (_, SyntaxKind::Colon) => Some(prev),
            (SyntaxKind::Colon, _) => prev.prev_leaf(),
            _ => None,
        };
        if let SyntaxKind::Ident(param) = before_colon.kind();
        then {
            ctx.from = match ctx.leaf.kind() {
                SyntaxKind::Colon | SyntaxKind::Space { .. } => ctx.cursor,
                _ => ctx.leaf.offset(),
            };
            ctx.named_param_value_completions(&callee, &param);
            return true;
        }
    }

    // Parameters: "func(|)", "func(hi|)", "func(12,|)".
    if_chain! {
        if let Some(deciding) = if ctx.leaf.kind().is_trivia() {
            ctx.leaf.prev_leaf()
        } else {
            Some(ctx.leaf.clone())
        };
        if matches!(
            deciding.kind(),
            SyntaxKind::LeftParen
                | SyntaxKind::Comma
                | SyntaxKind::Ident(_)
        );
        then {
            ctx.from = match deciding.kind() {
                SyntaxKind::Ident(_) => deciding.offset(),
                _ => ctx.cursor,
            };

            // Exclude arguments which are already present.
            let exclude: Vec<_> = args.items().filter_map(|arg| match arg {
                ast::Arg::Named(named) => Some(named.name()),
                _ => None,
            }).collect();

            ctx.param_completions(&callee, set, &exclude);
            return true;
        }
    }

    false
}

/// Complete symbols.
///
/// Exception: Math identifiers which can also be symbols are handled separately
/// in `math_completions`.
fn complete_symbols(ctx: &mut CompletionContext) -> bool {
    // Whether a colon is necessary.
    let needs_colon = !ctx.after.starts_with(':');

    // Behind half-completed symbol: "$arrow:|$".
    if_chain! {
        if matches!(ctx.leaf.kind(), SyntaxKind::Atom(s) if s == ":");
        if let Some(prev) = ctx.leaf.prev_leaf();
        if matches!(prev.kind(), SyntaxKind::Ident(_));
        then {
            ctx.from = prev.offset();
            ctx.symbol_completions(false);
            return true;
        }
    }

    // Start of a symbol: ":|".
    // Checking for a text node ensures that "\:" isn't completed.
    if ctx.before.ends_with(':')
        && matches!(ctx.leaf.kind(), SyntaxKind::Text(_) | SyntaxKind::Atom(_))
    {
        ctx.from = ctx.cursor;
        ctx.symbol_completions(needs_colon);
        return true;
    }

    // An existing symbol: ":arrow:".
    if matches!(ctx.leaf.kind(), SyntaxKind::Symbol(_)) {
        // We want to complete behind the colon, therefore plus 1.
        let has_colon = ctx.after.starts_with(':');
        ctx.from = ctx.leaf.offset() + (has_colon as usize);
        ctx.symbol_completions(has_colon && needs_colon);
        return true;
    }

    // Behind half-completed symbol: ":bar|" or ":arrow:dou|".
    if_chain! {
        if matches!(
            ctx.leaf.kind(),
            SyntaxKind::Text(_) | SyntaxKind::Atom(_) | SyntaxKind::Ident(_)
        );
        if let Some(prev) = ctx.leaf.prev_leaf();
        if matches!(prev.kind(), SyntaxKind::Symbol(_)) || matches!(
            prev.kind(),
            SyntaxKind::Text(s) | SyntaxKind::Atom(s) if s == ":"
        );
        then {
            // We want to complete behind the colon, therefore plus 1.
            ctx.from = prev.offset() + 1;
            ctx.symbol_completions(needs_colon);
            return true;
        }
    }

    false
}

/// Complete in markup mode.
fn complete_markup(ctx: &mut CompletionContext) -> bool {
    // Bail if we aren't even in markup.
    if !matches!(ctx.leaf.parent_kind(), None | Some(SyntaxKind::Markup { .. })) {
        return false;
    }

    // Start of an interpolated identifier: "#|".
    // Checking for a text node ensures that "\#" isn't completed.
    if ctx.before.ends_with('#') && matches!(ctx.leaf.kind(), SyntaxKind::Text(_)) {
        ctx.from = ctx.cursor;
        ctx.expr_completions(true);
        return true;
    }

    // An existing identifier: "#pa|".
    if matches!(ctx.leaf.kind(), SyntaxKind::Ident(_)) {
        // We want to complete behind the hashtag, therefore plus 1.
        ctx.from = ctx.leaf.offset() + 1;
        ctx.expr_completions(true);
        return true;
    }

    // Behind a half-completed binding: "#let x = |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_leaf();
        if matches!(prev.kind(), SyntaxKind::Eq);
        if matches!(prev.parent_kind(), Some(SyntaxKind::LetBinding));
        then {
            ctx.from = ctx.cursor;
            ctx.expr_completions(false);
            return true;
        }
    }

    // Anywhere: "|".
    if ctx.explicit {
        ctx.from = ctx.cursor;
        ctx.markup_completions();
        return true;
    }

    false
}

/// Complete in math mode.
fn complete_math(ctx: &mut CompletionContext) -> bool {
    if !matches!(
        ctx.leaf.parent_kind(),
        Some(SyntaxKind::Math) | Some(SyntaxKind::Frac) | Some(SyntaxKind::Script)
    ) {
        return false;
    }

    // Start of an interpolated identifier: "#|".
    if matches!(ctx.leaf.kind(), SyntaxKind::Atom(s) if s == "#") {
        ctx.from = ctx.cursor;
        ctx.expr_completions(true);
        return true;
    }

    // Behind existing atom or identifier: "$a|$" or "$abc|$".
    if matches!(ctx.leaf.kind(), SyntaxKind::Atom(_) | SyntaxKind::Ident(_)) {
        ctx.from = ctx.leaf.offset();
        ctx.math_completions();
        return true;
    }

    // Anywhere: "$|$".
    if ctx.explicit {
        ctx.from = ctx.cursor;
        ctx.math_completions();
        return true;
    }

    false
}

/// Complete in code mode.
fn complete_code(ctx: &mut CompletionContext) -> bool {
    if matches!(
        ctx.leaf.parent_kind(),
        None | Some(SyntaxKind::Markup { .. }) | Some(SyntaxKind::Math)
    ) {
        return false;
    }

    // An existing identifier: "{ pa| }".
    if matches!(ctx.leaf.kind(), SyntaxKind::Ident(_)) {
        ctx.from = ctx.leaf.offset();
        ctx.expr_completions(false);
        return true;
    }

    // Anywhere: "{ | }".
    // But not within or after an expression.
    if ctx.explicit
        && (ctx.leaf.kind().is_trivia()
            || matches!(ctx.leaf.kind(), SyntaxKind::LeftParen | SyntaxKind::LeftBrace))
    {
        ctx.from = ctx.cursor;
        ctx.expr_completions(false);
        return true;
    }

    false
}

/// Context for autocompletion.
struct CompletionContext<'a> {
    world: &'a dyn World,
    scope: &'a Scope,
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
        world: &'a dyn World,
        source: &'a Source,
        cursor: usize,
        explicit: bool,
    ) -> Option<Self> {
        let text = source.text();
        let leaf = LinkedNode::new(source.root()).leaf_at(cursor)?;
        Some(Self {
            world,
            scope: &world.library().scope,
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
            *apply = Some(format_eco!("{prefix}{current}{suffix}"));
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

    /// Add completions for a subset of the global scope.
    fn scope_completions(&mut self, filter: impl Fn(&Value) -> bool) {
        for (name, value) in self.scope.iter() {
            if filter(value) {
                self.value_completion(Some(name.clone()), value, None);
            }
        }
    }

    /// Add completions for the parameters of a function.
    fn param_completions(
        &mut self,
        callee: &ast::Ident,
        set: bool,
        exclude: &[ast::Ident],
    ) {
        let info = if_chain! {
            if let Some(Value::Func(func)) = self.scope.get(callee);
            if let Some(info) = func.info();
            then { info }
            else { return; }
        };

        if callee.as_str() == "text" {
            self.font_completions();
        }

        for param in &info.params {
            if exclude.iter().any(|ident| ident.as_str() == param.name) {
                continue;
            }

            if set && !param.settable {
                continue;
            }

            if param.named {
                self.completions.push(Completion {
                    kind: CompletionKind::Param,
                    label: param.name.into(),
                    apply: Some(format_eco!("{}: ${{}}", param.name)),
                    detail: Some(plain_docs_sentence(param.docs).into()),
                });
            }

            if param.positional {
                self.cast_completions(&param.cast);
            }
        }

        if self.before.ends_with(',') {
            self.enrich(" ", "");
        }
    }

    /// Add completions for the values of a function parameter.
    fn named_param_value_completions(&mut self, callee: &ast::Ident, name: &str) {
        let param = if_chain! {
            if let Some(Value::Func(func)) = self.scope.get(callee);
            if let Some(info) = func.info();
            if let Some(param) = info.param(name);
            if param.named;
            then { param }
            else { return; }
        };

        self.cast_completions(&param.cast);

        if self.before.ends_with(':') {
            self.enrich(" ", "");
        }
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
                self.value_completion(None, value, Some(docs));
            }
            CastInfo::Type("none") => {
                self.snippet_completion("none", "none", "Nonexistent.")
            }
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
                self.scope_completions(|value| value.type_name() == "color");
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
                    apply: Some(format_eco!("${{{ty}}}")),
                    detail: Some(format_eco!("A value of type {ty}.")),
                });
                self.scope_completions(|value| value.type_name() == *ty);
            }
            CastInfo::Union(union) => {
                for info in union {
                    self.cast_completions(info);
                }
            }
        }
    }

    /// Add a completion for a specific value.
    fn value_completion(
        &mut self,
        label: Option<EcoString>,
        value: &Value,
        docs: Option<&'static str>,
    ) {
        let mut label = label.unwrap_or_else(|| value.repr().into());
        let mut apply = None;

        if label.starts_with('"') {
            let trimmed = label.trim_matches('"').into();
            apply = Some(label);
            label = trimmed;
        }

        let detail = docs.map(Into::into).or_else(|| match value {
            Value::Func(func) => {
                func.info().map(|info| plain_docs_sentence(info.docs).into())
            }
            Value::Color(color) => Some(format_eco!("The color {color:?}.")),
            Value::Auto => Some("A smart default.".into()),
            _ => None,
        });

        self.completions.push(Completion {
            kind: match value {
                Value::Func(_) => CompletionKind::Func,
                _ => CompletionKind::Constant,
            },
            label,
            apply,
            detail,
        });
    }

    /// Add completions for all font families.
    fn font_completions(&mut self) {
        for (family, iter) in self.world.book().families() {
            let detail = summarize_font_family(iter);
            self.completions.push(Completion {
                kind: CompletionKind::Font,
                label: family.into(),
                apply: Some(format_eco!("\"{family}\"")),
                detail: Some(detail.into()),
            })
        }
    }

    /// Add completions for all symbols.
    fn symbol_completions(&mut self, needs_colon: bool) {
        self.symbol_completions_where(needs_colon, |_| true);
    }

    /// Add completions for a subset of all symbols.
    fn symbol_completions_where(
        &mut self,
        needs_colon: bool,
        filter: impl Fn(char) -> bool,
    ) {
        self.completions.reserve(symmie::list().len());
        for &(name, c) in symmie::list() {
            if filter(c) {
                self.completions.push(Completion {
                    kind: CompletionKind::Symbol(c),
                    label: name.into(),
                    apply: None,
                    detail: None,
                });
            }
        }
        if needs_colon {
            self.enrich("", ":");
        }
    }

    /// Add completions for markup snippets.
    #[rustfmt::skip]
    fn markup_completions(&mut self) {
        self.snippet_completion(
            "linebreak",
            "\\\n${}",
            "Inserts a forced linebreak.",
        );

        self.snippet_completion(
            "symbol",
            ":${}:",
            "Inserts a symbol.",
        );

        self.snippet_completion(
            "strong text",
            "*${strong}*",
            "Strongly emphasizes content by increasing the font weight.",
        );

        self.snippet_completion(
            "emphasized text",
            "_${emphasized}_",
            "Emphasizes content by setting it in italic font style.",
        );

        self.snippet_completion(
            "raw text",
            "`${text}`",
            "Displays text verbatim, in monospace.",
        );

        self.snippet_completion(
            "code listing",
            "```${lang}\n${code}\n```",
            "Inserts computer code with syntax highlighting.",
        );

        self.snippet_completion(
            "hyperlink",
            "https://${example.com}",
            "Links to a URL.",
        );

        self.snippet_completion(
            "math (inline)",
            "$${x}$",
            "Inserts an inline-level mathematical formula.",
        );

        self.snippet_completion(
            "math (block)",
            "$ ${sum_x^2} $",
            "Inserts a block-level mathematical formula.",
        );

        self.snippet_completion(
            "label",
            "<${name}>",
            "Makes the preceding element referencable.",
        );

        self.snippet_completion(
            "reference",
            "@${name}",
            "Inserts a reference to a label.",
        );

        self.snippet_completion(
            "heading",
            "= ${title}",
            "Inserts a section heading.",
        );

        self.snippet_completion(
            "list item",
            "- ${item}",
            "Inserts an item of a bullet list.",
        );

        self.snippet_completion(
            "enumeration item",
            "+ ${item}",
            "Inserts an item of a numbered list.",
        );

        self.snippet_completion(
            "enumeration item (numbered)",
            "${number}. ${item}",
            "Inserts an explicitly numbered list item.",
        );

        self.snippet_completion(
            "term list item",
            "/ ${term}: ${description}",
            "Inserts an item of a term list.",
        );

        self.snippet_completion(
            "expression",
            "#${}",
            "Variables, function calls, and more.",
        );

        self.snippet_completion(
            "code block",
            "{ ${} }",
            "Switches into code mode.",
        );

        self.snippet_completion(
            "content block",
            "[${content}]",
            "Inserts a nested content block that isolates styles.",
        );
    }

    /// Add completions for math snippets.
    #[rustfmt::skip]
    fn math_completions(&mut self) {
        // Exclude non-technical symbols.
        self.symbol_completions_where(false, |c| match c as u32 {
            9728..=9983 => false,
            9984..=10175 => false,
            127744..=128511 => false,
            128512..=128591 => false,
            128640..=128767 => false,
            129280..=129535 => false,
            129648..=129791 => false,
            127136..=127231 => false,
            127024..=127135 => false,
            126976..=127023 => false,
            _ => true,
        });

        self.scope_completions(|value| {
            matches!(
                value,
                Value::Func(func) if func.info().map_or(false, |info| {
                    info.category == "math"
                }),
            )
        });

        self.snippet_completion(
            "subscript",
            "${x}_${2:2}",
            "Sets something in subscript.",
        );

        self.snippet_completion(
            "superscript",
            "${x}^${2:2}",
            "Sets something in superscript.",
        );

        self.snippet_completion(
            "fraction",
            "${x}/${y}",
            "Inserts a fraction.",
        );
    }

    /// Add completions for expression snippets.
    #[rustfmt::skip]
    fn expr_completions(&mut self,  short_form: bool) {
        self.scope_completions(|value| {
            !short_form || matches!(
                value,
                Value::Func(func) if func.info().map_or(true, |info| {
                    info.category != "math"
                }),
            )
        });

        self.snippet_completion(
            "variable",
            "${variable}",
            "Accesses a variable.",
        );

        self.snippet_completion(
            "function call",
            "${function}(${arguments})[${body}]",
            "Evaluates a function.",
        );

        self.snippet_completion(
            "set rule",
            "set ${}",
            "Sets style properties on an element.",
        );

        self.snippet_completion(
            "show rule",
            "show ${}",
            "Redefines the look of an element.",
        );

        self.snippet_completion(
            "let binding",
            "let ${name} = ${value}",
            "Saves a value in a variable.",
        );

        self.snippet_completion(
            "let binding (function)",
            "let ${name}(${params}) = ${output}",
            "Defines a function.",
        );

        self.snippet_completion(
            "if conditional",
            "if ${1 < 2} {\n\t${}\n}",
            "Computes or inserts something conditionally.",
        );

        self.snippet_completion(
            "if-else conditional",
            "if ${1 < 2} {\n\t${}\n} else {\n\t${}\n}",
            "Computes or inserts different things based on a condition.",
        );

        self.snippet_completion(
            "while loop",
            "while ${1 < 2} {\n\t${}\n}",
            "Computes or inserts somthing while a condition is met.",
        );

        self.snippet_completion(
            "for loop",
            "for ${value} in ${(1, 2, 3)} {\n\t${}\n}",
            "Computes or inserts somthing for each value in a collection.",
        );

        self.snippet_completion(
            "for loop (with key)",
            "for ${key}, ${value} in ${(a: 1, b: 2)} {\n\t${}\n}",
            "Computes or inserts somthing for each key and value in a collection.",
        );

        self.snippet_completion(
            "break",
            "break",
            "Exits early from a loop.",
        );

        self.snippet_completion(
            "continue",
            "continue",
            "Continues with the next iteration of a loop.",
        );

        self.snippet_completion(
            "return",
            "return ${output}",
            "Returns early from a function.",
        );

        self.snippet_completion(
            "import",
            "import ${items} from \"${file.typ}\"",
            "Imports variables from another file.",
        );

        self.snippet_completion(
            "include",
            "include \"${file.typ}\"",
            "Includes content from another file.",
        );

        if short_form {
            return;
        }

        self.snippet_completion(
            "code block",
            "{ ${} }",
            "Inserts a nested code block.",
        );

        self.snippet_completion(
            "content block",
            "[${content}]",
            "Switches into markup mode.",
        );

        self.snippet_completion(
            "array",
            "(${1, 2, 3})",
            "Creates a sequence of values.",
        );

        self.snippet_completion(
            "dictionary",
            "(${a: 1, b: 2})",
            "Creates a mapping from names to value.",
        );

        self.snippet_completion(
            "function",
            "(${params}) => ${output}",
            "Creates an unnamed function.",
        );
    }

    /// Add completions for all functions from the global scope.
    fn set_rule_completions(&mut self) {
        self.scope_completions(|value| {
            matches!(
                value,
                Value::Func(func) if func.info().map_or(false, |info| {
                    info.params.iter().any(|param| param.settable)
                }),
            )
        });
    }

    /// Add completions for selectors.
    fn show_rule_selector_completions(&mut self) {
        self.scope_completions(
            |value| matches!(value, Value::Func(func) if func.select(None).is_ok()),
        );

        self.enrich("", ": ");

        self.snippet_completion(
            "text selector",
            "\"${text}\": ${}",
            "Replace occurances of specific text.",
        );

        self.snippet_completion(
            "regex selector",
            "regex(\"${regex}\"): ${}",
            "Replace matches of a regular expression.",
        );
    }

    /// Add completions for recipes.
    fn show_rule_recipe_completions(&mut self) {
        self.snippet_completion(
            "replacement",
            "[${content}]",
            "Replace the selected element with content.",
        );

        self.snippet_completion(
            "replacement (string)",
            "\"${text}\"",
            "Replace the selected element with a string of text.",
        );

        self.snippet_completion(
            "transformation",
            "element => [${content}]",
            "Transform the element with a function.",
        );

        self.scope_completions(|value| matches!(value, Value::Func(_)));
    }
}
