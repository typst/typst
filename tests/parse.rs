#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use typstc::func::Scope;
use typstc::size::Size;
use typstc::syntax::*;
use typstc::{function, parse};


mod token_shorthands {
    pub use super::Token::{
        Whitespace as W,
        LineComment as LC, BlockComment as BC, StarSlash as SS,
        LeftBracket as LB, RightBracket as RB,
        LeftParen as LP, RightParen as RP,
        LeftBrace as LBR, RightBrace as RBR,
        Colon as CL, Comma as CM, Equals as EQ,
        ExprIdent as ID, ExprStr as STR, ExprSize as SIZE,
        ExprNumber as NUM, ExprBool as BOOL,
        Star as ST, Underscore as U, Backtick as B, Text as T,
    };
}

mod node_shorthands {
    use super::Node;
    pub use Node::{
        Space as S, Newline as N, Text,
        ToggleItalic as I, ToggleBolder as B, ToggleMonospace as M,
        Func,
    };
    pub fn T(text: &str) -> Node { Node::Text(text.to_string()) }
}

macro_rules! F {
    (@body None) => (None);
    (@body Some([$($tts:tt)*])) => ({
        let nodes = vec![$($tts)*].into_iter()
            .map(|v| Spanned { v, span: Span::ZERO })
            .collect();

        Some(SyntaxTree { nodes })
    });

    ($($body:tt)*) => ({
        Func(FuncCall(Box::new(DebugFn {
            pos: vec![],
            key: vec![],
            body: F!(@body $($body)*),
        })))
    });
}

function! {
    #[derive(Debug, PartialEq)]
    pub struct DebugFn {
        pos: Vec<Spanned<Expression>>,
        key: Vec<Pair>,
        body: Option<SyntaxTree>,
    }

    parse(args, body, ctx) {
        DebugFn {
            pos: args.iter_pos().collect(),
            key: args.iter_keys().collect(),
            body: parse!(optional: body, ctx),
        }
    }

    layout() { vec![] }
}

impl DebugFn {
    fn compare(&self, other: &DebugFn) -> bool {
        self.pos.iter().zip(&other.pos).all(|(a, b)| a.v == b.v)
            && self.key.iter().zip(&other.key)
                .all(|(a, b)| a.key.v == b.key.v && a.value.v == b.value.v)
            && match (&self.body, &other.body) {
                (Some(a), Some(b)) => compare(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

fn downcast(func: &FuncCall) -> &DebugFn {
    func.0.downcast::<DebugFn>().expect("not a debug fn")
}

fn compare(a: &SyntaxTree, b: &SyntaxTree) -> bool {
    for (x, y) in a.nodes.iter().zip(&b.nodes) {
        use node_shorthands::*;
        let same = match (&x.v, &y.v) {
            (S, S) | (N, N) | (I, I) | (B, B) | (M, M) => true,
            (Text(t1), Text(t2)) => t1 == t2,
            (Func(f1), Func(f2)) => {
                downcast(f1).compare(downcast(f2))
            }
            _ => false,
        };

        if !same { return false; }
    }
    true
}

/// Parses the test syntax.
macro_rules! tokens {
    ($($task:ident $src:expr =>($line:expr)=> [$($tts:tt)*])*) => ({
        #[allow(unused_mut)]
        let mut cases = Vec::new();
        $(cases.push(($line, $src, tokens!(@$task [$($tts)*])));)*
        cases
    });

    (@t [$($tts:tt)*]) => ({
        use token_shorthands::*;
        Target::Tokenize(vec![$($tts)*])
    });

    (@ts [$($tts:tt)*]) => ({
        use token_shorthands::*;
        Target::TokenizeSpanned(tokens!(@__spans [$($tts)*]))
    });

    (@p [$($tts:tt)*]) => ({
        use node_shorthands::*;

        let nodes = vec![$($tts)*].into_iter()
            .map(|v| Spanned { v, span: Span::ZERO })
            .collect();

        Target::Parse(SyntaxTree { nodes })
    });

    (@ps [$($tts:tt)*]) => ({
        use node_shorthands::*;
        Target::ParseSpanned(tokens!(@__spans [$($tts)*]))
    });

    (@__spans [$(($sl:tt:$sc:tt, $el:tt:$ec:tt, $v:expr)),* $(,)?]) => ({
        vec![
            $(Spanned { v: $v, span: Span {
                start: Position { line: $sl, column: $sc },
                end:   Position { line: $el, column: $ec },
            }}),*
        ]
    });
}

#[derive(Debug)]
enum Target {
    Tokenize(Vec<Token<'static>>),
    TokenizeSpanned(Vec<Spanned<Token<'static>>>),
    Parse(SyntaxTree),
    ParseSpanned(SyntaxTree),
}

fn main() {
    let tests = include!("cache/parse");
    let mut errors = false;

    let len = tests.len();
    println!();
    println!("Running {} test{}", len, if len > 1 { "s" } else { "" });

    // Go through all test files.
    for (file, cases) in tests.into_iter() {
        print!("Testing: {}. ", file);

        let mut okay = 0;
        let mut failed = 0;

        // Go through all tests in a test file.
        for (line, src, target) in cases.into_iter() {
            let (correct, expected, found) = test_case(src, target);

            // Check whether the tokenization works correctly.
            if correct {
                okay += 1;
            } else {
                if failed == 0 {
                    println!();
                }

                println!(" - Case failed in file {}.rs in line {}.", file, line);
                println!("   - Source:   {:?}", src);
                println!("   - Expected: {:?}", expected);
                println!("   - Found:    {:?}", found);
                println!();

                failed += 1;
                errors = true;
            }
        }

        // Print a small summary.
        print!("{} okay, {} failed.", okay, failed);
        if failed == 0 {
            print!(" ✔")
        }
        println!();
    }

    println!();

    if errors {
        std::process::exit(-1);
    }
}

fn test_case(src: &str, target: Target) -> (bool, String, String) {
    match target {
        Target::Tokenize(tokens) => {
            let found: Vec<_> = tokenize(src).map(Spanned::value).collect();
            (found == tokens, format!("{:?}", tokens), format!("{:?}", found))
        }

        Target::TokenizeSpanned(tokens) => {
            let found: Vec<_> = tokenize(src).collect();
            (found == tokens, format!("{:?}", tokens), format!("{:?}", found))
        }

        Target::Parse(tree) => {
            let scope = Scope::with_debug::<DebugFn>();
            let (found, _, errs) = parse(src, ParseContext { scope: &scope });
            (compare(&tree, &found), format!("{:?}", tree), format!("{:?}", found))
        }

        Target::ParseSpanned(tree) => {
            let scope = Scope::with_debug::<DebugFn>();
            let (found, _, _) = parse(src, ParseContext { scope: &scope });
            (tree == found, format!("{:?}", tree), format!("{:?}", found))
        }
    }
}
