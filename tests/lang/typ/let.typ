// Automatically initialized with `none`.
#let x
[eq x, none]

// Initialized with `1`.
#let y = 1
[eq y, 1]

// Initialize with template, not terminated by semicolon in template.
#let v = [Hello; there]

// Not terminated by line break due to parens.
#let x = (
    1,
    2,
    3,
)
[eq x, (1, 2, 3)]

// Multiple bindings in one line.
#let x = "a"; #let y = "b"; [eq x + y, "ab"]

// Invalid name.
// Error: 1:6-1:7 expected identifier, found integer
#let 1

// Terminated by end of line before binding name.
// Error: 1:5-1:5 expected identifier
#let
x = 5

// No name at all.
// Error: 1:11-1:11 expected identifier
The Fi#let;rst

// Terminated with just a line break.
#let v = "a"
The Second [eq v, "a"]

// Terminated with semicolon + line break.
#let v = "a";
The Third [eq v, "a"]

// Terminated with just a semicolon.
The#let v = "a"; Fourth [eq v, "a"]

// Terminated by semicolon even though we are in a paren group.
// Error: 2:25-2:25 expected expression
// Error: 1:25-1:25 expected closing paren
The#let array = (1, 2 + ;Fifth [eq array, (1, 2)]

// Not terminated.
// Error: 1:16-1:16 expected semicolon or line break
The#let v = "a"Sixth [eq v, "a"]

// Not terminated.
// Error: 1:16-1:16 expected semicolon or line break
The#let v = "a" [eq v, "a"] Seventh
