// Test function calls.

--- call-basic ---

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

--- call-aliased-function ---
// Call function assigned to variable.
#let alias = type
#test(alias(alias), type)

--- call-complex-callee-expression ---
// Callee expressions.
#{
  // Wrapped in parens.
  test((type)("hi"), str)

  // Call the return value of a function.
  let adder(dx) = x => x + dx
  test(adder(2)(5), 7)
}

--- call-bad-type-bool-literal ---
// Error: 2-6 expected function, found boolean
#true()

--- call-bad-type-string-var ---
#let x = "x"

// Error: 2-3 expected function, found string
#x()

--- call-shadowed-builtin-function ---
#let image = "image"

// Error: 2-7 expected function, found string
// Hint: 2-7 use `std.image` to access the shadowed standard library function
#image("image")

--- call-bad-type-int-expr ---
#let f(x) = x

// Error: 2-6 expected function, found integer
#f(1)(2)

--- call-bad-type-content-expr ---
#let f(x) = x

// Error: 2-6 expected function, found content
#f[1](2)

--- call-args-trailing-comma ---
// Trailing comma.
#test(1 + 1, 2,)

--- call-args-duplicate ---
// Error: 26-30 duplicate argument: font
#set text(font: "Arial", font: "Helvetica")

--- call-args-bad-positional-as-named ---
// Error: 4-15 the argument `amount` is positional
// Hint: 4-15 try removing `amount:`
#h(amount: 0.5)

--- call-args-bad-colon ---
// Error: 7-8 unexpected colon
#func(:)

--- call-args-bad-token ---
// Error: 10-12 unexpected end of block comment
// Hint: 10-12 consider escaping the `*` with a backslash or opening the block comment with `/*`
#func(a:1*/)

--- call-args-missing-comma ---
// Error: 8 expected comma
#func(1 2)

--- call-args-bad-name-and-incomplete-pair ---
// Error: 7-8 expected identifier, found integer
// Error: 9 expected expression
#func(1:)

--- call-args-bad-name-int ---
// Error: 7-8 expected identifier, found integer
#func(1:2)

--- call-args-bad-name-string ---
// Error: 7-12 expected identifier, found string
#func("abc": 2)

--- call-args-bad-name-group ---
// Error: 7-10 expected identifier, found group
#func((x):1)

--- call-args-lone-underscore ---
// Test that lone underscore works.
#test((1, 2, 3).map(_ => {}).len(), 3)

--- call-args-spread-override ---
// Test standard argument overriding.
#{
  let f(style: "normal", weight: "regular") = {
    "(style: " + style + ", weight: " + weight + ")"
  }

  let myf(..args) = f(weight: "bold", ..args)
  test(myf(), "(style: normal, weight: bold)")
  test(myf(weight: "black"), "(style: normal, weight: black)")
  test(myf(style: "italic"), "(style: italic, weight: bold)")
}

--- call-args-spread-forward ---
// Test multiple calls.
#{
  let f(b, c: "!") = b + c
  let g(a, ..sink) = a + f(..sink)
  test(g("a", "b", c: "c"), "abc")
}

--- call-args-spread-type-repr ---
// Test doing things with arguments.
#{
  let save(..args) = {
    test(type(args), arguments)
    test(repr(args), "(three: true, 1, 2)")
  }

  save(1, 2, three: true)
}

--- call-args-spread-array-and-dict ---
// Test spreading array and dictionary.
#{
  let more = (3, -3, 6, 10)
  test(calc.min(1, 2, ..more), -3)
  test(calc.max(..more, 9), 10)
  test(calc.max(..more, 11), 11)
}

#{
  let more = (c: 3, d: 4)
  let tostr(..args) = repr(args)
  test(tostr(a: 1, ..more, b: 2), "(a: 1, c: 3, d: 4, b: 2)")
}

--- call-args-spread-none ---
// None is spreadable.
#let f() = none
#f(..none)
#f(..if false {})
#f(..for x in () [])

--- call-args-spread-string-invalid ---
// Error: 11-19 cannot spread string
#calc.min(.."nope")

--- call-args-content-block-unclosed ---
// Error: 6-7 unclosed delimiter
#func[`a]`

--- issue-886-args-sink ---
// Test bugs with argument sinks.
#let foo(..body) = repr(body.pos())
#foo(a: "1", b: "2", 1, 2, 3, 4, 5, 6)

--- issue-3144-unexpected-arrow ---
#let f(a: 10) = a(1) + 1
#test(f(a: _ => 5), 6)

--- issue-3502-space-and-comments-around-destructuring-colon ---
#let ( key :  /* hi */ binding ) = ( key: "ok" )
#test(binding, "ok")

--- issue-3502-space-around-dict-colon ---
#test(( key : "value" ).key, "value")

--- issue-3502-space-around-param-colon ---
// Test that a space after a named parameter is permissible.
#let f( param : v ) = param
#test(f( param /* ok */ : 2 ), 2)

--- call-args-unclosed ---
// Error: 7-8 unclosed delimiter
#{func(}

--- call-args-unclosed-string ---
// Error: 6-7 unclosed delimiter
// Error: 1:7-2:1 unclosed string
#func("]
