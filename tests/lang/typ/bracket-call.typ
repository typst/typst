// Basic call, whitespace insignificant.
#[f], #[ f ], #[
    f
]

#[f bold]

#[f 1,]

#[f a:2]

#[f 1, a: (3, 4), 2, b: "5"]

---
// Body and no body.
#[f][#[f]]

// Lots of potential bodies.
#[f][f]#[f]

// Multi-paragraph body.
#[box][
    First

    Second
]

---
// Chained.
#[f | f]

// Multi-chain.
#[f|f|f]

// With body.
// Error: 1:7-1:8 expected identifier, found integer
#[f | 1 | box][💕]

// Error: 2:3-2:3 expected identifier
// Error: 1:4-1:4 expected identifier
#[||f true]

// Error: 1:7-1:7 expected identifier
#[f 1|]

// Error: 2:3-2:3 expected identifier
// Error: 1:4-1:4 expected identifier
#[|][Nope]

// Error: 2:6-2:6 expected closing paren
// Error: 1:9-1:10 expected expression, found closing paren
#[f (|f )]

// With actual functions.
#[box width: 1cm | image "res/rhino.png"]

---
// Error: 1:5-1:7 expected expression, found end of block comment
#[f */]

// Error: 1:8-1:9 expected expression, found colon
#[f a:1:]

// Error: 1:6-1:6 expected comma
#[f 1 2]

// Error: 2:5-2:6 expected identifier
// Error: 1:7-1:7 expected expression
#[f 1:]

// Error: 1:5-1:6 expected identifier
#[f 1:2]

// Error: 1:5-1:8 expected identifier
#[f (x):1]

---
// Ref: false
// Error: 2:3-2:4 expected function, found string
#let x = "string"
#[x]

// Error: 1:3-1:4 expected identifier, found invalid token
#[# 1]

// Error: 4:1-4:1 expected identifier
// Error: 3:1-3:1 expected closing bracket
#[

---
// Ref: false
// Error: 2:3-2:4 expected identifier, found closing paren
// Error: 3:1-3:1 expected closing bracket
#[)

---
// Error: 3:1-3:1 expected closing bracket
#[f [*]

---
// Error: 3:1-3:1 expected closing bracket
#[f][`a]`

---
// Error: 3:1-3:1 expected quote
// Error: 2:1-2:1 expected closing bracket
#[f "]
