//! Parse a stream of math tokens into a Typst value.

use ecow::{EcoString, EcoVec, eco_format};
use indexmap::IndexMap;
use indexmap::map::Entry;
use typst_library::foundations::{
    Arg, Args, Content, Func, IntoValue, NativeElement, Str, SymbolElem, Value,
};
use typst_library::math::{AttachElem, FracElem, LrElem, PrimesElem, RootElem};
use typst_library::text::{SpaceElem, TextElem};
use typst_syntax::ast::MathKind;
use typst_syntax::{Span, Spanned};

use super::tokens::{ArgStart, Marker, Mode, Token, TokenInfo, TokenStream, Trivia};

/// Parse a math token stream into a single value.
pub fn parse(tokens: &mut TokenStream) -> Value {
    math_expression(tokens, Side::Closed, &[])
        .map(|spanned| spanned.v) // The overall span is handled by our caller.
        .unwrap_or_else(|| Content::empty().into_value())
}

/// A math typesetting operator.
///
/// Associativity is implicit: if an operator is open on both sides, then it is
/// right-assoc if `left == right` and left-assoc if `left + 1 == right`.
#[derive(Debug)]
struct Operator {
    /// Whether the operator needs a left operand.
    left: Side,
    /// Whether the operator needs a right operand.
    right: Side,
    /// How to finish the operator and produce a value.
    finish: Finish,
}

/// Precedence of an operator side. Isomorphic to an Option, but with specific
/// semantic meaning.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Side {
    Closed,
    Open(Prec),
}

impl Side {
    /// An operator on the left is tighter if it is `Open` and has a strictly
    /// greater precedence than an operator on the right.
    ///
    /// This means right-associative operators should have `left == right`, and
    /// left-associative operators should have `left + 1 == right`.
    fn tighter_than(&self, right: Prec) -> bool {
        // If left and right are the same, the operator acts right-associative.
        matches!(self, Side::Open(left) if *left > right)
    }
}

/// Operator precedence.
type Prec = u8;

// Precedence of specific operators. Higher is more tightly binding.
// The values increment by two so that left-assoc ops can add 1 on the right.
const JUXT_PREC: Prec = 0;
const FRACTION_PREC: Prec = 2;
const MATH_FUNC_PREC: Prec = 4;
const ROOT_PREC: Prec = 6;
const ATTACH_PREC: Prec = 8;
const PRIME_PREC: Prec = 10;
const BANG_PREC: Prec = 12;

/// Ways to finish an operator to produce a value.
#[derive(Debug)]
enum Finish {
    Value { value: Value },
    Expression { expr: ExprKind },
    MaybeChain { expr: ExprKind, with: &'static [MathKind] },
    Delims { left: char },
    ParseFuncArgs { func: Func },
}

/// Types of operator expressions.
#[derive(Debug)]
enum ExprKind {
    Juxtapose,
    Attach { primes: Option<u32>, to: AttachTo },
    Root { index: Option<char> },
    Frac,
    Bang,
}

/// Records the order we encounter `^` or `_` in an attachment chain.
#[derive(Debug)]
enum AttachTo {
    Neither,
    Top,
    Bot,
    TopBot,
    BotTop,
}

/// Determine the operator to use for a Token.
fn math_op(
    token: Token,
    trivia: Trivia,
    has_lhs: bool,
    active_chain: Option<ExprKind>,
) -> Operator {
    let has_direct_lhs = has_lhs && trivia == Trivia::Direct;
    let (kind, text) = match token {
        Token::Kind(kind, text) => (kind, text),
        Token::Value(value) => {
            return Operator {
                left: Side::Closed,
                right: Side::Closed,
                finish: Finish::Value { value },
            };
        }
        Token::FuncCall(func) => {
            return Operator {
                left: Side::Closed,
                right: Side::Closed,
                finish: Finish::ParseFuncArgs { func },
            };
        }
        Token::ArgStart(_) => unreachable!("only generated when parsing function args"),
    };
    match kind {
        // Underscore is a right-associative infix op that chains with Hat.
        MathKind::Underscore => {
            let (chain, expr) = match active_chain {
                Some(ExprKind::Attach { primes, to: AttachTo::Top }) => {
                    (false, ExprKind::Attach { primes, to: AttachTo::TopBot })
                }
                Some(ExprKind::Attach { primes, to: AttachTo::Neither }) => {
                    // No top yet, might continue chain.
                    (true, ExprKind::Attach { primes, to: AttachTo::Bot })
                }
                // Otherwise we'll be starting a new attach with just ourself.
                _ => (true, ExprKind::Attach { primes: None, to: AttachTo::Bot }),
            };
            Operator {
                left: Side::Open(ATTACH_PREC),
                right: Side::Open(ATTACH_PREC),
                finish: if chain {
                    Finish::MaybeChain { expr, with: &[MathKind::Hat] }
                } else {
                    Finish::Expression { expr }
                },
            }
        }
        // Hat is a right-associative infix op that chains with Underscore.
        MathKind::Hat => {
            let (chain, expr) = match active_chain {
                Some(ExprKind::Attach { primes, to: AttachTo::Bot }) => {
                    (false, ExprKind::Attach { primes, to: AttachTo::BotTop })
                }
                Some(ExprKind::Attach { primes, to: AttachTo::Neither }) => {
                    // No bot yet, might continue chain.
                    (true, ExprKind::Attach { primes, to: AttachTo::Top })
                }
                // Otherwise we'll be starting a new attach with just ourself.
                _ => (true, ExprKind::Attach { primes: None, to: AttachTo::Top }),
            };
            Operator {
                left: Side::Open(ATTACH_PREC),
                right: Side::Open(ATTACH_PREC),
                finish: if chain {
                    Finish::MaybeChain { expr, with: &[MathKind::Underscore] }
                } else {
                    Finish::Expression { expr }
                },
            }
        }
        // Primes are a postfix operator with high precedence that chain with
        // either Hat or Underscore on the right. Hat/Underscore do not
        // themselves chain with Primes.
        MathKind::Primes { count } if has_direct_lhs => Operator {
            left: Side::Open(PRIME_PREC),
            right: Side::Closed,
            finish: Finish::MaybeChain {
                expr: ExprKind::Attach { primes: Some(count), to: AttachTo::Neither },
                // Primes never continue a chain, but they can always start one.
                with: &[MathKind::Hat, MathKind::Underscore],
            },
        },
        // If not direct with a lhs, primes still render, but don't form an attachment.
        MathKind::Primes { count } => Operator {
            left: Side::Closed,
            right: Side::Closed,
            finish: Finish::Value {
                value: PrimesElem::new(count as usize).into_value(),
            },
        },
        // Slash is a left-associative infix operator with low precedence.
        MathKind::Slash => Operator {
            // Fraction precedence is also used in `remove_parens()` below.
            left: Side::Open(FRACTION_PREC),
            right: Side::Open(FRACTION_PREC + 1),
            finish: Finish::Expression { expr: ExprKind::Frac },
        },
        // Root is a prefix operator with precedence higher than slash.
        MathKind::Root { index } => Operator {
            left: Side::Closed,
            right: Side::Open(ROOT_PREC),
            finish: Finish::Expression { expr: ExprKind::Root { index } },
        },
        // We want factorials to group to text, so we also make the exclamation
        // mark a tightly binding operator if there is no leading trivia.
        MathKind::Bang if has_direct_lhs => Operator {
            left: Side::Open(BANG_PREC),
            right: Side::Closed,
            finish: Finish::Expression { expr: ExprKind::Bang },
        },
        // Delimiters.
        MathKind::Opening(left) => Operator {
            left: Side::Closed,
            right: Side::Closed,
            finish: Finish::Delims { left },
        },
        // If there is no operator between tokens, this is an atomic expression
        // which is closed on the left and right. More than one of these in a
        // row will become the juxtaposition operator.
        kind => match kind.render_as_symbol() {
            Some(c) => Operator {
                left: Side::Closed,
                right: Side::Closed,
                finish: Finish::Value { value: SymbolElem::new(c.into()).into_value() },
            },
            None => Operator {
                left: Side::Closed,
                right: Side::Closed,
                finish: Finish::Value { value: TextElem::new(text).into_value() },
            },
        },
    }
}

/// Should we stop parsing because our parent expects this kind of token?
fn at_stop(token: &Token, stop_kinds: &[MathKind]) -> bool {
    matches!(token, Token::Kind(kind, _text) if stop_kinds.iter().any(|k| k == kind))
}

/// A standard pratt parser that additionally parses: juxtaposed elements,
/// chained sub/superscript operators, and parentheses removal.
fn math_expression(
    tokens: &mut TokenStream,
    parent_op: Side,
    chain_kinds: &[MathKind],
) -> Option<Spanned<Value>> {
    // TODO: If we pass in a `&mut Vec` as a param, we can just use one vector as an arena.
    let mut parsed: Vec<Value> = Vec::new();
    let mut lhs_start = 0;
    let mut juxt = Side::Closed;
    let mut active_chain: Option<ExprKind> = None;
    let mut initial_span = Span::detached();

    loop {
        // `confirm` is a closure with a mutable borrow on `tokens` and will
        // return a marker to the token location when called.
        let Some((TokenInfo { token, trivia, at_math_func }, confirm)) =
            tokens.peek_with_confirm()
        else {
            break;
        };
        if at_stop(&token, chain_kinds) {
            break;
        }
        let has_lhs = !parsed.is_empty();
        let chain = active_chain.take();
        if juxt != Side::Closed {
            active_chain = Some(ExprKind::Juxtapose);
        }
        let op = math_op(token, trivia, has_lhs, chain);

        // `lhs_start` is the initial index into `parsed` that says what values
        // an operator will use as its left side. `mark` is a Marker containing
        // the operator's span, and can only be produced by calling `confirm()`.
        let mark;
        (lhs_start, mark) = match (has_lhs, op.left) {
            // Nothing, but expected nothing. Yay.
            (false, Side::Closed) => (0, confirm()),
            // Closed but with a lhs, we infer the juxtaposition operator.
            (true, Side::Closed) => {
                // Treat juxtaposition as a higher precedence op if we're at the
                // open paren of a math function.
                let juxt_prec = if at_math_func { MATH_FUNC_PREC } else { JUXT_PREC };
                if parent_op.tighter_than(juxt_prec) {
                    break;
                }
                let mark = confirm();
                juxt = Side::Open(juxt_prec);
                active_chain = Some(ExprKind::Juxtapose);
                // Respect spaces between elements when juxtaposing.
                match trivia {
                    Trivia::HasSpaces { span } => {
                        let elem = SpaceElem::shared().clone().spanned(span);
                        parsed.push(elem.into_value());
                    }
                    Trivia::Direct | Trivia::OnlyComments => {}
                }
                // The actual operator continues as normal, but treats its left
                // side as starting after the juxtaposed elements.
                (parsed.len(), mark)
            }
            // Oops, precedence too low.
            (_, Side::Open(left_prec)) if parent_op.tighter_than(left_prec) => {
                break;
            }
            // Happy path :)
            (true, Side::Open(left_prec)) => {
                if juxt.tighter_than(left_prec) {
                    // `JUXT_PREC` is zero, which isn't tighter than anything,
                    // so our left side must be a math function.
                    assert_eq!(juxt, Side::Open(MATH_FUNC_PREC));
                    // Since our left side was a math function, we must have at
                    // least two elements at the end of `parsed`: our evaluated
                    // identifier and the delimiters.
                    let delims = parsed.pop().unwrap().display();
                    let identifier = parsed.pop().unwrap().display();
                    if parsed.is_empty() {
                        // If empty, we aren't juxtaposing anything else.
                        active_chain = None;
                    }
                    let juxtaposed = Content::sequence([identifier, delims]);
                    parsed.push(juxtaposed.into_value());
                    (parsed.len() - 1, confirm())
                } else {
                    // Otherwise, we give only _one_ lhs value to the operator.
                    (lhs_start, confirm())
                }
            }
            // Sad path :(
            (false, Side::Open(_)) => {
                let mark = confirm();
                tokens.error_at(mark, "expected a value to the left of the operator");
                // Don't try to continue the operator, but we do keep parsing.
                continue;
            }
        };
        if initial_span == Span::detached() {
            initial_span = mark.span;
        }

        // Parse an operator's right side and push it onto `parsed`.
        if let Side::Open(right_prec) = op.right {
            let kinds = match &op.finish {
                Finish::MaybeChain { expr: _, with: kinds } => *kinds,
                _ => &[],
            };
            let rhs = math_expression(tokens, Side::Open(right_prec), kinds);
            let Some(value) = rhs else {
                tokens.error_at(mark, "expected a value to the right of the operator");
                continue;
            };
            parsed.push(value.v);
        }

        // Finish the operator expression!
        let value = match op.finish {
            Finish::Value { value } => value.spanned(mark.span),
            Finish::Expression { expr } => {
                let op_values = parsed.drain(lhs_start..);
                finish_expression(expr, op_values, mark.span)
            }
            Finish::MaybeChain { expr, with: kinds } => {
                if tokens.just_peek().is_some_and(|peek| at_stop(&peek.token, kinds)) {
                    active_chain = Some(expr);
                    continue;
                }
                let op_values = parsed.drain(lhs_start..);
                finish_expression(expr, op_values, mark.span)
            }
            Finish::Delims { left } => {
                parse_delimiters(tokens, left, parent_op, juxt, mark)
            }
            Finish::ParseFuncArgs { func } => parse_function(tokens, func, mark),
        };

        // Push our value so the next operator can inspect it.
        parsed.push(value);
    }

    let value = if let Some(expr) = active_chain {
        finish_expression(expr, parsed.into_iter(), initial_span)
    } else if let Some(value) = parsed.pop() {
        assert!(parsed.is_empty());
        value
    } else if !parent_op.tighter_than(JUXT_PREC) {
        // If looser than juxtaposition, give back an empty sequence.
        Content::empty().into_value()
    } else {
        return None;
    };
    Some(Spanned::new(value, initial_span))
}

/// Use our parsed values to finish off the expression.
fn finish_expression(
    expr: ExprKind,
    mut vals: impl Iterator<Item = Value>,
    span: Span,
) -> Value {
    let mut next_content = || vals.next().unwrap().display();

    let content: Content = match expr {
        ExprKind::Juxtapose => {
            let sequence = vals.by_ref().map(Value::display);
            Content::sequence(sequence)
        }
        ExprKind::Bang => {
            let sequence = [next_content(), SymbolElem::packed('!')];
            Content::sequence(sequence)
        }
        ExprKind::Attach { primes, to } => {
            let mut attach = AttachElem::new(next_content());
            // Note: We must construct the attach this way due to the merging
            // system in `typst-library/attach.rs`, which checks if a field was
            // set at all, even if it was set to `None` (otherwise we could use
            // the builder-pattern and the `with_b` etc. functions).
            if let Some(count) = primes {
                attach = attach.with_tr(Some(PrimesElem::new(count as usize).pack()));
            }
            attach = match to {
                AttachTo::Neither => attach,
                AttachTo::Bot => attach.with_b(Some(next_content())),
                AttachTo::Top => attach.with_t(Some(next_content())),
                AttachTo::BotTop => {
                    attach.with_b(Some(next_content())).with_t(Some(next_content()))
                }
                AttachTo::TopBot => {
                    attach.with_t(Some(next_content())).with_b(Some(next_content()))
                }
            };
            attach.pack()
        }
        ExprKind::Frac => {
            let num = next_content();
            let denom = next_content();
            FracElem::new(num, denom).pack()
        }
        ExprKind::Root { index } => {
            let radicand = next_content();
            let index = index.map(|c| TextElem::packed(c).spanned(span));
            RootElem::new(radicand).with_index(index).pack()
        }
    };
    assert!(vals.next().is_none());
    content.spanned(span).into_value()
}

/// Parse delimiters. If just ascii parentheses, might only return the body
/// based on the surrounding operators.
fn parse_delimiters(
    tokens: &mut TokenStream,
    opening: char,
    parent_op: Side,
    juxt: Side,
    mark: Marker,
) -> Value {
    let (body, mode_end) = tokens
        .enter_mode(Mode::Delims, |tokens| math_expression(tokens, Side::Closed, &[]));

    let closing = match mode_end {
        // Remove parentheses if they're being used for grouping.
        Some((')', _)) if opening == '(' && remove_parens(tokens, parent_op, juxt) => {
            return body.map_or(Content::empty().into_value(), |b| b.v);
        }
        Some((closing, end_mark)) => {
            Some(SymbolElem::packed(closing).spanned(end_mark.span))
        }
        None => None,
    };
    let opening = SymbolElem::packed(opening).spanned(mark.span);
    let body = if let Some(Spanned { v, span }) = body {
        v.display().spanned(span)
    } else {
        Content::empty()
    };

    let content = if let Some(closing) = closing {
        LrElem::new(Content::sequence([opening, body, closing])).pack()
    } else {
        Content::sequence([opening, body])
    };
    content.spanned(mark.span).into_value()
}

/// Whether to remove parens based on our surrounding operators.
fn remove_parens(tokens: &TokenStream, parent_op: Side, juxt: Side) -> bool {
    // Remove parens for any parent op as long as there's no juxtaposition op.
    if parent_op != Side::Closed && juxt == Side::Closed {
        true
    } else if let Some(peek) = tokens.just_peek()
        // For upcoming operators, only fractions remove parens.
        && matches!(peek.token, Token::Kind(MathKind::Slash, _))
        // We need to compare precedence in case juxt is a math function. If
        // there's no juxtaposition or we're tighter, we'll remove parens.
        && !juxt.tighter_than(FRACTION_PREC)
    {
        true
    } else {
        // Otherwise, we'll keep the parens.
        false
    }
}

/// Parse and call a function.
fn parse_function(tokens: &mut TokenStream, func: Func, start: Marker) -> Value {
    let (items, Some((')', end))) = tokens.enter_mode(Mode::Args, parse_args) else {
        tokens.error_at(start, "unclosed delimiter");
        return Value::default();
    };
    let args = Args { span: start.span, items };
    tokens.call_func(func, args, (start, end))
}

/// State for parsing function arguments.
#[derive(Default)]
struct ArgParser {
    /// Positional arguments. If we're parsing over two-dimensions, then
    /// `two_dim_idx` divides this into array arguments and non-array args so
    /// we can reuse the same vector.
    pos: Vec<Spanned<Value>>,
    /// The start of the non-array args if parsing two-dimensions.
    two_dim_idx: Option<usize>,
    /// Named arguments plus whether they came from syntax or from a spread
    /// operator.
    named: IndexMap<Str, (Spanned<Value>, NamedSource)>,
}

/// Where did this named argument come from?
enum NamedSource {
    Syntax,
    Spread,
}

impl ArgParser {
    /// At a semicolon, split any positional arguments after `two_dim_idx` into
    /// a new array.
    fn semicolon(&mut self) {
        let idx = self.two_dim_idx.take().unwrap_or(0);
        let array = self.pos.drain(idx..).map(|spanned| spanned.v).collect();
        let value = Spanned::new(Value::Array(array), Span::detached());
        self.pos.push(value);
        self.two_dim_idx = Some(idx + 1);
    }

    /// Consume the arg parser and generate the final arguments array.
    fn finish(mut self) -> EcoVec<Arg> {
        if self.two_dim_idx.is_some_and(|idx| idx != self.pos.len()) {
            self.semicolon();
        }
        self.pos
            .into_iter()
            .map(|value| Arg { span: value.span, name: None, value })
            .chain(self.named.into_iter().map(|(name, (value, _))| Arg {
                span: value.span,
                name: Some(name),
                value,
            }))
            .collect()
    }
}

/// Parse function arguments.
fn parse_args(tokens: &mut TokenStream) -> EcoVec<Arg> {
    let mut args = ArgParser::default();
    let mut got_arg = false;
    loop {
        let Some((peek, confirm)) = tokens.peek_with_confirm() else {
            // If we peek a `None`, that means we encountered a mode-ending
            // token. If comma or semicolon, we keep parsing.
            let semicolon = tokens.advance_if_at(';');
            if !semicolon && !tokens.advance_if_at(',') {
                // Either at the close paren or the end of the token stream.
                return args.finish();
            }
            if !got_arg {
                // Insert empty content if no argument.
                let value = Content::empty().into_value();
                args.pos.push(Spanned::new(value, Span::detached()));
            }
            if semicolon {
                args.semicolon();
            }
            got_arg = false;
            continue;
        };
        got_arg = true;

        let arg_modifier = match peek.token {
            // If we have an arg start modifier, then forward the token stream
            // by calling `confirm`.
            Token::ArgStart(arg_kind) => Some((arg_kind, confirm())),
            _ => {
                // `confirm` holds a mutable borrow of `tokens`, so we have to
                // drop it before we can call `math_expression` below.
                drop(confirm);
                None
            }
        };

        let value = math_expression(tokens, Side::Closed, &[]).unwrap();

        match arg_modifier {
            Some((ArgStart::Named { name }, mark)) => {
                add_named_arg(tokens, &mut args, name, value, mark);
            }
            Some((ArgStart::Spread, mark)) => {
                add_spread_arg(tokens, &mut args, value, mark)
            }
            None => args.pos.push(value),
        }
    }
}

/// Add a named argument with its value.
fn add_named_arg(
    tokens: &mut TokenStream,
    args: &mut ArgParser,
    name: EcoString,
    value: Spanned<Value>,
    mark: Marker,
) {
    match args.named.entry(name.into()) {
        Entry::Vacant(entry) => {
            entry.insert((value, NamedSource::Syntax));
        }
        Entry::Occupied(mut entry) => match entry.get().1 {
            NamedSource::Syntax => {
                // Only error on duplicates if both came from syntax.
                let msg = eco_format!("duplicate argument: {}", entry.key());
                tokens.error_at(mark, msg);
            }
            NamedSource::Spread => {
                // Otherwise, overwrite the existing value.
                entry.insert((value, NamedSource::Syntax));
            }
        },
    }
}

/// Add the spread result of a value.
fn add_spread_arg(
    tokens: &mut TokenStream,
    args: &mut ArgParser,
    Spanned { v: value, span }: Spanned<Value>,
    mark: Marker,
) {
    // We apply the overall value's span to each spread item.
    let with_span = |v| Spanned::new(v, span);
    match value {
        Value::None => {}
        Value::Array(array) => args.pos.extend(array.into_iter().map(with_span)),
        Value::Dict(dict) => {
            for (key, val) in dict {
                match args.named.entry(key) {
                    Entry::Vacant(entry) => {
                        entry.insert((with_span(val), NamedSource::Spread));
                    }
                    Entry::Occupied(mut entry) => {
                        // Only overwrite the value, ignore whether it came
                        // from syntax or spread.
                        entry.get_mut().0 = with_span(val);
                    }
                }
            }
        }
        Value::Args(new_args) => {
            for item in new_args.items {
                match item.name {
                    Some(name) => match args.named.entry(name) {
                        Entry::Vacant(entry) => {
                            entry.insert((item.value, NamedSource::Spread));
                        }
                        Entry::Occupied(mut entry) => {
                            // Only overwrite the value, ignore whether it came
                            // from syntax or spread.
                            entry.get_mut().0 = item.value;
                        }
                    },
                    None => args.pos.push(item.value),
                }
            }
        }
        _ => tokens.error_from(mark, eco_format!("cannot spread {}", value.ty())),
    }
}
