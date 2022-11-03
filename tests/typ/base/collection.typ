// Test collection functions.
// Ref: false

---
// Test the `len` method.
#test(().len(), 0)
#test(("A", "B", "C").len(), 3)
#test("Hello World!".len(), 12)
#test((a: 1, b: 2).len(), 2)

---
// The the `first` and `last` methods.
#test(().first(), none)
#test(().last(), none)
#test((1,).first(), 1)
#test((2,).last(), 2)
#test((1, 2, 3).first(), 1)
#test((1, 2, 3).last(), 3)

---
// Test the `push` and `pop` methods.
{
  let tasks = (a: (1, 2, 3), b: (4, 5, 6))
  tasks("a").pop()
  tasks("b").push(7)
  test(tasks("a"), (1, 2))
  test(tasks("b"), (4, 5, 6, 7))
}

---
// Test the `insert` and `remove` methods.
{
  let array = (0, 1, 2, 4, 5)
  array.insert(3, 3)
  test(array, range(6))
  array.remove(1)
  test(array, (0, 2, 3, 4, 5))
}

---
// Error: 2:17-2:19 missing argument: index
#let numbers = ()
{ numbers.insert() }

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
// Error: 3-31 array index out of bounds (index: 12, len: 10)
{ range(10).slice(9, count: 3) }

---
// Error: 3-25 array index out of bounds (index: -4, len: 3)
{ (1, 2, 3).slice(0, -4) }

---
// Test the `position` method.
#test(("Hi", "❤️", "Love").position(s => s == "❤️"), 1)
#test(("Bye", "💘", "Apart").position(s => s == "❤️"), none)
#test(("A", "B", "CDEF", "G").position(v => v.len() > 2), 2)

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
{(true, false).join()}

---
// Error: 2-20 cannot join string with integer
{("a", "b").join(1)}

---
// Test joining content.
// Ref: true
{([One], [Two], [Three]).join([, ], last: [ and ])}.

---
// Test the `sorted` method.
#test(().sorted(), ())
#test(((true, false) * 10).sorted(), (false,) * 10 + (true,) * 10)
#test(("it", "the", "hi", "text").sorted(), ("hi", "it", "text", "the"))
#test((2, 1, 3, 10, 5, 8, 6, -7, 2).sorted(), (-7, 1, 2, 2, 3, 5, 6, 8, 10))

---
// Error: 2-26 cannot order content and content
{([Hi], [There]).sorted()}

---
// Test dictionary methods.
#let dict = (a: 3, c: 2, b: 1)
#test("c" in dict, true)
#test(dict.len(), 3)
#test(dict.values(), (3, 1, 2))
#test(dict.pairs((k, v) => k + str(v)).join(), "a3b1c2")

{ dict.remove("c") }
#test("c" in dict, false)
#test(dict, (a: 3, b: 1))
