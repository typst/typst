// Test integrated numbering patterns.

--- numbering ---
#let t(pat: "1", step: 1, ..vals) = {
  let num = 0
  for val in vals.pos() {
    if type(val) == int {
      num = val
    } else {
      test(numbering(pat, num), val)
      num += step
    }
  }
}

// Arabic.
#t(pat: "1", "0", "1", "2", "3", "4", "5", "6", 107, "107", "108")

// Greek.
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

// Symbols.
#t(pat: "*", "-", "*", "†", "‡", "§", "¶", "‖", "**")

// Hebrew.
#t(pat: "א", step: 2, 9, "ט׳", "י״א", "י״ג")

// Chinese.
#t(pat: "一", step: 2, 9, "九", "十一", "十三", "十五", "十七", "十九")
#t(pat: "壹", step: 2, 9, "玖", "拾壹", "拾叁", "拾伍", "拾柒", "拾玖")

// Japanese.
#t(pat: "イ", "-", "イ", "ロ", "ハ", 47, "ス", "イイ", "イロ", "イハ", 2256, "スス", "イイイ")
#t(pat: "い", "-", "い", "ろ", "は", 47, "す", "いい", "いろ", "いは")
#t(pat: "あ", "-", "あ", "い", "う", "え", "お", "か", "き", "く")
#t(pat: "ア", "-", "ア", "イ", "ウ", "エ", "オ", "カ", "キ", "ク")

// Korean.
#t(pat: "가", "-", "가", "나", "다", 47, "다마", "다바", "다사", "다아")
#t(pat: "ㄱ", "-", "ㄱ", "ㄴ", "ㄷ", 47, "ㄷㅁ")

// Arabic Indic.
#t(pat: "\u{0661}", 1475, "١٤٧٥")
#t(pat: "\u{06F1}", 1475, "۱۴۷۵")

// Devanagari.
#t(pat: "\u{0967}", 1, "१")
#t(pat: "\u{0967}", 10, "१०")
#t(pat: "\u{0967}", 123456789, "१२३४५६७८९")

// Bengali.
#t(pat: "\u{09E7}", 1, "১")
#t(pat: "\u{09E7}", 10, "১০")
#t(pat: "\u{09E7}", 123456789, "১২৩৪৫৬৭৮৯")

// Bengali Consonants.
#t(pat: "\u{0995}", 1, "ক")
#t(pat: "\u{0995}", 32, "হ")
#t(pat: "\u{0995}", 32*2 , "কহ")

// Circled number.
#t(pat: "①", 1, "①")
#t(pat: "①", 50, "㊿")

// Double-circled number.
#t(pat: "⓵", 1, "⓵")
#t(pat: "⓵", 10, "⓾")

--- numbering-negative ---
// Error: 17-19 number must be at least zero
#numbering("1", -1)
