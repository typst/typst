// Test closures.
// Ref: false

---
// Don't parse closure directly in content.
// Ref: true

#let x = "x"

// Should output `x => y`.
#x => y

---
// Basic closure without captures.
#{
  let adder = (x, y) => x + y
  test(adder(2, 3), 5)
}

---
// Pass closure as argument and return closure.
// Also uses shorthand syntax for a single argument.
#{
  let chain = (f, g) => (x) => f(g(x))
  let f = x => x + 1
  let g = x => 2 * x
  let h = chain(f, g)
  test(h(2), 5)
}

---
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

---
// Redefined variable.
#{
  let x = 1
  let f() = {
    let x = x + 2
    x
  }
  test(f(), 3)
}

---
// Import bindings.
#{
  let b = "module.typ"
  let f() = {
    import b: b
    b
  }
  test(f(), 1)
}

---
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

---
// Let + closure bindings.
#{
  let g = "hi"
  let f() = {
    let g() = "bye"
    g()
  }
  test(f(), "bye")
}

---
// Parameter bindings.
#{
  let x = 5
  let g() = {
    let f(x, y: x) = x + y
    f
  }

  test(g()(8), 13)
}

---
// Don't leak environment.
#{
  // Error: 16-17 unknown variable: x
  let func() = x
  let x = "hi"
  func()
}

---
// Too few arguments.
#{
  let types(x, y) = "[" + str(type(x)) + ", " + str(type(y)) + "]"
  test(types(14%, 12pt), "[ratio, length]")

  // Error: 13-21 missing argument: y
  test(types("nope"), "[string, none]")
}

---
// Too many arguments.
#{
  let f(x) = x + 1

  // Error: 8-13 unexpected argument
  f(1, "two", () => x)
}

---
// Mutable method with capture in argument.
#let x = "b"
#let f() = {
  let a = (b: 5)
  a.at(x) = 10
  a
}
#f()

---
#let x = ()
#let f() = {
  // Error: 3-4 variables from outside the function are read-only and cannot be modified
  x.at(1) = 2
}
#f()

---
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

---
// Parameter unpacking.
#let f((a, b), ..c) = (a, b, c)
#test(f((1, 2), 3, 4), (1, 2, (3, 4)))

#let f((k: a, b), c: 3, (d,)) = (a, b, c, d)
#test(f((k: 1, b: 2), (4,)), (1, 2, 3, 4))

// Error: 22-23 duplicate parameter: a
#let f((a: b), (c,), a) = none

// Error: 8-14 expected identifier, found array
#let f((a, b): 0) = none

// Error: 10-19 expected identifier, found destructuring pattern
#let f(..(a, b: c)) = none

// Error: 10-16 expected identifier, found array
#let f(..(a, b)) = none

// Error: 10-19 expected identifier, found destructuring pattern
#let f(..(a, b: c)) = none

---
// Error: 11-12 duplicate parameter: x
#let f(x, x) = none

---
// Error: 14-15 duplicate parameter: a
// Error: 23-24 duplicate parameter: b
// Error: 35-36 duplicate parameter: b
#let f(a, b, a: none, b: none, c, b) = none

---
// Error: 13-14 duplicate parameter: a
#let f(a, ..a) = none

---
// Error: 7-17 expected identifier, named pair or argument sink, found keyed pair
#((a, "named": b) => none)

---
// Error: 10-15 expected identifier, found string
#let foo("key": b) = key

---
// Error: 10-14 expected identifier, found `none`
#let foo(none: b) = key

---
// Error: 11 expected comma
#let foo(_: 3) = none
