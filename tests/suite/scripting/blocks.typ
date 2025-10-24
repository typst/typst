// Test code blocks.

--- code-block-basic-syntax render ---

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

--- code-block-empty render ---
// Nothing evaluates to none.
#test({}, none)

--- code-block-let render ---
// Let evaluates to none.
#test({ let v = 0 }, none)

--- code-block-single-expression render ---
// Evaluates to single expression.
#test({ "hello" }, "hello")

--- code-block-multiple-expressions-single-line render ---
// Evaluates to string.
#test({ let x = "m"; x + "y" }, "my")

--- code-block-join-let-with-expression render ---
// Evaluated to int.
#test({
  let x = 1
  let y = 2
  x + y
}, 3)

--- code-block-join-expression-with-none render ---
// String is joined with trailing none, evaluates to string.
#test({
  type("")
  none
}, str)

--- code-block-join-int-with-content render ---
// Some things can't be joined.
#{
  [A]
  // Error: 3-4 cannot join content with integer
  1
  [B]
}

--- code-block-scope-in-markup render ---
// Block directly in markup also creates a scope.
#{ let x = 1 }

// Error: 7-8 unknown variable: x
#test(x, 1)

--- code-block-scope-in-let render ---
// Block in expression does create a scope.
#let a = {
  let b = 1
  b
}

#test(a, 1)

// Error: 3-4 unknown variable: b
#{b}

--- code-block-double-scope render ---
// Double block creates a scope.
#{{
  import "module.typ": b
  test(b, 1)
}}

// Error: 2-3 unknown variable: b
#b

--- code-block-nested-scopes render ---
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

--- code-block-multiple-literals-without-semicolon render ---
// Multiple unseparated expressions in one line.
// Error: 4 expected semicolon or line break
#{1 2}

--- code-block-multiple-expressions-without-semicolon render ---
// Error: 13 expected semicolon or line break
// Error: 23 expected semicolon or line break
#{let x = -1 let y = 3 x + y}

--- code-block-incomplete-expressions render ---
#{
  // Error: 7-10 expected pattern, found string
  for "v"

  // Error: 8 expected keyword `in`
  // Error: 22 expected block
  for v let z = 1 + 2

  z
}

--- code-block-unclosed render ---
// Error: 2-3 unclosed delimiter
#{

--- code-block-unopened render ---
// Error: 2-3 unexpected closing brace
#}

--- single-right-bracket render ---
// Error: 1-2 unexpected closing bracket
// Hint: 1-2 try using a backslash escape: \]
]

--- right-bracket-nesting render ---
[
= [ Hi ]]
- how [
  - are ]
// Error: 10-11 unexpected closing bracket
// Hint: 10-11 try using a backslash escape: \]
  - error][]
[[]]

--- right-bracket-hash render ---
// Error: 2-3 unexpected closing bracket
#]

--- right-bracket-in-blocks render ---
// Error: 3-4 unclosed delimiter
// Error: 6-7 unexpected closing bracket
// Hint: 6-7 try using a backslash escape: \]
[#{]}]

// Error: 4-5 unexpected closing bracket
// Hint: 4-5 try using a backslash escape: \]
#[]]

// Error: 4-5 unclosed delimiter
// Error: 7-8 unexpected closing bracket
// Hint: 7-8 try using a backslash escape: \]
#[#{]}]

// Error: 2-3 unclosed delimiter
// Error: 3-4 unclosed delimiter
// Error: 4-5 unexpected closing bracket
// Hint: 4-5 try using a backslash escape: \]
#{{]}}

--- content-block-in-markup-scope render ---
// Content blocks also create a scope.
#[#let x = 1]

// Error: 2-3 unknown variable: x
#x
