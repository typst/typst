--- verbatim-basic-content ---
// Basic verbatim: simple content, formatting, and sequences
#test(verbatim([*Hey*]), `[*Hey*]`.text)
#test(verbatim([A _sequence_]), `[A _sequence_]`.text)
#test(verbatim([A _longer_ *sequence*!]), `[A _longer_ *sequence*!]`.text)

--- verbatim-bracket-edges ---
// Bracket edge cases: single brackets, nested, and adjacent
#test(verbatim([*Hey* {]), `[*Hey* {]`.text)
#test(verbatim([*Hey* }]), `[*Hey* }]`.text)
#test(verbatim([[#{"*Hey*"}]]), `[[#{"*Hey*"}]]`.text)
#test(verbatim([[ [#{{"*Hey*"}}]]]), `[[ [#{{"*Hey*"}}]]]`.text)
#test(verbatim([[[#{{"*Hey*"}}]]]), `[[[#{{"*Hey*"}}]]]`.text)

--- verbatim-interpolation ---
// Interpolation and dynamic content in verbatim
#test(verbatim([#{"*Hey*"}]), `[#{"*Hey*"}]`.text)
#test(verbatim([#{{"*Hey*"}}]), `[#{{"*Hey*"}}]`.text)

--- verbatim-variable-content ---
// Verbatim with variable assignments
#let some-content = [Some _italic_ and *bold* text]
#test(verbatim(some-content), `[Some _italic_ and *bold* text]`.text)

--- verbatim-function-usage ---
// Verbatim with function calls and nested function usage
#let some-content = [Some _italic_ and *bold* text]
#test(
  verbatim([Some function calls: #rect(height: 10pt, fill: blue), #align(right)[more *content*!] #some-content]),
  `[Some function calls: #rect(height: 10pt, fill: blue), #align(right)[more *content*!] #some-content]`.text
)
#let example(c) = { return (content: c, verbatim: verbatim(c)) }
#test(example([A _longer_ *sequence*!]).verbatim, `[A _longer_ *sequence*!]`.text)

--- verbatim-inline-math ---
// Inline math and math-related content
#test(verbatim([$x^2$]), `[$x^2$]`.text)
#test(
  verbatim([Inline math: $x^2 lim_(n -> infinity) x / n = Pr[x] "text" 123; \$ and "$" or $ cos]),
  `[Inline math: $x^2 lim_(n -> infinity) x / n = Pr[x] "text" 123; \$ and "$" or $ cos]`.text
)
#let math_brackets = [$[a, b]$]
#test(verbatim(math_brackets), `[$[a, b]$]`.text)
#let math_brackets_complex = [Formula: $sum_(i=1)^n [x_i]$ and arrays]
#test(verbatim(math_brackets_complex), `[Formula: $sum_(i=1)^n [x_i]$ and arrays]`.text)

--- verbatim-metadata-context ---
// Verbatim in metadata and context queries
#let card(title, body) = [
  #metadata((title: verbatim(title), body: verbatim(body))) <card>
]
#card("Title 123", [*Body* _123_ $x^2$])
#context {
  test(query(<card>).at(0).value.title, `"Title 123"`.text)
  test(query(<card>).at(0).value.body, `[*Body* _123_ $x^2$]`.text)
}

--- verbatim-nested-brackets ---
// Deeply nested and mixed brackets
#let nested1 = [Text with [nested] brackets]
#test(verbatim(nested1), `[Text with [nested] brackets]`.text)
#let deep_nested = [[[[[very deep]]]]]
#test(verbatim(deep_nested), `[[[[[very deep]]]]]`.text)
#let mixed_brackets = [Text with (parentheses) and {braces}]
#test(verbatim(mixed_brackets), `[Text with (parentheses) and {braces}]`.text)
#let mixed_brackets_seq = [Text {curly} (paren) [square] `backtick` more text]
#test(verbatim(mixed_brackets_seq), "[Text {curly} (paren) [square] `backtick` more text]")

--- verbatim-string-content ---
// Strings, string escapes, and string with brackets
#let string_brackets = [Content with "string [with] brackets" inside]
#test(verbatim(string_brackets), `[Content with "string [with] brackets" inside]`.text)
#let unbalanced_in_string = [Text with "unbalanced \[ bracket in string"]
#test(verbatim(unbalanced_in_string), `[Text with "unbalanced \[ bracket in string"]`.text)
#let complex_strings = [Text with "string containing \" quotes and [brackets]" here]
#test(verbatim(complex_strings), `[Text with "string containing \" quotes and [brackets]" here]`.text)
#let plain_string = "abc"
#test(verbatim(plain_string), `"abc"`.text)
#test(verbatim("abc"), `"abc"`.text)

--- verbatim-escaped-characters ---
// Escaped brackets, backslashes, and special characters
#let escaped = [Text with \[ escaped \] brackets]
#test(verbatim(escaped), `[Text with \[ escaped \] brackets]`.text)
#let escaped_chars = [Text with \\ backslash and \[ bracket and \] bracket]
#test(verbatim(escaped_chars), `[Text with \\ backslash and \[ bracket and \] bracket]`.text)

--- verbatim-multiline-content ---
// Multiline, newlines, and line continuation
#let with_newlines = [Content
with
newlines]
#test(
  verbatim(with_newlines).replace("\r\n", "\n"),
  `[Content
with
newlines]`.text
)
#let newlines_edges = [
  Leading and trailing spaces
]
#test(verbatim(newlines_edges).replace("\r\n", "\n"), "[
  Leading and trailing spaces
]")
#let line_continuation = [Text with \
line continuation character]
#test(verbatim(line_continuation).replace("\r\n", "\n"), `[Text with \
line continuation character]`.text)

--- verbatim-empty-and-whitespace ---
// Empty, whitespace-only, and edge whitespace
#let empty = []
#test(verbatim(empty), `[]`.text)
#let whitespace_only = [   ]
#test(verbatim(whitespace_only), `[   ]`.text)
#let whitespace_edges = [  Leading and trailing spaces  ]
#test(verbatim(whitespace_edges), `[  Leading and trailing spaces  ]`.text)
#let empty_nested = [Outer [ ] inner [   ] content]
#test(verbatim(empty_nested), `[Outer [ ] inner [   ] content]`.text)

--- verbatim-adjacent-blocks ---
// Multiple adjacent verbatim blocks and variable assignments
#let multi1 = [first]; #let multi2 = [second]
#test(verbatim(multi1), `[first]`.text)
#test(verbatim(multi2), `[second]`.text)
#let a = [First]; #let b = [Second]; #let c = [Third]
#test(verbatim(a), `[First]`.text)
#test(verbatim(b), `[Second]`.text)
#test(verbatim(c), `[Third]`.text)

--- verbatim-semicolon-context ---
// Semicolon and block context handling
#let before_semicolon = [content]; #let after = [more]
#test(verbatim(before_semicolon), `[content]`.text)
#let after_semicolon = {
  let x = 42;
  [Content after semicolon]
}
#test(verbatim(after_semicolon), `[Content after semicolon]`.text)

--- verbatim-raw-code-content ---
// Raw/code blocks, regex, and code with brackets
#let code_with_brackets = [#("array[0]")]
#test(verbatim(code_with_brackets), `[#("array[0]")]`.text)
#let raw_and_code = [Text with `[raw brackets]` and #("code[0]") expressions]
#test(verbatim(raw_and_code), "[Text with `[raw brackets]` and #(\"code[0]\") expressions]")
#let regex_content = [Pattern: #regex("[a-z]+[.*]") matches text]
#test(verbatim(regex_content), `[Pattern: #regex("[a-z]+[.*]") matches text]`.text)
#let func_arg_content = text(weight: "bold")[Bold text here]
#test(verbatim(func_arg_content), `text(weight: "bold")[Bold text here]`.text)
#let nested_functions = upper(strong([Nested content]))
#test(verbatim(nested_functions), "upper(strong([Nested content]))")

--- verbatim-json-unicode ---
// JSON-like, unicode, and international content
#let unicode_content = [Content with 🦀 emoji and ñ characters]
#test(verbatim(unicode_content), `[Content with 🦀 emoji and ñ characters]`.text)
#let international = [Hällo Wörld with symbols: ★♠♥♦ and more text]
#test(verbatim(international), `[Hällo Wörld with symbols: ★♠♥♦ and more text]`.text)
#let unicode_brackets = [Text with ［full-width brackets］ and 【double brackets】]
#test(verbatim(unicode_brackets), `[Text with ［full-width brackets］ and 【double brackets】]`.text)
#let json_like = [Data: {"array": [1, 2, 3], "nested": {"key": "[value]"}}]
#test(verbatim(json_like), `[Data: {"array": [1, 2, 3], "nested": {"key": "[value]"}}]`.text)

--- verbatim-long-content ---
// Long lines, long words, and stress tests
#let long_content = [This is a very long line of content that goes on and on and on and contains many words and should test whether the bracket matching algorithm performs well with longer text segments that might span across multiple internal spans or buffers]
#test(verbatim(long_content), `[This is a very long line of content that goes on and on and on and contains many words and should test whether the bracket matching algorithm performs well with longer text segments that might span across multiple internal spans or buffers]`.text)
#let long_word = [Antidisestablishmentarianism_and_other_very_long_words_that_might_cause_issues_with_span_boundaries]
#test(verbatim(long_word), `[Antidisestablishmentarianism_and_other_very_long_words_that_might_cause_issues_with_span_boundaries]`.text)

--- verbatim-deep-nesting-stress ---
// Deeply nested and stress bracket matching
#let deep_nested_5 = [Level 1 [Level 2 [Level 3 [Level 4 [Level 5] back to 4] back to 3] back to 2] back to 1]
#test(verbatim(deep_nested_5), `[Level 1 [Level 2 [Level 3 [Level 4 [Level 5] back to 4] back to 3] back to 2] back to 1]`.text)

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
#test(verbatim(with_tabs), `[Text	with	tabs	between	words]`.text)
#let zero_width = [Text​with​zero​width​spaces]  // Contains zero-width spaces
#test(verbatim(zero_width), `[Text​with​zero​width​spaces]`.text)

--- verbatim-minimal-cases ---
// Minimal and edge cases: single chars, brackets, semicolons, empty lines
#{[ #test(verbatim("abc"), `"abc"`.text) ]}
#{[ #test(verbatim(["abc"]), `["abc"]`.text) ]}
#{[ #test(verbatim([abc]), `[abc]`.text) ]}
#{[ #test(verbatim([{]), "[{]") ]}
#{[ #test(verbatim([;]), "[;]") ]}
#{[ #test(verbatim([
]).replace("\r\n", "\n"), "[
]") ]}
#let complex_expr = [Content with array access]
#test(verbatim(complex_expr), `[Content with array access]`.text)
