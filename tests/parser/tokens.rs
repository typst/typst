// Whitespace.
t ""             => []
t " "            => [S(0)]
t "    "         => [S(0)]
t "\t"           => [S(0)]
t "  \t"         => [S(0)]
t "\n"           => [S(1)]
t "\n "          => [S(1)]
t "  \n"         => [S(1)]
t "  \n   "      => [S(1)]
t "  \n\t \n  "  => [S(2)]
t "\r\n"         => [S(1)]
t " \r\r\n \x0D" => [S(3)]
t "\n\r"         => [S(2)]

// Comments.
t "a // bc\n "        => [T("a"), S(0), LC(" bc"),  S(1)]
t "a //a//b\n "       => [T("a"), S(0), LC("a//b"), S(1)]
t "a //a//b\r\n"      => [T("a"), S(0), LC("a//b"), S(1)]
t "a //a//b\n\nhello" => [T("a"), S(0), LC("a//b"), S(2), T("hello")]
t "/**/"              => [BC("")]
t "_/*_/*a*/*/"       => [Underscore, BC("_/*a*/")]
t "/*/*/"             => [BC("/*/")]
t "abc*/"             => [T("abc"), Invalid("*/")]

// Header only tokens.
th "["                 => [Func("", None, false)]
th "]"                 => [Invalid("]")]
th "(){}:=,"           => [LP, RP, LB, RB, Colon, Equals, Comma]
th "a:b"               => [Id("a"), Colon, Id("b")]
th "="                 => [Equals]
th ","                 => [Comma]
th r#""hello\"world""# => [Str(r#"hello\"world"#)]
th r#""hi", 12pt"#     => [Str("hi"), Comma, S(0), Size(12.0)]
th "\"hi\""            => [T("\"hi"), T("\"")]
th "a: true, x=1"      => [Id("a"), Colon, S(0), Bool(true), Comma, S(0),
                           Id("x"), Equals, Num(1.0)]
th "120%"              => [Num(1.2)]
th "🌓, 🌍,"           => [T("🌓"), Comma, S(0), T("🌍"), Comma]
tb "a: b"              => [T("a"), T(":"), S(0), T("b")]
tb "c=d, "             => [T("c"), T("=d"), T(","), S(0)]

// Body only tokens.
tb "_*`"           => [Underscore, Star, Backtick]
tb "[func]*bold*"  => [Func("func", None, true), Star, T("bold"), Star]
tb "hi_you_ there" => [T("hi"), Underscore, T("you"), Underscore, S(0), T("there")]
th "_*`"           => [Invalid("_"), Invalid("*"), Invalid("`")]

// Nested functions.
tb "[f: [=][*]]"    => [Func("f: [=][*]", None, true)]
tb "[_][[,],],"     => [Func("_", Some("[,],"), true), T(",")]
tb "[=][=][=]"      => [Func("=", Some("="), true), Func("=", None, true)]
tb "[=][[=][=][=]]" => [Func("=", Some("[=][=][=]")), true]

// Escapes.
tb r"\[" => [T("[")]
tb r"\]" => [T("]")]
tb r"\\" => [T(r"\")]
tb r"\/" => [T("/")]
tb r"\*" => [T("*")]
tb r"\_" => [T("_")]
tb r"\`" => [T("`")]

// Unescapable special symbols.
th r"\:" => [T(r"\"), T(":")]
th r"\=" => [T(r"\"), T("=")]
th r"\:" => [T(r"\"), Colon]
th r"\=" => [T(r"\"), Equals]
th r"\," => [T(r"\"), Comma]

// Spans.
tbs "hello"          => [(0:0, 0:5, T("hello"))]
tbs "ab\r\nc"        => [(0:0, 0:2, T("ab")), (0:2, 1:0, S(1)), (1:0, 1:1, T("c"))]
tbs "[x = \"(1)\"]*" => [(0:0, 0:11, Func("x = \"(1)\"", None, true)), (0:11, 0:12, Star)]
tbs "// ab\r\n\nf"   => [(0:0, 0:5, LC(" ab")), (0:5, 2:0, S(2)), (2:0, 2:1, T("f"))]
tbs "/*b*/_"         => [(0:0, 0:5, BC("b")), (0:5, 0:6, Underscore)]
ths "a=10"           => [(0:0, 0:1, Id("a")), (0:1, 0:2, Equals), (0:2, 0:4, Num(10.0))]
