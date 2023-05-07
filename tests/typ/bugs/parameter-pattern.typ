// Test that underscore works in parameter patterns.
// Ref: false

---
#test((1, 2, 3).zip((1, 2, 3)).map(((_, x)) => x), (1, 2, 3))
