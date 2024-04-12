// Test closures.

--- closure-without-params-non-atomic ---
// Don't parse closure directly in content.

#let x = "x"

// Should output `x => y`.
#x => y

--- closure-without-captures ---
// Basic closure without captures.
#{
  let adder = (x, y) => x + y
  test(adder(2, 3), 5)
}

--- closure-as-arg ---
// Pass closure as argument and return closure.
// Also uses shorthand syntax for a single argument.
#{
  let chain = (f, g) => (x) => f(g(x))
  let f = x => x + 1
  let g = x => 2 * x
  let h = chain(f, g)
  test(h(2), 5)
}

--- closure-capture-from-popped-stack-frame ---
// Capture environment.
#{
  let mark = "!"
  let greet = {
    let hi = "Hi"
    name => {
        hi + ", " + name + mark
    }
  }

  test(greet("Typst"), "Hi, Typst!")

  // Changing the captured variable after the closure definition has no effect.
  mark = "?"
  test(greet("Typst"), "Hi, Typst!")
}

--- closure-shadows-outer-var ---
// Redefined variable.
#{
  let x = 1
  let f() = {
    let x = x + 2
    x
  }
  test(f(), 3)
}

--- closure-shadows-outer-var-import ---
// Import bindings.
#{
  let b = "module.typ"
  let f() = {
    import b: b
    b
  }
  test(f(), 1)
}

--- closure-shadows-outer-var-for-loop ---
// For loop bindings.
#{
  let v = (1, 2, 3)
  let f() = {
    let s = 0
    for v in v { s += v }
    s
  }
  test(f(), 6)
}

--- closure-let-basic ---
// Let + closure bindings.
#{
  let g = "hi"
  let f() = {
    let g() = "bye"
    g()
  }
  test(f(), "bye")
}

--- closure-let-args ---
// Parameter bindings.
#{
  let x = 5
  let g() = {
    let f(x, y: x) = x + y
    f
  }

  test(g()(8), 13)
}

--- closure-bad-capture ---
// Don't leak environment.
#{
  // Error: 16-17 unknown variable: x
  let func() = x
  let x = "hi"
  func()
}

--- closure-missing-arg-positional ---
// Too few arguments.
#{
  let types(x, y) = "[" + str(type(x)) + ", " + str(type(y)) + "]"
  test(types(14%, 12pt), "[ratio, length]")

  // Error: 8-21 missing argument: y
  test(types("nope"), "[string, none]")
}

--- closure-too-many-args-positional ---
// Too many arguments.
#{
  let f(x) = x + 1

  // Error: 8-13 unexpected argument
  f(1, "two", () => x)
}

--- closure-capture-in-lvalue ---
// Mutable method with capture in argument.
#let x = "b"
#let f() = {
  let a = (b: 5)
  a.at(x) = 10
  a
}
#f()

--- closure-capture-mutate ---
#let x = ()
#let f() = {
  // Error: 3-4 variables from outside the function are read-only and cannot be modified
  x.at(1) = 2
}
#f()

--- closure-named-args-basic ---
// Named arguments.
#{
  let greet(name, birthday: false) = {
    if birthday { "Happy Birthday, " } else { "Hey, " } + name + "!"
  }

  test(greet("Typst"), "Hey, Typst!")
  test(greet("Typst", birthday: true), "Happy Birthday, Typst!")

  // Error: 23-35 unexpected argument: whatever
  test(greet("Typst", whatever: 10))
}

--- closure-args-sink ---
// Parameter unpacking.
#let f((a, b), ..c) = (a, b, c)
#test(f((1, 2), 3, 4), (1, 2, (3, 4)))

#let f((k: a, b), c: 3, (d,)) = (a, b, c, d)
#test(f((k: 1, b: 2), (4,)), (1, 2, 3, 4))

// Error: 8-14 expected identifier, found destructuring pattern
#let f((a, b): 0) = none

// Error: 10-19 expected pattern, found array
#let f(..(a, b: c)) = none

// Error: 10-16 expected pattern, found array
#let f(..(a, b)) = none

--- closure-param-duplicate-positional ---
// Error: 11-12 duplicate parameter: x
#let f(x, x) = none

--- closure-body-multiple-expressions ---
// Error: 21 expected comma
// Error: 22-23 expected pattern, found integer
// Error: 24-25 unexpected plus
// Error: 26-27 expected pattern, found integer
#let f = (x: () => 1 2 + 3) => 4

--- closure-param-duplicate-mixed ---
// Error: 14-15 duplicate parameter: a
// Error: 23-24 duplicate parameter: b
// Error: 35-36 duplicate parameter: b
#let f(a, b, a: none, b: none, c, b) = none

--- closure-param-duplicate-spread ---
// Error: 13-14 duplicate parameter: a
#let f(a, ..a) = none

--- closure-pattern-bad-string ---
// Error: 7-14 expected pattern, found string
#((a, "named": b) => none)

--- closure-let-pattern-bad-string ---
// Error: 10-15 expected pattern, found string
#let foo("key": b) = key

--- closure-param-keyword ---
// Error: 10-14 expected pattern, found `none`
// Hint: 10-14 keyword `none` is not allowed as an identifier; try `none_` instead
#let foo(none: b) = key

--- closure-param-named-underscore ---
// Error: 10-11 expected identifier, found underscore
#let foo(_: 3) = none

--- issue-non-atomic-closure ---
// Ensure that we can't have non-atomic closures.
#let x = 1
#let c = [#(x) => (1, 2)]
#test(c.children.last(), [(1, 2)]))
