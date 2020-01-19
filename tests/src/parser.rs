use std::fmt::Debug;

use typstc::func::Scope;
use typstc::size::Size;
use typstc::syntax::*;
use typstc::{function, parse};

mod spanless;
use spanless::SpanlessEq;


/// The result of a single test case.
enum Case {
    Okay,
    Failed {
        line: usize,
        src: &'static str,
        expected: String,
        found: String,
    }
}

/// Test all tests.
fn test(tests: Vec<(&str, Vec<Case>)>) {
    println!();

    let mut errors = false;

    let len = tests.len();
    println!("Running {} test{}", len, if len > 1 { "s" } else { "" });

    for (file, cases) in tests {
        print!("Testing: {}. ", file);

        let mut okay = 0;
        let mut failed = 0;

        for case in cases {
            match case {
                Case::Okay => okay += 1,
                Case::Failed { line, src, expected, found } => {
                    println!();
                    println!(" ❌  Case failed in file {}.rs in line {}.", file, line);
                    println!("   - Source:   {:?}", src);
                    println!("   - Expected: {}", expected);
                    println!("   - Found:    {}", found);

                    failed += 1;
                }
            }
        }

        // Print a small summary.
        print!("{} okay, {} failed.", okay, failed);
        if failed == 0 {
            print!(" ✔")
        } else {
            errors = true;
        }

        println!();
    }

    println!();

    if errors {
        std::process::exit(-1);
    }
}

/// The main test macro.
macro_rules! tokens {
    ($($task:ident $src:expr =>($line:expr)=> [$($e:tt)*])*) => ({
        vec![$({
            let (okay, expected, found) = case!($task $src, [$($e)*]);
            if okay {
                Case::Okay
            } else {
                Case::Failed {
                    line: $line,
                    src: $src,
                    expected: format(expected),
                    found: format(found),
                }
            }
        }),*]
    });
}

//// Indented formatting for failed cases.
fn format(thing: impl Debug) -> String {
    format!("{:#?}", thing).replace('\n', "\n     ")
}

/// Evaluates a single test.
macro_rules! case {
    (t $($rest:tt)*) => (case!(@tokenize SpanlessEq::spanless_eq, $($rest)*));
    (ts $($rest:tt)*) => (case!(@tokenize PartialEq::eq, $($rest)*));

    (@tokenize $cmp:expr, $src:expr, [$($e:tt)*]) => ({
        let expected = list!(tokens [$($e)*]);
        let found = tokenize($src).collect::<Vec<_>>();
        ($cmp(&found, &expected), expected, found)
    });

    (p $($rest:tt)*) => (case!(@parse SpanlessEq::spanless_eq, $($rest)*));
    (ps $($rest:tt)*) => (case!(@parse PartialEq::eq, $($rest)*));

    (@parse $cmp:expr, $src:expr, [$($e:tt)*]) => ({
        let expected = SyntaxModel { nodes: list!(nodes [$($e)*]) };
        let found = parse($src, ParseContext { scope: &scope() }).0;
        ($cmp(&found, &expected), expected, found)
    });

    (c $src:expr, [$($e:tt)*]) => ({
        let expected = Colorization { tokens: list!(decorations [$($e)*]) };
        let found = parse($src, ParseContext { scope: &scope() }).1;
        (expected == found, expected, found)
    });

    (e $src:expr, [$($e:tt)*]) => ({
        let expected = list!([$($e)*]).into_iter()
            .map(|s| s.map(|m| m.to_string()))
            .collect();

        let found = parse($src, ParseContext { scope: &scope() }).2;
        (expected == found, expected, found)
    });
}

/// A scope containing the `DebugFn` as a fallback.
fn scope() -> Scope {
    Scope::with_fallback::<DebugFn>()
}

/// Parses possibly-spanned lists of token or node expressions.
macro_rules! list {
    (expr [$($item:expr),* $(,)?]) => ({
        #[allow(unused_imports)]
        use cuts::expr::*;
        Tuple { items: vec![$(zspan($item)),*] }
    });

    (expr [$($key:expr =>($_:expr)=> $value:expr),* $(,)?]) => ({
        #[allow(unused_imports)]
        use cuts::expr::*;
        Object {
            pairs: vec![$(Pair {
                key: zspan(Ident($key.to_string())),
                value: zspan($value),
            }),*]
        }
    });

    ($cut:ident [$($e:tt)*]) => ({
        #[allow(unused_imports)]
        use cuts::$cut::*;
        list!([$($e)*])
    });

    ([$(($sl:tt:$sc:tt, $el:tt:$ec:tt, $v:expr)),* $(,)?]) => ({
        vec![
            $(Spanned { v: $v, span: Span {
                start: Position { line: $sl, column: $sc },
                end:   Position { line: $el, column: $ec },
            }}),*
        ]
    });

    ([$($e:tt)*]) => (vec![$($e)*].into_iter().map(zspan).collect::<Vec<_>>());
}

/// Composes a function expression.
macro_rules! func {
    ($name:expr $(,pos: [$($p:tt)*])? $(,key: [$($k:tt)*])?; $($b:tt)*) => ({
        #![allow(unused_mut, unused_assignments)]

        let mut positional = Tuple::new();
        let mut keyword = Object::new();

        $(positional = list!(expr [$($p)*]);)?
        $(keyword = list!(expr [$($k)*]);)?

        Node::Model(Box::new(DebugFn {
            header: FuncHeader {
                name: zspan(Ident($name.to_string())),
                args: FuncArgs {
                    positional,
                    keyword,
                },
            },
            body: func!(@body $($b)*),
        }))
    });

    (@body Some($($b:tt)*)) => (Some(SyntaxModel{ nodes: list!(nodes $($b)*) }));
    (@body None) => (None);
}

function! {
    /// Most functions in the tests are parsed into the debug function for easy
    /// inspection of arguments and body.
    #[derive(Debug, PartialEq)]
    pub struct DebugFn {
        header: FuncHeader,
        body: Option<SyntaxTree>,
    }

    parse(header, body, ctx) {
        let cloned = header.clone();
        header.args.clear();
        DebugFn {
            header: cloned,
            body: parse!(optional: body, ctx),
        }
    }

    layout() { vec![] }
}

/// Span an element with a zero span.
fn zspan<T>(v: T) -> Spanned<T> {
    Spanned { v, span: Span::ZERO }
}

/// Abbreviations for tokens, nodes, colors and expressions.
#[allow(non_snake_case, dead_code)]
mod cuts {
    pub mod tokens {
        pub use typstc::syntax::Token::{
            Whitespace as W,
            LineComment as LC,
            BlockComment as BC,
            StarSlash as SS,
            LeftBracket as LB,
            RightBracket as RB,
            LeftParen as LP,
            RightParen as RP,
            LeftBrace as LBR,
            RightBrace as RBR,
            Colon as CL,
            Comma as CM,
            Equals as EQ,
            ExprIdent as ID,
            ExprStr as STR,
            ExprSize as SIZE,
            ExprNumber as NUM,
            ExprBool as BOOL,
            Star as S,
            Underscore as U,
            Backtick as B,
            Text as T,
        };
    }

    pub mod nodes {
        use typstc::syntax::Node;

        pub use Node::{
            Space as S,
            Newline as N,
            ToggleItalic as I,
            ToggleBolder as B,
            ToggleMonospace as M,
        };

        pub fn T(text: &str) -> Node {
            Node::Text(text.to_string())
        }
    }

    pub mod decorations {
        pub use typstc::syntax::Decoration::*;
    }

    pub mod expr {
        use typstc::syntax::{Expression, Ident};

        pub use Expression::{
            Number as NUM,
            Size as SIZE,
            Bool as BOOL,
        };

        pub fn ID(text: &str) -> Expression {
            Expression::Ident(Ident(text.to_string()))
        }

        pub fn STR(text: &str) -> Expression {
            Expression::Str(text.to_string())
        }
    }
}

fn main() {
    test(include!("../cache/parser-tests.rs"))
}
