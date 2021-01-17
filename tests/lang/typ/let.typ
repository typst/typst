// Ref: false

// Automatically initialized with `none`.
#let x
[eq x, none]

// Initialized with `1`.
#let y = 1
[eq y, 1]

// Multiple bindings in one line.
#let x = "a"; #let y = "b"; [eq x + y, "ab"]

// No name.
// Error: 1:6-1:7 expected identifier, found integer
#let 1

---
// Terminated with just a line break.
#let v = "a"
First
[eq v, "a"]

// Terminated with just a semicolon.
#let v = "a"; Second
[eq v, "a"]

// Terminated with semicolon + line break.
#let v = "a";
Third
[eq v, "a"]

// Terminated by semicolon even though we are in a paren group.
// Error: 2:22-2:22 expected expression
// Error: 1:22-1:22 expected closing paren
#let array = (1, 2 + ;Fourth
[eq array, (1, 2)]

// Not terminated.
// Error: 1:14-1:20 expected semicolon or line break, found identifier
#let v = "a" Unseen Fifth
[eq v, "a"]

// Not terminated by semicolon in template.
#let v = [Hello; there]

// Not terminated by line break due to parens.
#let x = (
    1,
    2,
    3,
)
[eq x, (1, 2, 3)]
