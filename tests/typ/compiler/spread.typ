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
    test(type(args), "arguments")
    test(repr(args), "(1, 2, three: true)")
  }

  save(1, 2, three: true)
}

---
// Test spreading array and dictionary.
#{
  let more = (3, -3, 6, 10)
  test(min(1, 2, ..more), -3)
  test(max(..more, 9), 10)
  test(max(..more, 11), 11)
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
// Error: 8-14 cannot spread string
#min(.."nope")

---
// Error: 10-14 expected identifier, found boolean
#let f(..true) = none

---
// Error: 15-16 only one argument sink is allowed
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
// Error: 12-18 cannot spread dictionary into array
#{(1, 2, ..(a: 1))}

---
// Error: 6-12 cannot spread array into dictionary
#{(..(1, 2), a: 1)}
