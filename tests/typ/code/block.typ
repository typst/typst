// Test code blocks.
// Ref: false

---
// Ref: true

// Evaluates to join of none, [My ] and the two loop bodies.
{
  let parts = ("my fri", "end.")
  [Hello, ]
  for s in parts [{s}]
}

// Evaluates to join of the templates and strings.
{
  [How]
  if true {
    " are"
  }
  [ ]
  if false [Nope]
  [you] + "?"
}

---
// Nothing evaluates to none.
#test({}, none)

// Let evaluates to none.
#test({ let v = 0 }, none)

// Evaluates to single expression.
#test({ "hello" }, "hello")

// Evaluates to string.
#test({ let x = "m"; x + "y" }, "my")

// Evaluated to int.
#test({
  let x = 1
  let y = 2
  x + y
}, 3)

// String is joined with trailing none, evaluates to string.
#test({
  type("")
  none
}, "string")

---
// Some things can't be joined.
{
  [A]
  // Error: 3-4 cannot join template with integer
  1
  [B]
}

---
// Block directly in template also creates a scope.
{ let x = 1 }

// Error: 7-8 unknown variable
#test(x, 1)

---
// Block in expression does create a scope.
#let a = {
  let b = 1
  b
}

#test(a, 1)

// Error: 2-3 unknown variable
{b}

---
// Block creates a scope.
{
  import b from "target.typc"
  test(b, 1)
}

// Error: 2-3 unknown variable
{b}

---
// Multiple nested scopes.
{
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

---
// Multiple unseparated expressions in one line.

// Error: 2-4 expected expression, found invalid token
{1u}

// Should output `1`.
// Error: 3 expected semicolon or line break
{1 2}

// Should output `2`.
// Error: 12 expected semicolon or line break
// Error: 22 expected semicolon or line break
{let x = -1 let y = 3 x + y}

// Should output `3`.
{
  // Error: 7-10 expected identifier, found string
  for "v"

  // Error: 8 expected keyword `in`
  for v let z = 1 + 2

  z
}

---
// Error: 2:1 expected closing brace
{

---
// Error: 1-2 unexpected closing brace
}
