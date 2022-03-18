// Test method calls.
// Ref: false

---
// Test whitespace around dot.
#test( "Hi there" . split() , ("Hi", "there"))

---
// Test mutating indexed value.
{
  let matrix = (((1,), (2,)), ((3,), (4,)))
  matrix(1)(0).push(5)
  test(matrix, (((1,), (2,)), ((3, 5), (4,))))
}

---
// Test multiline chain in code block.
{
  let rewritten = "Hello. This is a sentence. And one more."
    .split(".")
    .map(s => s.trim())
    .filter(s => s != "")
    .map(s => s + "!")
    .join([\ ])

  test(rewritten, [Hello!\ This is a sentence!\ And one more!])
}

---
// Error: 2:3-2:16 type array has no method `fun`
#let numbers = ()
{ numbers.fun() }

---
// Error: 2:3-2:44 cannot mutate a temporary value
#let numbers = (1, 2, 3)
{ numbers.map(v => v / 2).sorted().map(str).remove(4) }

---
// Error: 2:3-2:19 cannot mutate a temporary value
#let numbers = (1, 2, 3)
{ numbers.sorted() = 1 }

---
// Error: 3-6 cannot mutate a constant
{ box = 1 }

---
// Error: 3-6 cannot mutate a constant
{ box.push(1) }
