// Test bracketed function calls.

---
// Whitespace insignificant.
#[f], #[ f ]

// Body and no body.
#[f][#[f]]

// Tight functions.
#[f]#[f]

// Multi-paragraph body.
#[align right][
    First

    Second
]
