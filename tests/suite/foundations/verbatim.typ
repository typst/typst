--- verbatim ---

// Test basic verbatim functionality
#test(verbatim([*Hey*]), `[*Hey*]`.text)
#test(verbatim([*Hey* {]), `[*Hey* {]`.text)
#test(verbatim([*Hey* }]), `[*Hey* }]`.text)
#test(verbatim([#{"*Hey*"}]), `[#{"*Hey*"}]`.text)
#test(verbatim([#{{"*Hey*"}}]), `[#{{"*Hey*"}}]`.text)
#test(verbatim([[#{"*Hey*"}]]), `[[#{"*Hey*"}]]`.text)
#test(verbatim([[#{{"*Hey*"}}]]), `[[#{{"*Hey*"}}]]`.text)
#test(verbatim([[[#{{"*Hey*"}}]]]), `[[[#{{"*Hey*"}}]]]`.text)
#test(verbatim([ [[#{{"*Hey*"}}]]]), `[ [[#{{"*Hey*"}}]]]`.text)
#test(verbatim([[ [#{{"*Hey*"}}]]]), `[[ [#{{"*Hey*"}}]]]`.text)
#test(verbatim([A _sequence_]), `[A _sequence_]`.text)
#test(verbatim([A _longer_ *sequence*!]), `[A _longer_ *sequence*!]`.text)

// Test content assigned to variables
#let some-content = [Some _italic_ and *bold* text]
#test(verbatim(some-content), `[Some _italic_ and *bold* text]`.text)
#test(verbatim([$x^2$]), `[$x^2$]`.text)

// Test complex content with function calls
#test(
  verbatim([Some function calls: #rect(height: 10pt, fill: blue), #align(right)[more *content*!] #some-content]),
  `[Some function calls: #rect(height: 10pt, fill: blue), #align(right)[more *content*!] #some-content]`.text
)

// Test inline math expressions
#test(
  verbatim([Inline math: $x^2 lim_(n -> infinity) x / n = Pr[x] "text" 123; \$ and "$" or $ cos]),
  `[Inline math: $x^2 lim_(n -> infinity) x / n = Pr[x] "text" 123; \$ and "$" or $ cos]`.text
)

// Test function that uses verbatim internally
#let example(c) = { return (content: c, verbatim: verbatim(c)) }
#test(example([A _longer_ *sequence*!]).verbatim, `[A _longer_ *sequence*!]`.text)

// Test verbatim in metadata context
#let card(title, body) = [
  #metadata((title: verbatim(title), body: verbatim(body))) <card>
]
#card("Title 123", [*Body* _123_ $x^2$])
#context {
  test(query(<card>).at(0).value.title, `"Title 123"`.text)
  test(query(<card>).at(0).value.body, `[*Body* _123_ $x^2$]`.text)
}

// ===== STRESS TESTS FOR BRACKET MATCHING =====
// These tests are designed to potentially break the bracket-matching algorithm

// Nested brackets in different contexts
#let nested1 = [Text with [nested] brackets]
#test(verbatim(nested1), `[Text with [nested] brackets]`.text)

// Brackets in strings within content
#let string_brackets = [Content with "string [with] brackets" inside]
#test(verbatim(string_brackets), `[Content with "string [with] brackets" inside]`.text)

// Escaped brackets
#let escaped = [Text with \[ escaped \] brackets]
#test(verbatim(escaped), `[Text with \[ escaped \] brackets]`.text)

// Code expressions with brackets
#let code_with_brackets = [#("array[0]")]
#test(verbatim(code_with_brackets), `[#("array[0]")]`.text)

// Multiple content blocks on same line
#let multi1 = [first]; #let multi2 = [second]
#test(verbatim(multi1), `[first]`.text)
#test(verbatim(multi2), `[second]`.text)

// Content with semicolons (should stop search)
#let before_semicolon = [content]; #let after = [more]
#test(verbatim(before_semicolon), `[content]`.text)

// Content with braces (should stop search)
#let with_braces = { let content = [inside]; content }
#test(verbatim(with_braces), `[inside]`.text)

// Math with brackets
#let math_brackets = [$[a, b]$]
#test(verbatim(math_brackets), `[$[a, b]$]`.text)

// Very deeply nested brackets
#let deep_nested = [[[[[very deep]]]]]
#test(verbatim(deep_nested), `[[[[[very deep]]]]]`.text)

// Mixed bracket types (should only match square brackets)
#let mixed_brackets = [Text with (parentheses) and {braces}]
#test(verbatim(mixed_brackets), `[Text with (parentheses) and {braces}]`.text)

// Content with newlines inside
#let with_newlines = [Content
with
newlines]
#test(
  verbatim(with_newlines).replace("\r\n", "\n"),
  `[Content
with
newlines]`.text
)

// Empty content blocks
#let empty = []
#test(verbatim(empty), `[]`.text)

// Content with only whitespace
#let whitespace_only = [   ]
#test(verbatim(whitespace_only), `[   ]`.text)

// Unbalanced brackets in strings (should not confuse the matcher)
#let unbalanced_in_string = [Text with "unbalanced \[ bracket in string"]
#test(verbatim(unbalanced_in_string), `[Text with "unbalanced \[ bracket in string"]`.text)

// Unicode content
#let unicode_content = [Content with 🦀 emoji and ñ characters]
#test(verbatim(unicode_content), `[Content with 🦀 emoji and ñ characters]`.text)

// Very long content line (test performance)
#let long_content = [This is a very long line of content that goes on and on and on and contains many words and should test whether the bracket matching algorithm performs well with longer text segments that might span across multiple internal spans or buffers]
#test(verbatim(long_content), `[This is a very long line of content that goes on and on and on and contains many words and should test whether the bracket matching algorithm performs well with longer text segments that might span across multiple internal spans or buffers]`.text)

// Complex expression with array access
#let complex_expr = [Content with array access]
#test(verbatim(complex_expr), `[Content with array access]`.text)

// ===== EXTENSIVE TESTS FOR BRACKET MATCHING =====
// These tests are designed to "stress-test" the bracket-matching algorithm

// Deeply nested brackets (5 levels)
#let deep_nested_5 = [Level 1 [Level 2 [Level 3 [Level 4 [Level 5] back to 4] back to 3] back to 2] back to 1]
#test(verbatim(deep_nested_5), `[Level 1 [Level 2 [Level 3 [Level 4 [Level 5] back to 4] back to 3] back to 2] back to 1]`.text)

// Brackets in raw strings and code
#let raw_and_code = [Text with `[raw brackets]` and #("code[0]") expressions]
#test(verbatim(raw_and_code), "[Text with `[raw brackets]` and #(\"code[0]\") expressions]")

// Adjacent content blocks with shared variables
#let first_part = [First]
#let second_part = [Second]
#test(verbatim(first_part), `[First]`.text)
#test(verbatim(second_part), `[Second]`.text)

// Content with backslash escapes
#let escaped_chars = [Text with \\ backslash and \[ bracket and \] bracket]
#test(verbatim(escaped_chars), `[Text with \\ backslash and \[ bracket and \] bracket]`.text)

// Content with mathematical expressions containing brackets
#let math_brackets_complex = [Formula: $sum_(i=1)^n [x_i]$ and arrays]
#test(verbatim(math_brackets_complex), `[Formula: $sum_(i=1)^n [x_i]$ and arrays]`.text)

// Non-content values should work too
#let plain_string = "abc"
#test(verbatim(plain_string), `"abc"`.text)
#test(verbatim("abc"), `"abc"`.text)

// Miscellaneous test cases in code blocks
#{[ #test(verbatim("abc"), `"abc"`.text) ]}
#{[ #test(verbatim(["abc"]), `["abc"]`.text) ]}
#{[ #test(verbatim([abc]), `[abc]`.text) ]}
#{[ #test(verbatim([{]), "[{]") ]}
#{[ #test(verbatim([;]), "[;]") ]}
#{[ #test(verbatim([
]).replace("\r\n", "\n"), "[
]") ]}

// Content with regex-like patterns
#let regex_content = [Pattern: #regex("[a-z]+[.*]") matches text]
#test(verbatim(regex_content), `[Pattern: #regex("[a-z]+[.*]") matches text]`.text)

// Unicode brackets and similar characters
#let unicode_brackets = [Text with ［full-width brackets］ and 【double brackets】]
#test(verbatim(unicode_brackets), `[Text with ［full-width brackets］ and 【double brackets】]`.text)

// Content with JSON-like structures
#let json_like = [Data: {"array": [1, 2, 3], "nested": {"key": "[value]"}}]
#test(verbatim(json_like), `[Data: {"array": [1, 2, 3], "nested": {"key": "[value]"}}]`.text)

// Content with string literals containing escaped quotes
#let complex_strings = [Text with "string containing \" quotes and [brackets]" here]
#test(verbatim(complex_strings), `[Text with "string containing \" quotes and [brackets]" here]`.text)

// Content blocks within function arguments
#let func_arg_content = text(weight: "bold")[Bold text here]
#test(verbatim(func_arg_content), `text(weight: "bold")[Bold text here]`.text)

// Content with extremely long single word (boundary test)
#let long_word = [Antidisestablishmentarianism_and_other_very_long_words_that_might_cause_issues_with_span_boundaries]
#test(verbatim(long_word), `[Antidisestablishmentarianism_and_other_very_long_words_that_might_cause_issues_with_span_boundaries]`.text)

// Mixed bracket types in sequence
#let mixed_brackets_seq = [Text {curly} (paren) [square] `backtick` more text]
#test(verbatim(mixed_brackets_seq), "[Text {curly} (paren) [square] `backtick` more text]")

// Content immediately after semicolon (delimiter test)
#let after_semicolon = {
  let x = 42;
  [Content after semicolon]
}
#test(verbatim(after_semicolon), `[Content after semicolon]`.text)

// Content with trailing and leading newlines
#let newlines_edges = [
  Leading and trailing spaces
]
#test(verbatim(newlines_edges).replace("\r\n", "\n"), "[
  Leading and trailing spaces
]")

// Content with trailing and leading whitespace
#let whitespace_edges = [  Leading and trailing spaces  ]
#test(verbatim(whitespace_edges), `[  Leading and trailing spaces  ]`.text)

// Content with line continuation characters
#let line_continuation = [Text with \
line continuation character]
#test(verbatim(line_continuation).replace("\r\n", "\n"), `[Text with \
line continuation character]`.text)

// Nested function calls with content
#let nested_functions = upper(strong([Nested content]))
#test(verbatim(nested_functions), "upper(strong([Nested content]))")

// Content with zero-width characters
#let zero_width = [Text​with​zero​width​spaces]  // Contains zero-width spaces
#test(verbatim(zero_width), `[Text​with​zero​width​spaces]`.text)

// Multiple bracketed expressions on same line with assignments
#let a = [First]; #let b = [Second]; #let c = [Third]
#test(verbatim(a), `[First]`.text)
#test(verbatim(b), `[Second]`.text)
#test(verbatim(c), `[Third]`.text)

// Content with international characters and symbols
#let international = [Hällo Wörld with symbols: ★♠♥♦ and more text]
#test(verbatim(international), `[Hällo Wörld with symbols: ★♠♥♦ and more text]`.text)

// Empty and whitespace-only nested brackets
#let empty_nested = [Outer [ ] inner [   ] content]
#test(verbatim(empty_nested), `[Outer [ ] inner [   ] content]`.text)

// Content with tab characters
#let with_tabs = [Text	with	tabs	between	words]
#test(verbatim(with_tabs), `[Text	with	tabs	between	words]`.text)

// Very complex mixed content (stress test)
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
