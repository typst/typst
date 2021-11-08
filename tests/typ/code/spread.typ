// Test argument sinks and spreading.
// Ref: false

---
// Test standard argument overriding.
{
  let font(style: "normal", weight: "regular") = {
    "(style: " + style + ", weight: " + weight + ")"
  }

  let myfont(..args) = font(weight: "bold", ..args)
  test(myfont(), "(style: normal, weight: bold)")
  test(myfont(weight: "black"), "(style: normal, weight: black)")
  test(myfont(style: "italic"), "(style: italic, weight: bold)")
}

---
// Test multiple calls.
{
  let f(b, c: "!") = b + c
  let g(a, ..sink) = a + f(..sink)
  test(g("a", "b", c: "c"), "abc")
}

---
// Test storing arguments in a variable.
{
  let args
  let save(..sink) = {
    args = sink
  }

  save(1, 2, three: true)
  test(type(args), "arguments")
  test(repr(args), "(1, 2, three: true)")
}

---
// Test spreading array and dictionary.
{
  let more = (3, -3, 6, 10)
  test(min(1, 2, ..more), -3)
  test(max(..more, 9), 10)
  test(max(..more, 11), 11)
}

{
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
// Error: 8-14 expected identifier
#let f(..true) = none

---
// Error: 15-16 only one argument sink is allowed
#let f(..a, ..b) = none

---
// Error: 3-6 spreading is not allowed here
{(..x)}

---
// Error: 9-17 spreading is not allowed here
{(1, 2, ..(1, 2))}
