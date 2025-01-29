// Test method calls.

--- method-whitespace ---
// Test whitespace around dot.
#test( "Hi there" . split() , ("Hi", "there"))

--- method-mutating ---
// Test mutating indexed value.
#{
  let matrix = (((1,), (2,)), ((3,), (4,)))
  matrix.at(1).at(0).push(5)
  test(matrix, (((1,), (2,)), ((3, 5), (4,))))
}

--- method-multiline ---
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

--- method-unknown ---
// Error: 2:10-2:13 type array has no method `fun`
#let numbers = ()
#numbers.fun()

--- method-unknown-but-field-exists ---
// Error: 2:4-2:10 element line has no method `stroke`
// Hint: 2:4-2:10 did you mean to access the field `stroke`?
#let l = line(stroke: red)
#l.stroke()

--- method-mutate-on-temporary ---
// Error: 2:2-2:43 cannot mutate a temporary value
#let numbers = (1, 2, 3)
#numbers.map(v => v / 2).sorted().map(str).remove(4)

--- assign-to-method-invalid ---
// Error: 2:3-2:19 cannot mutate a temporary value
#let numbers = (1, 2, 3)
#(numbers.sorted() = 1)

--- method-mutate-on-std-constant ---
// Error: 2-5 cannot mutate a constant: box
#box.push(1)
