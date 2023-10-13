// Test argument sinks and spreading.
// Ref: false

---
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

---
// Test multiple calls.
#{
  let f(b, c: "!") = b + c
  let g(a, ..sink) = a + f(..sink)
  test(g("a", "b", c: "c"), "abc")
}

---
// Test doing things with arguments.
#{
  let save(..args) = {
    test(type(args), arguments)
    test(repr(args), "(three: true, 1, 2)")
  }

  save(1, 2, three: true)
}

---
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

---
// None is spreadable.
#let f() = none
#f(..none)
#f(..if false {})
#f(..for x in () [])

---
// unnamed spread
#let f(.., a) = a
#test(f(1, 2, 3), 3)

---
// Error: 13-19 cannot spread string
#calc.min(.."nope")

---
// Error: 10-14 expected identifier, found boolean
#let f(..true) = none

---
// Error: 13-16 only one argument sink is allowed
#let f(..a, ..b) = none

---
// Test spreading into array and dictionary.
#{
  let l = (1, 2, 3)
  let r = (5, 6, 7)
  test((..l, 4, ..r), range(1, 8))
  test((..none), ())
}

#{
  let x = (a: 1)
  let y = (b: 2)
  let z = (a: 3)
  test((:..x, ..y, ..z), (a: 3, b: 2))
  test((..(a: 1), b: 2), (a: 1, b: 2))
}

---
// Error: 11-17 cannot spread dictionary into array
#(1, 2, ..(a: 1))

---
// Error: 5-11 cannot spread array into dictionary
#(..(1, 2), a: 1)

---
// Spread at beginning.
#{
  let f(..a, b) = (a, b)
  test(repr(f(1)), "((), 1)")
  test(repr(f(1, 2, 3)), "((1, 2), 3)")
  test(repr(f(1, 2, 3, 4, 5)), "((1, 2, 3, 4), 5)")
}

---
// Spread in the middle.
#{
  let f(a, ..b, c) = (a, b, c)
  test(repr(f(1, 2)), "(1, (), 2)")
  test(repr(f(1, 2, 3, 4, 5)), "(1, (2, 3, 4), 5)")
}

---
#{
  let f(..a, b, c, d) = none

  // Error: 4-10 missing argument: d
  f(1, 2)
}
