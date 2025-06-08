--- repr ---
#let t(a, b) = test(repr(a), b.text)

// Literal values.
#t(auto, `auto`)
#t(true, `true`)
#t(false, `false`)

// Numerical values.
#t(12.0, `12.0`)
#t(3.14, `3.14`)
#t(1234567890.0, `1234567890.0`)
#t(0123456789.0, `123456789.0`)
#t(0.0, `0.0`)
#t(-0.0, `-0.0`)
#t(-1.0, `-1.0`)
#t(-9876543210.0, `-9876543210.0`)
#t(-0987654321.0, `-987654321.0`)
#t(-3.14, `-3.14`)
#t(4.0 - 8.0, `-4.0`)
#t(float.inf, `float.inf`)
#t(-float.inf, `-float.inf`)
#t(float.nan, `float.nan`)

// Strings and escaping.
#t("hi", `"hi"`)
#t("a\n[]\"\u{1F680}string", `"a\n[]\"🚀string"`)

// Array and dictionary.
#t((1, 2, false, ), `(1, 2, false)`)
#t((a: 1, b: "2"), `(a: 1, b: "2")`)

// Functions.
#let f(x) = x
#t(f, `f`)
#t(rect , `rect`)
#t(() => none, `(..) => ..`)

// Types.
#t(int, `int`)
#t(type("hi"), `str`)
#t(type((a: 1)), `dictionary`)

// Constants.
#t(ltr, `ltr`)
#t(left, `left`)

// Content.
#t([*Hey*], `strong(body: [Hey])`)
#t([A _sequence_], `sequence([A], [ ], emph(body: [sequence]))`)
#t([A _longer_ *sequence*!], ```
sequence(
  [A],
  [ ],
  emph(body: [longer]),
  [ ],
  strong(body: [sequence]),
  [!],
)
```)

#test(repr([*Hey*], verbatim: true), `[*Hey*]`.text)
#test(repr([*Hey* {], verbatim: true), `[*Hey* {]`.text)
#test(repr([*Hey* }], verbatim: true), `[*Hey* }]`.text)
#test(repr([#{"*Hey*"}], verbatim: true), `[#{"*Hey*"}]`.text)
#test(repr([#{{"*Hey*"}}], verbatim: true), `[#{{"*Hey*"}}]`.text)
#test(repr([[#{"*Hey*"}]], verbatim: true), `[[#{"*Hey*"}]]`.text)
#test(repr([[#{{"*Hey*"}}]], verbatim: true), `[[#{{"*Hey*"}}]]`.text)
#test(repr([[[#{{"*Hey*"}}]]], verbatim: true), `[[[#{{"*Hey*"}}]]]`.text)
#test(repr([ [[#{{"*Hey*"}}]]], verbatim: true), `[ [[#{{"*Hey*"}}]]]`.text)
#test(repr([[ [#{{"*Hey*"}}]]], verbatim: true), `[[ [#{{"*Hey*"}}]]]`.text)
#test(repr([A _sequence_], verbatim: true), `[A _sequence_]`.text)
#test(repr([A _longer_ *sequence*!], verbatim: true), `[A _longer_ *sequence*!]`.text)
#let some-content = [Some _italic_ and *bold* text]
#test(repr(some-content, verbatim: true), `[Some _italic_ and *bold* text]`.text)
#test(repr([$x^2$], verbatim: true), `[$x^2$]`.text)
// #test(repr([#some-content], verbatim: true), `[#some-content]`.text)
#test(
  repr([Some function calls: #rect(height: 10pt, fill: blue), #align(right)[more *content*!] #some-content], verbatim: true),
  `[Some function calls: #rect(height: 10pt, fill: blue), #align(right)[more *content*!] #some-content]`.text
)
#test(
  repr([Inline math: $x^2 lim_(n -> infinity) x / n = Pr[x] "text" 123; \$ and "$" or $ cos], 
  verbatim: true),
  `[Inline math: $x^2 lim_(n -> infinity) x / n = Pr[x] "text" 123; \$ and "$" or $ cos]`.text
)
// #test(repr(example([*Hey*])), `example([*Hey*])`.text)
#let example(c) = { return (content: c, verbatim: repr(c, verbatim: true)) }
#test(example([A _longer_ *sequence*!]).verbatim, `[A _longer_ *sequence*!]`.text)

// Content > Verbatim use case > Metadata and content.
#let card(title, body) = [
  #metadata((title: repr(title, verbatim: true), body: repr(body, verbatim: true))) <card>
]
#card("Title 123", [*Body* _123_ $x^2$])
#context {
  test(query(<card>).at(0).value.title, `"Title 123"`.text)
  test(query(<card>).at(0).value.body, `[*Body* _123_ $x^2$]`.text)
}

// Colors and strokes.
#t(rgb("f7a205"), `rgb("#f7a205")`)
#t(2pt + rgb("f7a205"), `2pt + rgb("#f7a205")`)
#t(blue, `rgb("#0074d9")`)
#t(color.linear-rgb(blue), `color.linear-rgb(0%, 17.46%, 69.39%)`)
#t(oklab(blue), `oklab(56.22%, -0.05, -0.17)`)
#t(oklch(blue), `oklch(56.22%, 0.177, 253.71deg)`)
#t(cmyk(blue), `cmyk(100%, 46.54%, 0%, 14.9%)`)
#t(color.hsl(blue), `color.hsl(207.93deg, 100%, 42.55%)`)
#t(color.hsv(blue), `color.hsv(207.93deg, 100%, 85.1%)`)
#t(luma(blue), `luma(45.53%)`)

// Gradients.
#t(
  gradient.linear(blue, red),
  `gradient.linear((rgb("#0074d9"), 0%), (rgb("#ff4136"), 100%))`,
)
#t(
  gradient.linear(blue, red, dir: ttb),
  `gradient.linear(dir: rtl, (rgb("#0074d9"), 0%), (rgb("#ff4136"), 100%))`,
)
#t(
  gradient.linear(blue, red, relative: "self", angle: 45deg),
  `gradient.linear(angle: 45deg, relative: "self", (rgb("#0074d9"), 0%), (rgb("#ff4136"), 100%))`,
)
#t(
  gradient.linear(blue, red, space: rgb, angle: 45deg),
  `gradient.linear(angle: 45deg, space: rgb, (rgb("#0074d9"), 0%), (rgb("#ff4136"), 100%))`,
)

// === STRESS TESTS FOR BRACKET MATCHING ===
// These tests are designed to potentially break the bracket-matching algorithm

// Nested brackets in different contexts
#let nested1 = [Text with [nested] brackets]
#test(repr(nested1, verbatim: true), `[Text with [nested] brackets]`.text)

// Brackets in strings within content
#let string_brackets = [Content with "string [with] brackets" inside]
#test(repr(string_brackets, verbatim: true), `[Content with "string [with] brackets" inside]`.text)

// Escaped brackets
#let escaped = [Text with \[ escaped \] brackets]
#test(repr(escaped, verbatim: true), `[Text with \[ escaped \] brackets]`.text)

// Code expressions with brackets
#let code_with_brackets = [#("array[0]")]
#test(repr(code_with_brackets, verbatim: true), `[#("array[0]")]`.text)

// Multiple content blocks on same line
#let multi1 = [first]; #let multi2 = [second]
#test(repr(multi1, verbatim: true), `[first]`.text)
#test(repr(multi2, verbatim: true), `[second]`.text)

// Content with semicolons (should stop search)
#let before_semicolon = [content]; #let after = [more]
#test(repr(before_semicolon, verbatim: true), `[content]`.text)

// Content with braces (should stop search)
#let with_braces = { let content = [inside]; content }
#test(repr(with_braces, verbatim: true), `[inside]`.text)

// Raw strings with brackets
// #let raw_brackets = [```[raw with brackets]```]
// #test(repr(raw_brackets, verbatim: true), `[```[raw with brackets]```]`.text)

// Math with brackets
#let math_brackets = [$[a, b]$]
#test(repr(math_brackets, verbatim: true), `[$[a, b]$]`.text)

// Very deeply nested brackets
#let deep_nested = [[[[[very deep]]]]]
#test(repr(deep_nested, verbatim: true), `[[[[[very deep]]]]]`.text)

// Mixed bracket types (should only match square brackets)
#let mixed_brackets = [Text with (parentheses) and {braces}]
#test(repr(mixed_brackets, verbatim: true), `[Text with (parentheses) and {braces}]`.text)

// Content with newlines inside
#let with_newlines = [Content
with
newlines]
#test(repr(with_newlines, verbatim: true), `[Content
with
newlines]`.text)

// Empty content blocks
#let empty = []
#test(repr(empty, verbatim: true), `[]`.text)

// Content with only whitespace
#let whitespace_only = [   ]
#test(repr(whitespace_only, verbatim: true), `[   ]`.text)

// // Adjacent brackets without content between
// #let adjacent = [first][second]
// // This test might be tricky - which bracket pair should it find?

// Unbalanced brackets in strings (should not confuse the matcher)
#let unbalanced_in_string = [Text with "unbalanced \[ bracket in string"]
#test(repr(unbalanced_in_string, verbatim: true), `[Text with "unbalanced \[ bracket in string"]`.text)

// Unicode content
#let unicode_content = [Content with 🦀 emoji and ñ characters]
#test(repr(unicode_content, verbatim: true), `[Content with 🦀 emoji and ñ characters]`.text)

// Very long content line (test performance)
#let long_content = [This is a very long line of content that goes on and on and on and contains many words and should test whether the bracket matching algorithm performs well with longer text segments that might span across multiple internal spans or buffers]
#test(repr(long_content, verbatim: true), `[This is a very long line of content that goes on and on and on and contains many words and should test whether the bracket matching algorithm performs well with longer text segments that might span across multiple internal spans or buffers]`.text)

// Complex expression with array access
#let complex_expr = [Content with array access]
#test(repr(complex_expr, verbatim: true), `[Content with array access]`.text)

// ===== EXTENSIVE TESTS FOR BRACKET MATCHING =====
// These tests are designed to "stress-test" the bracket-matching algorithm

// Deeply nested brackets (5 levels)
#let deep_nested = [Level 1 [Level 2 [Level 3 [Level 4 [Level 5] back to 4] back to 3] back to 2] back to 1]
#test(repr(deep_nested, verbatim: true), `[Level 1 [Level 2 [Level 3 [Level 4 [Level 5] back to 4] back to 3] back to 2] back to 1]`.text)

// Brackets in raw strings and code
#let raw_and_code = [Text with `[raw brackets]` and #("code[0]") expressions]
#test(repr(raw_and_code, verbatim: true), "[Text with `[raw brackets]` and #(\"code[0]\") expressions]")

// Adjacent content blocks with shared variables
#let first_part = [First]
#let second_part = [Second]
#test(repr(first_part, verbatim: true), `[First]`.text)
#test(repr(second_part, verbatim: true), `[Second]`.text)

// Content with backslash escapes
#let escaped_chars = [Text with \\ backslash and \[ bracket and \] bracket]
#test(repr(escaped_chars, verbatim: true), `[Text with \\ backslash and \[ bracket and \] bracket]`.text)

// Content with mathematical expressions containing brackets
#let math_brackets = [Formula: $sum_(i=1)^n [x_i]$ and arrays]
#test(repr(math_brackets, verbatim: true), `[Formula: $sum_(i=1)^n [x_i]$ and arrays]`.text)

// Plain string should still work
#let plain_string = "abc"
#test(repr(plain_string, verbatim: true), `"abc"`.text)
#test(repr("abc", verbatim: true), `"abc"`.text)

// Miscellaneous
#{[ #test(repr("abc", verbatim: true), `"abc"`.text) ]}
#{[ #test(repr(["abc"], verbatim: true), `["abc"]`.text) ]}
#{[ #test(repr([abc], verbatim: true), `[abc]`.text) ]}
#{[ #test(repr([{], verbatim: true), "[{]") ]}
#{[ #test(repr([;], verbatim: true), "[;]") ]}
#{[ #test(repr([
], verbatim: true), "[
]") ]}

// // String concatenation with different types of quotes
// #let string_concat = "a" + `b` + `"c"`
// #test(repr(string_concat, verbatim: true), "\"a\" + `b` + `\"c\"`")
// #test(repr("a" + `b` + `"c"`, verbatim: true), "\"a\" + `b` + `\"c\"`")

// Content with regex-like patterns
#let regex_content = [Pattern: #regex("[a-z]+[.*]") matches text]
#test(repr(regex_content, verbatim: true), `[Pattern: #regex("[a-z]+[.*]") matches text]`.text)

// Unicode brackets and similar characters
#let unicode_brackets = [Text with ［full-width brackets］ and 【double brackets】]
#test(repr(unicode_brackets, verbatim: true), `[Text with ［full-width brackets］ and 【double brackets】]`.text)

// Content with JSON-like structures
#let json_like = [Data: {"array": [1, 2, 3], "nested": {"key": "[value]"}}]
#test(repr(json_like, verbatim: true), `[Data: {"array": [1, 2, 3], "nested": {"key": "[value]"}}]`.text)

// Content with string literals containing escaped quotes
#let complex_strings = [Text with "string containing \" quotes and [brackets]" here]
#test(repr(complex_strings, verbatim: true), `[Text with "string containing \" quotes and [brackets]" here]`.text)

// Content blocks within function arguments
#let func_arg_content = text(weight: "bold")[Bold text here]
#test(repr(func_arg_content, verbatim: true), `text(weight: "bold")[Bold text here]`.text)

// Content with extremely long single word (boundary test)
#let long_word = [Antidisestablishmentarianism_and_other_very_long_words_that_might_cause_issues_with_span_boundaries]
#test(repr(long_word, verbatim: true), `[Antidisestablishmentarianism_and_other_very_long_words_that_might_cause_issues_with_span_boundaries]`.text)

// Mixed bracket types in sequence
#let mixed_brackets = [Text {curly} (paren) [square] `backtick` more text]
#test(repr(mixed_brackets, verbatim: true), "[Text {curly} (paren) [square] `backtick` more text]")

// Content immediately after semicolon (delimiter test)
#let after_semicolon = {
  let x = 42;
  [Content after semicolon]
}
#test(repr(after_semicolon, verbatim: true), `[Content after semicolon]`.text)

// Content with trailing and leading newlines 1
#let newlines_edges = [
  Leading and trailing spaces
]
#test(repr(newlines_edges, verbatim: true), "[
  Leading and trailing spaces
]")

// Content with trailing and leading whitespace 2
#let whitespace_edges = [  Leading and trailing spaces  ]
#test(repr(whitespace_edges, verbatim: true), `[  Leading and trailing spaces  ]`.text)

// Content with line continuation characters
#let line_continuation = [Text with \
line continuation character]
#test(repr(line_continuation, verbatim: true), `[Text with \
line continuation character]`.text)

// Nested function calls with content
#let nested_functions = upper(strong([Nested content]))
#test(repr(nested_functions, verbatim: true), "upper(strong([Nested content]))")

// Content with zero-width characters
#let zero_width = [Text​with​zero​width​spaces]  // Contains zero-width spaces
#test(repr(zero_width, verbatim: true), `[Text​with​zero​width​spaces]`.text)

// Multiple bracketed expressions on same line with assignments
#let a = [First]; #let b = [Second]; #let c = [Third]
#test(repr(a, verbatim: true), `[First]`.text)
#test(repr(b, verbatim: true), `[Second]`.text)
#test(repr(c, verbatim: true), `[Third]`.text)

// Content with international characters and symbols
#let international = [Hällo Wörld with symbols: ★♠♥♦ and more text]
#test(repr(international, verbatim: true), `[Hällo Wörld with symbols: ★♠♥♦ and more text]`.text)

// Empty and whitespace-only nested brackets
#let empty_nested = [Outer [ ] inner [   ] content]
#test(repr(empty_nested, verbatim: true), `[Outer [ ] inner [   ] content]`.text)

// Content with tab characters
#let with_tabs = [Text	with	tabs	between	words]
#test(repr(with_tabs, verbatim: true), `[Text	with	tabs	between	words]`.text)

// Very complex mixed content (stress test)
#let complex_mix = [
  Multi-line with "quotes [nested]" and 
  `code [blocks]` plus $x^2 [...]_2$ and
  // comment [syntax] 
  {curly: "json[like]"} structures
]
#test(repr(complex_mix, verbatim: true), "[
  Multi-line with \"quotes [nested]\" and 
  `code [blocks]` plus $x^2 [...]_2$ and
  // comment [syntax] 
  {curly: \"json[like]\"} structures
]")
