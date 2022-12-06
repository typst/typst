#![allow(non_snake_case)]

use std::num::NonZeroUsize;
use std::sync::Arc;

use super::*;
use crate::geom::{AbsUnit, AngleUnit};

use ErrorPos::*;
use Option::None;
use SyntaxKind::*;
use TokenMode::{Code, Markup};

use std::fmt::Debug;

#[track_caller]
pub fn check<T>(text: &str, found: T, expected: T)
where
    T: Debug + PartialEq,
{
    if found != expected {
        println!("source:   {text:?}");
        println!("expected: {expected:#?}");
        println!("found:    {found:#?}");
        panic!("test failed");
    }
}

fn Space(newlines: usize) -> SyntaxKind {
    SyntaxKind::Space { newlines }
}

fn Raw(text: &str, lang: Option<&str>, block: bool) -> SyntaxKind {
    SyntaxKind::Raw(Arc::new(RawFields {
        text: text.into(),
        lang: lang.map(Into::into),
        block,
    }))
}

fn Str(string: &str) -> SyntaxKind {
    SyntaxKind::Str(string.into())
}

fn Text(string: &str) -> SyntaxKind {
    SyntaxKind::Text(string.into())
}

fn Ident(ident: &str) -> SyntaxKind {
    SyntaxKind::Ident(ident.into())
}

fn Error(pos: ErrorPos, message: &str) -> SyntaxKind {
    SyntaxKind::Error(pos, message.into())
}

/// Building blocks for suffix testing.
///
/// We extend each test case with a collection of different suffixes to make
/// sure tokens end at the correct position. These suffixes are split into
/// blocks, which can be disabled/enabled per test case. For example, when
/// testing identifiers we disable letter suffixes because these would
/// mingle with the identifiers.
///
/// Suffix blocks:
/// - ' ': spacing
/// - 'a': letters
/// - '1': numbers
/// - '/': symbols
const BLOCKS: &str = " a1/";

// Suffixes described by four-tuples of:
//
// - block the suffix is part of
// - mode in which the suffix is applicable
// - the suffix string
// - the resulting suffix NodeKind
fn suffixes() -> impl Iterator<Item = (char, Option<TokenMode>, &'static str, SyntaxKind)>
{
    [
        // Whitespace suffixes.
        (' ', None, " ", Space(0)),
        (' ', None, "\n", Space(1)),
        (' ', None, "\r", Space(1)),
        (' ', None, "\r\n", Space(1)),
        // Letter suffixes.
        ('a', Some(Markup), "hello", Text("hello")),
        ('a', Some(Markup), "💚", Text("💚")),
        ('a', Some(Code), "val", Ident("val")),
        ('a', Some(Code), "α", Ident("α")),
        ('a', Some(Code), "_", Ident("_")),
        // Number suffixes.
        ('1', Some(Code), "2", Int(2)),
        ('1', Some(Code), ".2", Float(0.2)),
        // Symbol suffixes.
        ('/', None, "[", LeftBracket),
        ('/', None, "//", LineComment),
        ('/', None, "/**/", BlockComment),
        ('/', Some(Markup), "*", Star),
        ('/', Some(Markup), r"\\", Escape('\\')),
        ('/', Some(Markup), "#let", Let),
        ('/', Some(Code), "(", LeftParen),
        ('/', Some(Code), ":", Colon),
        ('/', Some(Code), "+=", PlusEq),
    ]
    .into_iter()
}

macro_rules! t {
    (Both $($tts:tt)*) => {
        t!(Markup $($tts)*);
        t!(Code $($tts)*);
    };
    ($mode:ident $([$blocks:literal])?: $text:expr => $($token:expr),*) => {{
        // Test without suffix.
        t!(@$mode: $text => $($token),*);

        // Test with each applicable suffix.
        for (block, mode, suffix, ref token) in suffixes() {
            let text = $text;
            #[allow(unused_variables)]
            let blocks = BLOCKS;
            $(let blocks = $blocks;)?
            assert!(!blocks.contains(|c| !BLOCKS.contains(c)));
            if (mode.is_none() || mode == Some($mode)) && blocks.contains(block) {
                t!(@$mode: format!("{}{}", text, suffix) => $($token,)* token);
            }
        }
    }};
    (@$mode:ident: $text:expr => $($token:expr),*) => {{
        let text = $text;
        let found = Tokens::new(&text, $mode).collect::<Vec<_>>();
        let expected = vec![$($token.clone()),*];
        check(&text, found, expected);
    }};
}

#[test]
fn test_tokenize_brackets() {
    // Test in markup.
    t!(Markup: "{"       => LeftBrace);
    t!(Markup: "}"       => RightBrace);
    t!(Markup: "["       => LeftBracket);
    t!(Markup: "]"       => RightBracket);
    t!(Markup[" /"]: "(" => Text("("));
    t!(Markup[" /"]: ")" => Text(")"));

    // Test in code.
    t!(Code: "{" => LeftBrace);
    t!(Code: "}" => RightBrace);
    t!(Code: "[" => LeftBracket);
    t!(Code: "]" => RightBracket);
    t!(Code: "(" => LeftParen);
    t!(Code: ")" => RightParen);
}

#[test]
fn test_tokenize_whitespace() {
    // Test basic whitespace.
    t!(Both["a1/"]: ""         => );
    t!(Both["a1/"]: " "        => Space(0));
    t!(Both["a1/"]: "    "     => Space(0));
    t!(Both["a1/"]: "\t"       => Space(0));
    t!(Both["a1/"]: "  \t"     => Space(0));
    t!(Both["a1/"]: "\u{202F}" => Space(0));

    // Test newline counting.
    t!(Both["a1/"]: "\n"           => Space(1));
    t!(Both["a1/"]: "\n "          => Space(1));
    t!(Both["a1/"]: "  \n"         => Space(1));
    t!(Both["a1/"]: "  \n   "      => Space(1));
    t!(Both["a1/"]: "\r\n"         => Space(1));
    t!(Both["a1/"]: "\r\n\r"       => Space(2));
    t!(Both["a1/"]: "  \n\t \n  "  => Space(2));
    t!(Both["a1/"]: "\n\r"         => Space(2));
    t!(Both["a1/"]: " \r\r\n \x0D" => Space(3));
}

#[test]
fn test_tokenize_text() {
    // Test basic text.
    t!(Markup[" /"]: "hello"      => Text("hello"));
    t!(Markup[" /"]: "reha-world" => Text("reha-world"));

    // Test code symbols in text.
    t!(Markup[" /"]: "a():\"b" => Text("a()"), Colon, SmartQuote { double: true }, Text("b"));
    t!(Markup[" /"]: ";,|/+"  => Text(";,|/+"));
    t!(Markup[" /"]: "=-a"     => Eq, Minus, Text("a"));
    t!(Markup[" "]: "#123"     => Text("#123"));

    // Test text ends.
    t!(Markup[""]: "hello " => Text("hello"), Space(0));
    t!(Markup[""]: "hello~" => Text("hello"), Shorthand('\u{00A0}'));
}

#[test]
fn test_tokenize_escape_sequences() {
    // Test escapable symbols.
    t!(Markup: r"\\" => Escape('\\'));
    t!(Markup: r"\/" => Escape('/'));
    t!(Markup: r"\[" => Escape('['));
    t!(Markup: r"\]" => Escape(']'));
    t!(Markup: r"\{" => Escape('{'));
    t!(Markup: r"\}" => Escape('}'));
    t!(Markup: r"\*" => Escape('*'));
    t!(Markup: r"\_" => Escape('_'));
    t!(Markup: r"\=" => Escape('='));
    t!(Markup: r"\~" => Escape('~'));
    t!(Markup: r"\'" => Escape('\''));
    t!(Markup: r#"\""# => Escape('"'));
    t!(Markup: r"\`" => Escape('`'));
    t!(Markup: r"\$" => Escape('$'));
    t!(Markup: r"\#" => Escape('#'));
    t!(Markup: r"\a"   => Escape('a'));
    t!(Markup: r"\u"   => Escape('u'));
    t!(Markup: r"\1"   => Escape('1'));

    // Test basic unicode escapes.
    t!(Markup: r"\u{}"     => Error(Full, "invalid unicode escape sequence"));
    t!(Markup: r"\u{2603}" => Escape('☃'));
    t!(Markup: r"\u{P}"    => Error(Full, "invalid unicode escape sequence"));

    // Test unclosed unicode escapes.
    t!(Markup[" /"]: r"\u{"     => Error(End, "expected closing brace"));
    t!(Markup[" /"]: r"\u{1"    => Error(End, "expected closing brace"));
    t!(Markup[" /"]: r"\u{26A4" => Error(End, "expected closing brace"));
    t!(Markup[" /"]: r"\u{1Q3P" => Error(End, "expected closing brace"));
    t!(Markup: r"\u{1🏕}"       => Error(End, "expected closing brace"), Text("🏕"), RightBrace);
}

#[test]
fn test_tokenize_markup_symbols() {
    // Test markup tokens.
    t!(Markup[" a1"]: "*"   => Star);
    t!(Markup: "_"          => Underscore);
    t!(Markup[""]: "==="    => Eq, Eq, Eq);
    t!(Markup["a1/"]: "= "  => Eq, Space(0));
    t!(Markup[" "]: r"\"    => Linebreak);
    t!(Markup: "~"          => Shorthand('\u{00A0}'));
    t!(Markup["a1/"]: "-?"  => Shorthand('\u{00AD}'));
    t!(Markup["a "]: r"a--" => Text("a"), Shorthand('\u{2013}'));
    t!(Markup["a1/"]: "- "  => Minus, Space(0));
    t!(Markup[" "]: "+"     => Plus);
    t!(Markup[" "]: "1."    => EnumNumbering(NonZeroUsize::new(1).unwrap()));
    t!(Markup[" "]: "1.a"   => EnumNumbering(NonZeroUsize::new(1).unwrap()), Text("a"));
    t!(Markup[" /"]: "a1."  => Text("a1."));
}

#[test]
fn test_tokenize_code_symbols() {
    // Test all symbols.
    t!(Code: ","        => Comma);
    t!(Code: ";"        => Semicolon);
    t!(Code: ":"        => Colon);
    t!(Code: "+"        => Plus);
    t!(Code: "-"        => Minus);
    t!(Code[" a1"]: "*" => Star);
    t!(Code[" a1"]: "/" => Slash);
    t!(Code[" a/"]: "." => Dot);
    t!(Code: "="        => Eq);
    t!(Code: "=="       => EqEq);
    t!(Code: "!="       => ExclEq);
    t!(Code[" /"]: "<"  => Lt);
    t!(Code: "<="       => LtEq);
    t!(Code: ">"        => Gt);
    t!(Code: ">="       => GtEq);
    t!(Code: "+="       => PlusEq);
    t!(Code: "-="       => HyphEq);
    t!(Code: "*="       => StarEq);
    t!(Code: "/="       => SlashEq);
    t!(Code: ".."       => Dots);
    t!(Code: "=>"       => Arrow);

    // Test combinations.
    t!(Code: "<=>"        => LtEq, Gt);
    t!(Code[" a/"]: "..." => Dots, Dot);

    // Test hyphen as symbol vs part of identifier.
    t!(Code[" /"]: "-1"   => Minus, Int(1));
    t!(Code[" /"]: "-a"   => Minus, Ident("a"));
    t!(Code[" /"]: "--1"  => Minus, Minus, Int(1));
    t!(Code[" /"]: "--_a" => Minus, Minus, Ident("_a"));
    t!(Code[" /"]: "a-b"  => Ident("a-b"));

    // Test invalid.
    t!(Code: r"\" => Error(Full, "not valid here"));
}

#[test]
fn test_tokenize_keywords() {
    // A list of a few (not all) keywords.
    let list = [
        ("not", Not),
        ("let", Let),
        ("if", If),
        ("else", Else),
        ("for", For),
        ("in", In),
        ("import", Import),
    ];

    for (s, t) in list.clone() {
        t!(Markup[" "]: format!("#{}", s) => t);
        t!(Markup[" "]: format!("#{0}#{0}", s) => t, t);
        t!(Markup[" /"]: format!("# {}", s) => Text(&format!("# {s}")));
    }

    for (s, t) in list {
        t!(Code[" "]: s => t);
        t!(Markup[" /"]: s => Text(s));
    }

    // Test simple identifier.
    t!(Markup[" "]: "#letter" => Ident("letter"));
    t!(Code[" /"]: "falser"   => Ident("falser"));
    t!(Code[" /"]: "None"     => Ident("None"));
    t!(Code[" /"]: "True"     => Ident("True"));
}

#[test]
fn test_tokenize_raw_blocks() {
    // Test basic raw block.
    t!(Markup: "``"     => Raw("", None, false));
    t!(Markup: "`raw`"  => Raw("raw", None, false));
    t!(Markup[""]: "`]" => Error(End, "expected 1 backtick"));

    // Test special symbols in raw block.
    t!(Markup: "`[brackets]`" => Raw("[brackets]", None, false));
    t!(Markup[""]: r"`\`` "   => Raw(r"\", None, false), Error(End, "expected 1 backtick"));

    // Test separated closing backticks.
    t!(Markup: "```not `y`e`t```" => Raw("`y`e`t", Some("not"), false));

    // Test more backticks.
    t!(Markup: "``nope``"             => Raw("", None, false), Text("nope"), Raw("", None, false));
    t!(Markup: "````🚀````"           => Raw("", None, false));
    t!(Markup[""]: "`````👩‍🚀````noend" => Error(End, "expected 5 backticks"));
    t!(Markup[""]: "````raw``````"    => Raw("", Some("raw"), false), Raw("", None, false));
}

#[test]
fn test_tokenize_idents() {
    // Test valid identifiers.
    t!(Code[" /"]: "x"           => Ident("x"));
    t!(Code[" /"]: "value"       => Ident("value"));
    t!(Code[" /"]: "__main__"    => Ident("__main__"));
    t!(Code[" /"]: "_snake_case" => Ident("_snake_case"));

    // Test non-ascii.
    t!(Code[" /"]: "α"    => Ident("α"));
    t!(Code[" /"]: "ម្តាយ" => Ident("ម្តាយ"));

    // Test hyphen parsed as identifier.
    t!(Code[" /"]: "kebab-case" => Ident("kebab-case"));
    t!(Code[" /"]: "one-10"     => Ident("one-10"));
}

#[test]
fn test_tokenize_numeric() {
    let ints = [("7", 7), ("012", 12)];
    let floats = [
        (".3", 0.3),
        ("0.3", 0.3),
        ("3.", 3.0),
        ("3.0", 3.0),
        ("14.3", 14.3),
        ("10e2", 1000.0),
        ("10e+0", 10.0),
        ("10e+1", 100.0),
        ("10e-2", 0.1),
        ("10.e1", 100.0),
        ("10.e-1", 1.0),
        (".1e1", 1.0),
        ("10E2", 1000.0),
    ];

    // Test integers.
    for &(s, v) in &ints {
        t!(Code[" /"]: s => Int(v));
    }

    // Test floats.
    for &(s, v) in &floats {
        t!(Code[" /"]: s => Float(v));
    }

    // Test attached numbers.
    t!(Code[" /"]: ".2.3"  => Float(0.2), Float(0.3));
    t!(Code[" /"]: "1.2.3"  => Float(1.2), Float(0.3));
    t!(Code[" /"]: "1e-2+3" => Float(0.01), Plus, Int(3));

    // Test float from too large integer.
    let large = i64::MAX as f64 + 1.0;
    t!(Code[" /"]: large.to_string() => Float(large));

    // Combined integers and floats.
    let nums = ints.iter().map(|&(k, v)| (k, v as f64)).chain(floats);

    let suffixes: &[(&str, fn(f64) -> SyntaxKind)] = &[
        ("mm", |x| Numeric(x, Unit::Length(AbsUnit::Mm))),
        ("pt", |x| Numeric(x, Unit::Length(AbsUnit::Pt))),
        ("cm", |x| Numeric(x, Unit::Length(AbsUnit::Cm))),
        ("in", |x| Numeric(x, Unit::Length(AbsUnit::In))),
        ("rad", |x| Numeric(x, Unit::Angle(AngleUnit::Rad))),
        ("deg", |x| Numeric(x, Unit::Angle(AngleUnit::Deg))),
        ("em", |x| Numeric(x, Unit::Em)),
        ("fr", |x| Numeric(x, Unit::Fr)),
        ("%", |x| Numeric(x, Unit::Percent)),
    ];

    // Numeric types.
    for &(suffix, build) in suffixes {
        for (s, v) in nums.clone() {
            t!(Code[" /"]: format!("{}{}", s, suffix) => build(v));
        }
    }

    // Multiple dots close the number.
    t!(Code[" /"]: "1..2"   => Int(1), Dots, Int(2));
    t!(Code[" /"]: "1..2.3" => Int(1), Dots, Float(2.3));
    t!(Code[" /"]: "1.2..3" => Float(1.2), Dots, Int(3));

    // Test invalid.
    t!(Code[" /"]: "1foo" => Error(Full, "invalid number suffix"));
}

#[test]
fn test_tokenize_strings() {
    // Test basic strings.
    t!(Code: "\"hi\""        => Str("hi"));
    t!(Code: "\"hi\nthere\"" => Str("hi\nthere"));
    t!(Code: "\"🌎\""        => Str("🌎"));

    // Test unterminated.
    t!(Code[""]: "\"hi" => Error(End, "expected quote"));

    // Test escaped quote.
    t!(Code: r#""a\"bc""# => Str("a\"bc"));
    t!(Code[""]: r#""\""# => Error(End, "expected quote"));
}

#[test]
fn test_tokenize_line_comments() {
    // Test line comment with no trailing newline.
    t!(Both[""]: "//" => LineComment);

    // Test line comment ends at newline.
    t!(Both["a1/"]: "//bc\n"   => LineComment, Space(1));
    t!(Both["a1/"]: "// bc \n" => LineComment, Space(1));
    t!(Both["a1/"]: "//bc\r\n" => LineComment, Space(1));

    // Test nested line comments.
    t!(Both["a1/"]: "//a//b\n" => LineComment, Space(1));
}

#[test]
fn test_tokenize_block_comments() {
    // Test basic block comments.
    t!(Both[""]: "/*" => BlockComment);
    t!(Both: "/**/"   => BlockComment);
    t!(Both: "/*🏞*/" => BlockComment);
    t!(Both: "/*\n*/" => BlockComment);

    // Test depth 1 and 2 nested block comments.
    t!(Both: "/* /* */ */"  => BlockComment);
    t!(Both: "/*/*/**/*/*/" => BlockComment);

    // Test two nested, one unclosed block comments.
    t!(Both[""]: "/*/*/**/*/" => BlockComment);

    // Test all combinations of up to two following slashes and stars.
    t!(Both[""]: "/*"   => BlockComment);
    t!(Both[""]: "/*/"  => BlockComment);
    t!(Both[""]: "/**"  => BlockComment);
    t!(Both[""]: "/*//" => BlockComment);
    t!(Both[""]: "/*/*" => BlockComment);
    t!(Both[""]: "/**/" => BlockComment);
    t!(Both[""]: "/***" => BlockComment);

    // Test unexpected terminator.
    t!(Both: "/*Hi*/*/" => BlockComment,
        Error(Full, "unexpected end of block comment"));
}
