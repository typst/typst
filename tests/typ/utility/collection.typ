// Test collection functions.
// Ref: false

---
// Test the `len` method.
#test(().len(), 0)
#test(("A", "B", "C").len(), 3)
#test("Hello World!".len(), 12)
#test((a: 1, b: 2).len(), 2)

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
// Test the `find` method.
#test(("Hi", "❤️", "Love").find("❤️"), 1)
#test(("Bye", "💘", "Apart").find("❤️"), none)

---
// Test the `slice` method.
#test((1, 2, 3, 4).slice(2), (3, 4))
#test(range(10).slice(2, 6), (2, 3, 4, 5))
#test(range(10).slice(4, count: 3), (4, 5, 6))

---
// Error: 3-31 array index out of bounds (index: 12, len: 10)
{ range(10).slice(9, count: 3) }

---
// Error: 2:17-2:19 missing argument: index
#let numbers = ()
{ numbers.insert() }

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
