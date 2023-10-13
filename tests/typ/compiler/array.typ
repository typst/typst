// Test arrays.
// Ref: false

---
// Ref: true

#set page(width: 150pt)

// Empty.
#()

// Not an array, just a parenthesized expression.
#(1)

// One item and trailing comma.
#(-1,)

// No trailing comma.
#(true, false)

// Multiple lines and items and trailing comma.
#("1"
    , rgb("002")
    ,)

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
// Error: 2-17 array index out of bounds (index: 5, len: 3) and no default value was specified
#(1, 2, 3).at(5)

---
// Test lvalue out of bounds.
#{
  let array = (1, 2, 3)
  // Error: 3-14 array index out of bounds (index: 3, len: 3) and no default value was specified
  array.at(3) = 5
}

---
// Test default value.
#test((1, 2, 3).at(2, default: 5), 3)
#test((1, 2, 3).at(3, default: 5), 5)

---
// Test remove with default value.

#{
  let array = (1, 2, 3)
  test(array.remove(2, default: 5), 3)
}

#{
  let array = (1, 2, 3)
  test(array.remove(3, default: 5), 5)
}

---
// Test bad lvalue.
// Error: 2:3-2:14 cannot mutate a temporary value
#let array = (1, 2, 3)
#(array.len() = 4)

---
// Test bad lvalue.
// Error: 2:9-2:13 type array has no method `yolo`
#let array = (1, 2, 3)
#(array.yolo() = 4)

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
// Error: 2-12 array is empty
#().first()

---
// Error: 2-11 array is empty
#().last()

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
// Error: 2:2-2:18 missing argument: index
#let numbers = ()
#numbers.insert()

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
// Error: 2-30 array index out of bounds (index: 12, len: 10)
#range(10).slice(9, count: 3)

---
// Error: 2-24 array index out of bounds (index: -4, len: 3)
#(1, 2, 3).slice(0, -4)

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
// Error: 20-22 unexpected argument
#(1, 2, 3).fold(0, () => none)

---
// Test the `sum` method.
#test(().sum(default: 0), 0)
#test(().sum(default: []), [])
#test((1, 2, 3).sum(), 6)

---
// Error: 2-10 cannot calculate sum of empty array with no default
#().sum()

---
// Test the `product` method.
#test(().product(default: 0), 0)
#test(().product(default: []), [])
#test(([ab], 3).product(), [ab]*3)
#test((1, 2, 3).product(), 6)

---
// Error: 2-14 cannot calculate product of empty array with no default
#().product()

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
// Error: 2-22 cannot join boolean with boolean
#(true, false).join()

---
// Error: 2-20 cannot join string with integer
#("a", "b").join(1)

---
// Test joining content.
// Ref: true
#([One], [Two], [Three]).join([, ], last: [ and ]).

---
// Test the `intersperse` method
#test(().intersperse("a"), ())
#test((1,).intersperse("a"), (1,))
#test((1, 2).intersperse("a"), (1, "a", 2))
#test((1, 2, "b").intersperse("a"), (1, "a", 2, "a", "b"))

---
// Test the `sorted` method.
#test(().sorted(), ())
#test(().sorted(key: x => x), ())
#test(((true, false) * 10).sorted(), (false,) * 10 + (true,) * 10)
#test(("it", "the", "hi", "text").sorted(), ("hi", "it", "text", "the"))
#test(("I", "the", "hi", "text").sorted(key: x => x), ("I", "hi", "text", "the"))
#test(("I", "the", "hi", "text").sorted(key: x => x.len()), ("I", "hi", "the", "text"))
#test((2, 1, 3, 10, 5, 8, 6, -7, 2).sorted(), (-7, 1, 2, 2, 3, 5, 6, 8, 10))
#test((2, 1, 3, -10, -5, 8, 6, -7, 2).sorted(key: x => x), (-10, -7, -5, 1, 2, 2, 3, 6, 8))
#test((2, 1, 3, -10, -5, 8, 6, -7, 2).sorted(key: x => x * x), (1, 2, 2, 3, -5, 6, -7, 8, -10))

---
// Test the `zip` method.
#test(().zip(()), ())
#test((1,).zip(()), ())
#test((1,).zip((2,)), ((1, 2),))
#test((1, 2).zip((3, 4)), ((1, 3), (2, 4)))
#test((1, 2, 3, 4).zip((5, 6)), ((1, 5), (2, 6)))
#test(((1, 2), 3).zip((4, 5)), (((1, 2), 4), (3, 5)))
#test((1, "hi").zip((true, false)), ((1, true), ("hi", false)))
#test((1, 2, 3).zip((3, 4, 5), (6, 7, 8)), ((1, 3, 6), (2, 4, 7), (3, 5, 8)))
#test(().zip((), ()), ())
#test((1,).zip((2,), (3,)), ((1, 2, 3),))

---
// Test the `enumerate` method.
#test(().enumerate(), ())
#test(().enumerate(start: 5), ())
#test(("a", "b", "c").enumerate(), ((0, "a"), (1, "b"), (2, "c")))
#test(("a", "b", "c").enumerate(start: 1), ((1, "a"), (2, "b"), (3, "c")))
#test(("a", "b", "c").enumerate(start: 42), ((42, "a"), (43, "b"), (44, "c")))
#test(("a", "b", "c").enumerate(start: -7), ((-7, "a"), (-6, "b"), (-5, "c")))

---
// Test the `dedup` method.
#test(().dedup(), ())
#test((1,).dedup(), (1,))
#test((1, 1).dedup(), (1,))
#test((1, 2, 1).dedup(), (1, 2))
#test(("Jane", "John", "Eric").dedup(), ("Jane", "John", "Eric"))
#test(("Jane", "John", "Eric", "John").dedup(), ("Jane", "John", "Eric"))

---
// Test the `dedup` with the `key` argument.
#test((1, 2, 3, 4, 5, 6).dedup(key: x => calc.rem(x, 2)), (1, 2))
#test((1, 2, 3, 4, 5, 6).dedup(key: x => calc.rem(x, 3)), (1, 2, 3))
#test(("Hello", "World", "Hi", "There").dedup(key: x => x.len()), ("Hello", "Hi"))
#test(("Hello", "World", "Hi", "There").dedup(key: x => x.at(0)), ("Hello", "World", "There"))

---
// Error: 32-37 cannot divide by zero
#(1, 2, 0, 3).sorted(key: x => 5 / x)

---
// Error: 2-26 cannot compare content and content
#([Hi], [There]).sorted()

---
// Error: 2-26 cannot compare 3em with 2pt
#(1pt, 2pt, 3em).sorted()

---
// Error: 2-18 array index out of bounds (index: -4, len: 3) and no default value was specified
#(1, 2, 3).at(-4)

---
// Error: 3-4 unclosed delimiter
#{(}

// Error: 2-3 unclosed delimiter
#{)}

// Error: 4-6 unexpected end of block comment
#(1*/2)

// Error: 6-8 invalid number suffix: u
#(1, 1u 2)

// Error: 3-4 unexpected comma
#(,1)

// Missing expression makes named pair incomplete, making this an empty array.
// Error: 5 expected expression
#(a:)

// Named pair after this is already identified as an array.
// Error: 6-10 expected expression, found named pair
#(1, b: 2)

// Keyed pair after this is already identified as an array.
// Error: 6-14 expected expression, found keyed pair
#(1, "key": 2)
