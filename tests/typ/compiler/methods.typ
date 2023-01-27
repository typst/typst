// Test method calls.
// Ref: false

---
// Test whitespace around dot.
#test( "Hi there" . split() , ("Hi", "there"))

---
// Test mutating indexed value.
#{
  let matrix = (((1,), (2,)), ((3,), (4,)))
  matrix.at(1).at(0).push(5)
  test(matrix, (((1,), (2,)), ((3, 5), (4,))))
}

---
// Test multiline chain in code block.
#{
  let rewritten = "Hello. This is a sentence. And one more."
    .split(".")
    .map(s => s.trim())
    .filter(s => s != "")
    .map(s => s + "!")
    .join("\n ")

  test(rewritten, "Hello!\n This is a sentence!\n And one more!")
}

---
// Error: 2:4-2:17 type array has no method `fun`
#let numbers = ()
#{ numbers.fun() }

---
// Error: 2:4-2:45 cannot mutate a temporary value
#let numbers = (1, 2, 3)
#{ numbers.map(v => v / 2).sorted().map(str).remove(4) }

---
// Error: 2:4-2:20 cannot mutate a temporary value
#let numbers = (1, 2, 3)
#{ numbers.sorted() = 1 }

---
// Error: 4-7 cannot mutate a constant
#{ box.push(1) }
