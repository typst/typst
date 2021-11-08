// Test function calls.
// Ref: false

---
// Ref: true

// Ommitted space.
[#font(weight:"bold")Bold]

// Call return value of function with body.
#let f(x, body) = (y) => [#x] + body + [#y]
#f(1)[2](3)

// Don't parse this as a function.
// Should output `<function test> (it)`.
#test (it)

#let f(body) = body
#f[A]
#f()[A]
#f([A])

#let g(a, b) = a + b
#g[A][B]
#g([A], [B])
#g()[A][B]

---
// Trailing comma.
#test(1 + 1, 2,)

// Call function assigned to variable.
#let alias = type
#test(alias(alias), "function")

// Callee expressions.
{
  // Wrapped in parens.
  test((type)("hi"), "string")

  // Call the return value of a function.
  let adder(dx) = x => x + dx
  test(adder(2)(5), 7)
}

---
// Error: 2-6 expected function or collection, found boolean
{true()}

---
#let x = "x"

// Error: 1-3 expected function or collection, found string
#x()

---
#let f(x) = x

// Error: 1-6 expected function or collection, found integer
#f(1)(2)

---
#let f(x) = x

// Error: 1-6 expected function or collection, found template
#f[1](2)

---
// Error: 7-8 expected expression, found colon
#func(:)

// Error: 10-12 expected expression, found end of block comment
#func(a:1*/)

// Error: 8 expected comma
#func(1 2)

// Error: 7-8 expected identifier
// Error: 9 expected expression
#func(1:)

// Error: 7-8 expected identifier
#func(1:2)

// Error: 7-10 expected identifier
{func((x):1)}

---
// Error: 2:1 expected closing bracket
#func[`a]`

---
// Error: 7 expected closing paren
{func(}

---
// Error: 2:1 expected quote
// Error: 2:1 expected closing paren
#func("]
