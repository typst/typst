// Test parenthesized function calls.
// Ref: false

---
// Whitespace insignificant.
#[test type(1), "integer"]
#[test type (1), "integer"]

// From variable.
#let alias = type
#[test alias(alias), "function"]
