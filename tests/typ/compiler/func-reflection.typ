// Test the function reflection API.
// Ref: false

---
// Test `function.arguments` of a function expression.

#let f() = {}
#test(function.arguments(f), ())

#let add(x, y) = x + y
#test(function.arguments(add), (
  (name: 0, ),
  (name: 1, ),
))

#let add(x, y, z: 0) = x + y + z
#let args = function.arguments(add)
#test(args.map((x) => x.name), (0, 1, "z"))
#test(args.at(2).name, "z")
#test((args.at(2).default)(), 0)

#let add_all(..x) = x.pos().add()
#let args = function.arguments(add_all)
#test(args.len(), 1)
#test(args.at(0).name, "x")
#test("default" in args.at(0), false)
#test("sink" in args.at(0), true)
#test(args.at(0).sink, true)

---
// Test `function.arguments` of a native declaration.

#test(function.arguments(calc.rem), ((name: 0), (name: 1)))
#test(function.arguments(panic), ((name: "values", sink: true),))
#test(function.arguments(str.len), ((name: 0),))
#test(function.arguments(str.at), ((name: 0), (name: 1), (name: "default")))

#let args = function.arguments(eval)
#test(args.len(), 3)
#test(args.at(0).name, 0)
#test(args.at(1).name, "mode")
#test((args.at(1).default)(), "code")
#test(args.at(2).name, "scope")
#test((args.at(2).default)(), (:))

---
// Test `function.arguments` of an element function.

#test(function.arguments(emph), ((name: 0),))

#let args = function.arguments(sub)
#test(args.len(), 4)
#test(args.at(0).name, "typographic")
#test((args.at(0).default)(), true)
#test(args.at(1).name, "baseline")
#test((args.at(1).default)(), 0.2em)
#test(args.at(2).name, "size")
#test((args.at(2).default)(), 0.6em)
#test(args.at(3).name, 0)
