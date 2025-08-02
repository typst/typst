//! Tokenize unparsed math nodes.
use std::ops::ControlFlow;

use ecow::{EcoString, EcoVec, eco_vec};
use typst_library::diag::{SourceDiagnostic, SourceResult};
use typst_library::foundations::{Args, Content, Func, Value};
use typst_syntax::Span;
use typst_syntax::ast::{MathKind, MathTokenNode, MathTokenView};

use crate::{Eval, Vm, call};

/// A token stream with a type-safe interface for parsing and evaluating tokens
/// and managing errors.
pub struct TokenStream<'ast, 'vm, 'a> {
    lexer: Lexer<'ast, 'vm, 'a>,
    mode: Mode,
    next: Option<(TokenInfo, Marker)>,
}

/// The internal interface for lexing math tokens.
struct Lexer<'ast, 'vm, 'a> {
    vm: &'vm mut Vm<'a>,
    errors: EcoVec<SourceDiagnostic>,
    token_view: MathTokenView<'ast>,
    /// The index of the next token to be lexed.
    cursor: usize,
}

/// The token stream's lexing mode. Causes the stream to return a false `None`
/// value when the next token would end the current mode. Also informs the
/// lexer whether we're at the start of an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// Arguments end at any RightParen, Comma, or Semicolon.
    Args,
    /// Delimiters end at any `MathKind::Closing`.
    Delims,
}

/// Information about a token for use by the parser.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub trivia: Trivia,
    pub at_math_func: bool,
}

/// An evaluated token.
#[derive(Debug, Clone)]
pub enum Token {
    Value(Value),
    FuncCall(Func),
    Kind(MathKind, EcoString),
    ArgStart(ArgStart),
}

/// Tokens that cause special behavior at the start of arguments. Will only be
/// generated in [`Mode::Args`].
///
/// This is split out as a separate struct for use in argument parsing.
#[derive(Debug, Clone)]
pub enum ArgStart {
    Spread,
    Named { name: EcoString },
}

/// Information about trivia (comments and whitespace) preceding a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trivia {
    /// No trivia; the token directly follows its prior.
    Direct,
    /// Trivia exists, but only comments. Note that this would require the
    /// comments to all be block comments.
    OnlyComments,
    /// The trivia contains spaces, which may cause us to insert a space in the
    /// content sequence whose span is the first space's span.
    HasSpaces { span: Span },
}

/// A marker with the token's initial span and overall length.
#[derive(Debug)]
pub struct Marker {
    pub span: Span,
}

impl<'ast, 'vm, 'a> TokenStream<'ast, 'vm, 'a> {
    /// Create a new token stream.
    pub fn new(vm: &'vm mut Vm<'a>, token_view: MathTokenView<'ast>) -> Self {
        let mut lexer = Lexer { vm, errors: eco_vec![], token_view, cursor: 0 };
        let next = lexer.lex(false);
        Self { lexer, mode: Mode::Normal, next }
    }

    /// Finish the token stream by converting a final value into either spanned
    /// content or the set of errors that occurred.
    pub fn finish(self, value: Value, span: Span) -> SourceResult<Content> {
        assert!(self.next.is_none());
        if self.lexer.errors.is_empty() {
            Ok(value.display().spanned(span))
        } else {
            Err(self.lexer.errors)
        }
    }

    /// Call a function using the vm and reporting any errors. Returns a default
    /// value if errors occurred.
    pub fn call_func(
        &mut self,
        func: Func,
        args: Args,
        (start, _end): (Marker, Marker),
    ) -> Value {
        match call::call_func(self.lexer.vm, func, args, start.span) {
            Ok(value) => value.spanned(start.span),
            Err(diag_vec) => {
                self.lexer.errors.extend(diag_vec);
                Value::default()
            }
        }
    }

    /// Produce an error at the given marker.
    pub fn error_at(&mut self, mark: Marker, message: impl Into<EcoString>) {
        let Marker { span } = mark;
        let diag = SourceDiagnostic::error(span, message);
        self.lexer.errors.push(diag);
    }

    /// Produce an error from the given marker up to (excluding) the next token.
    pub fn error_from(&mut self, mark: Marker, message: impl Into<EcoString>) {
        let Marker { span } = mark;
        let diag = SourceDiagnostic::error(span, message);
        self.lexer.errors.push(diag);
    }

    /// Returns the next token plus a closure for confirming it and advancing
    /// the stream forward.
    ///
    /// This is the main interface for the token stream because it allows
    /// inspecting the next token without deciding to keep it (by not calling
    /// the closure), while the borrow checker ensures the stream doesn't
    /// change until the token is confirmed or the confirmation is cancelled.
    pub fn peek_with_confirm<'x>(
        &'x mut self,
    ) -> Option<(TokenInfo, impl FnOnce() -> Marker + use<'x, 'vm, 'a, 'ast>)> {
        self.just_peek().map(|info| (info, || self.advance()))
    }

    /// Peek the next token with no option to confirm.
    pub fn just_peek(&self) -> Option<TokenInfo> {
        if self.at_mode_end().is_some() {
            return None;
        }
        self.next.as_ref().map(|(info, _)| info.clone())
    }

    /// Advance the stream if we're at a specific mode-ending character.
    pub fn advance_if_at(&mut self, c: char) -> bool {
        let at_char = self.at_mode_end() == Some(c);
        if at_char {
            self.advance();
        }
        at_char
    }

    /// Enter a new mode and call the given function. Returns the final marker
    /// and the mode-ending character unless we encountered the end of the token
    /// stream itself.
    ///
    /// This allows us to emulate a stack of modes using the call stack itself!
    pub fn enter_mode<T>(
        &mut self,
        mode: Mode,
        func: impl FnOnce(&mut Self) -> T,
    ) -> (T, Option<(char, Marker)>) {
        let previous = self.mode;
        self.mode = mode;
        let value = func(self);
        let mode_end = self.at_mode_end();
        self.mode = previous;
        assert!(mode_end.is_some() || self.next.is_none());
        let end_info = mode_end.map(|c| (c, self.advance()));
        (value, end_info)
    }

    /// Returns the character of the next token if it ends our current mode.
    fn at_mode_end(&self) -> Option<char> {
        match (self.mode, &self.next.as_ref()?.0.token) {
            (Mode::Normal, _) => None,
            (Mode::Delims, Token::Kind(MathKind::Closing(c), _)) => Some(*c),
            (Mode::Args, Token::Kind(MathKind::Closing(')'), _)) => Some(')'),
            (Mode::Args, Token::Kind(MathKind::Comma, _)) => Some(','),
            (Mode::Args, Token::Kind(MathKind::Semicolon, _)) => Some(';'),
            _ => None,
        }
    }

    /// Advance the parser unconditionally. Assumes that `next` has already been
    /// verified as `Some`.
    fn advance(&mut self) -> Marker {
        let (prev_info, prev_mark) = self.next.take().unwrap();

        let at_arg_start = match prev_info.token {
            Token::FuncCall(_) => true,
            Token::Kind(MathKind::Comma | MathKind::Semicolon, _) => {
                self.mode == Mode::Args
            }
            _ => false,
        };
        self.next = self.lexer.lex(at_arg_start);

        prev_mark
    }
}

impl<'ast, 'vm, 'a> Lexer<'ast, 'vm, 'a> {
    /// Get the token node `n` spots ahead of the index.
    fn peek_n(&self, n: usize) -> Option<MathTokenNode<'ast>> {
        let (n, _) = self.token_view.get(self.cursor + n)?;
        Some(n)
    }

    /// Lex the next token and move the lexer's index forward.
    ///
    /// Currently this function just takes `at_arg_start` (instead of the whole
    /// [`Mode`] and other metadata) because that's the minimal actual info
    /// needed to lex math correctly and it makes this simpler.
    fn lex(&mut self, at_arg_start: bool) -> Option<(TokenInfo, Marker)> {
        let mark;
        let mut trivia = Trivia::Direct;
        let token = loop {
            let (token_node, span) = self.token_view.get(self.cursor)?;
            self.cursor += 1;
            match self.lex_token_or_trivia(token_node, span, at_arg_start) {
                ControlFlow::Break((token, n_extra)) => {
                    mark = Marker { span };
                    self.cursor += n_extra;
                    break token;
                }
                // Skip trivia preceding real tokens and continue the loop.
                ControlFlow::Continue(is_space) => match trivia {
                    Trivia::OnlyComments | Trivia::Direct if is_space => {
                        trivia = Trivia::HasSpaces { span };
                    }
                    Trivia::Direct => trivia = Trivia::OnlyComments,
                    _ => {}
                },
            }
        };
        let at_math_func = self.at_math_func(&token, trivia);
        Some((TokenInfo { trivia, token, at_math_func }, mark))
    }

    /// Whether this token is an opening paren directly preceded by an
    /// identifier-like kind, which we treat as a math function call, i.e.
    /// juxtaposition with a higher precedence than fractions.
    fn at_math_func(&mut self, token: &Token, trivia: Trivia) -> bool {
        // let-chains <3
        if let Token::Kind(MathKind::Opening(_), _) = token
            && trivia == Trivia::Direct
            && self.cursor >= 2
            && let Some((prev, _)) = self.token_view.get(self.cursor - 2)
            && prev.acts_as_math_function()
        {
            true
        } else {
            false
        }
    }

    /// Lex the next token using [`ControlFlow`] to communicate about trivia to
    /// the caller.
    ///
    /// This also returns the number of extra nodes used by the token in order
    /// so this and the other functions don't need to mutate `self.index`.
    fn lex_token_or_trivia(
        &mut self,
        token_node: MathTokenNode<'ast>,
        span: Span,
        at_arg_start: bool,
    ) -> ControlFlow<(Token, usize), bool> {
        let (token, n_extra) = match token_node {
            MathTokenNode::Trivia { is_space } => {
                return ControlFlow::Continue(is_space);
            }
            MathTokenNode::ParsedCode(code) => match code.eval(self.vm) {
                Ok(value) => (Token::Value(value), 0),
                Err(err_vec) => {
                    self.errors.extend(err_vec);
                    (Token::Value(Value::default()), 0)
                }
            },
            MathTokenNode::ParsedExpr(expr) => match expr.eval(self.vm) {
                Ok(value) => (Token::Value(value), 0),
                Err(err_vec) => {
                    self.errors.extend(err_vec);
                    (Token::Value(Value::default()), 0)
                }
            },
            MathTokenNode::FieldAccess(fields) => {
                let result = fields.eval(self.vm);
                self.lex_ident_or_fields(result, span)
            }
            MathTokenNode::MathIdent(ident) => {
                // First, try to lex a named function argument. This must happen
                // before we try to evaluate the identifier.
                if at_arg_start
                    && let Some((n, Ok(name))) = self.maybe_named_arg(ident.get())
                {
                    (Token::ArgStart(ArgStart::Named { name }), n)
                } else {
                    let result = ident.eval(self.vm);
                    self.lex_ident_or_fields(result, span)
                }
            }
            MathTokenNode::Kinds(kind, text) => {
                if at_arg_start
                    && matches!(
                        kind,
                        MathKind::Text { ident_like: true, .. }
                            | MathKind::Minus
                            | MathKind::Underscore
                    )
                    && let Some((n, result)) = self.maybe_named_arg(text)
                {
                    match result {
                        Ok(name) => (Token::ArgStart(ArgStart::Named { name }), n),
                        Err(msg) => {
                            self.errors.push(SourceDiagnostic::error(span, msg));
                            (Token::Value(Value::default()), n)
                        }
                    }
                } else if at_arg_start && let Some(n) = self.maybe_spread(kind) {
                    (Token::ArgStart(ArgStart::Spread), n)
                } else {
                    (Token::Kind(kind, text.clone()), 0)
                }
            }
        };
        ControlFlow::Break((token, n_extra))
    }

    /// Lex identifiers and fields starting with their evaluation results. If we
    /// lex a function call, the returned length will include the paren.
    fn lex_ident_or_fields(
        &mut self,
        result: SourceResult<Value>,
        span: Span,
    ) -> (Token, usize) {
        let value = match result {
            Ok(value) => value.spanned(span),
            Err(err_vec) => {
                self.errors.extend(err_vec);
                Value::default()
            }
        };
        if let Some(peek) = self.peek_n(0)
            && matches!(peek, MathTokenNode::Kinds(MathKind::Opening('('), _))
            && let Ok(func) = value.clone().cast::<Func>()
        {
            (Token::FuncCall(func), 1)
        } else {
            (Token::Value(value), 0)
        }
    }

    /// Try to lex multiple tokens as a single named argument. The returned
    /// length includes the colon.
    fn maybe_named_arg(
        &self,
        text: &EcoString,
    ) -> Option<(usize, Result<EcoString, &'static str>)> {
        let mut name = text.clone();
        let mut n = 0;
        loop {
            let token = self.peek_n(n);
            n += 1;
            match token {
                Some(MathTokenNode::MathIdent(math_ident)) => {
                    name.push_str(math_ident.get())
                }
                Some(MathTokenNode::Kinds(
                    MathKind::Text { ident_like: true, .. }
                    | MathKind::Minus
                    | MathKind::Underscore,
                    text,
                )) => name.push_str(text),
                Some(MathTokenNode::Kinds(MathKind::Colon, _)) => break,
                _ => return None,
            }
        }
        if name == "_" {
            // Disallow plain underscore, it can never be an actual parameter.
            Some((n, Err("expected identifier, found underscore")))
        } else {
            Some((n, Ok(name)))
        }
    }

    /// Try to lex a spread operator if we're at dots and the following tokens
    /// don't end the function argument. The returned length includes both dots.
    fn maybe_spread(&self, kind: MathKind) -> Option<usize> {
        if let MathKind::Dot = kind
            && let Some(MathTokenNode::Kinds(MathKind::Dot, _)) = self.peek_n(0)
            && let Some(after_dots) = self.peek_n(1)
            && !matches!(
                after_dots,
                MathTokenNode::Kinds(
                    MathKind::Semicolon | MathKind::Comma | MathKind::Closing(')'),
                    _,
                ) | MathTokenNode::Trivia { .. },
            )
        {
            Some(1)
        } else {
            None
        }
    }
}
