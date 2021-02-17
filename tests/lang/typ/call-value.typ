// Test function calls.
// Ref: false

---
// Whitespace is significant.
#test(type(1), "integer")
#test (type (1), "integer")

// From variable.
#let alias = type
#test(alias(alias), "function")

// Returns template.
#test(type(font(12pt)), "template")
