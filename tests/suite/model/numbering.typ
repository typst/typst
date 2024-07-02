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
#assert.eq(numbering("α", 0), "𐆊")

#assert.eq(numbering("α", 1),  "αʹ")
#assert.eq(numbering("α", 2),  "βʹ")
#assert.eq(numbering("α", 3),  "γʹ")
#assert.eq(numbering("α", 4),  "δʹ")
#assert.eq(numbering("α", 5),  "εʹ")
#assert.eq(numbering("α", 6),  "ϛʹ")
#assert.eq(numbering("α", 7),  "ζʹ")
#assert.eq(numbering("α", 8),  "ηʹ")
#assert.eq(numbering("α", 9),  "θʹ")
#assert.eq(numbering("α", 10), "ιʹ")

#assert.eq(numbering("Α", 1),  "Αʹ")
#assert.eq(numbering("Α", 2),  "Βʹ")
#assert.eq(numbering("Α", 3),  "Γʹ")
#assert.eq(numbering("Α", 4),  "Δʹ")
#assert.eq(numbering("Α", 5),  "Εʹ")
#assert.eq(numbering("Α", 6),  "Ϛʹ")
#assert.eq(numbering("Α", 7),  "Ζʹ")
#assert.eq(numbering("Α", 8),  "Ηʹ")
#assert.eq(numbering("Α", 9),  "Θʹ")
#assert.eq(numbering("Α", 10), "Ιʹ")

#assert.eq(numbering("α", 11), "ιαʹ")
#assert.eq(numbering("α", 12), "ιβʹ")
#assert.eq(numbering("α", 13), "ιγʹ")
#assert.eq(numbering("α", 14), "ιδʹ")
#assert.eq(numbering("α", 15), "ιεʹ")
#assert.eq(numbering("α", 16), "ιϛʹ")
#assert.eq(numbering("α", 17), "ιζʹ")
#assert.eq(numbering("α", 18), "ιηʹ")
#assert.eq(numbering("α", 19), "ιθʹ")
#assert.eq(numbering("α", 20), "κʹ")

#assert.eq(numbering("Α", 11), "ΙΑʹ")
#assert.eq(numbering("Α", 12), "ΙΒʹ")
#assert.eq(numbering("Α", 13), "ΙΓʹ")
#assert.eq(numbering("Α", 14), "ΙΔʹ")
#assert.eq(numbering("Α", 15), "ΙΕʹ")
#assert.eq(numbering("Α", 16), "ΙϚʹ")
#assert.eq(numbering("Α", 17), "ΙΖʹ")
#assert.eq(numbering("Α", 18), "ΙΗʹ")
#assert.eq(numbering("Α", 19), "ΙΘʹ")
#assert.eq(numbering("Α", 20), "Κʹ")

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
