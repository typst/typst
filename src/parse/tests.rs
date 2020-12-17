//! Parser tests.

#![allow(non_snake_case)]

use std::fmt::Debug;

use super::parse;
use crate::color::RgbaColor;
use crate::diag::Deco;
use crate::eval::DictKey;
use crate::geom::Unit;
use crate::syntax::*;

// ------------------------------ Construct Syntax Nodes ------------------------------ //

use Deco::*;
use SynNode::{Emph as E, Linebreak as L, Parbreak as P, Space as S, Strong as B};

fn T(text: &str) -> SynNode {
    SynNode::Text(text.to_string())
}

macro_rules! H {
    ($level:expr, $($tts:tt)*) => {
        SynNode::Heading(NodeHeading {
            level: Spanned::zero($level),
            contents: Tree![@$($tts)*],
        })
    };
}

macro_rules! R {
    ($lang:expr, $inline:expr, $($line:expr),* $(,)?) => {{
        SynNode::Raw(NodeRaw {
            lang: $lang,
            lines: vec![$($line.to_string()) ,*],
            inline: $inline,
        })
    }};
}

fn Lang(lang: &str) -> Option<Ident> {
    Some(Ident(lang.to_string()))
}

macro_rules! F {
    ($($tts:tt)*) => { SynNode::Expr(Expr::Call(Call!(@$($tts)*))) }
}

// ------------------------------- Construct Expressions ------------------------------ //

use BinOp::*;
use UnOp::*;
use Unit::*;

fn Id(ident: &str) -> Expr {
    Expr::Lit(Lit::Ident(Ident(ident.to_string())))
}

fn Bool(b: bool) -> Expr {
    Expr::Lit(Lit::Bool(b))
}

fn Int(int: i64) -> Expr {
    Expr::Lit(Lit::Int(int))
}

fn Float(float: f64) -> Expr {
    Expr::Lit(Lit::Float(float))
}

fn Percent(percent: f64) -> Expr {
    Expr::Lit(Lit::Percent(percent))
}

fn Length(val: f64, unit: Unit) -> Expr {
    Expr::Lit(Lit::Length(val, unit))
}

fn Color(color: RgbaColor) -> Expr {
    Expr::Lit(Lit::Color(color))
}

fn Str(string: &str) -> Expr {
    Expr::Lit(Lit::Str(string.to_string()))
}

macro_rules! Call {
    (@$name:expr $(, $span:expr)? $(; $($tts:tt)*)?) => {{
        let name = Into::<Spanned<&str>>::into($name);
        #[allow(unused)]
        let mut span = Span::ZERO;
        $(span = $span.into();)?
        ExprCall {
            name: name.map(|n| Ident(n.to_string())),
            args: Dict![@$($($tts)*)?].span_with(span),
        }
    }};
    ($($tts:tt)*) => { Expr::Call(Call![@$($tts)*]) };
}

fn Unary(op: impl Into<Spanned<UnOp>>, expr: impl Into<Spanned<Expr>>) -> Expr {
    Expr::Unary(ExprUnary {
        op: op.into(),
        expr: expr.into().map(Box::new),
    })
}

fn Binary(
    op: impl Into<Spanned<BinOp>>,
    lhs: impl Into<Spanned<Expr>>,
    rhs: impl Into<Spanned<Expr>>,
) -> Expr {
    Expr::Binary(ExprBinary {
        lhs: lhs.into().map(Box::new),
        op: op.into(),
        rhs: rhs.into().map(Box::new),
    })
}

macro_rules! Dict {
    (@dict=$dict:expr,) => {};
    (@dict=$dict:expr, $key:expr => $expr:expr $(, $($tts:tt)*)?) => {{
        let key = Into::<Spanned<&str>>::into($key);
        let key = key.map(Into::<DictKey>::into);
        let expr = Into::<Spanned<Expr>>::into($expr);
        $dict.0.push(LitDictEntry { key: Some(key), expr });
        Dict![@dict=$dict, $($($tts)*)?];
    }};
    (@dict=$dict:expr, $expr:expr $(, $($tts:tt)*)?) => {
        let expr = Into::<Spanned<Expr>>::into($expr);
        $dict.0.push(LitDictEntry { key: None, expr });
        Dict![@dict=$dict, $($($tts)*)?];
    };
    (@$($tts:tt)*) => {{
        #[allow(unused)]
        let mut dict = LitDict::new();
        Dict![@dict=dict, $($tts)*];
        dict
    }};
    ($($tts:tt)*) => { Expr::Lit(Lit::Dict(Dict![@$($tts)*])) };
}

macro_rules! Tree {
    (@$($node:expr),* $(,)?) => {
        vec![$(Into::<Spanned<SynNode>>::into($node)),*]
    };
    ($($tts:tt)*) => { Expr::Lit(Lit::Content(Tree![@$($tts)*])) };
}

// ------------------------------------ Test Macros ----------------------------------- //

// Test syntax trees with or without spans.
macro_rules! t { ($($tts:tt)*) => {test!(@spans=false, $($tts)*)} }
macro_rules! ts { ($($tts:tt)*) => {test!(@spans=true, $($tts)*)} }
macro_rules! test {
    (@spans=$spans:expr, $src:expr => $($tts:tt)*) => {
        let exp = Tree![@$($tts)*];
        let pass = parse($src);
        check($src, exp, pass.output, $spans);
    };
}

// Test expressions.
macro_rules! v {
    ($src:expr => $($tts:tt)*) => {
        t!(concat!("[val: ", $src, "]") => F!("val"; $($tts)*));
    }
}

// Test error messages.
macro_rules! e {
    ($src:expr => $($tts:tt)*) => {
        let exp = vec![$($tts)*];
        let pass = parse($src);
        let found = pass.feedback.diags.iter()
            .map(|s| s.as_ref().map(|e| e.message.as_str()))
            .collect::<Vec<_>>();
        check($src, exp, found, true);
    };
}

// Test decorations.
macro_rules! d {
    ($src:expr => $($tts:tt)*) => {
        let exp = vec![$($tts)*];
        let pass = parse($src);
        check($src, exp, pass.feedback.decos, true);
    };
}

/// Assert that expected and found are equal, printing both and panicking
/// and the source of their test case if they aren't.
///
/// When `cmp_spans` is false, spans are ignored.
#[track_caller]
pub fn check<T>(src: &str, exp: T, found: T, cmp_spans: bool)
where
    T: Debug + PartialEq,
{
    Span::set_cmp(cmp_spans);
    let equal = exp == found;
    Span::set_cmp(true);

    if !equal {
        println!("source:   {:?}", src);
        if cmp_spans {
            println!("expected: {:#?}", exp);
            println!("found:    {:#?}", found);
        } else {
            println!("expected: {:?}", exp);
            println!("found:    {:?}", found);
        }
        panic!("test failed");
    }
}

pub fn s<T>(start: u32, end: u32, v: T) -> Spanned<T> {
    v.span_with(Span::new(start, end))
}

// Enables tests to optionally specify spans.
impl<T> From<T> for Spanned<T> {
    fn from(t: T) -> Self {
        Spanned::zero(t)
    }
}

// --------------------------------------- Tests -------------------------------------- //

#[test]
fn test_parse_groups() {
    e!("[)" => s(1, 2, "expected function name, found closing paren"),
               s(2, 2, "expected closing bracket"));

    e!("[v:{]}" => s(4, 4, "expected closing brace"),
                   s(5, 6, "unexpected closing brace"));
}

#[test]
fn test_parse_simple_nodes() {
    t!(""               => );
    t!("hi"             => T("hi"));
    t!("*hi"            => B, T("hi"));
    t!("hi_"            => T("hi"), E);
    t!("hi you"         => T("hi"), S, T("you"));
    t!("special~name"   => T("special"), T("\u{00A0}"), T("name"));
    t!("special\\~name" => T("special"), T("~"), T("name"));
    t!("\\u{1f303}"     => T("🌃"));
    t!("\n\n\nhello"    => P, T("hello"));
    t!(r"a\ b"          => T("a"), L, S, T("b"));

    e!("\\u{d421c809}"    => s(0, 12, "invalid unicode escape sequence"));
    e!("\\u{abc"          => s(6, 6, "expected closing brace"));
    t!("💜\n\n 🌍"       => T("💜"), P, T("🌍"));

    ts!("hi"   => s(0, 2, T("hi")));
    ts!("*Hi*" => s(0, 1, B), s(1, 3, T("Hi")), s(3, 4, B));
    ts!("💜\n\n 🌍" => s(0, 4, T("💜")), s(4, 7, P), s(7, 11, T("🌍")));
}

#[test]
fn test_parse_raw() {
    t!("`py`"            => R![None, true, "py"]);
    t!("`hi\nyou"        => R![None, true, "hi", "you"]);
    t!(r"`` hi\`du``"    => R![None, true, r"hi\`du"]);

    // More than one backtick with optional language tag.
    t!("``` console.log(\n\"alert\"\n)" => R![None, false, "console.log(", "\"alert\"", ")"]);
    t!("````typst \r\n Typst uses ``` to indicate code blocks````!"
        => R![Lang("typst"), false, " Typst uses ``` to indicate code blocks"], T("!"));

    // Trimming of whitespace.
    t!("`` a ``"         => R![None, true, "a"]);
    t!("`` a  ``"        => R![None, true, "a "]);
    t!("`` ` ``"         => R![None, true, "`"]);
    t!("```  `   ```"    => R![None, true, " `  "]);
    t!("```  `   \n ```" => R![None, false, " `   "]);

    // Errors.
    e!("`hi\nyou"         => s(7, 7, "expected backtick(s)"));
    e!("``` hi\nyou"      => s(10, 10, "expected backtick(s)"));

    // TODO: Bring back when spans/errors are in place.
    // ts!("``java out``" => s(0, 12, R![Lang(s(2, 6, "java")), true, "out"]));
    // e!("```🌍 hi\nyou```" => s(3, 7, "invalid identifier"));
}

#[test]
fn test_parse_comments() {
    // In body.
    t!("hi// you\nw"          => T("hi"), S, T("w"));
    t!("first//\n//\nsecond"  => T("first"), S, S, T("second"));
    t!("first//\n \nsecond"   => T("first"), P, T("second"));
    t!("first/*\n \n*/second" => T("first"), T("second"));
    e!("🌎\n*/n" => s(5, 7, "unexpected end of block comment"));

    // In header.
    t!("[val:/*12pt*/]"          => F!("val"));
    t!("[val \n /* \n */:]"      => F!("val"));
    e!("[val \n /* \n */:]"      => );
    e!("[val : 12, /* \n */ 14]" => );
}

#[test]
fn test_parse_headings() {
    t!("## Hello world!" => H![1, S, T("Hello"), S, T("world!")]);

    // Handle various whitespace usages.
    t!("####Simple"                         => H![3, T("Simple")]);
    t!("  #    Whitespace!"                 => S, H![0, S, T("Whitespace!")]);
    t!("  /* TODO: Improve */  ## Analysis" => S, S, H!(1, S, T("Analysis")));
    t!("# Heading \n ends"                  => H![0, S, T("Heading")], S, T("ends"));

    // Complex heading contents.
    t!("Some text [box][### Valuable facts]" => T("Some"), S, T("text"), S,
        F!("box"; Tree![H!(2, S, T("Valuable"), S, T("facts"))])
    );
    t!("### Grandiose stuff [box][Get it \n\n straight]" => H![
        2,
        S, T("Grandiose"), S, T("stuff"), S,
        F!("box"; Tree![T("Get"), S, T("it"), P, T("straight")])
    ]);
    t!("###### Multiline \\ headings" => H![5, S, T("Multiline"), S, L, S, T("headings")]);

    // Things that should not become headings.
    t!("\\## Text"      => T("#"), T("#"), S, T("Text"));
    t!(" ###### # Text" => S, H![5, S, T("#"), S, T("Text")]);
    t!("I am #1"        => T("I"), S, T("am"), S, T("#"), T("1"));
    t!("[box][\n] # hi" => F!("box"; Tree![S]), S, T("#"), S, T("hi"));

    // Depth warnings.
    e!("########" => s(0, 8, "section depth should be at most 6"));
}

#[test]
fn test_parse_function_names() {
    // No closing bracket.
    t!("[" => F!(""));
    e!("[" => s(1, 1, "expected function name"),
              s(1, 1, "expected closing bracket"));

    // No name.
    e!("[]"   => s(1, 1, "expected function name"));
    e!("[\"]" => s(1, 3, "expected function name, found string"),
                 s(3, 3, "expected closing bracket"));

    // A valid name.
    t!("[hi]"  => F!("hi"));
    t!("[  f]" => F!("f"));

    // An invalid name.
    e!("[12]"   => s(1, 3, "expected function name, found integer"));
    e!("[  🌎]" => s(3, 7, "expected function name, found invalid token"));
}

#[test]
fn test_parse_chaining() {
    // Things the parser has to make sense of
    t!("[hi: (5.0, 2.1 >> you]" => F!("hi"; Dict![Float(5.0), Float(2.1)], Tree![F!("you")]));
    t!("[box >> pad: 1pt][Hi]"  => F!("box"; Tree![
        F!("pad"; Length(1.0, Pt), Tree!(T("Hi")))
    ]));
    t!("[bold: 400, >> emph >> sub: 1cm]" => F!("bold"; Int(400), Tree![
        F!("emph"; Tree!(F!("sub"; Length(1.0, Cm))))
    ]));

    // Errors for unclosed / empty predecessor groups
    e!("[hi: (5.0, 2.1 >> you]" => s(15, 15, "expected closing paren"));
    e!("[>> abc]"               => s(1, 1, "expected function name"));
    e!("[box >>][Hi]"           => s(7, 7, "expected function name"));
}

#[test]
fn test_parse_colon_starting_func_args() {
    // Just colon without args.
    e!("[val:]" => );

    // Wrong token.
    t!("[val=]"     => F!("val"));
    e!("[val=]"     => s(4, 4, "expected colon"));
    e!("[val/🌎:$]" => s(4, 4, "expected colon"));

    // String in invalid header without colon still parsed as string
    // _Note_: No "expected quote" error because not even the string was
    //       expected.
    e!("[val/\"]" => s(4, 4, "expected colon"),
                     s(7, 7, "expected closing bracket"));
}

#[test]
fn test_parse_function_bodies() {
    t!("[val: 1][*Hi*]" => F!("val"; Int(1), Tree![B, T("Hi"), B]));
    e!(" [val][ */]"    => s(8, 10, "unexpected end of block comment"));

    // Raw in body.
    t!("[val][`Hi]`" => F!("val"; Tree![R![None, true, "Hi]"]]));
    e!("[val][`Hi]`" => s(11, 11, "expected closing bracket"));

    // Crazy.
    t!("[v][[v][v][v]]" => F!("v"; Tree![F!("v"; Tree![T("v")]), F!("v")]));

    // Spanned.
    ts!(" [box][Oh my]" =>
        s(0, 1, S),
        s(1, 13, F!(s(2, 5, "box"), 5 .. 5;
            s(6, 13, Tree![
                s(7, 9, T("Oh")), s(9, 10, S), s(10, 12, T("my")),
            ])
        ))
    );
}

#[test]
fn test_parse_values() {
    // Simple.
    v!("_"         => Id("_"));
    v!("name"      => Id("name"));
    v!("ke-bab"    => Id("ke-bab"));
    v!("α"         => Id("α"));
    v!("\"hi\""    => Str("hi"));
    v!("true"      => Bool(true));
    v!("false"     => Bool(false));
    v!("1.0e-4"    => Float(1e-4));
    v!("3.15"      => Float(3.15));
    v!("50%"       => Percent(50.0));
    v!("4.5cm"     => Length(4.5, Cm));
    v!("12e1pt"    => Length(12e1, Pt));
    v!("#f7a20500" => Color(RgbaColor::new(0xf7, 0xa2, 0x05, 0x00)));
    v!("\"a\n[]\\\"string\"" => Str("a\n[]\"string"));

    // Content.
    v!("{_hi_}"        => Tree![E, T("hi"), E]);
    e!("[val: {_hi_}]" => );
    v!("[hi]"          => Tree![F!("hi")]);
    e!("[val: [hi]]"   => );

    // Healed colors.
    v!("#12345"            => Color(RgbaColor::with_healed(0, 0, 0, 0xff, true)));
    e!("[val: #12345]"     => s(6, 12, "invalid color"));
    e!("[val: #a5]"        => s(6, 9,  "invalid color"));
    e!("[val: #14b2ah]"    => s(6, 13, "invalid color"));
    e!("[val: #f075ff011]" => s(6, 16, "invalid color"));

    // Unclosed string.
    v!("\"hello"        => Str("hello]"));
    e!("[val: \"hello]" => s(13, 13, "expected quote"),
                           s(13, 13, "expected closing bracket"));

    // Spanned.
    ts!("[val: 1.4]" => s(0, 10, F!(s(1, 4, "val"), 6 .. 9; s(6, 9, Float(1.4)))));
}

#[test]
fn test_parse_expressions() {
    // Coerced dict.
    v!("(hi)" => Id("hi"));

    // Operations.
    v!("-1"          => Unary(Neg, Int(1)));
    v!("-- 1"        => Unary(Neg, Unary(Neg, Int(1))));
    v!("--css"       => Unary(Neg, Unary(Neg, Id("css"))));
    v!("3.2in + 6pt" => Binary(Add, Length(3.2, In), Length(6.0, Pt)));
    v!("5 - 0.01"    => Binary(Sub, Int(5), Float(0.01)));
    v!("(3mm * 2)"   => Binary(Mul, Length(3.0, Mm), Int(2)));
    v!("12e-3cm/1pt" => Binary(Div, Length(12e-3, Cm), Length(1.0, Pt)));

    // More complex.
    v!("(3.2in + 6pt)*(5/2-1)" => Binary(
        Mul,
        Binary(Add, Length(3.2, In), Length(6.0, Pt)),
        Binary(Sub, Binary(Div, Int(5), Int(2)), Int(1))
    ));
    v!("(6.3E+2+4* - 3.2pt)/2" => Binary(
        Div,
        Binary(Add, Float(6.3e2), Binary(
            Mul,
            Int(4),
            Unary(Neg, Length(3.2, Pt))
        )),
        Int(2)
    ));

    // Associativity of multiplication and division.
    v!("3/4*5" => Binary(Mul, Binary(Div, Int(3), Int(4)), Int(5)));

    // Spanned.
    ts!("[val: 1 + 3]" => s(0, 12, F!(
        s(1, 4, "val"), 6 .. 11; s(6, 11, Binary(
            s(8, 9, Add),
            s(6, 7, Int(1)),
            s(10, 11, Int(3))
        ))
    )));

    // Span of parenthesized expression contains parens.
    ts!("[val: (1)]" => s(0, 10, F!(s(1, 4, "val"), 6 .. 9; s(6, 9, Int(1)))));

    // Invalid expressions.
    v!("4pt--"        => Length(4.0, Pt));
    e!("[val: 4pt--]" => s(10, 11, "missing factor"),
                         s(6, 10, "missing right summand"));

    v!("3mm+4pt*"        => Binary(Add, Length(3.0, Mm), Length(4.0, Pt)));
    e!("[val: 3mm+4pt*]" => s(10, 14, "missing right factor"));
}

#[test]
fn test_parse_dicts() {
    // Okay.
    v!("()"                 => Dict![]);
    v!("(false)"            => Bool(false));
    v!("(true,)"            => Dict![Bool(true)]);
    v!("(key=val)"          => Dict!["key" => Id("val")]);
    v!("(1, 2)"             => Dict![Int(1), Int(2)]);
    v!("(1, key=\"value\")" => Dict![Int(1), "key" => Str("value")]);

    // Decorations.
    d!("[val: key=hi]"    => s(6, 9, DictKey));
    d!("[val: (key=hi)]"  => s(7, 10, DictKey));
    d!("[val: f(key=hi)]" => s(8, 11, DictKey));

    // Spanned with spacing around keyword arguments.
    ts!("[val: \n hi \n = /* //\n */ \"s\n\"]" => s(0, 30, F!(
        s(1, 4, "val"),
        8 .. 29; s(8, 10, "hi") => s(25, 29, Str("s\n"))
    )));
    e!("[val: \n hi \n = /* //\n */ \"s\n\"]" => );
}

#[test]
fn test_parse_dicts_compute_func_calls() {
    v!("empty()"                  => Call!("empty"));
    v!("add ( 1 , 2 )"            => Call!("add"; Int(1), Int(2)));
    v!("items(\"fire\", #f93a6d)" => Call!("items";
        Str("fire"), Color(RgbaColor::new(0xf9, 0x3a, 0x6d, 0xff))
    ));

    // More complex.
    v!("css(1pt, rgb(90, 102, 254), \"solid\")" => Call!(
        "css";
        Length(1.0, Pt),
        Call!("rgb"; Int(90), Int(102), Int(254)),
        Str("solid"),
    ));

    // Unclosed.
    v!("lang(中文]"       => Call!("lang"; Id("中文")));
    e!("[val: lang(中文]" => s(17, 17, "expected closing paren"));

    // Invalid name.
    v!("👠(\"abc\", 13e-5)"        => Dict!(Str("abc"), Float(13.0e-5)));
    e!("[val: 👠(\"abc\", 13e-5)]" => s(6, 10, "invalid token"));
}

#[test]
fn test_parse_dicts_nested() {
    v!("(1, ( ab=(), d = (3, 14pt) )), false" =>
        Dict![
            Int(1),
            Dict!(
                "ab" => Dict![],
                "d"  => Dict!(Int(3), Length(14.0, Pt)),
            ),
        ],
        Bool(false),
    );
}

#[test]
fn test_parse_dicts_errors() {
    // Expected value.
    e!("[val: (=)]"         => s(7, 8, "unexpected equals sign"));
    e!("[val: (,)]"         => s(7, 8, "unexpected comma"));
    v!("(\x07 abc,)"        => Dict![Id("abc")]);
    e!("[val: (\x07 abc,)]" => s(7, 8, "invalid token"));
    e!("[val: (key=,)]"     => s(11, 12, "expected value, found comma"));
    e!("[val: hi,)]"        => s(9, 10, "unexpected closing paren"));

    // Expected comma.
    v!("(true false)"        => Dict![Bool(true), Bool(false)]);
    e!("[val: (true false)]" => s(11, 11, "expected comma"));

    // Expected closing paren.
    e!("[val: (#000]" => s(11, 11, "expected closing paren"));
    e!("[val: (key]"  => s(10, 10, "expected closing paren"));
    e!("[val: (key=]" => s(11, 11, "expected value"),
                         s(11, 11, "expected closing paren"));

    // Bad key.
    v!("true=you"        => Bool(true), Id("you"));
    e!("[val: true=you]" => s(10, 11, "unexpected equals sign"));

    // Unexpected equals sign.
    v!("z=y=4"        => "z" => Id("y"), Int(4));
    e!("[val: z=y=4]" => s(9, 10, "unexpected equals sign"));
}
