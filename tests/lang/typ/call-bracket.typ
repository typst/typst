// Test bracketed function calls.

---
// Whitespace is insignificant.
#[ f ]

// Alternatives for embedding.
#[f f()], #[f #[f]], #[f][#[f]],

// Tight functions.
#[f]#[f]

// Multi-paragraph body.
#[align right][
    First

    Second
]

---
// Chained once.
#[f | f]

// Chained twice.
#[f|f|f]

// With body.
// Error: 7-8 expected identifier, found integer
#[f | 1 | box][💕]

// With actual functions.
#[box width: 1cm | image "res/rhino.png"]

---
// Error: 8-8 expected identifier
#[f 1 |]

// Error: 4-4 expected identifier
#[ | f true]

// Error: 2:3-2:3 expected identifier
// Error: 1:4-1:4 expected identifier
#[|][Nope]

// Pipe wins over parens.
// Error: 2:6-2:6 expected closing paren
// Error: 1:9-1:10 expected expression, found closing paren
#[f (|f )]
