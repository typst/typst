// Spaces, Newlines, Brackets.
""                => []
" "               => [S]
"    "            => [S]
"\t"              => [S]
"  \t"            => [S]
"\n"              => [N]
"\n "             => [N, S]
"  \n"            => [S, N]
"  \n   "         => [S, N, S]
"["               => [LB]
"]"               => [RB]

// Header only tokens.
"[:]"             => [LB, Colon, RB]
"[=]"             => [LB, Equals, RB]
"[,]"             => [LB, Comma, RB]
":"               => [T(":")]
"="               => [T("=")]
","               => [T(",")]
r#"["hi"]"#       => [LB, Quoted("hi"), RB]
r#""hi""#         => [T(r#""hi""#)]

// Body only tokens.
"_"               => [Underscore]
"*"               => [Star]
"`"               => [Backtick]
"[_]"             => [LB, T("_"), RB]
"[*]"             => [LB, T("*"), RB]
"[`]"             => [LB, T("`"), RB]

// Comments.
"//line"          => [LineComment("line")]
"/*block*/"       => [BlockComment("block")]
"*/"              => [StarSlash]

// Plain text.
"A"               => [T("A")]
"Hello"           => [T("Hello")]
"Hello-World"     => [T("Hello-World")]
r#"A"B"#          => [T(r#"A"B"#)]
"🌍"              => [T("🌍")]

// Escapes.
r"\["             => [T("[")]
r"\]"             => [T("]")]
r"\\"             => [T(r"\")]
r"[\[]"           => [LB, T("["), RB]
r"[\]]"           => [LB, T("]"), RB]
r"[\\]"           => [LB, T(r"\"), RB]
r"\:"             => [T(":")]
r"\="             => [T("=")]
r"\/"             => [T("/")]
r"[\:]"           => [LB, T(":"), RB]
r"[\=]"           => [LB, T("="), RB]
r"[\,]"           => [LB, T(","), RB]
r"\*"             => [T("*")]
r"\_"             => [T("_")]
r"\`"             => [T("`")]
r"[\*]"           => [LB, T("*"), RB]
r"[\_]"           => [LB, T("_"), RB]
r"[\`]"           => [LB, T("`"), RB]

// Whitespace.
"Hello World"     => [T("Hello"), S, T("World")]
"Hello  World"    => [T("Hello"), S, T("World")]
"Hello \t World"  => [T("Hello"), S, T("World")]

// Newline.
"First\n"         => [T("First"), N]
"First \n"        => [T("First"), S, N]
"First\n "        => [T("First"), N, S]
"First \n "       => [T("First"), S, N, S]
"First\nSecond"   => [T("First"), N, T("Second")]
"First\r\nSecond" => [T("First"), N, T("Second")]
"First \nSecond"  => [T("First"), S, N, T("Second")]
"First\n Second"  => [T("First"), N, S, T("Second")]
"First \n Second" => [T("First"), S, N, S, T("Second")]
