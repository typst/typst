// Test integrated numbering patterns.

---
#for i in range(0, 9) {
  numbering("*", i)
  [ and ]
  numbering("I.a", i, i)
  [ for #i]
  parbreak()
}

---
// Error: 17-19 number must be at least zero
#numbering("1", -1)
