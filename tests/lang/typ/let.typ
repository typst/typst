// Automatically initialized with `none`.
#let x
#[test x, none]

// Initialized with `1`.
#let y = 1
#[test y, 1]

// Initialize with template, not terminated by semicolon in template.
#let v = [Hello; there]

// Not terminated by line break due to parens.
#let x = (
    1,
    2,
    3,
)
#[test x, (1, 2, 3)]

// Multiple bindings in one line.
#let x = "a"; #let y = "b"; #[test x + y, "ab"]

// Invalid name.
// Error: 1:6-1:7 expected identifier, found integer
#let 1

// Invalid name.
// Error: 1:6-1:7 expected identifier, found integer
#let 1 = 2

// Missing binding name.
// Error: 1:5-1:5 expected identifier
#let
x = 5

// Missing right-hand side.
// Error: 1:9-1:9 expected expression
#let a =

// No name at all.
// Error: 1:11-1:11 expected identifier
The Fi#let;rst

// Terminated with just a line break.
#let v = "a"
The Second #[test v, "a"]

// Terminated with semicolon + line break.
#let v = "a";
The Third #[test v, "a"]

// Terminated with just a semicolon.
The#let v = "a"; Fourth #[test v, "a"]

// Terminated by semicolon even though we are in a paren group.
// Error: 2:25-2:25 expected expression
// Error: 1:25-1:25 expected closing paren
The#let array = (1, 2 + ;Fifth #[test array, (1, 2)]

// Not terminated.
// Error: 1:16-1:16 expected semicolon or line break
The#let v = "a"Sixth #[test v, "a"]

// Not terminated.
// Error: 1:16-1:16 expected semicolon or line break
The#let v = "a" #[test v, "a"] Seventh
