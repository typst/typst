#![allow(non_snake_case)]

use super::parse;
use crate::color::RgbaColor;
use crate::compute::table::SpannedEntry;
use crate::length::Length;
use crate::syntax::decoration::Decoration::*;
use crate::syntax::span::Spanned;
use crate::syntax::tests::*;
use crate::syntax::tree::*;

// ------------------------------ Construct Syntax Nodes ------------------------------ //

use SyntaxNode::{
    Linebreak as L, Parbreak as P, Spacing as S, ToggleBolder as B, ToggleItalic as I,
};

fn T(text: &str) -> SyntaxNode {
    SyntaxNode::Text(text.to_string())
}

macro_rules! H {
    ($level:expr, $($tts:tt)*) => {
        SyntaxNode::Heading(Heading {
            level: Spanned::zero($level),
            tree: Tree![@$($tts)*],
        })
    };
}

macro_rules! R {
    ($($line:expr),* $(,)?) => {
        SyntaxNode::Raw(vec![$($line.to_string()),*])
    };
}

macro_rules! C {
    ($lang:expr, $($line:expr),* $(,)?) => {{
        let lines = vec![$($line.to_string()) ,*];
        SyntaxNode::Code(Code {
            lang: $lang,
            block: lines.len() > 1,
            lines,
        })
    }};
}

fn Lang<'a, T: Into<Spanned<&'a str>>>(lang: T) -> Option<Spanned<Ident>> {
    Some(Into::<Spanned<&str>>::into(lang).map(|s| Ident(s.to_string())))
}

macro_rules! F {
    ($($tts:tt)*) => { SyntaxNode::Call(Call!(@$($tts)*)) }
}

// ------------------------------- Construct Expressions ------------------------------ //

use Expr::{Bool, Color, Length as Len, Number as Num};

fn Id(ident: &str) -> Expr {
    Expr::Ident(Ident(ident.to_string()))
}
fn Str(string: &str) -> Expr {
    Expr::Str(string.to_string())
}

macro_rules! Table {
    (@table=$table:expr,) => {};
    (@table=$table:expr, $key:expr => $value:expr $(, $($tts:tt)*)?) => {{
        let key = Into::<Spanned<&str>>::into($key);
        let val = Into::<Spanned<Expr>>::into($value);
        $table.insert(key.v, SpannedEntry::new(key.span, val));
        Table![@table=$table, $($($tts)*)?];
    }};
    (@table=$table:expr, $value:expr $(, $($tts:tt)*)?) => {
        let val = Into::<Spanned<Expr>>::into($value);
        $table.push(SpannedEntry::val(val));
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

macro_rules! Tree {
    (@$($node:expr),* $(,)?) => {
        vec![$(Into::<Spanned<SyntaxNode>>::into($node)),*]
    };
    ($($tts:tt)*) => { Expr::Tree(Tree![@$($tts)*]) };
}

macro_rules! Call {
    (@$name:expr $(; $($tts:tt)*)?) => {{
        let name = Into::<Spanned<&str>>::into($name);
        CallExpr {
            name: name.map(|n| Ident(n.to_string())),
            args: Table![@$($($tts)*)?],
        }
    }};
    ($($tts:tt)*) => { Expr::Call(Call![@$($tts)*]) };
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
        let pass = parse($src);
        check($src, exp, pass.feedback.decorations, true);
    };
}

// --------------------------------------- Tests -------------------------------------- //

#[test]
fn test_parse_groups() {
    e!("[)" => s(0,1, 0,2, "expected function name, found closing paren"),
               s(0,2, 0,2, "expected closing bracket"));

    e!("[v:{]}" => s(0,4, 0,4, "expected closing brace"),
                   s(0,5, 0,6, "unexpected closing brace"));
}

#[test]
fn test_parse_simple_nodes() {
    t!(""               => );
    t!("hi"             => T("hi"));
    t!("*hi"            => B, T("hi"));
    t!("hi_"            => T("hi"), I);
    t!("hi you"         => T("hi"), S, T("you"));
    t!("special~name"   => T("special"), T("\u{00A0}"), T("name"));
    t!("special\\~name" => T("special"), T("~"), T("name"));
    t!("\\u{1f303}"     => T("🌃"));
    t!("\n\n\nhello"    => P, T("hello"));
    t!(r"a\ b"          => T("a"), L, S, T("b"));
    t!("`py`"           => R!["py"]);
    t!("`hi\nyou"       => R!["hi", "you"]);
    e!("`hi\nyou"       => s(1,3, 1,3, "expected backtick"));
    t!("`hi\\`du`"      => R!["hi`du"]);

    ts!("```java out```" => s(0,0, 0,14, C![Lang(s(0,3, 0,7, "java")), "out"]));
    t!("``` console.log(\n\"alert\"\n)" => C![None, "console.log(", "\"alert\"", ")"]);
    t!("```typst \r\n Typst uses `\\`` to indicate code blocks" => C![
        Lang("typst"), " Typst uses ``` to indicate code blocks"
    ]);

    e!("``` hi\nyou"      => s(1,3, 1,3,  "expected backticks"));
    e!("```🌍 hi\nyou```" => s(0,3, 0,4,  "invalid identifier"));
    e!("\\u{d421c809}"    => s(0,0, 0,12, "invalid unicode escape sequence"));
    e!("\\u{abc"          => s(0,6, 0,6, "expected closing brace"));
    t!("💜\n\n 🌍"       => T("💜"), P, T("🌍"));

    ts!("hi"   => s(0,0, 0,2, T("hi")));
    ts!("*Hi*" => s(0,0, 0,1, B), s(0,1, 0,3, T("Hi")), s(0,3, 0,4, B));
    ts!("💜\n\n 🌍" => s(0,0, 0,1, T("💜")), s(0,1, 2,1, P), s(2,1, 2,2, T("🌍")));
}

#[test]
fn test_parse_comments() {
    // In body.
    t!("hi// you\nw"          => T("hi"), S, T("w"));
    t!("first//\n//\nsecond"  => T("first"), S, S, T("second"));
    t!("first//\n \nsecond"   => T("first"), P, T("second"));
    t!("first/*\n \n*/second" => T("first"), T("second"));
    e!("🌎\n*/n" => s(1,0, 1,2, "unexpected end of block comment"));

    // In header.
    t!("[val:/*12pt*/]"          => F!("val"));
    t!("[val \n /* \n */:]"      => F!("val"));
    e!("[val \n /* \n */:]"      => );
    e!("[val : 12, /* \n */ 14]" => );
}

#[test]
fn test_parse_headings() {
    t!("## Hello world!" => H![1, T("Hello"), S, T("world!")]);

    // Handle various whitespace usages.
    t!("####Simple"                         => H![3, T("Simple")]);
    t!("  #    Whitespace!"                 => S, H![0, T("Whitespace!")]);
    t!("  /* TODO: Improve */  ## Analysis" => S, S, H!(1, T("Analysis")));

    // Complex heading contents.
    t!("Some text [box][### Valuable facts]" => T("Some"), S, T("text"), S,
        F!("box"; Tree![H!(2, T("Valuable"), S, T("facts"))])
    );
    t!("### Grandiose stuff [box][Get it \n\n straight]" => H![2,
        T("Grandiose"), S, T("stuff"), S,
        F!("box"; Tree![T("Get"), S, T("it"), P, T("straight")])
    ]);
    t!("###### Multiline \\ headings" => H![5, T("Multiline"), S, L, S, T("headings")]);

    // Things that should not become headings.
    t!("\\## Text"      => T("#"), T("#"), S, T("Text"));
    t!(" ###### # Text" => S, H!(5, T("#"), S, T("Text")));
    t!("I am #1"        => T("I"), S, T("am"), S, T("#"), T("1"));
    t!("[box][\n] # hi" => F!("box"; Tree![S]), S, T("#"), S, T("hi"));

    // Depth warnings.
    e!("########" => s(0,0, 0,8, "section depth larger than 6 has no effect"));
}

#[test]
fn test_parse_function_names() {
    // No closing bracket.
    t!("[" => F!(""));
    e!("[" => s(0,1, 0,1, "expected function name"),
                s(0,1, 0,1, "expected closing bracket"));

    // No name.
    e!("[]"   => s(0,1, 0,1, "expected function name"));
    e!("[\"]" => s(0,1, 0,3, "expected function name, found string"),
                    s(0,3, 0,3, "expected closing bracket"));

    // A valid name.
    t!("[hi]"  => F!("hi"));
    t!("[  f]" => F!("f"));

    // An invalid name.
    e!("[12]"   => s(0,1, 0,3, "expected function name, found number"));
    e!("[  🌎]" => s(0,3, 0,4, "expected function name, found invalid token"));
}

#[test]
fn test_parse_chaining() {
    // Things the parser has to make sense of
    t!("[hi: (5.0, 2.1 >> you]" => F!("hi"; Table![Num(5.0), Num(2.1)], Tree![F!("you")]));
    t!("[box >>][Hi]"           => F!("box"; Tree![T("Hi")]));
    t!("[box >> pad: 1pt][Hi]"  => F!("box"; Tree![
        F!("pad"; Len(Length::pt(1.0)), Tree!(T("Hi")))
    ]));
    t!("[bold: 400, >> emph >> sub: 1cm]" => F!("bold"; Num(400.0), Tree![
        F!("emph"; Tree!(F!("sub"; Len(Length::cm(1.0)))))
    ]));

    // Errors for unclosed / empty predecessor groups
    e!("[hi: (5.0, 2.1 >> you]" => s(0, 15, 0, 15, "expected closing paren"));
    e!("[>> abc]" => s(0, 1, 0, 1, "expected function name"));
}

#[test]
fn test_parse_colon_starting_func_args() {
    // Just colon without args.
    e!("[val:]" => );

    // Wrong token.
    t!("[val=]"     => F!("val"));
    e!("[val=]"     => s(0,4, 0,4, "expected colon"));
    e!("[val/🌎:$]" => s(0,4, 0,4, "expected colon"));

    // String in invalid header without colon still parsed as string
    // Note: No "expected quote" error because not even the string was
    //       expected.
    e!("[val/\"]" => s(0,4, 0,4, "expected colon"),
                        s(0,7, 0,7, "expected closing bracket"));
}

#[test]
fn test_parse_function_bodies() {
    t!("[val: 1][*Hi*]" => F!("val"; Num(1.0), Tree![B, T("Hi"), B]));
    e!(" [val][ */ ]"   => s(0,8, 0,10, "unexpected end of block comment"));

    // Raw in body.
    t!("[val][`Hi]`" => F!("val"; Tree![R!["Hi]"]]));
    e!("[val][`Hi]`" => s(0,11, 0,11, "expected closing bracket"));

    // Crazy.
    t!("[v][[v][v][v]]" => F!("v"; Tree![F!("v"; Tree![T("v")]), F!("v")]));

    // Spanned.
    ts!(" [box][Oh my]" =>
        s(0,0, 0,1, S),
        s(0,1, 0,13, F!(s(0,2, 0,5, "box");
            s(0,6, 0,13, Tree![
                s(0,7, 0,9, T("Oh")), s(0,9, 0,10, S), s(0,10, 0,12, T("my"))
            ])
        ))
    );
}

#[test]
fn test_parse_values() {
    // Simple.
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

    // Content.
    v!("{_hi_}"        => Tree![I, T("hi"), I]);
    e!("[val: {_hi_}]" => );
    v!("[hi]"          => Tree![F!("hi")]);
    e!("[val: [hi]]"   => );

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
    ts!("[val: 1.4]" => s(0,0, 0,10, F!(s(0,1, 0,4, "val"); s(0,6, 0,9, Num(1.4)))));
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
    ts!("[val: 1 + 3]" => s(0,0, 0,12, F!(
        s(0,1, 0,4, "val"); s(0,6, 0,11, Add(
            s(0,6, 0,7, Num(1.0)),
            s(0,10, 0,11, Num(3.0)),
        ))
    )));

    // Span of parenthesized expression contains parens.
    ts!("[val: (1)]" => s(0,0, 0,10, F!(s(0,1, 0,4, "val"); s(0,6, 0,9, Num(1.0)))));

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
    d!("[val: key=hi]"    => s(0,6, 0,9, TableKey));
    d!("[val: (key=hi)]"  => s(0,7, 0,10, TableKey));
    d!("[val: f(key=hi)]" => s(0,8, 0,11, TableKey));

    // Spanned with spacing around keyword arguments.
    ts!("[val: \n hi \n = /* //\n */ \"s\n\"]" => s(0,0, 4,2, F!(
        s(0,1, 0,4, "val"); s(1,1, 1,3, "hi") => s(3,4, 4,1, Str("s\n"))
    )));
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
    e!("[val: hi,)]"        => s(0,9, 0,10, "expected value, found closing paren"));

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
