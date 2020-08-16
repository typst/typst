//! Parsing of source code into syntax trees.

use std::str::FromStr;

use crate::{Feedback, Pass};
use super::decoration::Decoration;
use super::expr::*;
use super::scope::Scope;
use super::span::{Pos, Span, Spanned};
use super::tokens::{is_newline_char, Token, TokenMode, Tokens};
use super::tree::{SyntaxNode, SyntaxTree};

/// A function which parses a function call into a dynamic node.
pub type CallParser = dyn Fn(FuncCall, &ParseState) -> Pass<SyntaxNode>;

/// An invocation of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncCall {
    pub name: Spanned<Ident>,
    pub args: TableExpr,
}

/// The state which can influence how a string of source code is parsed.
///
/// Parsing is pure - when passed in the same state and source code, the output
/// must be the same.
pub struct ParseState {
    /// The scope containing all function definitions.
    pub scope: Scope,
}

/// Parse a string of source code.
///
/// All spans in the resulting tree and feedback are offset by the given
/// `offset` position. This is used to make spans of a function body relative to
/// the start of the function as a whole as opposed to the start of the
/// function's body.
pub fn parse(src: &str, offset: Pos, state: &ParseState) -> Pass<SyntaxTree> {
    let mut tree = SyntaxTree::new();
    let mut par = SyntaxTree::new();
    let mut feedback = Feedback::new();

    for token in Tokens::new(src, offset, TokenMode::Body) {
        let span = token.span;
        let node = match token.v {
            // Starting from two newlines counts as a paragraph break, a single
            // newline does not.
            Token::Space(newlines) => if newlines < 2 {
                SyntaxNode::Spacing
            } else {
                // End the current paragraph if it is not empty.
                if let (Some(first), Some(last)) = (par.first(), par.last()) {
                    let span = Span::merge(first.span, last.span);
                    let node = SyntaxNode::Par(std::mem::take(&mut par));
                    tree.push(Spanned::new(node, span));
                }
                continue;
            }

            Token::Function { header, body, terminated } => {
                let parsed = FuncParser::new(header, body, state).parse();
                feedback.extend_offset(parsed.feedback, span.start);
                if !terminated {
                    error!(@feedback, Span::at(span.end), "expected closing bracket");
                }
                parsed.output
            }

            Token::Star => SyntaxNode::ToggleBolder,
            Token::Underscore => SyntaxNode::ToggleItalic,
            Token::Backslash => SyntaxNode::Linebreak,
            Token::Raw { raw, terminated } => {
                if !terminated {
                    error!(@feedback, Span::at(span.end), "expected backtick");
                }
                SyntaxNode::Raw(unescape_raw(raw))
            }
            Token::Text(text) => SyntaxNode::Text(text.to_string()),

            Token::LineComment(_) | Token::BlockComment(_) => continue,
            unexpected => {
                error!(@feedback, span, "unexpected {}", unexpected.name());
                continue;
            }
        };

        par.push(Spanned::new(node, span));
    }

    if let (Some(first), Some(last)) = (par.first(), par.last()) {
        let span = Span::merge(first.span, last.span);
        let node = SyntaxNode::Par(par);
        tree.push(Spanned::new(node, span));
    }

    Pass::new(tree, feedback)
}

struct FuncParser<'s> {
    state: &'s ParseState,
    /// The tokens inside the header.
    tokens: Tokens<'s>,
    peeked: Option<Option<Spanned<Token<'s>>>>,
    body: Option<Spanned<&'s str>>,
    feedback: Feedback,
}

impl<'s> FuncParser<'s> {
    fn new(
        header: &'s str,
        body: Option<Spanned<&'s str>>,
        state: &'s ParseState,
    ) -> Self {
        Self {
            state,
            // Start at column 1 because the opening bracket is also part of
            // the function, but not part of the `header` string.
            tokens: Tokens::new(header, Pos::new(0, 1), TokenMode::Header),
            peeked: None,
            body,
            feedback: Feedback::new(),
        }
    }

    fn parse(mut self) -> Pass<SyntaxNode> {
        let (parser, mut call) = if let Some(call) = self.parse_func_header() {
            let name = call.name.v.as_str();
            let (parser, deco) = match self.state.scope.get_parser(name) {
                // The function exists in the scope.
                Some(parser) => (parser, Decoration::ResolvedFunc),

                // The function does not exist in the scope. The parser that is
                // returned here is a fallback parser which exists to make sure
                // the content of the function is not totally dropped (on a best
                // effort basis).
                None => {
                    error!(@self.feedback, call.name.span, "unknown function");
                    let parser = self.state.scope.get_fallback_parser();
                    (parser, Decoration::UnresolvedFunc)
                }
            };

            self.feedback.decorations.push(Spanned::new(deco, call.name.span));
            (parser, call)
        } else {
            // Parse the call with the fallback parser even when the header is
            // completely unparsable.
            let parser = self.state.scope.get_fallback_parser();
            let call = FuncCall {
                name: Spanned::new(Ident(String::new()), Span::ZERO),
                args: TableExpr::new(),
            };
            (parser, call)
        };

        if let Some(body) = self.body {
            call.args.push(TableExprEntry {
                key: Span::ZERO,
                val: body.map(|src| {
                    let parsed = parse(src, body.span.start, &self.state);
                    self.feedback.extend(parsed.feedback);
                    Expr::Tree(parsed.output)
                }),
            });
        }

        let parsed = parser(call, self.state);
        self.feedback.extend(parsed.feedback);

        Pass::new(parsed.output, self.feedback)
    }

    fn parse_func_header(&mut self) -> Option<FuncCall> {
        let after_bracket = self.pos();

        self.skip_white();
        let name = try_opt_or!(self.parse_ident(), {
            self.expected_found_or_at("function name", after_bracket);
            return None;
        });

        self.skip_white();
        let args = match self.eat().map(Spanned::value) {
            Some(Token::Colon) => self.parse_table(false).0.v,
            Some(_) => {
                self.expected_at("colon", name.span.end);
                TableExpr::new()
            }
            None => TableExpr::new(),
        };

        Some(FuncCall { name, args })
    }
}

// Parsing expressions and values
impl FuncParser<'_> {
    fn parse_ident(&mut self) -> Option<Spanned<Ident>> {
        self.peek().and_then(|token| match token.v {
            Token::Ident(id) => self.eat_span(Ident(id.to_string())),
            _ => None,
        })
    }

    fn parse_expr(&mut self) -> Option<Spanned<Expr>> {
        self.parse_binops("summand", Self::parse_term, |token| match token {
            Token::Plus => Some(Expr::Add),
            Token::Hyphen => Some(Expr::Sub),
            _ => None,
        })
    }

    fn parse_term(&mut self) -> Option<Spanned<Expr>> {
        self.parse_binops("factor", Self::parse_factor, |token| match token {
            Token::Star => Some(Expr::Mul),
            Token::Slash => Some(Expr::Div),
            _ => None,
        })
    }

    /// Parse expression of the form `<operand> (<op> <operand>)*`.
    fn parse_binops(
        &mut self,
        operand_name: &str,
        mut parse_operand: impl FnMut(&mut Self) -> Option<Spanned<Expr>>,
        mut parse_op: impl FnMut(Token) -> Option<
            fn(Box<Spanned<Expr>>, Box<Spanned<Expr>>) -> Expr
        >,
    ) -> Option<Spanned<Expr>> {
        let mut left = parse_operand(self)?;

        self.skip_white();
        while let Some(token) = self.peek() {
            if let Some(op) = parse_op(token.v) {
                self.eat();
                self.skip_white();

                if let Some(right) = parse_operand(self) {
                    let span = Span::merge(left.span, right.span);
                    let v = op(Box::new(left), Box::new(right));
                    left = Spanned::new(v, span);
                    self.skip_white();
                    continue;
                }

                error!(
                    @self.feedback, Span::merge(left.span, token.span),
                    "missing right {}", operand_name,
                );
            }
            break;
        }

        Some(left)
    }

    fn parse_factor(&mut self) -> Option<Spanned<Expr>> {
        if let Some(hyph) = self.check_eat(Token::Hyphen) {
            self.skip_white();
            if let Some(factor) = self.parse_factor() {
                let span = Span::merge(hyph.span, factor.span);
                Some(Spanned::new(Expr::Neg(Box::new(factor)), span))
            } else {
                error!(@self.feedback, hyph.span, "dangling minus");
                None
            }
        } else {
            self.parse_value()
        }
    }

    fn parse_value(&mut self) -> Option<Spanned<Expr>> {
        let Spanned { v: token, span } = self.peek()?;
        match token {
            // This could be a function call or an identifier.
            Token::Ident(id) => {
                let name = Spanned::new(Ident(id.to_string()), span);
                self.eat();
                self.skip_white();
                Some(if self.check(Token::LeftParen) {
                    self.parse_func_call(name).map(|call| Expr::Call(call))
                } else {
                    name.map(|id| Expr::Ident(id))
                })
            }

            Token::Str { string, terminated } => {
                if !terminated {
                    self.expected_at("quote", span.end);
                }
                self.eat_span(Expr::Str(unescape_string(string)))
            }

            Token::Bool(b) => self.eat_span(Expr::Bool(b)),
            Token::Number(n) => self.eat_span(Expr::Number(n)),
            Token::Length(s) => self.eat_span(Expr::Length(s)),
            Token::Hex(s) => {
                if let Ok(color) = RgbaColor::from_str(s) {
                    self.eat_span(Expr::Color(color))
                } else {
                    // Heal color by assuming black.
                    error!(@self.feedback, span, "invalid color");
                    let healed = RgbaColor::new_healed(0, 0, 0, 255);
                    self.eat_span(Expr::Color(healed))
                }
            }

            // This could be a table or a parenthesized expression. We parse as
            // a table in any case and coerce the table into a value if it is
            // coercable (length 1 and no trailing comma).
            Token::LeftParen => {
                let (table, coercable) = self.parse_table(true);
                Some(if coercable {
                    table.map(|v| {
                        v.into_values()
                            .next()
                            .expect("table is coercable").val.v
                    })
                } else {
                    table.map(|tab| Expr::Table(tab))
                })
            }

            _ => None,
        }
    }

    fn parse_func_call(&mut self, name: Spanned<Ident>) -> Spanned<FuncCall> {
        let args = self.parse_table(true).0;
        let span = Span::merge(name.span, args.span);
        Spanned::new(FuncCall { name, args: args.v }, span)
    }

    /// The boolean tells you whether the table can be coerced into an expression
    /// (this is the case when it's length 1 and has no trailing comma).
    fn parse_table(&mut self, parens: bool) -> (Spanned<TableExpr>, bool) {
        let start = self.pos();
        if parens {
            self.assert(Token::LeftParen);
        }

        let mut table = TableExpr::new();
        let mut coercable = true;

        loop {
            self.skip_white();
            if self.eof() || (parens && self.check(Token::RightParen)) {
                break;
            }

            let behind_arg;

            if let Some(ident) = self.parse_ident() {
                // This could be a keyword argument, a function call or a simple
                // identifier.
                self.skip_white();

                if self.check_eat(Token::Equals).is_some() {
                    self.skip_white();

                    let key = ident;
                    self.feedback.decorations
                        .push(Spanned::new(Decoration::TableKey, key.span));

                    let val = try_opt_or!(self.parse_expr(), {
                        self.expected("value");
                        continue;
                    });

                    coercable = false;
                    behind_arg = val.span.end;
                    table.insert(key.v.0, TableExprEntry::new(key.span, val));

                } else if self.check(Token::LeftParen) {
                    let call = self.parse_func_call(ident);
                    let expr = call.map(|call| Expr::Call(call));

                    behind_arg = expr.span.end;
                    table.push(TableExprEntry::val(expr));
                } else {
                    let expr = ident.map(|id| Expr::Ident(id));

                    behind_arg = expr.span.end;
                    table.push(TableExprEntry::val(expr));
                }
            } else {
                // It's a positional argument.
                let expr = try_opt_or!(self.parse_expr(), {
                    self.expected("value");
                    continue;
                });
                behind_arg = expr.span.end;
                table.push(TableExprEntry::val(expr));
            }

            self.skip_white();
            if self.eof() || (parens && self.check(Token::RightParen)) {
                break;
            }

            self.expect_at(Token::Comma, behind_arg);
            coercable = false;
        }

        if parens {
            self.expect(Token::RightParen);
        }

        coercable = coercable && !table.is_empty();

        let end = self.pos();
        (Spanned::new(table, Span::new(start, end)), coercable)
    }
}

// Error handling
impl FuncParser<'_> {
    fn expect(&mut self, token: Token<'_>) -> bool {
        if self.check(token) {
            self.eat();
            true
        } else {
            self.expected(token.name());
            false
        }
    }

    fn expect_at(&mut self, token: Token<'_>, pos: Pos) -> bool {
        if self.check(token) {
            self.eat();
            true
        } else {
            self.expected_at(token.name(), pos);
            false
        }
    }

    fn expected(&mut self, thing: &str) {
        if let Some(found) = self.eat() {
            error!(
                @self.feedback, found.span,
                "expected {}, found {}", thing, found.v.name(),
            );
        } else {
            error!(@self.feedback, Span::at(self.pos()), "expected {}", thing);
        }
    }

    fn expected_at(&mut self, thing: &str, pos: Pos) {
        error!(@self.feedback, Span::at(pos), "expected {}", thing);
    }

    fn expected_found_or_at(&mut self, thing: &str, pos: Pos) {
        if self.eof() {
            self.expected_at(thing, pos)
        } else {
            self.expected(thing);
        }
    }
}

// Parsing primitives
impl<'s> FuncParser<'s> {
    fn skip_white(&mut self) {
        loop {
            match self.peek().map(Spanned::value) {
                Some(Token::Space(_))
                | Some(Token::LineComment(_))
                | Some(Token::BlockComment(_)) => { self.eat(); }
                _ => break,
            }
        }
    }

    fn eat(&mut self) -> Option<Spanned<Token<'s>>> {
        self.peeked.take().unwrap_or_else(|| self.tokens.next())
    }

    fn eat_span<T>(&mut self, v: T) -> Option<Spanned<T>> {
        self.eat().map(|spanned| spanned.map(|_| v))
    }

    fn peek(&mut self) -> Option<Spanned<Token<'s>>> {
        let tokens = &mut self.tokens;
        *self.peeked.get_or_insert_with(|| tokens.next())
    }

    fn assert(&mut self, token: Token<'_>) {
        assert!(self.check_eat(token).is_some());
    }

    fn check(&mut self, token: Token<'_>) -> bool {
        self.peek().map(Spanned::value) == Some(token)
    }

    fn check_eat(&mut self, token: Token<'_>) -> Option<Spanned<Token<'s>>> {
        if self.check(token) {
            self.eat()
        } else {
            None
        }
    }

    fn eof(&mut self) -> bool {
        self.peek().is_none()
    }

    fn pos(&self) -> Pos {
        self.peeked
            .flatten()
            .map(|s| s.span.start)
            .unwrap_or_else(|| self.tokens.pos())
    }
}

fn unescape_string(string: &str) -> String {
    let mut iter = string.chars();
    let mut out = String::with_capacity(string.len());

    while let Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(c) => { out.push('\\'); out.push(c); }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }

    out
}

/// Unescape raw markup and split it into into lines.
fn unescape_raw(raw: &str) -> Vec<String> {
    let mut iter = raw.chars().peekable();
    let mut line = String::new();
    let mut lines = Vec::new();

    while let Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                Some('`') => line.push('`'),
                Some(c) => { line.push('\\'); line.push(c); }
                None => line.push('\\'),
            }
        } else if is_newline_char(c) {
            if c == '\r' && iter.peek() == Some(&'\n') {
                iter.next();
            }

            lines.push(std::mem::take(&mut line));
        } else {
            line.push(c);
        }
    }

    lines.push(line);
    lines
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::syntax::tests::*;
    use crate::length::Length;
    use super::*;
    use Decoration::*;

    // ----------------------- Construct Syntax Nodes ----------------------- //

    use SyntaxNode::{
        Spacing as S,
        Linebreak as L,
        ToggleItalic as I,
        ToggleBolder as B,
    };

    fn T(text: &str) -> SyntaxNode { SyntaxNode::Text(text.to_string()) }

    macro_rules! R {
        ($($line:expr),* $(,)?) => {
            SyntaxNode::Raw(vec![$($line.to_string()),*])
        };
    }

    macro_rules! P {
        ($($tts:tt)*) => { SyntaxNode::Par(Tree![@$($tts)*]) };
    }

    macro_rules! F {
        ($($tts:tt)*) => { SyntaxNode::boxed(DebugNode(Call!(@$($tts)*))) }
    }

    // ------------------------ Construct Expressions ----------------------- //

    use Expr::{Bool, Number as Num, Length as Len, Color};

    fn Id(ident: &str) -> Expr { Expr::Ident(Ident(ident.to_string())) }
    fn Str(string: &str) -> Expr { Expr::Str(string.to_string()) }

    macro_rules! Tree {
        (@$($node:expr),* $(,)?) => {
            vec![$(Into::<Spanned<SyntaxNode>>::into($node)),*]
        };
        ($($tts:tt)*) => { Expr::Tree(Tree![@$($tts)*]) };
    }

    macro_rules! Table {
        (@table=$table:expr,) => {};
        (@table=$table:expr, $key:expr => $value:expr $(, $($tts:tt)*)?) => {{
            let key = Into::<Spanned<&str>>::into($key);
            let val = Into::<Spanned<Expr>>::into($value);
            $table.insert(key.v, TableExprEntry::new(key.span, val));
            Table![@table=$table, $($($tts)*)?];
        }};
        (@table=$table:expr, $value:expr $(, $($tts:tt)*)?) => {
            let val = Into::<Spanned<Expr>>::into($value);
            $table.push(TableExprEntry::val(val));
            Table![@table=$table, $($($tts)*)?];
        };
        (@$($tts:tt)*) => {{
            #[allow(unused_mut)]
            let mut table = TableExpr::new();
            Table![@table=table, $($tts)*];
            table
        }};
        ($($tts:tt)*) => { Expr::Table(Table![@$($tts)*]) };
    }

    fn Neg<T: Into<Spanned<Expr>>>(e1: T) -> Expr {
        Expr::Neg(Box::new(e1.into()))
    }
    fn Add<T: Into<Spanned<Expr>>>(e1: T, e2: T) -> Expr {
        Expr::Add(Box::new(e1.into()), Box::new(e2.into()))
    }
    fn Sub<T: Into<Spanned<Expr>>>(e1: T, e2: T) -> Expr {
        Expr::Sub(Box::new(e1.into()), Box::new(e2.into()))
    }
    fn Mul<T: Into<Spanned<Expr>>>(e1: T, e2: T) -> Expr {
        Expr::Mul(Box::new(e1.into()), Box::new(e2.into()))
    }
    fn Div<T: Into<Spanned<Expr>>>(e1: T, e2: T) -> Expr {
        Expr::Div(Box::new(e1.into()), Box::new(e2.into()))
    }

    macro_rules! Call {
        (@$name:expr $(; $($tts:tt)*)?) => {{
            let name = Into::<Spanned<&str>>::into($name);
            FuncCall {
                name: name.map(|n| Ident(n.to_string())),
                args: Table![@$($($tts)*)?],
            }
        }};
        ($($tts:tt)*) => { Expr::Call(Call![@$($tts)*]) };
    }

    // ----------------------------- Test Macros ---------------------------- //

    // Test syntax trees with or without spans.
    macro_rules! t { ($($tts:tt)*) => {test!(@spans=false, $($tts)*)} }
    macro_rules! ts { ($($tts:tt)*) => {test!(@spans=true, $($tts)*)} }
    macro_rules! test {
        (@spans=$spans:expr, $src:expr => $($tts:tt)*) => {
            let exp = Tree![@$($tts)*];
            let pass = parse_default($src);
            check($src, exp, pass.output, $spans);
        };
    }

    // Test expressions.
    macro_rules! v {
        ($src:expr => $($tts:tt)*) => {
            t!(concat!("[val: ", $src, "]") => P![F!("val"; $($tts)*)]);
        }
    }

    // Test error messages.
    macro_rules! e {
        ($src:expr => $($tts:tt)*) => {
            let exp = vec![$($tts)*];
            let pass = parse_default($src);
            let found = pass.feedback.diagnostics.iter()
                .map(|s| s.as_ref().map(|e| e.message.as_str()))
                .collect::<Vec<_>>();
            check($src, exp, found, true);
        };
    }

    // Test decorations.
    macro_rules! d {
        ($src:expr => $($tts:tt)*) => {
            let exp = vec![$($tts)*];
            let pass = parse_default($src);
            check($src, exp, pass.feedback.decorations, true);
        };
    }

    fn parse_default(src: &str) -> Pass<SyntaxTree> {
        let mut scope = Scope::new(Box::new(debug_func));
        scope.insert("box", Box::new(debug_func));
        scope.insert("val", Box::new(debug_func));
        scope.insert("f", Box::new(debug_func));
        let state = ParseState { scope };
        parse(src, Pos::ZERO, &state)
    }

    // -------------------------------- Tests ------------------------------- //

    #[test]
    fn test_unescape_strings() {
        fn test(string: &str, expected: &str) {
            assert_eq!(unescape_string(string), expected.to_string());
        }

        test(r#"hello world"#,  "hello world");
        test(r#"hello\nworld"#, "hello\nworld");
        test(r#"a\"bc"#,        "a\"bc");
        test(r#"a\\"#,          "a\\");
        test(r#"a\\\nbc"#,      "a\\\nbc");
        test(r#"a\tbc"#,        "a\tbc");
        test(r"🌎",             "🌎");
        test(r"🌎\",            r"🌎\");
        test(r"\🌎",            r"\🌎");
    }

    #[test]
    fn test_unescape_raws() {
        fn test(raw: &str, expected: Vec<&str>) {
            assert_eq!(unescape_raw(raw), expected);
        }

        test("raw\\`",     vec!["raw`"]);
        test("raw\ntext",  vec!["raw", "text"]);
        test("a\r\nb",     vec!["a", "b"]);
        test("a\n\nb",     vec!["a", "", "b"]);
        test("a\r\x0Bb",   vec!["a", "", "b"]);
        test("a\r\n\r\nb", vec!["a", "", "b"]);
        test("raw\\a",     vec!["raw\\a"]);
        test("raw\\",      vec!["raw\\"]);
    }

    #[test]
    fn test_parse_simple_nodes() {
        t!(""            => );
        t!("hi"          => P![T("hi")]);
        t!("*hi"         => P![B, T("hi")]);
        t!("hi_"         => P![T("hi"), I]);
        t!("hi you"      => P![T("hi"), S, T("you")]);
        t!("\n\n\nhello" => P![T("hello")]);
        t!(r"a\ b"       => P![T("a"), L, S, T("b")]);
        t!("`py`"        => P![R!["py"]]);
        t!("`hi\nyou"    => P![R!["hi", "you"]]);
        e!("`hi\nyou"    => s(1,3, 1,3, "expected backtick"));
        t!("`hi\\`du`"   => P![R!["hi`du"]]);
        t!("💜\n\n 🌍"  => P![T("💜")], P![T("🌍")]);

        ts!("hi"   => s(0,0, 0,2, P![s(0,0, 0,2, T("hi"))]));
        ts!("*Hi*" => s(0,0, 0,4, P![
            s(0,0, 0,1, B), s(0,1, 0,3, T("Hi")), s(0,3, 0,4, B),
        ]));
        ts!("💜\n\n 🌍"  =>
            s(0,0, 0,1, P![s(0,0, 0,1, T("💜"))]),
            s(2,1, 2,2, P![s(2,1, 2,2, T("🌍"))]),
        );
    }

    #[test]
    fn test_parse_comments() {
        // In body.
        t!("hi// you\nw"          => P![T("hi"), S, T("w")]);
        t!("first//\n//\nsecond"  => P![T("first"), S, S, T("second")]);
        t!("first//\n \nsecond"   => P![T("first")], P![T("second")]);
        t!("first/*\n \n*/second" => P![T("first"), T("second")]);
        e!("🌎\n*/n" => s(1,0, 1,2, "unexpected end of block comment"));

        // In header.
        t!("[val:/*12pt*/]" => P![F!("val")]);
        t!("[val \n /* \n */:]" => P![F!("val")]);
        e!("[val \n /* \n */:]" => );
        e!("[val : 12, /* \n */ 14]" => );
    }

    #[test]
    fn test_parse_function_names() {
        // No closing bracket.
        t!("[" => P![F!("")]);
        e!("[" => s(0,1, 0,1, "expected function name"),
                  s(0,1, 0,1, "expected closing bracket"));

        // No name.
        e!("[]" => s(0,1, 0,1, "expected function name"));
        e!("[\"]" => s(0,1, 0,3, "expected function name, found string"),
                     s(0,3, 0,3, "expected closing bracket"));

        // An unknown name.
        t!("[hi]" => P![F!("hi")]);
        e!("[hi]" => s(0,1, 0,3, "unknown function"));
        d!("[hi]" => s(0,1, 0,3, UnresolvedFunc));

        // A valid name.
        t!("[f]"   => P![F!("f")]);
        t!("[  f]" => P![F!("f")]);
        d!("[  f]" => s(0,3, 0,4, ResolvedFunc));

        // An invalid name.
        e!("[12]"   => s(0,1, 0,3, "expected function name, found number"));
        e!("[  🌎]" => s(0,3, 0,4, "expected function name, found invalid token"));
    }

    #[test]
    fn test_parse_colon_starting_func_args() {
        // Just colon without args.
        e!("[val:]" => );

        // Wrong token.
        t!("[val=]"     => P![F!("val")]);
        e!("[val=]"     => s(0,4, 0,4, "expected colon"));
        e!("[val/🌎:$]" => s(0,4, 0,4, "expected colon"));
        d!("[val=]"     => s(0,1, 0,4, ResolvedFunc));

        // String in invalid header without colon still parsed as string
        // Note: No "expected quote" error because not even the string was
        //       expected.
        e!("[val/\"]" => s(0,4, 0,4, "expected colon"),
                         s(0,7, 0,7, "expected closing bracket"));
    }

    #[test]
    fn test_parse_function_bodies() {
        t!("[val: 1][*Hi*]" => P![F!("val"; Num(1.0), Tree![P![B, T("Hi"), B]])]);
        e!(" [val][ */ ]" => s(0,8, 0,10, "unexpected end of block comment"));

        // Spanned.
        ts!(" [box][Oh my]" => s(0,0, 0,13, P![
            s(0,0, 0,1, S),
            s(0,1, 0,13, F!(s(0,1, 0,4, "box");
                s(0,6, 0,11, Tree![s(0,6, 0,11, P![
                    s(0,6, 0,8, T("Oh")), s(0,8, 0,9, S), s(0,9, 0,11, T("my"))
                ])])
            ))
        ]));
    }

    #[test]
    fn test_parse_simple_values() {
        v!("_"         => Id("_"));
        v!("name"      => Id("name"));
        v!("α"         => Id("α"));
        v!("\"hi\""    => Str("hi"));
        v!("true"      => Bool(true));
        v!("false"     => Bool(false));
        v!("1.0e-4"    => Num(1e-4));
        v!("3.14"      => Num(3.14));
        v!("50%"       => Num(0.5));
        v!("4.5cm"     => Len(Length::cm(4.5)));
        v!("12e1pt"    => Len(Length::pt(12e1)));
        v!("#f7a20500" => Color(RgbaColor::new(0xf7, 0xa2, 0x05, 0x00)));
        v!("\"a\n[]\\\"string\"" => Str("a\n[]\"string"));

        // Healed colors.
        v!("#12345"            => Color(RgbaColor::new_healed(0, 0, 0, 0xff)));
        e!("[val: #12345]"     => s(0,6, 0,12, "invalid color"));
        e!("[val: #a5]"        => s(0,6, 0,9,  "invalid color"));
        e!("[val: #14b2ah]"    => s(0,6, 0,13, "invalid color"));
        e!("[val: #f075ff011]" => s(0,6, 0,16, "invalid color"));

        // Unclosed string.
        v!("\"hello"        => Str("hello]"));
        e!("[val: \"hello]" => s(0,13, 0,13, "expected quote"),
                               s(0,13, 0,13, "expected closing bracket"));

        // Spanned.
        ts!("[val: 1.4]" => s(0,0, 0,10, P![
            s(0,0, 0,10, F!(s(0,1, 0,4, "val"); s(0,6, 0,9, Num(1.4))))
        ]));
    }

    #[test]
    fn test_parse_expressions() {
        // Coerced table.
        v!("(hi)" => Id("hi"));

        // Operations.
        v!("-1"          => Neg(Num(1.0)));
        v!("-- 1"        => Neg(Neg(Num(1.0))));
        v!("3.2in + 6pt" => Add(Len(Length::inches(3.2)), Len(Length::pt(6.0))));
        v!("5 - 0.01"    => Sub(Num(5.0), Num(0.01)));
        v!("(3mm * 2)"   => Mul(Len(Length::mm(3.0)), Num(2.0)));
        v!("12e-3cm/1pt" => Div(Len(Length::cm(12e-3)), Len(Length::pt(1.0))));

        // More complex.
        v!("(3.2in + 6pt)*(5/2-1)" => Mul(
            Add(Len(Length::inches(3.2)), Len(Length::pt(6.0))),
            Sub(Div(Num(5.0), Num(2.0)), Num(1.0))
        ));
        v!("(6.3E+2+4* - 3.2pt)/2" => Div(
            Add(Num(6.3e2), Mul(Num(4.0), Neg(Len(Length::pt(3.2))))),
            Num(2.0)
        ));

        // Associativity of multiplication and division.
        v!("3/4*5" => Mul(Div(Num(3.0), Num(4.0)), Num(5.0)));

        // Spanned.
        ts!("[val: 1 + 3]" => s(0,0, 0,12, P![s(0,0, 0,12, F!(
            s(0,1, 0,4, "val"); s(0,6, 0,11, Add(
                s(0,6, 0,7, Num(1.0)),
                s(0,10, 0,11, Num(3.0)),
            ))
        ))]));

        // Span of parenthesized expression contains parens.
        ts!("[val: (1)]" => s(0,0, 0,10, P![
            s(0,0, 0,10, F!(s(0,1, 0,4, "val"); s(0,6, 0,9, Num(1.0))))
        ]));

        // Invalid expressions.
        v!("4pt--"        => Len(Length::pt(4.0)));
        e!("[val: 4pt--]" => s(0,10, 0,11, "dangling minus"),
                             s(0,6, 0,10, "missing right summand"));

        v!("3mm+4pt*"        => Add(Len(Length::mm(3.0)), Len(Length::pt(4.0))));
        e!("[val: 3mm+4pt*]" => s(0,10, 0,14, "missing right factor"));
    }

    #[test]
    fn test_parse_tables() {
        // Okay.
        v!("()"                 => Table![]);
        v!("(false)"            => Bool(false));
        v!("(true,)"            => Table![Bool(true)]);
        v!("(key=val)"          => Table!["key" => Id("val")]);
        v!("(1, 2)"             => Table![Num(1.0), Num(2.0)]);
        v!("(1, key=\"value\")" => Table![Num(1.0), "key" => Str("value")]);

        // Decorations.
        d!("[val: key=hi]"    => s(0,6, 0,9, TableKey), s(0,1, 0,4, ResolvedFunc));
        d!("[val: (key=hi)]"  => s(0,7, 0,10, TableKey), s(0,1, 0,4, ResolvedFunc));
        d!("[val: f(key=hi)]" => s(0,8, 0,11, TableKey), s(0,1, 0,4, ResolvedFunc));

        // Spanned with spacing around keyword arguments.
        ts!("[val: \n hi \n = /* //\n */ \"s\n\"]" => s(0,0, 4,2, P![
            s(0,0, 4,2, F!(
                s(0,1, 0,4, "val");
                s(1,1, 1,3, "hi") => s(3,4, 4,1, Str("s\n"))
            ))
        ]));
        e!("[val: \n hi \n = /* //\n */ \"s\n\"]" => );
    }

    #[test]
    fn test_parse_tables_compute_func_calls() {
        v!("empty()"                  => Call!("empty"));
        v!("add ( 1 , 2 )"            => Call!("add"; Num(1.0), Num(2.0)));
        v!("items(\"fire\", #f93a6d)" => Call!("items";
            Str("fire"), Color(RgbaColor::new(0xf9, 0x3a, 0x6d, 0xff))
        ));

        // More complex.
        v!("css(1pt, rgb(90, 102, 254), \"solid\")" => Call!(
            "css";
            Len(Length::pt(1.0)),
            Call!("rgb"; Num(90.0), Num(102.0), Num(254.0)),
            Str("solid"),
        ));

        // Unclosed.
        v!("lang(中文]"       => Call!("lang"; Id("中文")));
        e!("[val: lang(中文]" => s(0,13, 0,13, "expected closing paren"));

        // Invalid name.
        v!("👠(\"abc\", 13e-5)"        => Table!(Str("abc"), Num(13.0e-5)));
        e!("[val: 👠(\"abc\", 13e-5)]" => s(0,6, 0,7, "expected value, found invalid token"));
    }

    #[test]
    fn test_parse_tables_nested() {
        v!("(1, ( ab=(), d = (3, 14pt) )), false" =>
            Table![
                Num(1.0),
                Table!(
                    "ab" => Table![],
                    "d"  => Table!(Num(3.0), Len(Length::pt(14.0))),
                ),
            ],
            Bool(false),
        );
    }

    #[test]
    fn test_parse_tables_errors() {
        // Expected value.
        e!("[val: (=)]"         => s(0,7, 0,8, "expected value, found equals sign"));
        e!("[val: (,)]"         => s(0,7, 0,8, "expected value, found comma"));
        v!("(\x07 abc,)"        => Table![Id("abc")]);
        e!("[val: (\x07 abc,)]" => s(0,7, 0,8, "expected value, found invalid token"));
        e!("[val: (key=,)]"     => s(0,11, 0,12, "expected value, found comma"));
        e!("[val: [hi]]"        => s(0,6, 0,10, "expected value, found function"));

        // Expected comma.
        v!("(true false)"        => Table![Bool(true), Bool(false)]);
        e!("[val: (true false)]" => s(0,11, 0,11, "expected comma"));

        // Expected closing paren.
        e!("[val: (#000]" => s(0,11, 0,11, "expected closing paren"));
        e!("[val: (key]"  => s(0,10, 0,10, "expected closing paren"));
        e!("[val: (key=]" => s(0,11, 0,11, "expected value"),
                             s(0,11, 0,11, "expected closing paren"));

        // Bad key.
        v!("true=you"        => Bool(true), Id("you"));
        e!("[val: true=you]" =>
            s(0,10, 0,10, "expected comma"),
            s(0,10, 0,11, "expected value, found equals sign"));

        // Unexpected equals sign.
        v!("z=y=4"        => Num(4.0), "z" => Id("y"));
        e!("[val: z=y=4]" =>
            s(0,9, 0,9, "expected comma"),
            s(0,9, 0,10, "expected value, found equals sign"));
    }
}
