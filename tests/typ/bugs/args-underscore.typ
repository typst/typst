// Test that lone underscore works.
// Ref: false

---
#test((1, 2, 3).map(_ => {}).len(), 3)
