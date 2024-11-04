// Test code blocks.

--- code-block-basic-syntax ---

// Evaluates to join of none, [My ] and the two loop bodies.
#{
  let parts = ("my fri", "end.")
  [Hello, ]
  for s in parts [#s]
}

// Evaluates to join of the content and strings.
#{
  [How]
  if true {
    " are"
  }
  [ ]
  if false [Nope]
  [you] + "?"
}

--- code-block-empty ---
// Nothing evaluates to none.
#test({}, none)

--- code-block-let ---
// Let evaluates to none.
#test({ let v = 0 }, none)

--- code-block-single-expression ---
// Evaluates to single expression.
#test({ "hello" }, "hello")

--- code-block-multiple-expressions-single-line ---
// Evaluates to string.
#test({ let x = "m"; x + "y" }, "my")

--- code-block-join-let-with-expression ---
// Evaluated to int.
#test({
  let x = 1
  let y = 2
  x + y
}, 3)

--- code-block-join-expression-with-none ---
// String is joined with trailing none, evaluates to string.
#test({
  type("")
  none
}, str)

--- code-block-join-int-with-content ---
// Some things can't be joined.
#{
  [A]
  // Error: 3-4 cannot join content with integer
  1
  [B]
}

--- code-block-scope-in-markup ---
// Block directly in markup also creates a scope.
#{ let x = 1 }

// Error: 7-8 unknown variable: x
#test(x, 1)

--- code-block-scope-in-let ---
// Block in expression does create a scope.
#let a = {
  let b = 1
  b
}

#test(a, 1)

// Error: 3-4 unknown variable: b
#{b}

--- code-block-double-scope ---
// Double block creates a scope.
#{{
  import "module.typ": b
  test(b, 1)
}}

// Error: 2-3 unknown variable: b
#b

--- code-block-nested-scopes ---
// Multiple nested scopes.
#{
  let a = "a1"
  {
    let a = "a2"
    {
      test(a, "a2")
      let a = "a3"
      test(a, "a3")
    }
    test(a, "a2")
  }
  test(a, "a1")
}

--- code-block-multiple-literals-without-semicolon ---
// Multiple unseparated expressions in one line.
// Error: 4 expected semicolon or line break
#{1 2}

--- code-block-multiple-expressions-without-semicolon ---
// Error: 13 expected semicolon or line break
// Error: 23 expected semicolon or line break
#{let x = -1 let y = 3 x + y}

--- code-block-incomplete-expressions ---
#{
  // Error: 7-10 expected pattern, found string
  for "v"

  // Error: 8 expected keyword `in`
  // Error: 22 expected block
  for v let z = 1 + 2

  z
}

--- code-block-unclosed ---
// Error: 2-3 unclosed delimiter
#{

--- code-block-unopened ---
// Error: 2-3 unexpected closing brace
#}

--- single-right-bracket ---
]

--- content-block-in-markup-scope ---
// Content blocks also create a scope.
#[#let x = 1]

// Error: 2-3 unknown variable: x
#x
