// Test arrays.
// Ref: false

---
// Ref: true

#set page(width: 150pt)

// Empty.
#{()}

// Not an array, just a parenthesized expression.
#{(1)}

// One item and trailing comma.
#{(-1,)}

// No trailing comma.
#{(true, false)}

// Multiple lines and items and trailing comma.
#{("1"
     , rgb("002")
     ,)}

---
// Test the `len` method.
#test(().len(), 0)
#test(("A", "B", "C").len(), 3)

---
// Test lvalue and rvalue access.
#{
  let array = (1, 2)
  array.at(1) += 5 + array.at(0)
  test(array, (1, 8))
}

---
// Test different lvalue method.
#{
  let array = (1, 2, 3)
  array.first() = 7
  array.at(1) *= 8
  test(array, (7, 16, 3))
}

---
// Test rvalue out of bounds.
// Error: 3-18 array index out of bounds (index: 5, len: 3)
#{(1, 2, 3).at(5)}

---
// Test lvalue out of bounds.
#{
  let array = (1, 2, 3)
  // Error: 3-14 array index out of bounds (index: 3, len: 3)
  array.at(3) = 5
}

---
// Test bad lvalue.
// Error: 2:4-2:15 cannot mutate a temporary value
#let array = (1, 2, 3)
#{ array.len() = 4 }

---
// Test bad lvalue.
// Error: 2:4-2:16 type array has no method `yolo`
#let array = (1, 2, 3)
#{ array.yolo() = 4 }

---
// Test negative indices.
#{
  let array = (1, 2, 3, 4)
  test(array.at(0), 1)
  test(array.at(-1), 4)
  test(array.at(-2), 3)
  test(array.at(-3), 2)
  test(array.at(-4), 1)
}

---
// The the `first` and `last` methods.
#test((1,).first(), 1)
#test((2,).last(), 2)
#test((1, 2, 3).first(), 1)
#test((1, 2, 3).last(), 3)

---
// Error: 4-14 array is empty
#{ ().first() }

---
// Error: 4-13 array is empty
#{ ().last() }

---
// Test the `push` and `pop` methods.
#{
  let tasks = (a: (1, 2, 3), b: (4, 5, 6))
  test(tasks.at("a").pop(), 3)
  tasks.b.push(7)
  test(tasks.a, (1, 2))
  test(tasks.at("b"), (4, 5, 6, 7))
}

---
// Test the `insert` and `remove` methods.
#{
  let array = (0, 1, 2, 4, 5)
  array.insert(3, 3)
  test(array, range(6))
  array.remove(1)
  test(array, (0, 2, 3, 4, 5))
}

---
// Error: 2:18-2:20 missing argument: index
#let numbers = ()
#{ numbers.insert() }

---
// Test the `slice` method.
#test((1, 2, 3, 4).slice(2), (3, 4))
#test(range(10).slice(2, 6), (2, 3, 4, 5))
#test(range(10).slice(4, count: 3), (4, 5, 6))
#test(range(10).slice(-5, count: 2), (5, 6))
#test((1, 2, 3).slice(2, -2), ())
#test((1, 2, 3).slice(-2, 2), (2,))
#test((1, 2, 3).slice(-3, 2), (1, 2))
#test("ABCD".split("").slice(1, -1).join("-"), "A-B-C-D")

---
// Error: 4-32 array index out of bounds (index: 12, len: 10)
#{ range(10).slice(9, count: 3) }

---
// Error: 4-26 array index out of bounds (index: -4, len: 3)
#{ (1, 2, 3).slice(0, -4) }

---
// Test the `position` method.
#test(("Hi", "❤️", "Love").position(s => s == "❤️"), 1)
#test(("Bye", "💘", "Apart").position(s => s == "❤️"), none)
#test(("A", "B", "CDEF", "G").position(v => v.len() > 2), 2)

---
// Test the `filter` method.
#test(().filter(calc.even), ())
#test((1, 2, 3, 4).filter(calc.even), (2, 4))
#test((7, 3, 2, 5, 1).filter(x => x < 5), (3, 2, 1))

---
// Test the `map` method.
#test(().map(x => x * 2), ())
#test((2, 3).map(x => x * 2), (4, 6))

---
// Test the `fold` method.
#test(().fold("hi", grid), "hi")
#test((1, 2, 3, 4).fold(0, (s, x) => s + x), 10)

---
// Error: 22-32 function must have exactly two parameters
#{ (1, 2, 3).fold(0, () => none) }

---
// Test the `rev` method.
#test(range(3).rev(), (2, 1, 0))

---
// Test the `join` method.
#test(().join(), none)
#test((1,).join(), 1)
#test(("a", "b", "c").join(), "abc")
#test("(" + ("a", "b", "c").join(", ") + ")", "(a, b, c)")

---
// Error: 3-23 cannot join boolean with boolean
#{(true, false).join()}

---
// Error: 3-21 cannot join string with integer
#{("a", "b").join(1)}

---
// Test joining content.
// Ref: true
#{([One], [Two], [Three]).join([, ], last: [ and ])}.

---
// Test the `sorted` method.
#test(().sorted(), ())
#test(((true, false) * 10).sorted(), (false,) * 10 + (true,) * 10)
#test(("it", "the", "hi", "text").sorted(), ("hi", "it", "text", "the"))
#test((2, 1, 3, 10, 5, 8, 6, -7, 2).sorted(), (-7, 1, 2, 2, 3, 5, 6, 8, 10))

---
// Error: 3-27 cannot order content and content
#{([Hi], [There]).sorted()}

---
// Error: 3-19 array index out of bounds (index: -4, len: 3)
#{(1, 2, 3).at(-4)}

---
// Error: 4 expected closing paren
#{(}

// Error: 3-4 unexpected closing paren
#{)}

// Error: 5-7 unexpected end of block comment
#{(1*/2)}

// Error: 7-9 invalid number suffix
#{(1, 1u 2)}

// Error: 4-5 unexpected comma
#{(,1)}

// Missing expression makes named pair incomplete, making this an empty array.
// Error: 6 expected expression
#{(a:)}

// Named pair after this is already identified as an array.
// Error: 7-11 expected expression, found named pair
#{(1, b: 2)}

// Keyed pair after this is already identified as an array.
// Error: 7-15 expected expression, found keyed pair
#{(1, "key": 2)}
