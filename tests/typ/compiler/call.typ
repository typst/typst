// Test function calls.
// Ref: false

---
// Ref: true

// Omitted space.
#let f() = {}
#[#f()*Bold*]

// Call return value of function with body.
#let f(x, body) = (y) => [#x] + body + [#y]
#f(1)[2](3)

// Don't parse this as a function.
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
#test(alias(alias), type)

// Callee expressions.
#{
  // Wrapped in parens.
  test((type)("hi"), str)

  // Call the return value of a function.
  let adder(dx) = x => x + dx
  test(adder(2)(5), 7)
}

---
// Error: 26-30 duplicate argument: font
#set text(font: "Arial", font: "Helvetica")

---
// Error: 2-6 expected function, found boolean
#true()

---
#let x = "x"

// Error: 2-3 expected function, found string
#x()

---
#let f(x) = x

// Error: 2-6 expected function, found integer
#f(1)(2)

---
#let f(x) = x

// Error: 2-6 expected function, found content
#f[1](2)

---
// Error: 7 expected expression
// Error: 8 expected expression
#func(:)

// Error: 10-12 unexpected end of block comment
#func(a:1*/)

// Error: 8 expected comma
#func(1 2)

// Error: 7-8 expected identifier, found integer
// Error: 9 expected expression
#func(1:)

// Error: 7-8 expected identifier, found integer
#func(1:2)

// Error: 7-12 expected identifier, found string
#func("abc": 2)

// Error: 7-10 expected identifier, found group
#func((x):1)

---
// Error: 6-7 unclosed delimiter
#func[`a]`

---
// Error: 7-8 unclosed delimiter
#{func(}

---
// Error: 6-7 unclosed delimiter
// Error: 1:7-2:1 unclosed string
#func("]
