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
#for i in range(0, 4) {
  numbering("A", i)
  [ for #i]
  linebreak()
}
#par[...]
#for i in range(26, 30) {
  numbering("A", i)
  [ for #i]
  linebreak()
}
#par[...]
#for i in range(702, 706) {
  numbering("A", i)
  [ for #i]
  linebreak()
}

---
// Error: 17-19 number must be at least zero
#numbering("1", -1)

---
#set text(lang: "he")

#for i in range(9, 21, step: 2) {
  numbering("א.", i)
  [ עבור #i]
  parbreak()
}

---
#for i in range(0, 4) {
  numbering("イ", i)
  [ (or ]
  numbering("い", i)
  [) for #i]
  linebreak()
}
#par[...]
#for i in range(47, 51) {
  numbering("イ", i)
  [ (or ]
  numbering("い", i)
  [) for #i]
  linebreak()
}
#par[...]
#for i in range(2256, 2260) {
  numbering("イ", i)
  [ for #i]
  linebreak()
}
