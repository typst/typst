// Test embedded expressions.

--- markup-expr-incomplete ---
// Error: 2-2 expected expression
#

--- markup-expr-incomplete-followed-by-text ---
// Error: 2-2 expected expression
#  hello

--- markup-expr-incomplete-followed-by-comment ---
// Error: 2-2 expected expression
#/* block comment */hello
// Error: 2-2 expected expression
#// line comment
hello
