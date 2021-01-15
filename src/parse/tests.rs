#![allow(non_snake_case)]

use std::fmt::Debug;

use super::parse;
use crate::diag::{Diag, Level, Pass};
use crate::geom::LengthUnit;
use crate::syntax::*;

use BinOp::*;
use Expr::{Float, Int, Length};
use Node::{Space, Strong};
use UnOp::{Neg, Pos};

macro_rules! t {
    ($src:literal
        nodes: [$($node:expr),* $(,)?]
        $(, errors: [$($err:expr),* $(,)?])?
        $(, warnings: [$($warn:expr),* $(,)?])?
        $(, spans: $spans:expr)?
        $(,)?
    ) => {{
        #[allow(unused)]
        let mut spans = false;
        $(spans = $spans;)?

        let Pass { output, feedback } = parse($src);
        check($src, Template![@$($node),*], output, spans);
        check(
            $src,
            vec![
                $($(into!($err).map(|s: &str| Diag::new(Level::Error, s)),)*)?
                $($(into!($warn).map(|s: &str| Diag::new(Level::Warning, s)),)*)?
            ],
            feedback.diags,
            true,
        );
    }};

    ($src:literal $($node:expr),* $(,)?) => {
        t!($src nodes: [$($node),*]);
    };
}

/// Assert that expected and found are equal, printing both and the source of
/// the test case if they aren't.
///
/// When `cmp_spans` is false, spans are ignored.
#[track_caller]
pub fn check<T>(src: &str, exp: T, found: T, cmp_spans: bool)
where
    T: Debug + PartialEq,
{
    Span::set_cmp(cmp_spans);

    if exp != found {
        println!("source:   {:?}", src);
        println!("expected: {:#?}", exp);
        println!("found:    {:#?}", found);
        panic!("test failed");
    }

    Span::set_cmp(true);
}

/// Shorthand for `Spanned::new`.
fn S<T>(span: impl Into<Span>, v: T) -> Spanned<T> {
    Spanned::new(v, span)
}

// Enables tests to optionally specify spans.
impl<T> From<T> for Spanned<T> {
    fn from(t: T) -> Self {
        Spanned::zero(t)
    }
}

/// Shorthand for `Into::<Spanned<_>>::into`.
macro_rules! into {
    ($val:expr) => {
        Into::<Spanned<_>>::into($val)
    };
}

fn Text(text: &str) -> Node {
    Node::Text(text.into())
}

fn Raw(lang: Option<&str>, lines: &[&str], inline: bool) -> Node {
    Node::Raw(NodeRaw {
        lang: lang.map(|id| Ident(id.into())),
        lines: lines.iter().map(ToString::to_string).collect(),
        inline,
    })
}

fn Id(ident: &str) -> Expr {
    Expr::Ident(Ident(ident.to_string()))
}

fn Str(string: &str) -> Expr {
    Expr::Str(string.to_string())
}

fn Binary(
    lhs: impl Into<Spanned<Expr>>,
    op: impl Into<Spanned<BinOp>>,
    rhs: impl Into<Spanned<Expr>>,
) -> Expr {
    Expr::Binary(ExprBinary {
        lhs: Box::new(lhs.into()),
        op: op.into(),
        rhs: Box::new(rhs.into()),
    })
}

fn Unary(op: impl Into<Spanned<UnOp>>, expr: impl Into<Spanned<Expr>>) -> Expr {
    Expr::Unary(ExprUnary {
        op: op.into(),
        expr: Box::new(expr.into()),
    })
}

fn Group(expr: Expr) -> Expr {
    Expr::Group(Box::new(expr))
}

macro_rules! Call {
    (@@$name:expr) => {
        Call!(@@$name, Args![])
    };
    (@@$name:expr, $args:expr) => {
        ExprCall {
            name: into!($name).map(|s: &str| Ident(s.into())),
            args: into!($args),
        }
    };
    (@$($tts:tt)*) => {
        Expr::Call(Call!(@@$($tts)*))
    };
    ($($tts:tt)*) => {
        Node::Expr(Call!(@$($tts)*))
    };
}

macro_rules! Args {
    (@$a:expr) => {
        Argument::Pos(into!($a))
    };
    (@$a:expr => $b:expr) => {
        Argument::Named(Named {
            name: into!($a).map(|s: &str| Ident(s.into())),
            expr: into!($b)
        })
    };
    ($($a:expr $(=> $b:expr)?),* $(,)?) => {
        vec![$(Args!(@$a $(=> $b)?)),*]
    };
}

macro_rules! Array {
    (@$($expr:expr),* $(,)?) => {
        vec![$(into!($expr)),*]
    };
    ($($tts:tt)*) => {
        Expr::Array(Array![@$($tts)*])
    };
}

macro_rules! Template {
    (@$($node:expr),* $(,)?) => {
        vec![$(into!($node)),*]
    };
    ($($tts:tt)*) => {
        Expr::Template(Template![@$($tts)*])
    };
}

macro_rules! Block {
    (@$expr:expr) => {
        Expr::Block(Box::new($expr))
    };
    ($expr:expr) => {
        Node::Expr(Block!(@$expr))
    };
}

#[test]
fn test_parse_raw() {
    // Basic, mostly tested in tokenizer and resolver.
    t!("`py`" nodes: [S(0..4, Raw(None, &["py"], true))], spans: true);
    t!("`endless"
        nodes: [Raw(None, &["endless"], true)],
        errors: [S(8..8, "expected backtick(s)")]);
}

#[test]
fn test_parse_groups() {
    // Test paren group.
    t!("{({1) + 3}"
        nodes: [Block!(Binary(Group(Block!(@Int(1))), Add, Int(3)))],
        errors: [S(4..4, "expected closing brace")]);

    // Test bracket group.
    t!("[)"
        nodes: [Call!("")],
        errors: [S(1..2, "expected function name, found closing paren"),
                 S(2..2, "expected closing bracket")]);

    t!("[v [*]"
        nodes: [Call!("v", Args![Template![Strong]])],
        errors: [S(6..6, "expected closing bracket")]);

    // Test brace group.
    t!("{1 + [}"
        nodes: [Block!(Binary(Int(1), Add, Template![]))],
        errors: [S(6..6, "expected closing bracket")]);

    // Test subheader group.
    t!("[v (|u )]"
        nodes: [Call!("v", Args![Array![], Template![Call!("u")]])],
        errors: [S(4..4, "expected closing paren"),
                 S(7..8, "expected expression, found closing paren")]);
}

#[test]
fn test_parse_blocks() {
    // Basic with spans.
    t!("{1}" nodes: [S(0..3, Block!(Int(1)))], spans: true);

    // Function calls.
    t!("{f()}" Block!(Call!(@"f")));
    t!("{[[f]]}" Block!(Template![Call!("f")]));

    // Missing or bad value.
    t!("{}{1u}"
        nodes: [],
        errors: [S(1..1, "expected expression"),
                 S(3..5, "expected expression, found invalid token")]);

    // Too much stuff.
    t!("{1 #{} end"
        nodes: [Block!(Int(1)), Space, Text("end")],
        errors: [S(3..4, "unexpected hex value"),
                 S(4..5, "unexpected opening brace")]);
}

#[test]
fn test_parse_expressions() {
    // Parentheses.
    t!("{(x)}{(1)}" Block!(Group(Id("x"))), Block!(Group(Int(1))));

    // Unary operations.
    t!("{+1}"  Block!(Unary(Pos, Int(1))));
    t!("{-1}"  Block!(Unary(Neg, Int(1))));
    t!("{--1}" Block!(Unary(Neg, Unary(Neg, Int(1)))));

    // Binary operations.
    t!(r#"{"x"+"y"}"# Block!(Binary(Str("x"), Add, Str("y"))));
    t!("{1-2}"        Block!(Binary(Int(1), Sub, Int(2))));
    t!("{a * b}"      Block!(Binary(Id("a"), Mul, Id("b"))));
    t!("{12pt/.4}"    Block!(Binary(Length(12.0, LengthUnit::Pt), Div, Float(0.4))));

    // Associativity.
    t!("{1+2+3}" Block!(Binary(Binary(Int(1), Add, Int(2)), Add, Int(3))));
    t!("{1/2*3}" Block!(Binary(Binary(Int(1), Div, Int(2)), Mul, Int(3))));

    // Precedence.
    t!("{1+2*-3}" Block!(Binary(
        Int(1), Add, Binary(Int(2), Mul, Unary(Neg, Int(3))),
    )));

    // Confusion with floating-point literal.
    t!("{1e-3-4e+4}" Block!(Binary(Float(1e-3), Sub, Float(4e+4))));

    // Spans + parentheses winning over precedence.
    t!("{(1+2)*3}"
        nodes: [S(0..9, Block!(Binary(
            S(1..6, Group(Binary(S(2..3, Int(1)), S(3..4, Add), S(4..5, Int(2))))),
            S(6..7, Mul),
            S(7..8, Int(3)),
        )))],
        spans: true);

    // Errors.
    t!("{-}{1+}{2*}"
        nodes: [Block!(Int(1)), Block!(Int(2))],
        errors: [S(2..2, "expected expression"),
                 S(6..6, "expected expression"),
                 S(10..10, "expected expression")]);
}
