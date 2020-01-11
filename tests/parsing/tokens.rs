// Whitespace.
""             => []
" "            => [W(0)]
"    "         => [W(0)]
"\t"           => [W(0)]
"  \t"         => [W(0)]
"\n"           => [W(1)]
"\n "          => [W(1)]
"  \n"         => [W(1)]
"  \n   "      => [W(1)]
"  \n\t \n  "  => [W(2)]
"\r\n"         => [W(1)]
" \r\r\n \x0D" => [W(3)]
"\n\r"         => [W(2)]

// Comments.
"a // bc\n "        => [T("a"), W(0), LC(" bc"), W(1)]
"a //a//b\n "       => [T("a"), W(0), LC("a//b"), W(1)]
"a //a//b\r\n"      => [T("a"), W(0), LC("a//b"), W(1)]
"a //a//b\n\nhello" => [T("a"), W(0), LC("a//b"), W(2), T("hello")]
"/**/"              => [BC("")]
"_/*_/*a*/*/"       => [U, BC("_/*a*/")]
"/*/*/"             => [BC("/*/")]
"abc*/"             => [T("abc"), SS]

// Header only tokens.
"["                   => [LB]
"]"                   => [RB]
"[(){}:=,]"           => [LB, LP, RP, LBR, RBR, CL, EQ, CM, RB]
"[a:b]"               => [LB, ID("a"), CL, ID("b"), RB]
"[🌓, 🌍,]"          => [LB, T("🌓"), CM, W(0), T("🌍"), CM, RB]
"[=]"                 => [LB, EQ, RB]
"[,]"                 => [LB, CM, RB]
"a: b"                => [T("a"), T(":"), W(0), T("b")]
"c=d, "               => [T("c"), T("=d"), T(","), W(0)]
r#"["hello\"world"]"# => [LB, STR(r#"hello\"world"#), RB]
r#"["hi", 12pt]"#     => [LB, STR("hi"), CM, W(0), SIZE(Size::pt(12.0)), RB]
"\"hi\""              => [T("\"hi"), T("\"")]
"[a: true, x=1]"      => [LB, ID("a"), CL, W(0), BOOL(true), CM, W(0),
                          ID("x"), EQ, NUM(1.0), RB]
"[120%]"              => [LB, NUM(1.2), RB]

// Body only tokens.
"_*`"           => [U, ST, B]
"[_*`]"         => [LB, T("_"), T("*"), T("`"), RB]
"hi_you_ there" => [T("hi"), U, T("you"), U, W(0), T("there")]

// Escapes.
r"\["   => [T("[")]
r"\]"   => [T("]")]
r"\\"   => [T(r"\")]
r"\/"   => [T("/")]
r"\*"   => [T("*")]
r"\_"   => [T("_")]
r"\`"   => [T("`")]

// Unescapable special symbols.
r"\:"   => [T(r"\"), T(":")]
r"\="   => [T(r"\"), T("=")]
r"[\:]" => [LB, T(r"\"), CL, RB]
r"[\=]" => [LB, T(r"\"), EQ, RB]
r"[\,]" => [LB, T(r"\"), CM, RB]
