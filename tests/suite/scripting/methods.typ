// Test method calls.

--- method-whitespace eval ---
// Test whitespace around dot.
#test( "Hi there" . split() , ("Hi", "there"))

--- method-mutating eval ---
// Test mutating indexed value.
#{
  let matrix = (((1,), (2,)), ((3,), (4,)))
  matrix.at(1).at(0).push(5)
  test(matrix, (((1,), (2,)), ((3, 5), (4,))))
}

--- method-multiline eval ---
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

--- method-unknown eval ---
#let numbers = ()
// Error: 10-13 type array has no method `fun`
#numbers.fun()

--- method-unknown-but-field-exists eval ---
#let l = line(stroke: red)
// Error: 4-10 element line has no method `stroke`
// Hint: 4-10 did you mean to access the field `stroke`?
#l.stroke()

--- method-mutate-on-temporary eval ---
#let numbers = (1, 2, 3)
// Error: 2-43 cannot mutate a temporary value
#numbers.map(v => v / 2).sorted().map(str).remove(4)

--- assign-to-method-invalid eval ---
#let numbers = (1, 2, 3)
// Error: 3-19 cannot mutate a temporary value
#(numbers.sorted() = 1)

--- method-mutate-on-std-constant eval ---
// Error: 2-5 cannot mutate a constant: box
#box.push(1)
