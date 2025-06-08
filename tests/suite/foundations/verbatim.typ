--- verbatim-basic-content ---
// Basic verbatim: simple content, formatting, and sequences
#test(verbatim([*Hey*]), "[*Hey*]")
#test(verbatim([A _sequence_]), "[A _sequence_]")
#test(verbatim([A _longer_ *sequence*!]), "[A _longer_ *sequence*!]")

--- verbatim-bracket-edges ---
// Bracket edge cases: single brackets, nested, and adjacent
#test(verbatim([*Hey* {]), "[*Hey* {]")
#test(verbatim([*Hey* }]), "[*Hey* }]")
#test(verbatim([[#{"*Hey*"}]]), "[[#{\"*Hey*\"}]]")
#test(verbatim([[ [#{{"*Hey*"}}]]]), "[[ [#{{\"*Hey*\"}}]]]")
#test(verbatim([[[#{{"*Hey*"}}]]]), "[[[#{{\"*Hey*\"}}]]]")

--- verbatim-interpolation ---
// Interpolation and dynamic content in verbatim
#test(verbatim([#{"*Hey*"}]), "[#{\"*Hey*\"}]")
#test(verbatim([#{{"*Hey*"}}]), "[#{{\"*Hey*\"}}]")

--- verbatim-variable-content ---
// Verbatim with variable assignments
#let some-content = [Some _italic_ and *bold* text]
#test(verbatim(some-content), "[Some _italic_ and *bold* text]")

--- verbatim-function-usage ---
// Verbatim with function calls and nested function usage
#let some-content = [Some _italic_ and *bold* text]
#test(
  verbatim([Some function calls: #rect(height: 10pt, fill: blue), #align(right)[more *content*!] #some-content]),
  "[Some function calls: #rect(height: 10pt, fill: blue), #align(right)[more *content*!] #some-content]"
)
#let example(c) = { return (content: c, verbatim: verbatim(c)) }
#test(example([A _longer_ *sequence*!]).verbatim, "[A _longer_ *sequence*!]")

--- verbatim-inline-math ---
// Inline math and math-related content
#test(verbatim([$x^2$]), "[$x^2$]")
#test(
  verbatim([Inline math: $x^2 lim_(n -> infinity) x / n = Pr[x] "text" 123; \$ and "$" or $ cos]),
  "[Inline math: $x^2 lim_(n -> infinity) x / n = Pr[x] \"text\" 123; \$ and \"$\" or $ cos]"
)
#let math_brackets = [$[a, b]$]
#test(verbatim(math_brackets), "[$[a, b]$]")
#let math_brackets_complex = [Formula: $sum_(i=1)^n [x_i]$ and arrays]
#test(verbatim(math_brackets_complex), "[Formula: $sum_(i=1)^n [x_i]$ and arrays]")

--- verbatim-metadata-context ---
// Verbatim in metadata and context queries
#let card(title, body) = [
  #metadata((title: verbatim(title), body: verbatim(body))) <card>
]
#card("Title 123", [*Body* _123_ $x^2$])
#context {
  test(query(<card>).at(0).value.title, "\"Title 123\"")
  test(query(<card>).at(0).value.body, "[*Body* _123_ $x^2$]")
}

--- verbatim-nested-brackets ---
// Deeply nested and mixed brackets
#let nested1 = [Text with [nested] brackets]
#test(verbatim(nested1), "[Text with [nested] brackets]")
#let deep_nested = [[[[[very deep]]]]]
#test(verbatim(deep_nested), "[[[[[very deep]]]]]")
#let mixed_brackets = [Text with (parentheses) and {braces}]
#test(verbatim(mixed_brackets), "[Text with (parentheses) and {braces}]")
#let mixed_brackets_seq = [Text {curly} (paren) [square] `backtick` more text]
#test(verbatim(mixed_brackets_seq), "[Text {curly} (paren) [square] `backtick` more text]")

--- verbatim-string-content ---
// Strings, string escapes, and string with brackets
#let string_brackets = [Content with "string [with] brackets" inside]
#test(verbatim(string_brackets), "[Content with \"string [with] brackets\" inside]")
#let unbalanced_in_string = [Text with "unbalanced \[ bracket in string"]
#test(verbatim(unbalanced_in_string), "[Text with \"unbalanced \\[ bracket in string\"]")
#let complex_strings = [Text with "string containing \" quotes and [brackets]" here]
#test(verbatim(complex_strings), "[Text with \"string containing \\\" quotes and [brackets]\" here]")
#let plain_string = "abc"
#test(verbatim(plain_string), "\"abc\"")
#test(verbatim("abc"), "\"abc\"")

--- verbatim-escaped-characters ---
// Escaped brackets, backslashes, and special characters
#let escaped = [Text with \[ escaped \] brackets]
#test(verbatim(escaped), "[Text with \\[ escaped \\] brackets]")
#let escaped_chars = [Text with \\ backslash and \[ bracket and \] bracket]
#test(verbatim(escaped_chars), "[Text with \\\\ backslash and \\[ bracket and \\] bracket]")

--- verbatim-multiline-content ---
// Multiline, newlines, and line continuation
#let with_newlines = [Content
with
newlines]
#test(
  verbatim(with_newlines).replace("\r\n", "\n"),
  "[Content
with
newlines]"
)
#let newlines_edges = [
  Leading and trailing spaces
]
#test(verbatim(newlines_edges).replace("\r\n", "\n"), "[
  Leading and trailing spaces
]")
#let line_continuation = [Text with \
line continuation character]
#test(verbatim(line_continuation).replace("\r\n", "\n"), "[Text with \\
line continuation character]")

--- verbatim-empty-and-whitespace ---
// Empty, whitespace-only, and edge whitespace
#let empty = []
#test(verbatim(empty), "[]")
#let whitespace_only = [   ]
#test(verbatim(whitespace_only), "[   ]")
#let whitespace_edges = [  Leading and trailing spaces  ]
#test(verbatim(whitespace_edges), "[  Leading and trailing spaces  ]")
#let empty_nested = [Outer [ ] inner [   ] content]
#test(verbatim(empty_nested), "[Outer [ ] inner [   ] content]")

--- verbatim-adjacent-blocks ---
// Multiple adjacent verbatim blocks and variable assignments
#let multi1 = [first]; #let multi2 = [second]
#test(verbatim(multi1), "[first]")
#test(verbatim(multi2), "[second]")
#let a = [First]; #let b = [Second]; #let c = [Third]
#test(verbatim(a), "[First]")
#test(verbatim(b), "[Second]")
#test(verbatim(c), "[Third]")

--- verbatim-semicolon-context ---
// Semicolon and block context handling
#let before_semicolon = [content]; #let after = [more]
#test(verbatim(before_semicolon), "[content]")
#let after_semicolon = {
  let x = 42;
  [Content after semicolon]
}
#test(verbatim(after_semicolon), "[Content after semicolon]")

--- verbatim-raw-code-content ---
// Raw/code blocks, regex, and code with brackets
#let code_with_brackets = [#("array[0]")]
#test(verbatim(code_with_brackets), "[#(\"array[0]\")]")
#let raw_and_code = [Text with `[raw brackets]` and #("code[0]") expressions]
#test(verbatim(raw_and_code), "[Text with `[raw brackets]` and #(\"code[0]\") expressions]")
#let regex_content = [Pattern: #regex("[a-z]+[.*]") matches text]
#test(verbatim(regex_content), "[Pattern: #regex(\"[a-z]+[.*]\") matches text]")
#let func_arg_content = text(weight: "bold")[Bold text here]
#test(verbatim(func_arg_content), "text(weight: \"bold\")[Bold text here]")
#let nested_functions = upper(strong([Nested content]))
#test(verbatim(nested_functions), "upper(strong([Nested content]))")

--- verbatim-json-unicode ---
// JSON-like, unicode, and international content
#let unicode_content = [Content with 🦀 emoji and ñ characters]
#test(verbatim(unicode_content), "[Content with 🦀 emoji and ñ characters]")
#let international = [Hällo Wörld with symbols: ★♠♥♦ and more text]
#test(verbatim(international), "[Hällo Wörld with symbols: ★♠♥♦ and more text]")
#let unicode_brackets = [Text with ［full-width brackets］ and 【double brackets】]
#test(verbatim(unicode_brackets), "[Text with ［full-width brackets］ and 【double brackets】]")
#let json_like = [Data: {"array": [1, 2, 3], "nested": {"key": "[value]"}}]
#test(verbatim(json_like), "[Data: {\"array\": [1, 2, 3], \"nested\": {\"key\": \"[value]\"}}]")

--- verbatim-long-content ---
// Long lines, long words, and stress tests
#let long_content = [This is a very long line of content that goes on and on and on and contains many words and should test whether the bracket matching algorithm performs well with longer text segments that might span across multiple internal spans or buffers]
#test(verbatim(long_content), "[This is a very long line of content that goes on and on and on and contains many words and should test whether the bracket matching algorithm performs well with longer text segments that might span across multiple internal spans or buffers]")
#let long_word = [Antidisestablishmentarianism_and_other_very_long_words_that_might_cause_issues_with_span_boundaries]
#test(verbatim(long_word), "[Antidisestablishmentarianism_and_other_very_long_words_that_might_cause_issues_with_span_boundaries]")

--- verbatim-deep-nesting-stress ---
// Deeply nested and stress bracket matching
#let deep_nested_5 = [Level 1 [Level 2 [Level 3 [Level 4 [Level 5] back to 4] back to 3] back to 2] back to 1]
#test(verbatim(deep_nested_5), "[Level 1 [Level 2 [Level 3 [Level 4 [Level 5] back to 4] back to 3] back to 2] back to 1]")

--- verbatim-complex-mixed ---
// Complex and mixed: multi-line, quotes, code, math, comments, structures
#let complex_mix = [
  Multi-line with "quotes [nested]" and 
  `code [blocks]` plus $x^2 [...]_2$ and
  // comment [syntax] 
  {curly: "json[like]"} structures
]
#test(verbatim(complex_mix).replace("\r\n", "\n"), "[
  Multi-line with \"quotes [nested]\" and 
  `code [blocks]` plus $x^2 [...]_2$ and
  // comment [syntax] 
  {curly: \"json[like]\"} structures
]")

--- verbatim-tabs-and-zero-width ---
// Tabs, zero-width spaces, and special whitespace
#let with_tabs = [Text	with	tabs	between	words]
#test(verbatim(with_tabs), "[Text	with	tabs	between	words]")
#let zero_width = [Text​with​zero​width​spaces]  // Contains zero-width spaces
#test(verbatim(zero_width), "[Text​with​zero​width​spaces]")

--- verbatim-minimal-cases ---
// Minimal and edge cases: single chars, brackets, semicolons, empty lines
#{[ #test(verbatim("abc"), "\"abc\"") ]}
#{[ #test(verbatim(["abc"]), "[\"abc\"]") ]}
#{[ #test(verbatim([abc]), "[abc]") ]}
#{[ #test(verbatim([{]), "[{]") ]}
#{[ #test(verbatim([;]), "[;]") ]}
#{[ #test(verbatim([
]).replace("\r\n", "\n"), "[
]") ]}
#let complex_expr = [Content with array access]
#test(verbatim(complex_expr), "[Content with array access]")
