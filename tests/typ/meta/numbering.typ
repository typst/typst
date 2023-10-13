// Test integrated numbering patterns.

---
#for i in range(0, 9) {
  numbering("*", i)
  [ and ]
  numbering("I.a", i, i)
  [ for #i \ ]
}

---
#for i in range(0, 4) {
  numbering("A", i)
  [ for #i \ ]
}
... \
#for i in range(26, 30) {
  numbering("A", i)
  [ for #i \ ]
}
... \
#for i in range(702, 706) {
  numbering("A", i)
  [ for #i \ ]
}

---
#set text(lang: "he")
#for i in range(9, 21, step: 2) {
  numbering("א.", i)
  [ עבור #i \ ]
}

---
#set text(lang: "zh", font: ("Linux Libertine", "Noto Serif CJK SC"))
#for i in range(9, 21, step: 2){
  numbering("一", i)
  [ and ]
  numbering("壹", i)
  [ for #i \ ]
}

---
#set text(lang: "jp", font: ("Linux Libertine", "Noto Serif CJK JP"))
#for i in range(0, 4) {
  numbering("イ", i)
  [ (or ]
  numbering("い", i)
  [) for #i \ ]
}
... \
#for i in range(47, 51) {
  numbering("イ", i)
  [ (or ]
  numbering("い", i)
  [) for #i \ ]
}
... \
#for i in range(2256, 2260) {
  numbering("イ", i)
  [ for #i \ ]
}

---
#set text(lang: "ko", font: ("Linux Libertine", "Noto Serif CJK KR"))
#for i in range(0, 4) {
  numbering("가", i)
  [ (or ]
  numbering("ㄱ", i)
  [) for #i \ ]
}
... \
#for i in range(47, 51) {
  numbering("가", i)
  [ (or ]
  numbering("ㄱ", i)
  [) for #i \ ]
}
... \
#for i in range(2256, 2260) {
  numbering("ㄱ", i)
  [ for #i \ ]
}

---
// Error: 17-19 number must be at least zero
#numbering("1", -1)
