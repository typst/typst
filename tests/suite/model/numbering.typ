// Test integrated numbering patterns.

--- numbering-symbol-and-roman ---
#for i in range(0, 9) {
  numbering("*", i)
  [ and ]
  numbering("I.a", i, i)
  [ for #i \ ]
}

--- numbering-latin ---
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

--- numbering-greek ---
0 is #numbering("α", 0)\
#for i in range(1, 30) {
  numbering("α", i)
  [ and ]
  numbering("Α", i)
  [ for #i \ ]
}
#for i in ( 2056839184, 5683, 9184, 2000000000 ) {
  numbering("α", i)
  [ for #i \ ]
}

#assert.eq(numbering("α", 1), "αʹ")
#assert.eq(numbering("α", 2056839184), "βΜκʹ, αΜ͵εχπγ, ͵θρπδ")
#assert.eq(numbering("α", 5683), "͵εχπγ")
#assert.eq(numbering("α", 9184), "͵θρπδ")
#assert.eq(numbering("α", 2000000000), "βΜκʹ")

--- numbering-hebrew ---
#set text(lang: "he")
#for i in range(9, 21, step: 2) {
  numbering("א.", i)
  [ עבור #i \ ]
}

--- numbering-chinese ---
#set text(lang: "zh", font: ("Linux Libertine", "Noto Serif CJK SC"))
#for i in range(9, 21, step: 2){
  numbering("一", i)
  [ and ]
  numbering("壹", i)
  [ for #i \ ]
}

--- numbering-japanese-iroha ---
#set text(lang: "ja", font: ("Linux Libertine", "Noto Serif CJK JP"))
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

--- numbering-korean ---
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

--- numbering-japanese-aiueo ---
#set text(lang: "jp", font: ("Linux Libertine", "Noto Serif CJK JP"))
#for i in range(0, 9) {
  numbering("あ", i)
  [ and ]
  numbering("I.あ", i, i)
  [ for #i \ ]
}

#for i in range(0, 9) {
  numbering("ア", i)
  [ and ]
  numbering("I.ア", i, i)
  [ for #i \ ]
}

--- numbering-arabic-indic ---
#assert.eq(numbering("\u{0661}", 1475), "١٤٧٥")
#assert.eq(numbering("\u{06F1}", 1475), "۱۴۷۵")

--- numbering-negative ---
// Error: 17-19 number must be at least zero
#numbering("1", -1)

--- numbering-circled-number ---
#assert.eq(numbering("①", 1), "①")
#assert.eq(numbering("①", 50), "㊿")

--- numbering-double-circled-number ---
#assert.eq(numbering("⓵", 1), "⓵")
#assert.eq(numbering("⓵", 10), "⓾")