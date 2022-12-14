use if_chain::if_chain;

use crate::model::Value;
use crate::syntax::{LinkedNode, Source, SyntaxKind};
use crate::util::{format_eco, EcoString};
use crate::World;

/// An autocompletion option.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Completion {
    /// The kind of item this completes to.
    pub kind: CompletionKind,
    /// The label the completion is shown with.
    pub label: EcoString,
    /// The completed version of the input, defaults to the label.
    ///
    /// May use snippet syntax like `${lhs} + ${rhs}`.
    pub apply: Option<EcoString>,
    /// Details about the completed item.
    pub detail: Option<EcoString>,
}

/// A kind of item that can be completed.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum CompletionKind {
    /// A syntactical structure.
    Syntax,
    /// A function name.
    Function,
    /// A constant of the given type.
    Constant,
    /// A symbol.
    Symbol,
}

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
        || complete_symbols(&mut ctx)
        || complete_markup(&mut ctx)
        || complete_math(&mut ctx)
        || complete_code(&mut ctx);

    Some((ctx.from, ctx.completions))
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
        ctx.set_rule_completions(ctx.cursor);
        return true;
    }

    // Behind the show keyword: "show |".
    if matches!(prev.kind(), SyntaxKind::Show) {
        ctx.show_rule_selector_completions(ctx.cursor);
        return true;
    }

    // Behind a half-completed show rule: "show strong: |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_leaf();
        if matches!(prev.kind(), SyntaxKind::Colon);
        if matches!(prev.parent_kind(), Some(SyntaxKind::ShowRule));
        then {
            ctx.show_rule_recipe_completions(ctx.cursor);
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
    let needs_colon = !ctx.text[ctx.cursor..].starts_with(':');

    // Behind half-completed symbol: "$arrow:|$".
    if_chain! {
        if matches!(ctx.leaf.kind(), SyntaxKind::Atom(s) if s == ":");
        if let Some(prev) = ctx.leaf.prev_leaf();
        if matches!(prev.kind(), SyntaxKind::Ident(_));
        then {
            ctx.symbol_completions(prev.offset(), false);
            return true;
        }
    }

    // Start of a symbol: ":|".
    // Checking for a text node ensures that "\:" isn't completed.
    if ctx.text[..ctx.cursor].ends_with(':')
        && matches!(ctx.leaf.kind(), SyntaxKind::Text(_) | SyntaxKind::Atom(_))
    {
        ctx.symbol_completions(ctx.cursor, needs_colon);
        return true;
    }

    // An existing symbol: ":arrow:".
    if matches!(ctx.leaf.kind(), SyntaxKind::Symbol(_)) {
        // We want to complete behind the colon, therefore plus 1.
        let has_colon = ctx.text[ctx.leaf.offset()..].starts_with(':');
        let from = ctx.leaf.offset() + (has_colon as usize);
        ctx.symbol_completions(from, has_colon && needs_colon);
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
            let from = prev.offset() + 1;
            ctx.symbol_completions(from, needs_colon);
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
    if ctx.text[..ctx.cursor].ends_with('#')
        && matches!(ctx.leaf.kind(), SyntaxKind::Text(_))
    {
        ctx.expr_completions(ctx.cursor, true);
        return true;
    }

    // An existing identifier: "#pa|".
    if matches!(ctx.leaf.kind(), SyntaxKind::Ident(_)) {
        // We want to complete behind the hashtag, therefore plus 1.
        let from = ctx.leaf.offset() + 1;
        ctx.expr_completions(from, true);
        return true;
    }

    // Behind a half-completed binding: "#let x = |".
    if_chain! {
        if let Some(prev) = ctx.leaf.prev_leaf();
        if matches!(prev.kind(), SyntaxKind::Eq);
        if matches!(prev.parent_kind(), Some(SyntaxKind::LetBinding));
        then {
            ctx.expr_completions(ctx.cursor, false);
            return true;
        }
    }

    // Anywhere: "|".
    if ctx.explicit {
        ctx.markup_completions(ctx.cursor);
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
        ctx.expr_completions(ctx.cursor, true);
        return true;
    }

    // Behind existing atom or identifier: "$a|$" or "$abc|$".
    if matches!(ctx.leaf.kind(), SyntaxKind::Atom(_) | SyntaxKind::Ident(_)) {
        let from = ctx.leaf.offset();
        ctx.symbol_completions(from, false);
        ctx.scope_completions(from);
        return true;
    }

    // Anywhere: "$|$".
    if ctx.explicit {
        ctx.math_completions(ctx.cursor);
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
        let from = ctx.leaf.offset();
        ctx.expr_completions(from, true);
        return true;
    }

    // Anywhere: "{ | }".
    // But not within or after an expression.
    if ctx.explicit
        && (ctx.leaf.kind().is_trivia()
            || matches!(ctx.leaf.kind(), SyntaxKind::LeftParen | SyntaxKind::LeftBrace))
    {
        ctx.expr_completions(ctx.cursor, false);
        return true;
    }

    false
}

/// Context for autocompletion.
struct CompletionContext<'a> {
    world: &'a dyn World,
    text: &'a str,
    leaf: LinkedNode<'a>,
    cursor: usize,
    explicit: bool,
    from: usize,
    completions: Vec<Completion>,
}

impl<'a> CompletionContext<'a> {
    /// Create a new autocompletion context.
    fn new(
        world: &'a dyn World,
        source: &'a Source,
        cursor: usize,
        explicit: bool,
    ) -> Option<Self> {
        let leaf = LinkedNode::new(source.root()).leaf_at(cursor)?;
        Some(Self {
            world,
            text: source.text(),
            leaf,
            cursor,
            explicit,
            from: cursor,
            completions: vec![],
        })
    }

    /// Add completions for all functions from the global scope.
    fn set_rule_completions(&mut self, from: usize) {
        self.scope_completions_where(
            from,
            |value| matches!(value, Value::Func(_)),
            "(${})",
        );
    }

    /// Add completions for selectors.
    fn show_rule_selector_completions(&mut self, from: usize) {
        self.snippet(
            "text selector",
            "\"${text}\": ${}",
            "Replace occurances of specific text.",
        );

        self.snippet(
            "regex selector",
            "regex(\"${regex}\"): ${}",
            "Replace matches of a regular expression.",
        );

        self.scope_completions_where(
            from,
            |value| matches!(value, Value::Func(func) if func.select(None).is_ok()),
            ": ${}",
        );
    }

    /// Add completions for selectors.
    fn show_rule_recipe_completions(&mut self, from: usize) {
        self.snippet(
            "replacement",
            "[${content}]",
            "Replace the selected element with content.",
        );

        self.snippet(
            "replacement (string)",
            "\"${text}\"",
            "Replace the selected element with a string of text.",
        );

        self.snippet(
            "transformation",
            "element => [${content}]",
            "Transform the element with a function.",
        );

        self.scope_completions_where(from, |value| matches!(value, Value::Func(_)), "");
    }

    /// Add completions for the global scope.
    fn scope_completions(&mut self, from: usize) {
        self.scope_completions_where(from, |_| true, "");
    }

    /// Add completions for a subset of the global scope.
    fn scope_completions_where(
        &mut self,
        from: usize,
        filter: fn(&Value) -> bool,
        extra: &str,
    ) {
        self.from = from;
        for (name, value) in self.world.library().scope.iter() {
            if filter(value) {
                let apply = (!extra.is_empty()).then(|| format_eco!("{name}{extra}"));
                self.completions.push(match value {
                    Value::Func(func) => Completion {
                        kind: CompletionKind::Function,
                        label: name.clone(),
                        apply,
                        detail: func.doc().map(Into::into),
                    },
                    v => Completion {
                        kind: CompletionKind::Constant,
                        label: name.clone(),
                        apply,
                        detail: Some(format_eco!(
                            "Constant of type `{}`.",
                            v.type_name()
                        )),
                    },
                });
            }
        }
    }

    /// Add completions for all symbols.
    fn symbol_completions(&mut self, from: usize, colon: bool) {
        self.from = from;
        self.completions.reserve(symmie::list().len());
        for &(name, c) in symmie::list() {
            self.completions.push(Completion {
                kind: CompletionKind::Symbol,
                label: name.into(),
                apply: colon.then(|| format_eco!("{name}:")),
                detail: Some(c.into()),
            });
        }
    }

    /// Add completions for markup snippets.
    #[rustfmt::skip]
    fn markup_completions(&mut self, from: usize) {
        self.from = from;

        self.snippet(
            "linebreak",
            "\\\n${}",
            "Inserts a forced linebreak.",
        );

        self.snippet(
            "symbol",
            ":${}:",
            "Inserts a symbol.",
        );

        self.snippet(
            "strong text",
            "*${strong}*",
            "Strongly emphasizes content by increasing the font weight.",
        );

        self.snippet(
            "emphasized text",
            "_${emphasized}_",
            "Emphasizes content by setting it in italic font style.",
        );

        self.snippet(
            "raw text",
            "`${text}`",
            "Displays text verbatim, in monospace.",
        );

        self.snippet(
            "code listing",
            "```${lang}\n${code}\n```",
            "Inserts computer code with syntax highlighting.",
        );

        self.snippet(
            "hyperlink",
            "https://${example.com}",
            "Links to a URL.",
        );

        self.snippet(
            "math (inline)",
            "$${x}$",
            "Inserts an inline-level mathematical formula.",
        );

        self.snippet(
            "math (block)",
            "$ ${sum_x^2} $",
            "Inserts a block-level mathematical formula.",
        );

        self.snippet(
            "label",
            "<${name}>",
            "Makes the preceding element referencable.",
        );

        self.snippet(
            "reference",
            "@${name}",
            "Inserts a reference to a label.",
        );

        self.snippet(
            "heading",
            "= ${title}",
            "Inserts a section heading.",
        );

        self.snippet(
            "list item",
            "- ${item}",
            "Inserts an item of an unordered list.",
        );

        self.snippet(
            "enumeration item",
            "+ ${item}",
            "Inserts an item of an ordered list.",
        );

        self.snippet(
            "enumeration item (numbered)",
            "${number}. ${item}",
            "Inserts an explicitly numbered item of an ordered list.",
        );

        self.snippet(
            "description list item",
            "/ ${term}: ${description}",
            "Inserts an item of a description list.",
        );

        self.snippet(
            "expression",
            "#${}",
            "Variables, function calls, and more.",
        );

        self.snippet(
            "code block",
            "{ ${} }",
            "Switches into code mode.",
        );

        self.snippet(
            "content block",
            "[${content}]",
            "Inserts a nested content block that isolates styles.",
        );
    }

    /// Add completions for math snippets.
    #[rustfmt::skip]
    fn math_completions(&mut self, from: usize) {
        self.symbol_completions(from, false);
        self.scope_completions(from);

        self.snippet(
            "subscript",
            "${x}_${2:2}",
            "Sets something in subscript.",
        );

        self.snippet(
            "superscript",
            "${x}^${2:2}",
            "Sets something in superscript.",
        );

        self.snippet(
            "fraction",
            "${x}/${y}",
            "Inserts a fraction.",
        );
    }

    /// Add completions for expression snippets.
    #[rustfmt::skip]
    fn expr_completions(&mut self, from: usize, short_form: bool) {
        self.scope_completions(from);

        self.snippet(
            "variable",
            "${variable}",
            "Accesses a variable.",
        );

        self.snippet(
            "function call",
            "${function}(${arguments})[${body}]",
            "Evaluates a function.",
        );

        self.snippet(
            "set rule",
            "set ${}",
            "Sets style properties on an element.",
        );

        self.snippet(
            "show rule",
            "show ${}",
            "Redefines the look of an element.",
        );

        self.snippet(
            "let binding",
            "let ${name} = ${value}",
            "Saves a value in a variable.",
        );

        self.snippet(
            "let binding (function)",
            "let ${name}(${params}) = ${output}",
            "Defines a function.",
        );

        self.snippet(
            "if conditional",
            "if ${1 < 2} {\n\t${}\n}",
            "Computes or inserts something conditionally.",
        );

        self.snippet(
            "if-else conditional",
            "if ${1 < 2} {\n\t${}\n} else {\n\t${}\n}",
            "Computes or inserts different things based on a condition.",
        );

        self.snippet(
            "while loop",
            "while ${1 < 2} {\n\t${}\n}",
            "Computes or inserts somthing while a condition is met.",
        );

        self.snippet(
            "for loop",
            "for ${value} in ${(1, 2, 3)} {\n\t${}\n}",
            "Computes or inserts somthing for each value in a collection.",
        );

        self.snippet(
            "for loop (with key)",
            "for ${key}, ${value} in ${(a: 1, b: 2)} {\n\t${}\n}",
            "Computes or inserts somthing for each key and value in a collection.",
        );

        self.snippet(
            "break",
            "break",
            "Exits early from a loop.",
        );

        self.snippet(
            "continue",
            "continue",
            "Continues with the next iteration of a loop.",
        );

        self.snippet(
            "return",
            "return ${output}",
            "Returns early from a function.",
        );

        self.snippet(
            "import",
            "import ${items} from \"${file.typ}\"",
            "Imports variables from another file.",
        );

        self.snippet(
            "include",
            "include \"${file.typ}\"",
            "Includes content from another file.",
        );

        if short_form {
            return;
        }

        self.snippet(
            "code block",
            "{ ${} }",
            "Inserts a nested code block.",
        );

        self.snippet(
            "content block",
            "[${content}]",
            "Switches into markup mode.",
        );

        self.snippet(
            "array",
            "(${1, 2, 3})",
            "Creates a sequence of values.",
        );

        self.snippet(
            "dictionary",
            "(${a: 1, b: 2})",
            "Creates a mapping from names to value.",
        );

        self.snippet(
            "anonymous function",
            "(${params}) => ${output}",
            "Creates an unnamed function.",
        );
    }

    /// Add a snippet completion.
    fn snippet(&mut self, label: &str, snippet: &str, detail: &str) {
        self.completions.push(Completion {
            kind: CompletionKind::Syntax,
            label: label.into(),
            apply: Some(snippet.into()),
            detail: Some(detail.into()),
        });
    }
}
