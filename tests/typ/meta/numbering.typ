// Test integrated numbering patterns.

---
#for i in range(1, 9) {
  numbering("*", i)
  [ and ]
  numbering("I.a", i, i)
  [ for #i]
  parbreak()
}

---
// Error: 17-18 number must be positive
#numbering("1", 0)

---
// Error: 17-19 number must be positive
#numbering("1", -1)
