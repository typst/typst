// Test integrated numbering patterns.

---
#for i in range(9) {
  numbering(i, "* and ")
  numbering(i, "I")
  [ for #i]
  parbreak()
}

---
// Error: 12-14 must be at least zero
#numbering(-1, "1")
