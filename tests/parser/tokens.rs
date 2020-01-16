// Whitespace.
t ""             => []
t " "            => [W(0)]
t "    "         => [W(0)]
t "\t"           => [W(0)]
t "  \t"         => [W(0)]
t "\n"           => [W(1)]
t "\n "          => [W(1)]
t "  \n"         => [W(1)]
t "  \n   "      => [W(1)]
t "  \n\t \n  "  => [W(2)]
t "\r\n"         => [W(1)]
t " \r\r\n \x0D" => [W(3)]
t "\n\r"         => [W(2)]

// Comments.
t "a // bc\n "        => [T("a"), W(0), LC(" bc"), W(1)]
t "a //a//b\n "       => [T("a"), W(0), LC("a//b"), W(1)]
t "a //a//b\r\n"      => [T("a"), W(0), LC("a//b"), W(1)]
t "a //a//b\n\nhello" => [T("a"), W(0), LC("a//b"), W(2), T("hello")]
t "/**/"              => [BC("")]
t "_/*_/*a*/*/"       => [U, BC("_/*a*/")]
t "/*/*/"             => [BC("/*/")]
t "abc*/"             => [T("abc"), SS]

// Header only tokens.
t "["                   => [LB]
t "]"                   => [RB]
t "[(){}:=,]"           => [LB, LP, RP, LBR, RBR, CL, EQ, CM, RB]
t "[a:b]"               => [LB, ID("a"), CL, ID("b"), RB]
t "[🌓, 🌍,]"           => [LB, T("🌓"), CM, W(0), T("🌍"), CM, RB]
t "[=]"                 => [LB, EQ, RB]
t "[,]"                 => [LB, CM, RB]
t "a: b"                => [T("a"), T(":"), W(0), T("b")]
t "c=d, "               => [T("c"), T("=d"), T(","), W(0)]
t r#"["hello\"world"]"# => [LB, STR(r#"hello\"world"#), RB]
t r#"["hi", 12pt]"#     => [LB, STR("hi"), CM, W(0), SIZE(Size::pt(12.0)), RB]
t "\"hi\""              => [T("\"hi"), T("\"")]
t "[a: true, x=1]"      => [LB, ID("a"), CL, W(0), BOOL(true), CM, W(0),
                          ID("x"), EQ, NUM(1.0), RB]
t "[120%]"              => [LB, NUM(1.2), RB]

// Body only tokens.
t "_*`"           => [U, S, B]
t "[func]*bold*"  => [LB, ID("func"), RB, S, T("bold"), S]
t "[_*`]"         => [LB, T("_"), T("*"), T("`"), RB]
t "hi_you_ there" => [T("hi"), U, T("you"), U, W(0), T("there")]

// Nested functions.
t "[f: [=][*]]"    => [LB, ID("f"), CL, W(0), LB, EQ, RB, LB, S, RB, RB]
t "[_][[,],],"     => [LB, T("_"), RB, LB, LB, CM, RB, T(","), RB, T(",")]
t "[=][=][=]"      => [LB, EQ, RB, LB, T("="), RB, LB, EQ, RB]
t "[=][[=][=][=]]" => [LB, EQ, RB, LB, LB, EQ, RB, LB, T("="), RB, LB, EQ, RB, RB]

// Escapes.
t r"\["   => [T("[")]
t r"\]"   => [T("]")]
t r"\\"   => [T(r"\")]
t r"\/"   => [T("/")]
t r"\*"   => [T("*")]
t r"\_"   => [T("_")]
t r"\`"   => [T("`")]

// Unescapable special symbols.
t r"\:"   => [T(r"\"), T(":")]
t r"\="   => [T(r"\"), T("=")]
t r"[\:]" => [LB, T(r"\"), CL, RB]
t r"[\=]" => [LB, T(r"\"), EQ, RB]
t r"[\,]" => [LB, T(r"\"), CM, RB]

// Spans
ts "hello"           => [(0:0, 0:5, T("hello"))]
ts "ab\r\nc"         => [(0:0, 0:2, T("ab")), (0:2, 1:0, W(1)), (1:0, 1:1, T("c"))]
ts "[a=10]"          => [(0:0, 0:1, LB), (0:1, 0:2, ID("a")), (0:2, 0:3, EQ),
                         (0:3, 0:5, NUM(10.0)), (0:5, 0:6, RB)]
ts r#"[x = "(1)"]*"# => [(0:0, 0:1, LB), (0:1, 0:2, ID("x")), (0:2, 0:3, W(0)),
                         (0:3, 0:4, EQ), (0:4, 0:5, W(0)), (0:5, 0:10, STR("(1)")),
                         (0:10, 0:11, RB), (0:11, 0:12, S)]
ts "// ab\r\n\nf"    => [(0:0, 0:5, LC(" ab")), (0:5, 2:0, W(2)), (2:0, 2:1, T("f"))]
ts "/*b*/_"          => [(0:0, 0:5, BC("b")), (0:5, 0:6, U)]
